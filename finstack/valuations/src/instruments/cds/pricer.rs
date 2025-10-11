//! Core CDS pricing engine and helpers.
//!
//! Provides deterministic valuation for Credit Default Swaps (CDS) with
//! support for ISDA-style premium schedules, accrual-on-default, and
//! multiple numerical integration methods for the protection leg.
//!
//! The engine exposes present value calculations for the protection and
//! premium legs, par spread, risky annuity, PV01/CS01, and a simple
//! bootstrapping helper for hazard curves. Heavy numerics are kept here to
//! isolate pricing policy from instrument data shapes.

use crate::instruments::cds::{CreditDefaultSwap, PayReceive};
use finstack_core::currency::Currency;
use finstack_core::dates::utils::add_months;
use finstack_core::dates::{next_cds_date, Date, DayCount};
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::market_data::MarketContext;
use finstack_core::math::{adaptive_simpson, gauss_legendre_integrate};
use finstack_core::money::Money;
use finstack_core::{Error, Result};

/// ISDA 2014 standard constants used by the engine
pub mod isda_constants {

    /// Standard recovery rate for senior unsecured (40%)
    pub const STANDARD_RECOVERY_SENIOR: f64 = 0.40;

    /// Standard recovery rate for subordinated (20%)
    pub const STANDARD_RECOVERY_SUB: f64 = 0.20;

    /// Standard integration points per year for protection leg
    pub const STANDARD_INTEGRATION_POINTS: usize = 40;

    /// Standard coupon payment day
    pub const STANDARD_COUPON_DAY: u8 = 20;

    /// Tolerance for numerical calculations
    pub const NUMERICAL_TOLERANCE: f64 = 1e-10;

    /// Business days per year for North America (US markets)
    pub const BUSINESS_DAYS_PER_YEAR_US: f64 = 252.0;

    /// Business days per year for Europe (UK markets)
    pub const BUSINESS_DAYS_PER_YEAR_UK: f64 = 250.0;

    /// Business days per year for Asia (Japan markets)
    pub const BUSINESS_DAYS_PER_YEAR_JP: f64 = 255.0;
}

/// Numerical integration method for protection leg
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationMethod {
    /// Simple midpoint rule with fixed steps (non-ISDA)
    Midpoint,
    /// Gaussian quadrature for higher accuracy
    GaussianQuadrature,
    /// Adaptive Simpson's rule
    AdaptiveSimpson,
    /// ISDA standard integration with exact points
    IsdaExact,
}

/// Configuration for CDS pricing
#[derive(Clone, Debug)]
pub struct CDSPricerConfig {
    /// Number of integration steps per year for protection leg (used with Midpoint method)
    pub steps_per_year: usize,
    /// Include accrual on default
    pub include_accrual: bool,
    /// Use exact day count fractions
    pub exact_daycount: bool,
    /// Tolerance for iterative calculations
    pub tolerance: f64,
    /// Integration method for protection leg calculation
    pub integration_method: IntegrationMethod,
    /// Use ISDA standard coupon dates (20th of Mar/Jun/Sep/Dec)
    pub use_isda_coupon_dates: bool,
    /// If true, compute par spread using full premium leg (incl. AoD) with 1bp spread.
    /// If false, use risky annuity (market-standard approximation).
    pub par_spread_uses_full_premium: bool,
    /// Gauss–Legendre order for GaussianQuadrature method (supported: 2,4,8,16)
    pub gl_order: usize,
    /// Maximum recursion depth for AdaptiveSimpson integration
    pub adaptive_max_depth: usize,
    /// Business days per year for settlement delay calculations (region-specific).
    /// Default: 252 (US), alternatives: 250 (UK), 255 (Japan)
    pub business_days_per_year: f64,
}

impl Default for CDSPricerConfig {
    fn default() -> Self {
        Self::isda_standard()
    }
}

impl CDSPricerConfig {
    /// Create an ISDA 2014 standard compliant configuration (North America/US market)
    pub fn isda_standard() -> Self {
        Self {
            steps_per_year: isda_constants::STANDARD_INTEGRATION_POINTS,
            include_accrual: true,
            exact_daycount: true,
            tolerance: isda_constants::NUMERICAL_TOLERANCE,
            integration_method: IntegrationMethod::IsdaExact,
            use_isda_coupon_dates: true,
            par_spread_uses_full_premium: false,
            gl_order: 8,
            adaptive_max_depth: 12,
            business_days_per_year: isda_constants::BUSINESS_DAYS_PER_YEAR_US,
        }
    }

    /// Create an ISDA configuration for European markets (UK conventions)
    pub fn isda_europe() -> Self {
        Self {
            business_days_per_year: isda_constants::BUSINESS_DAYS_PER_YEAR_UK,
            ..Self::isda_standard()
        }
    }

    /// Create an ISDA configuration for Asian markets (Japan conventions)
    pub fn isda_asia() -> Self {
        Self {
            business_days_per_year: isda_constants::BUSINESS_DAYS_PER_YEAR_JP,
            ..Self::isda_standard()
        }
    }

    /// Create a simplified configuration for faster but less accurate pricing
    pub fn simplified() -> Self {
        Self {
            steps_per_year: 365,
            include_accrual: true,
            exact_daycount: false,
            tolerance: 1e-7,
            integration_method: IntegrationMethod::Midpoint,
            use_isda_coupon_dates: false,
            par_spread_uses_full_premium: false,
            gl_order: 4,
            adaptive_max_depth: 10,
            business_days_per_year: isda_constants::BUSINESS_DAYS_PER_YEAR_US,
        }
    }
}

/// CDS pricing engine. Stateless wrapper carrying configuration.
pub struct CDSPricer {
    config: CDSPricerConfig,
}

impl Default for CDSPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSPricer {
    /// Create new pricer with default ISDA-compliant config
    pub fn new() -> Self {
        Self {
            config: CDSPricerConfig::default(),
        }
    }

    /// Create pricer with custom config
    pub fn with_config(config: CDSPricerConfig) -> Self {
        Self { config }
    }

    /// Calculate PV of protection leg with numerical integration
    pub fn pv_protection_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        _as_of: Date,
    ) -> Result<Money> {
        let base_date = disc.base_date();
        let t_start = self.year_fraction(base_date, cds.premium.start, cds.premium.dc)?;
        let t_end = self.year_fraction(base_date, cds.premium.end, cds.premium.dc)?;
        let recovery = cds.protection.recovery_rate;
        let delay_years =
            (cds.protection.settlement_delay as f64) / self.config.business_days_per_year;

        let protection_pv = match self.config.integration_method {
            IntegrationMethod::Midpoint => {
                self.protection_leg_midpoint(t_start, t_end, recovery, delay_years, disc, surv)?
            }
            IntegrationMethod::GaussianQuadrature => match self.protection_leg_gaussian_quadrature(
                t_start,
                t_end,
                recovery,
                delay_years,
                disc,
                surv,
            ) {
                Ok(pv) => pv,
                Err(_) => {
                    self.protection_leg_midpoint(t_start, t_end, recovery, delay_years, disc, surv)?
                }
            },
            IntegrationMethod::AdaptiveSimpson => match self.protection_leg_adaptive_simpson(
                t_start,
                t_end,
                recovery,
                delay_years,
                disc,
                surv,
            ) {
                Ok(pv) => pv,
                Err(_) => {
                    self.protection_leg_midpoint(t_start, t_end, recovery, delay_years, disc, surv)?
                }
            },
            IntegrationMethod::IsdaExact => {
                self.protection_leg_isda_exact(t_start, t_end, recovery, delay_years, disc, surv)?
            }
        };

        Ok(Money::new(
            protection_pv * cds.notional.amount(),
            cds.notional.currency(),
        ))
    }

    /// Calculate PV of premium leg with optional accrual-on-default
    pub fn pv_premium_leg(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let base_date = disc.base_date();
        let schedule = self.generate_schedule(cds, as_of)?;

        let mut premium_pv = 0.0;
        let spread = cds.premium.spread_bp * 1e-4;

        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];

            let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
            let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;

            let sp = surv.sp(t_end);
            let df = disc.df(t_end);

            premium_pv += spread * accrual * sp * df;

            if self.config.include_accrual {
                premium_pv += self.calculate_accrual_on_default(
                    spread,
                    self.year_fraction(base_date, start_date, cds.premium.dc)?,
                    t_end,
                    disc,
                    surv,
                )?;
            }
        }

        Ok(Money::new(
            premium_pv * cds.notional.amount(),
            cds.notional.currency(),
        ))
    }

    /// Calculate accrual-on-default for a period using configured method
    fn calculate_accrual_on_default(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let period_length = t_end - t_start;
        match self.config.integration_method {
            IntegrationMethod::Midpoint => {
                self.accrual_on_default_midpoint(spread, t_start, t_end, period_length, disc, surv)
            }
            IntegrationMethod::GaussianQuadrature | IntegrationMethod::AdaptiveSimpson => {
                match self.accrual_on_default_adaptive(
                    spread,
                    t_start,
                    t_end,
                    period_length,
                    disc,
                    surv,
                ) {
                    Ok(aod) => Ok(aod),
                    Err(_) => self.accrual_on_default_midpoint(
                        spread,
                        t_start,
                        t_end,
                        period_length,
                        disc,
                        surv,
                    ),
                }
            }
            IntegrationMethod::IsdaExact => self.accrual_on_default_isda_exact(
                spread,
                t_start,
                t_end,
                period_length,
                disc,
                surv,
            ),
        }
    }

    fn accrual_on_default_midpoint(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let num_steps = (period_length * self.config.steps_per_year as f64).ceil() as usize;
        let dt = period_length / num_steps as f64;
        let mut accrual_pv = 0.0;
        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let default_prob = surv.sp(t1) - surv.sp(t2);
            let t_default = (t1 + t2) * 0.5;
            let accrued_time = t_default - t_start;
            let df = disc.df(t_default);
            let accrual = spread * accrued_time;
            accrual_pv += accrual * default_prob * df;
        }
        Ok(accrual_pv)
    }

    fn accrual_on_default_adaptive(
        &self,
        spread: f64,
        t_start: f64,
        t_end: f64,
        _period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end || spread < 0.0 {
            return Err(Error::Internal);
        }
        let h = (t_end - t_start) * 1e-4;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let accrued_time = (t - t_start).max(0.0);
            let df = disc.df(t);
            spread * accrued_time * density * df
        };
        adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    fn accrual_on_default_isda_exact(
        &self,
        spread: f64,
        t_start: f64,
        _t_end: f64,
        period_length: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let steps = isda_constants::STANDARD_INTEGRATION_POINTS;
        let dt = period_length / steps as f64;
        let mut accrual_pv = 0.0;
        for i in 0..steps {
            let t = t_start + (i as f64 + 0.5) * dt;
            let accrual_fraction = (t - t_start) / period_length;
            let t1 = t - dt * 0.5;
            let t2 = t + dt * 0.5;
            let sp1 = if t1 >= 0.0 { surv.sp(t1) } else { 1.0 };
            let sp2 = surv.sp(t2);
            let default_prob = if sp1 > 0.0 && sp2 < sp1 {
                (sp1 - sp2) / dt
            } else {
                0.0
            };
            let df = disc.df(t);
            accrual_pv += spread * accrual_fraction * default_prob * df * dt;
        }
        Ok(accrual_pv)
    }

    fn protection_leg_midpoint(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        let num_steps = ((t_end - t_start) * self.config.steps_per_year as f64).ceil() as usize;
        let dt = (t_end - t_start) / num_steps as f64;
        let mut protection_pv = 0.0;
        for i in 0..num_steps {
            let t1 = t_start + i as f64 * dt;
            let t2 = t_start + (i + 1) as f64 * dt;
            let t_mid = (t1 + t2) * 0.5;
            let default_prob = surv.sp(t1) - surv.sp(t2);
            let df = disc.df(t_mid + delay_years);
            protection_pv += (1.0 - recovery) * default_prob * df;
        }
        Ok(protection_pv)
    }

    fn protection_leg_gaussian_quadrature(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end || !(0.0..=1.0).contains(&recovery) {
            return Err(Error::Internal);
        }
        let h = (t_end - t_start) * 1e-4;
        let lgd = 1.0 - recovery;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let df = disc.df(t + delay_years);
            lgd * density * df
        };
        gauss_legendre_integrate(integrand, t_start, t_end, self.config.gl_order)
    }

    fn protection_leg_adaptive_simpson(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end || !(0.0..=1.0).contains(&recovery) {
            return Err(Error::Internal);
        }
        let h = (t_end - t_start) * 1e-4;
        let lgd = 1.0 - recovery;
        let integrand = |t: f64| {
            let density = approx_default_density(surv, t, h, t_start, t_end);
            let df = disc.df(t + delay_years);
            lgd * density * df
        };
        adaptive_simpson(
            integrand,
            t_start,
            t_end,
            self.config.tolerance,
            self.config.adaptive_max_depth,
        )
    }

    fn protection_leg_isda_exact(
        &self,
        t_start: f64,
        t_end: f64,
        recovery: f64,
        delay_years: f64,
        disc: &dyn Discounting,
        surv: &dyn Survival,
    ) -> Result<f64> {
        if t_start >= t_end || !(0.0..=1.0).contains(&recovery) {
            return Err(Error::Internal);
        }
        let lgd = 1.0 - recovery;
        let steps_per_period = isda_constants::STANDARD_INTEGRATION_POINTS;
        let dt = (t_end - t_start) / steps_per_period as f64;
        let mut integral = 0.0;
        for i in 0..steps_per_period {
            let t1 = t_start + i as f64 * dt;
            let t2 = t1 + dt;
            let sp1 = surv.sp(t1);
            let sp2 = surv.sp(t2);
            if sp1 > sp2 && sp1 > 0.0 {
                let hazard_rate = -(sp2 / sp1).ln() / dt;
                let avg_t = (t1 + t2) * 0.5;
                let df_mid = disc.df(avg_t + delay_years);
                if hazard_rate.abs() > 1e-10 {
                    integral += (sp1 - sp2) * df_mid;
                } else {
                    let sp_mid = (sp1 + sp2) * 0.5;
                    integral += sp_mid * df_mid * hazard_rate * dt;
                }
            }
        }
        Ok(integral * lgd)
    }

    /// Generate payment schedule for CDS with ISDA standard dates support
    pub fn generate_schedule(&self, cds: &CreditDefaultSwap, _as_of: Date) -> Result<Vec<Date>> {
        if self.config.use_isda_coupon_dates {
            self.generate_isda_schedule(cds)
        } else {
            let sched = crate::cashflow::builder::build_dates(
                cds.premium.start,
                cds.premium.end,
                cds.premium.freq,
                cds.premium.stub,
                cds.premium.bdc,
                cds.premium.calendar_id.as_deref(),
            );
            Ok(sched.dates)
        }
    }

    /// Generate ISDA standard coupon dates (20th of Mar/Jun/Sep/Dec)
    pub fn generate_isda_schedule(&self, cds: &CreditDefaultSwap) -> Result<Vec<Date>> {
        let mut schedule = vec![cds.premium.start];
        let mut current = cds.premium.start;
        while current < cds.premium.end {
            current = next_cds_date(current);
            if current <= cds.premium.end {
                schedule.push(current);
            }
        }
        if schedule.last() != Some(&cds.premium.end) {
            schedule.push(cds.premium.end);
        }
        Ok(schedule)
    }

    /// Calculate par spread (bps) that sets NPV to zero
    pub fn par_spread(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;
        let denom = if self.config.par_spread_uses_full_premium {
            // Compute premium PV per 1bp including AoD
            let base_date = disc.base_date();
            let schedule = self.generate_schedule(cds, as_of)?;
            let mut ann = 0.0;
            for i in 0..schedule.len() - 1 {
                let start_date = schedule[i];
                let end_date = schedule[i + 1];
                let t_start = self.year_fraction(base_date, start_date, cds.premium.dc)?;
                let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
                let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;
                let sp = surv.sp(t_end);
                let df = disc.df(t_end);
                let per_bp = 1e-4;
                // coupon part per bp
                ann += per_bp * accrual * sp * df;
                // AoD part per bp in this period
                ann += self.calculate_accrual_on_default(per_bp, t_start, t_end, disc, surv)?;
            }
            ann
        } else {
            self.risky_annuity(cds, disc, surv, as_of)?
        };
        if denom.abs() < 1e-12 {
            return Err(finstack_core::Error::Internal);
        }
        Ok(protection_pv.amount() / (denom * cds.notional.amount()) * 10000.0)
    }

    /// Premium leg PV per 1 bp of spread, including accrual-on-default if configured.
    pub fn premium_leg_pv_per_bp(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let base_date = disc.base_date();
        let schedule = self.generate_schedule(cds, as_of)?;
        let mut per_bp_pv = 0.0;
        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];
            let t_start = self.year_fraction(base_date, start_date, cds.premium.dc)?;
            let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
            let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;
            let sp = surv.sp(t_end);
            let df = disc.df(t_end);
            let per_bp = 1e-4;
            per_bp_pv += per_bp * accrual * sp * df;
            if self.config.include_accrual {
                per_bp_pv +=
                    self.calculate_accrual_on_default(per_bp, t_start, t_end, disc, surv)?;
            }
        }
        Ok(per_bp_pv)
    }

    /// Risky annuity: PV of $1 paid on premium leg (per bp)
    pub fn risky_annuity(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let base_date = disc.base_date();
        let schedule = self.generate_schedule(cds, as_of)?;
        let mut annuity = 0.0;
        for i in 0..schedule.len() - 1 {
            let start_date = schedule[i];
            let end_date = schedule[i + 1];
            let t_end = self.year_fraction(base_date, end_date, cds.premium.dc)?;
            let accrual = self.year_fraction(start_date, end_date, cds.premium.dc)?;
            let sp = surv.sp(t_end);
            let df = disc.df(t_end);
            annuity += accrual * sp * df;
        }
        Ok(annuity)
    }

    /// Risky PV01: change in PV for 1bp change in spread
    pub fn risky_pv01(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<f64> {
        let risky_annuity = self.risky_annuity(cds, disc, surv, as_of)?;
        Ok(risky_annuity * cds.notional.amount() / 10000.0)
    }

    /// CS01 via risky PV01 approximation
    pub fn cs01(
        &self,
        cds: &CreditDefaultSwap,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
        let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
        let base_npv = self.npv(cds, disc, surv, as_of)?;
        let risky_pv01 = self.risky_pv01(cds, disc, surv, as_of)?;
        let bumped_npv = Money::new(risky_pv01, cds.notional.currency());
        Ok((bumped_npv.amount() - base_npv.amount()).abs())
    }

    /// Instrument NPV from the perspective of `PayReceive`
    pub fn npv(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;
        let premium_pv = self.pv_premium_leg(cds, disc, surv, as_of)?;
        match cds.side {
            PayReceive::PayFixed => protection_pv.checked_sub(premium_pv),
            PayReceive::ReceiveFixed => premium_pv.checked_sub(protection_pv),
        }
    }

    /// Instrument NPV including upfront override when provided.
    pub fn npv_with_upfront(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        surv: &dyn Survival,
        as_of: Date,
    ) -> Result<Money> {
        let mut pv = self.npv(cds, disc, surv, as_of)?;
        if let Some(upfront) = cds.pricing_overrides.upfront_payment {
            pv = (pv + upfront)?;
        }
        Ok(pv)
    }

    /// Resolve curves from MarketContext and compute NPV with upfront.
    pub fn npv_market(
        &self,
        cds: &CreditDefaultSwap,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let disc = curves.get_discount_ref(&cds.premium.disc_id)?;
        let surv = curves.get_hazard_ref(&cds.protection.credit_id)?;
        self.npv_with_upfront(cds, disc, surv, as_of)
    }

    /// Year fraction helper honoring exact day-count if configured
    fn year_fraction(&self, start: Date, end: Date, dc: DayCount) -> Result<f64> {
        if self.config.exact_daycount {
            dc.year_fraction(start, end, finstack_core::dates::DayCountCtx::default())
        } else {
            // Fallback: approximate using ACT/365F to avoid local constants
            DayCount::Act365F.year_fraction(
                start,
                end,
                finstack_core::dates::DayCountCtx::default(),
            )
        }
    }
}

// ----- Local helpers -----
#[inline]
fn approx_default_density(surv: &dyn Survival, t: f64, h: f64, t_start: f64, t_end: f64) -> f64 {
    // Finite-difference approximation of -dS/dt, clipped to [t_start, t_end]
    let hh = if h <= 0.0 {
        (t_end - t_start) * 1e-4
    } else {
        h
    };
    let (s_left, s_right, denom) = if t <= t_start + hh {
        (surv.sp(t), surv.sp((t + hh).min(t_end)), hh)
    } else if t >= t_end - hh {
        (surv.sp((t - hh).max(t_start)), surv.sp(t), hh)
    } else {
        (surv.sp(t - hh), surv.sp(t + hh), 2.0 * hh)
    };
    let deriv = (s_right - s_left) / denom; // ≈ dS/dt
    (-deriv).max(0.0)
}

/// Bootstrap hazard rates from CDS spreads to a simple hazard curve
pub struct CDSBootstrapper {
    config: CDSPricerConfig,
}

impl Default for CDSBootstrapper {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSBootstrapper {
    /// Create new bootstrapper with default config
    pub fn new() -> Self {
        Self {
            config: CDSPricerConfig::default(),
        }
    }

    /// Bootstrap hazard curve from CDS spreads (tenor years, spread bps)
    pub fn bootstrap_hazard_curve(
        &self,
        cds_spreads: &[(f64, f64)],
        recovery_rate: f64,
        disc: &dyn Discounting,
        base_date: Date,
    ) -> Result<HazardCurve> {
        let mut hazard_rates = Vec::new();
        let mut par_spreads = Vec::new();
        let pricer = CDSPricer::with_config(self.config.clone());

        for &(tenor, spread_bps) in cds_spreads {
            let cds = self.create_synthetic_cds(base_date, tenor, spread_bps, recovery_rate)?;
            let hazard_rate = self.solve_for_hazard_rate(&cds, disc, spread_bps, &pricer)?;
            hazard_rates.push((tenor, hazard_rate));
            par_spreads.push((tenor, spread_bps));
        }

        HazardCurve::builder("BOOTSTRAPPED")
            .base_date(base_date)
            .knots(hazard_rates)
            .recovery_rate(recovery_rate)
            .par_spreads(par_spreads)
            .build()
    }

    fn create_synthetic_cds(
        &self,
        base_date: Date,
        tenor_years: f64,
        spread_bps: f64,
        recovery_rate: f64,
    ) -> Result<CreditDefaultSwap> {
        let months = (tenor_years * 12.0).round() as i32;
        let end_date = add_months(base_date, months);
        Ok(CreditDefaultSwap::new_isda(
            finstack_core::types::InstrumentId::new(format!("SYNTHETIC_{:.1}Y", tenor_years)),
            Money::new(1_000_000.0, Currency::USD),
            PayReceive::PayFixed,
            crate::instruments::cds::CDSConvention::IsdaNa,
            spread_bps,
            base_date,
            end_date,
            recovery_rate,
            finstack_core::types::CurveId::new("DISC"),
            finstack_core::types::CurveId::new("CREDIT"),
        ))
    }

    fn solve_for_hazard_rate(
        &self,
        cds: &CreditDefaultSwap,
        disc: &dyn Discounting,
        target_spread_bps: f64,
        pricer: &CDSPricer,
    ) -> Result<f64> {
        let mut hazard_rate = target_spread_bps / 10000.0 / (1.0 - cds.protection.recovery_rate);
        for _ in 0..20 {
            let surv = self.create_flat_hazard_curve(hazard_rate, cds)?;
            let calculated_spread = pricer.par_spread(cds, disc, &surv, disc.base_date())?;
            let error = calculated_spread - target_spread_bps;
            if error.abs() < self.config.tolerance {
                return Ok(hazard_rate);
            }
            let bump = 0.0001;
            let surv_bumped = self.create_flat_hazard_curve(hazard_rate + bump, cds)?;
            let spread_bumped = pricer.par_spread(cds, disc, &surv_bumped, disc.base_date())?;
            let derivative = (spread_bumped - calculated_spread) / bump;
            if derivative.abs() < 1e-10 {
                return Err(Error::Internal);
            }
            hazard_rate -= error / derivative;
            hazard_rate = hazard_rate.clamp(0.0001, 0.5);
        }
        Err(Error::Internal)
    }

    fn create_flat_hazard_curve(
        &self,
        hazard_rate: f64,
        cds: &CreditDefaultSwap,
    ) -> Result<HazardCurve> {
        HazardCurve::builder("TEMP")
            .base_date(cds.premium.start)
            .recovery_rate(cds.protection.recovery_rate)
            .knots(vec![(1.0, hazard_rate), (10.0, hazard_rate)])
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::utils::add_months;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

    fn create_test_cds(
        id: impl Into<String>,
        start_date: Date,
        end_date: Date,
        spread_bp: f64,
        recovery_rate: f64,
    ) -> CreditDefaultSwap {
        CreditDefaultSwap::new_isda(
            finstack_core::types::InstrumentId::new(id),
            Money::new(10_000_000.0, Currency::USD),
            PayReceive::PayFixed,
            crate::instruments::cds::CDSConvention::IsdaNa,
            spread_bp,
            start_date,
            end_date,
            recovery_rate,
            finstack_core::types::CurveId::new("USD-OIS"),
            finstack_core::types::CurveId::new("TEST-CREDIT"),
        )
    }

    fn create_test_curves() -> (DiscountCurve, HazardCurve) {
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots(vec![(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.65)])
            .build()
            .unwrap();

        let credit = HazardCurve::builder("TEST-CREDIT")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.02), (3.0, 0.03), (5.0, 0.04), (10.0, 0.05)])
            .par_spreads(vec![
                (1.0, 100.0),
                (3.0, 150.0),
                (5.0, 200.0),
                (10.0, 250.0),
            ])
            .build()
            .unwrap();

        (disc, credit)
    }

    #[test]
    fn test_enhanced_protection_leg() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let cds = create_test_cds("TEST-CDS", as_of, add_months(as_of, 60), 100.0, 0.40);
        let pricer = CDSPricer::new();
        let protection_pv = pricer
            .pv_protection_leg(&cds, &disc, &credit, as_of)
            .unwrap();
        assert!(protection_pv.amount() > 0.0);
    }

    #[test]
    fn test_accrual_on_default() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let cds = create_test_cds("TEST-CDS", as_of, add_months(as_of, 60), 100.0, 0.40);
        let pricer_with = CDSPricer::new();
        let pricer_without = CDSPricer::with_config(CDSPricerConfig {
            include_accrual: false,
            integration_method: IntegrationMethod::Midpoint,
            ..Default::default()
        });
        let pv_with = pricer_with
            .pv_premium_leg(&cds, &disc, &credit, as_of)
            .unwrap();
        let pv_without = pricer_without
            .pv_premium_leg(&cds, &disc, &credit, as_of)
            .unwrap();
        assert!(pv_with.amount() > pv_without.amount());
    }

    #[test]
    fn test_par_spread_calculation() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let cds = create_test_cds("TEST-CDS", as_of, add_months(as_of, 60), 0.0, 0.40);
        let pricer = CDSPricer::new();
        let par_spread = pricer.par_spread(&cds, &disc, &credit, as_of).unwrap();
        assert!(par_spread > 0.0 && par_spread < 2000.0);
        let mut cds_at_par = cds.clone();
        cds_at_par.premium.spread_bp = par_spread;
        let npv = pricer.npv(&cds_at_par, &disc, &credit, as_of).unwrap();
        assert!(npv.amount().abs() < 15000.0);
    }

    #[test]
    fn test_settlement_delay_reduces_protection_pv() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let mut cds0 = create_test_cds("CDS-0D", as_of, add_months(as_of, 60), 100.0, 0.40);
        let mut cds20 = cds0.clone();
        cds0.protection.settlement_delay = 0;
        cds20.protection.settlement_delay = 20;
        let pricer = CDSPricer::with_config(CDSPricerConfig {
            integration_method: IntegrationMethod::GaussianQuadrature,
            ..Default::default()
        });
        let pv0 = pricer
            .pv_protection_leg(&cds0, &disc, &credit, as_of)
            .unwrap()
            .amount();
        let pv20 = pricer
            .pv_protection_leg(&cds20, &disc, &credit, as_of)
            .unwrap()
            .amount();
        assert!(pv20 < pv0);
    }

    #[test]
    fn test_par_spread_full_premium_option_runs() {
        let (disc, credit) = create_test_curves();
        let as_of = Date::from_calendar_date(2025, time::Month::January, 1).unwrap();
        let cds = create_test_cds("CDS-PAR", as_of, add_months(as_of, 60), 0.0, 0.40);
        let pricer_ra = CDSPricer::new();
        let pricer_full = CDSPricer::with_config(CDSPricerConfig {
            par_spread_uses_full_premium: true,
            ..Default::default()
        });
        let s1 = pricer_ra.par_spread(&cds, &disc, &credit, as_of).unwrap();
        let s2 = pricer_full.par_spread(&cds, &disc, &credit, as_of).unwrap();
        assert!(s1.is_finite() && s2.is_finite());
    }
}
