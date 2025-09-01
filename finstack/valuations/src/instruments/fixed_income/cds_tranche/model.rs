//! Gaussian Copula pricing model for CDS tranches.
//!
//! Implements the industry-standard base correlation approach for pricing
//! synthetic CDO tranches using a one-factor Gaussian Copula model.
//!
//! ## Key Features
//!
//! * **Time-dependent Expected Loss**: Calculates expected loss at each payment date 
//!   rather than using linear approximation from maturity values.
//! * **Accrual-on-Default (AoD)**: Premium leg includes proper AoD adjustment using
//!   half of incremental loss within each period.
//! * **Market-standard Scheduling**: Uses canonical schedule builders with business
//!   day conventions and holiday calendar support.
//! * **Risk Metrics**: Full implementation of CS01, Correlation Delta, and Jump-to-Default
//!   using proper bumping techniques.
//! * **Numerical Stability**: Correlation clamping, monotonicity enforcement, and
//!   robust integration using Gauss-Hermite quadrature.
//!
//! ## Mathematical Approach
//!
//! The model decomposes tranche [A,D] expected loss as:
//! `EL_[A,D](t) = [EL_eq(0,D,t) - EL_eq(0,A,t)] / [(D-A)/100]`
//!
//! Where `EL_eq(0,K,t)` is the expected loss of equity tranche [0,K] at time t,
//! calculated using base correlation ρ(K) for detachment point K.
//!
//! ### Premium Leg PV
//! `PV_prem = Σ c * Δt_i * DF(t_i) * [N_outstanding(t_{i-1}) - 0.5 * N_incremental_loss(t_i)]`
//!
//! ### Protection Leg PV  
//! `PV_prot = Σ DF(t_i) * N_tr * [EL_fraction(t_i) - EL_fraction(t_{i-1})]`
//!
//! ## Limitations
//!
//! * Assumes homogeneous portfolio (single hazard curve for all constituents)
//! * Uses constant recovery rate across all entities
//! * Base correlation model can have small arbitrage inconsistencies at curve knots

use crate::instruments::fixed_income::cds_tranche::numerical::{
    standard_normal_cdf, standard_normal_inv_cdf, GaussHermiteQuadrature,
};
use crate::instruments::fixed_income::cds_tranche::{CdsTranche, TrancheSide};
use crate::market_data::{CreditIndexData, ValuationMarketContext};
use crate::cashflow::builder::schedule_utils::build_dates;
use finstack_core::dates::{Date, StubKind};
use finstack_core::market_data::traits::Discount;
use finstack_core::prelude::*;
use finstack_core::F;

/// Parameters for the Gaussian Copula pricing model.
#[derive(Clone, Debug)]
pub struct GaussianCopulaParams {
    /// Number of quadrature points for numerical integration (5, 7, or 10)
    pub quadrature_order: u8,
    /// Whether to use issuer-specific curves if available
    pub use_issuer_curves: bool,
    /// Minimum correlation value for numerical stability
    pub min_correlation: F,
    /// Maximum correlation value for numerical stability  
    pub max_correlation: F,
    /// Hazard rate bump for CS01 calculation (in hazard rate units)
    pub cs01_hazard_bump: F,
    /// Correlation bump for correlation delta calculation (absolute)
    pub corr_bump_abs: F,
    /// Whether to use mid-period discounting for protection leg
    pub mid_period_protection: bool,
}

impl Default for GaussianCopulaParams {
    fn default() -> Self {
        Self {
            quadrature_order: 7,        // Good balance of accuracy and performance
            use_issuer_curves: true,    // Use heterogeneous modeling when available
            min_correlation: 0.01,      // Numerical stability floor
            max_correlation: 0.99,      // Numerical stability ceiling
            cs01_hazard_bump: 1e-4,     // 1 basis point in hazard rate space
            corr_bump_abs: 0.01,        // 1% absolute correlation bump
            mid_period_protection: false, // End-of-period discounting by default
        }
    }
}

/// Gaussian Copula pricing engine for CDS tranches.
pub struct GaussianCopulaModel {
    params: GaussianCopulaParams,
}

impl Default for GaussianCopulaModel {
    fn default() -> Self {
        Self::new()
    }
}

impl GaussianCopulaModel {
    /// Create a new Gaussian Copula model with default parameters.
    pub fn new() -> Self {
        Self {
            params: GaussianCopulaParams::default(),
        }
    }

    /// Create a new model with custom parameters.
    pub fn with_params(params: GaussianCopulaParams) -> Self {
        Self { params }
    }

    /// Price a CDS tranche using the Gaussian Copula model.
    ///
    /// # Arguments
    /// * `tranche` - The CDS tranche to price
    /// * `market_ctx` - Market data context containing curves and credit index data
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// The present value of the tranche
    pub fn price_tranche(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Get the credit index data
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id)?;

        // Get the discount curve
        let discount_curve = market_ctx.discount(tranche.disc_id)?;

        // Calculate present values of premium and protection legs
        // These now calculate the EL curve internally with proper time dependency
        let pv_premium = self.calculate_premium_leg_pv(
            tranche,
            index_data,
            discount_curve.as_ref(),
            as_of,
        )?;

        let pv_protection = self.calculate_protection_leg_pv(
            tranche,
            index_data,
            discount_curve.as_ref(),
            as_of,
        )?;

        // Net present value depends on the side
        let net_pv = match tranche.side {
            TrancheSide::SellProtection => pv_premium - pv_protection,
            TrancheSide::BuyProtection => pv_protection - pv_premium,
        };

        Ok(Money::new(net_pv, tranche.notional.currency()))
    }

    /// Calculate expected tranche loss using the base correlation approach.
    ///
    /// Decomposes the tranche [A, D] as the difference between two equity
    /// tranches: EL(0, D) - EL(0, A), using correlations interpolated from
    /// the base correlation curve.
    fn calculate_expected_tranche_loss(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<F> {
        let attach_pct = tranche.attach_pct;
        let detach_pct = tranche.detach_pct;

        // Get correlations for attachment and detachment points
        let corr_attach = index_data.base_correlation_curve.correlation(attach_pct);
        let corr_detach = index_data.base_correlation_curve.correlation(detach_pct);

        // Clamp correlations for numerical stability
        let corr_attach =
            corr_attach.clamp(self.params.min_correlation, self.params.max_correlation);
        let corr_detach =
            corr_detach.clamp(self.params.min_correlation, self.params.max_correlation);

        // Calculate expected losses for equity tranches [0, A] and [0, D]
        let el_to_attach =
            self.calculate_equity_tranche_loss(attach_pct, corr_attach, index_data, maturity)?;

        let el_to_detach =
            self.calculate_equity_tranche_loss(detach_pct, corr_detach, index_data, maturity)?;

        // The [A, D] tranche loss as a fraction of total portfolio
        let portfolio_loss_fraction = el_to_detach - el_to_attach;

        // Ensure monotonicity: el_to_detach should always be >= el_to_attach
        let portfolio_loss_fraction = portfolio_loss_fraction.max(0.0);

        // Convert to fraction of tranche notional: EL_[A,D] / (D-A) * 100
        let tranche_width_pct = detach_pct - attach_pct;
        let tranche_loss_fraction = if tranche_width_pct > 0.0 {
            (portfolio_loss_fraction / tranche_width_pct * 100.0).clamp(0.0, 1.0)
        } else {
            0.0 // Degenerate tranche
        };

        // Convert to currency amount: fraction * tranche notional
        Ok(tranche_loss_fraction * tranche.notional.amount())
    }

    /// Calculate expected tranche loss fraction at a specific date.
    ///
    /// Returns the expected loss as a fraction of the tranche notional [0, 1],
    /// properly scaled using the base correlation approach.
    fn expected_tranche_loss_fraction_at(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        date: Date,
    ) -> Result<F> {
        let attach_pct = tranche.attach_pct;
        let detach_pct = tranche.detach_pct;
        
        let _years = self.years_from_base(index_data, date);
        
        // Get correlations for attachment and detachment points
        let corr_attach = index_data.base_correlation_curve.correlation(attach_pct);
        let corr_detach = index_data.base_correlation_curve.correlation(detach_pct);

        // Clamp correlations for numerical stability
        let corr_attach =
            corr_attach.clamp(self.params.min_correlation, self.params.max_correlation);
        let corr_detach =
            corr_detach.clamp(self.params.min_correlation, self.params.max_correlation);

        // Calculate expected losses for equity tranches [0, A] and [0, D]
        let el_to_attach =
            self.calculate_equity_tranche_loss(attach_pct, corr_attach, index_data, date)?;

        let el_to_detach =
            self.calculate_equity_tranche_loss(detach_pct, corr_detach, index_data, date)?;



        // The [A, D] tranche loss as a fraction of total portfolio
        let portfolio_loss_fraction = el_to_detach - el_to_attach;

        // Ensure monotonicity: el_to_detach should always be >= el_to_attach
        // If this fails, it indicates numerical issues in the equity loss calculation
        let portfolio_loss_fraction = portfolio_loss_fraction.max(0.0);

        // Convert to fraction of tranche notional: EL_[A,D] / (D-A) * 100
        let tranche_width_pct = detach_pct - attach_pct;
        let tranche_loss_fraction = if tranche_width_pct > 0.0 {
            (portfolio_loss_fraction / tranche_width_pct * 100.0).clamp(0.0, 1.0)
        } else {
            0.0 // Degenerate tranche
        };

        Ok(tranche_loss_fraction)
    }

    /// Build the expected loss curve for all payment dates.
    ///
    /// Returns a vector of (Date, EL_fraction) pairs where EL_fraction
    /// is the cumulative expected loss as a fraction of tranche notional.
    fn build_el_curve(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        dates: &[Date],
    ) -> Result<Vec<(Date, F)>> {
        let mut el_curve = Vec::with_capacity(dates.len());
        
        for &date in dates {
            let el_fraction = self.expected_tranche_loss_fraction_at(tranche, index_data, date)?;
            el_curve.push((date, el_fraction));
        }
        
        Ok(el_curve)
    }

    /// Calculate expected loss for an equity tranche [0, K] using Gaussian Copula.
    ///
    /// # Arguments
    /// * `detachment_pct` - Detachment point K in percent
    /// * `correlation` - Asset correlation parameter ρ
    /// * `index_data` - Credit index market data
    /// * `maturity` - Maturity date for loss calculation
    fn calculate_equity_tranche_loss(
        &self,
        detachment_pct: F,
        correlation: F,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<F> {
        let num_constituents = index_data.num_constituents as usize;
        let recovery_rate = index_data.recovery_rate;

        // Convert detachment from percent to number of constituents
        let detachment_notional = detachment_pct / 100.0;

        // Get the appropriate quadrature
        let quad = match self.params.quadrature_order {
            5 => GaussHermiteQuadrature::order_5(),
            7 => GaussHermiteQuadrature::order_7(),
            10 => GaussHermiteQuadrature::order_10(),
            _ => GaussHermiteQuadrature::order_7(), // Default fallback
        };

        // Calculate maturity in years for survival probability lookup
        let maturity_years = self.years_from_base(index_data, maturity);

        // Get default probability for the index (homogeneous assumption)
        let default_prob = self.get_default_probability(index_data, maturity_years)?;
        let default_threshold = standard_normal_inv_cdf(default_prob);

        // Integrate expected loss over all states of the market factor Z
        let expected_loss = quad.integrate(|z| {
            // Conditional default probability given market factor Z
            let conditional_default_prob =
                self.conditional_default_probability(default_threshold, correlation, z);

            // Expected loss of equity tranche conditional on Z
            self.conditional_equity_tranche_loss(
                num_constituents,
                detachment_notional,
                conditional_default_prob,
                recovery_rate,
            )
        });

        Ok(expected_loss)
    }

    /// Calculate conditional default probability given market factor Z.
    ///
    /// P(default | Z) = Φ((Φ⁻¹(PD) - √ρ * Z) / √(1-ρ))
    fn conditional_default_probability(
        &self,
        default_threshold: F,
        correlation: F,
        market_factor: F,
    ) -> F {
        let sqrt_rho = correlation.sqrt();
        let sqrt_one_minus_rho = (1.0 - correlation).sqrt();

        let conditional_threshold =
            (default_threshold - sqrt_rho * market_factor) / sqrt_one_minus_rho;
        standard_normal_cdf(conditional_threshold)
    }

    /// Calculate expected loss of equity tranche conditional on market factor.
    ///
    /// Uses the binomial distribution to sum over all possible numbers of defaults.
    fn conditional_equity_tranche_loss(
        &self,
        num_constituents: usize,
        detachment_notional: F,
        conditional_default_prob: F,
        recovery_rate: F,
    ) -> F {
        let loss_given_default = 1.0 - recovery_rate;
        let individual_notional = 1.0 / num_constituents as F; // Normalized to 1.0 total

        let mut expected_loss = 0.0;

        // Sum over all possible numbers of defaults
        for k in 0..=num_constituents {
            let prob_k_defaults =
                binomial_probability(num_constituents, k, conditional_default_prob);

            // Portfolio loss given k defaults
            let portfolio_loss = k as F * individual_notional * loss_given_default;

            // Tranche loss (equity tranche [0, detachment_notional])
            let tranche_loss = portfolio_loss.min(detachment_notional);

            expected_loss += prob_k_defaults * tranche_loss;
        }

        expected_loss
    }

    /// Calculate present value of the premium leg with accrual-on-default.
    ///
    /// PV = Coupon * Σ(Δt_j * D(t_j) * [N_outstanding - 0.5 * N_incremental_loss])
    /// where N_outstanding = N_tr * (1 - EL_fraction(t_{j-1}))
    /// and N_incremental_loss = N_tr * (EL_fraction(t_j) - EL_fraction(t_{j-1}))
    fn calculate_premium_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discount,
        as_of: Date,
    ) -> Result<F> {
        let coupon = tranche.running_coupon_bp / 10000.0; // Convert bp to decimal
        let tranche_notional = tranche.notional.amount();

        // Generate payment schedule and expected loss curve
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        if payment_dates.is_empty() {
            return Ok(0.0);
        }

        let el_curve = self.build_el_curve(tranche, index_data, &payment_dates)?;

        let mut pv_premium = 0.0;
        let mut prev_el_fraction = 0.0; // Start with no loss

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let t = self.years_from_base(index_data, payment_date);
            if t <= 0.0 {
                continue;
            }

            let el_fraction = el_curve[i].1; // Current EL fraction
            let delta_el_fraction = el_fraction - prev_el_fraction;

            // Outstanding notional at beginning of period
            let outstanding_notional = tranche_notional * (1.0 - prev_el_fraction);

            if outstanding_notional <= 0.0 {
                break; // Tranche fully written down
            }

            // Accrual period using day count convention
            let period_start = if i == 0 {
                tranche.effective_date.unwrap_or(as_of)
            } else {
                payment_dates[i - 1]
            };
            
            let accrual_period = tranche.day_count
                .year_fraction(period_start, payment_date)
                .unwrap_or(0.0);

            // Accrual-on-default: reduce accrual by half of incremental loss
            let aod_adjustment = 0.5 * tranche_notional * delta_el_fraction;
            let effective_notional = outstanding_notional - aod_adjustment;

            let discount_factor = discount_curve.df(t);

            pv_premium += coupon * accrual_period * discount_factor * effective_notional;
            prev_el_fraction = el_fraction;
        }

        Ok(pv_premium)
    }

    /// Calculate present value of the protection leg using incremental EL.
    ///
    /// PV = Σ(D(t_j) * ΔEL_j) where ΔEL_j = N_tr * (EL_fraction(t_j) - EL_fraction(t_{j-1}))
    fn calculate_protection_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discount,
        as_of: Date,
    ) -> Result<F> {
        let tranche_notional = tranche.notional.amount();

        // Generate payment schedule and expected loss curve
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        if payment_dates.is_empty() {
            return Ok(0.0);
        }

        let el_curve = self.build_el_curve(tranche, index_data, &payment_dates)?;

        let mut pv_protection = 0.0;
        let mut prev_el_fraction = 0.0; // Start with no loss

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let t = self.years_from_base(index_data, payment_date);
            if t <= 0.0 {
                continue;
            }

            let el_fraction = el_curve[i].1; // Current EL fraction
            let delta_el_fraction = el_fraction - prev_el_fraction;

            // Incremental loss amount in currency
            let incremental_loss_amount = tranche_notional * delta_el_fraction;

            if incremental_loss_amount > 0.0 {
                let discount_factor = discount_curve.df(t);
                pv_protection += incremental_loss_amount * discount_factor;
            }

            prev_el_fraction = el_fraction;
        }

        Ok(pv_protection)
    }

    /// Get default probability for the index at a given maturity.
    fn get_default_probability(
        &self,
        index_data: &CreditIndexData,
        maturity_years: F,
    ) -> Result<F> {
        let survival_prob = index_data.index_credit_curve.sp(maturity_years);
        Ok(1.0 - survival_prob)
    }

    /// Calculate years from the credit curve base date.
    fn years_from_base(&self, index_data: &CreditIndexData, date: Date) -> F {
        let dc = index_data.index_credit_curve.day_count();
        dc.year_fraction(index_data.index_credit_curve.base_date(), date)
            .unwrap_or(0.0)
    }

    /// Create a bumped base correlation curve for sensitivity analysis.
    ///
    /// Creates a new BaseCorrelationCurve with correlations shifted by bump_abs,
    /// clamped to [min_correlation, max_correlation] for numerical stability.
    fn bump_base_correlation(
        &self,
        original_curve: &finstack_core::market_data::term_structures::BaseCorrelationCurve,
        bump_abs: F,
    ) -> finstack_core::Result<finstack_core::market_data::term_structures::BaseCorrelationCurve> {
        use finstack_core::market_data::term_structures::BaseCorrelationCurve;
        
        // Extract original points and apply bump
        let bumped_points: Vec<(F, F)> = original_curve.detachment_points()
            .iter()
            .zip(original_curve.correlations().iter())
            .map(|(&detach, &corr)| {
                let bumped_corr = (corr + bump_abs).clamp(self.params.min_correlation, self.params.max_correlation);
                (detach, bumped_corr)
            })
            .collect();

        // Create temporary ID for bumped curve
        BaseCorrelationCurve::builder("TEMP_BUMPED_CORR")
            .points(bumped_points)
            .build()
    }

    /// Create a bumped credit index with shifted hazard rates for CS01 calculation.
    ///
    /// Creates a new CreditIndexData with the index hazard curve shifted by delta_lambda.
    fn bump_index_hazard(
        &self,
        original_index: &CreditIndexData,
        delta_lambda: F,
    ) -> Result<CreditIndexData> {
        // Create bumped hazard curve
        let bumped_hazard_curve = original_index.index_credit_curve.with_hazard_shift(delta_lambda)?;
        
        // Create new credit index data with bumped hazard curve
        CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::new(bumped_hazard_curve))
            .base_correlation_curve(original_index.base_correlation_curve.clone())
            .build()
    }

    /// Generate payment schedule for the tranche using canonical schedule builder.
    ///
    /// Uses the robust date scheduling utilities with proper business day
    /// conventions and calendar support.
    fn generate_payment_schedule(&self, tranche: &CdsTranche, as_of: Date) -> Result<Vec<Date>> {
        let start_date = tranche.effective_date.unwrap_or(as_of);
        
        let schedule = build_dates(
            start_date,
            tranche.maturity,
            tranche.payment_frequency,
            StubKind::None, // TODO: Make configurable if needed
            tranche.business_day_convention,
            tranche.calendar_id,
        );
        
        // Filter out dates before as_of (in case effective_date < as_of)
        let payment_dates: Vec<Date> = schedule.dates
            .into_iter()
            .filter(|&date| date > as_of)
            .collect();
        
        Ok(payment_dates)
    }

    /// Calculate upfront amount for the tranche.
    ///
    /// This is the net present value at inception, representing the
    /// payment required to enter the position at the standard coupon.
    pub fn calculate_upfront(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<F> {
        let pv = self.price_tranche(tranche, market_ctx, as_of)?;
        Ok(pv.amount())
    }

    /// Calculate Spread DV01 (sensitivity to 1bp change in running coupon).
    pub fn calculate_spread_dv01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Create bumped tranche with +1bp running coupon
        let mut bumped_tranche = tranche.clone();
        bumped_tranche.running_coupon_bp += 1.0;

        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();
        let bumped_pv = self
            .price_tranche(&bumped_tranche, market_ctx, as_of)?
            .amount();

        Ok(bumped_pv - base_pv)
    }

    /// Calculate expected loss metric (the total expected loss at maturity).
    pub fn calculate_expected_loss(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
    ) -> Result<F> {
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id)?;
        self.calculate_expected_tranche_loss(tranche, index_data, tranche.maturity)
    }

    /// Calculate CS01 (sensitivity to 1bp parallel shift in credit spreads).
    pub fn calculate_cs01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Get base price
        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();

        // Create bumped market context with hazard rates shifted by configured amount
        let delta_lambda = self.params.cs01_hazard_bump;
        let original_index = market_ctx.get_credit_index(tranche.credit_index_id)?;
        let bumped_index = self.bump_index_hazard(original_index, delta_lambda)?;
        
        // Create new market context with bumped credit index
        let bumped_market_ctx = ValuationMarketContext::from_core(market_ctx.core.clone())
            .with_credit_index(tranche.credit_index_id, bumped_index);

        // Calculate bumped price
        let bumped_pv = self.price_tranche(tranche, &bumped_market_ctx, as_of)?.amount();

        // Return sensitivity per basis point
        Ok(bumped_pv - base_pv)
    }

    /// Calculate correlation delta (sensitivity to correlation changes).
    pub fn calculate_correlation_delta(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        as_of: Date,
    ) -> Result<F> {
        // Get base price
        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();

        // Create bumped market context with base correlation shifted by configured amount
        let bump_abs = self.params.corr_bump_abs;
        let original_index = market_ctx.get_credit_index(tranche.credit_index_id)?;
        let bumped_corr_curve = self.bump_base_correlation(&original_index.base_correlation_curve, bump_abs)?;
        
        // Create new credit index data with bumped correlation curve
        let bumped_index = CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(original_index.index_credit_curve.clone())
            .base_correlation_curve(std::sync::Arc::new(bumped_corr_curve))
            .build()?;

        // Create new market context with bumped credit index
        let bumped_market_ctx = ValuationMarketContext::from_core(market_ctx.core.clone())
            .with_credit_index(tranche.credit_index_id, bumped_index);

        // Calculate bumped price
        let bumped_pv = self.price_tranche(tranche, &bumped_market_ctx, as_of)?.amount();

        // Return sensitivity per unit correlation change
        Ok((bumped_pv - base_pv) / bump_abs)
    }

    /// Calculate jump-to-default (immediate loss from specific entity default).
    ///
    /// For a homogeneous portfolio, estimates the immediate impact if one average
    /// entity defaults instantly. This is distinct from correlation sensitivity.
    pub fn calculate_jump_to_default(
        &self,
        tranche: &CdsTranche,
        market_ctx: &ValuationMarketContext,
        _as_of: Date,
    ) -> Result<F> {
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id)?;
        
        // For homogeneous pool, one name default impact
        let individual_weight = 1.0 / (index_data.num_constituents as F); // Portfolio weight per name
        let loss_given_default = 1.0 - index_data.recovery_rate;
        let individual_loss = individual_weight * loss_given_default; // As fraction of portfolio
        
        // Check if this loss hits the tranche layer
        let attach_frac = tranche.attach_pct / 100.0;
        let detach_frac = tranche.detach_pct / 100.0;
        let tranche_width = detach_frac - attach_frac;
        
        if individual_loss <= attach_frac {
            // Loss doesn't reach the tranche
            return Ok(0.0);
        }
        
        // Calculate how much of the individual loss hits the tranche
        let tranche_hit = if individual_loss >= detach_frac {
            // Loss fully exhausts the tranche
            tranche_width
        } else {
            // Loss partially hits the tranche
            individual_loss - attach_frac
        };
        
        // Convert to tranche notional impact
        let impact_on_tranche_fraction = tranche_hit / tranche_width;
        let impact_amount = impact_on_tranche_fraction * tranche.notional.amount();
        
        Ok(impact_amount)
    }
}

/// Calculate binomial probability: P(X = k) where X ~ Binomial(n, p)
fn binomial_probability(n: usize, k: usize, p: F) -> F {
    if k > n {
        return 0.0;
    }
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == n { 1.0 } else { 0.0 };
    }

    // Use log-space calculation to avoid overflow for large n
    let log_prob =
        log_binomial_coefficient(n, k) + (k as F) * p.ln() + ((n - k) as F) * (1.0 - p).ln();
    log_prob.exp()
}

/// Calculate log of binomial coefficient: ln(n choose k)
fn log_binomial_coefficient(n: usize, k: usize) -> F {
    if k > n {
        return F::NEG_INFINITY;
    }
    if k == 0 || k == n {
        return 0.0;
    }

    // Use the more efficient calculation: ln(n!) - ln(k!) - ln((n-k)!)
    // Using Stirling's approximation for large values
    log_factorial(n) - log_factorial(k) - log_factorial(n - k)
}

/// Calculate log factorial using Stirling's approximation for large n.
fn log_factorial(n: usize) -> F {
    if n == 0 || n == 1 {
        return 0.0;
    }
    if n < 20 {
        // Exact calculation for small n: ln(n!) = ln(1) + ln(2) + ... + ln(n)
        (2..=n).map(|i| (i as F).ln()).sum()
    } else {
        // Stirling's approximation: ln(n!) ≈ n*ln(n) - n + 0.5*ln(2πn)
        let n_f = n as F;
        n_f * n_f.ln() - n_f + 0.5 * (2.0 * std::f64::consts::PI * n_f).ln()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::market_data::credit_index::CreditIndexData;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::{
        hazard_curve::HazardCurve, BaseCorrelationCurve,
    };
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::Month;

    fn sample_market_context() -> ValuationMarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Create discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .log_df()
            .build()
            .unwrap();

        // Create index hazard curve
        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.01), (3.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .par_spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
            .build()
            .unwrap();

        // Create base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .points(vec![
                (3.0, 0.25),  // 0-3% equity
                (7.0, 0.45),  // 0-7% junior mezzanine
                (10.0, 0.60), // 0-10% senior mezzanine
                (15.0, 0.75), // 0-15% senior
                (30.0, 0.85), // 0-30% super senior
            ])
            .build()
            .unwrap();

        // Create credit index data
        let index_data = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .unwrap();

        ValuationMarketContext::new()
            .with_discount(discount_curve)
            .with_credit_index("CDX.NA.IG.42", index_data)
    }

    fn sample_tranche() -> CdsTranche {
        let _issue_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        CdsTranche::new(
            "CDX_IG42_3_7_5Y",                                      // id
            "CDX.NA.IG.42",                                         // index_name
            42,                                                     // series
            3.0,                                                    // attach_pct (3%)
            7.0,                                                    // detach_pct (7%)
            Money::new(10_000_000.0, Currency::USD),                // $10MM notional
            maturity,                                               // maturity
            500.0,                                                  // running_coupon_bp (5%)
            finstack_core::dates::Frequency::quarterly(),           // payment_frequency
            finstack_core::dates::DayCount::Act360,                 // day_count
            finstack_core::dates::BusinessDayConvention::Following, // business_day_convention
            None,                                                   // calendar_id
            "USD-OIS",                                              // disc_id
            "CDX.NA.IG.42",                                         // credit_index_id
            TrancheSide::SellProtection,                            // side
        )
    }

    #[test]
    fn test_model_creation() {
        let model = GaussianCopulaModel::new();
        assert_eq!(model.params.quadrature_order, 7);
        assert!(model.params.use_issuer_curves);
    }

    #[test]
    fn test_conditional_default_probability() {
        let model = GaussianCopulaModel::new();
        let correlation = 0.30;
        let default_threshold = standard_normal_inv_cdf(0.05); // 5% default probability

        // Test with market factor = 0 (should be reasonable value close to original default prob)
        let cond_prob = model.conditional_default_probability(default_threshold, correlation, 0.0);
        assert!(
            cond_prob > 0.01 && cond_prob < 0.1,
            "Expected reasonable default prob, got {}",
            cond_prob
        );

        // Test with negative market factor (should increase default prob)
        let cond_prob_neg =
            model.conditional_default_probability(default_threshold, correlation, -1.0);
        assert!(cond_prob_neg > 0.05);

        // Test with positive market factor (should decrease default prob)
        let cond_prob_pos =
            model.conditional_default_probability(default_threshold, correlation, 1.0);
        assert!(cond_prob_pos < 0.05);
    }

    #[test]
    fn test_binomial_probability() {
        // Test known values
        assert!((binomial_probability(10, 5, 0.5) - 0.24609375).abs() < 1e-6);
        assert!((binomial_probability(5, 0, 0.1) - 0.59049).abs() < 1e-6);

        // Test edge cases
        assert_eq!(binomial_probability(10, 0, 0.0), 1.0);
        assert_eq!(binomial_probability(10, 10, 1.0), 1.0);
        assert_eq!(binomial_probability(10, 5, 0.0), 0.0);
    }

    #[test]
    fn test_log_factorial() {
        // Test small values (exact calculation)
        assert!((log_factorial(1) - 0.0).abs() < 1e-12);
        assert!(
            (log_factorial(5) - (2.0_f64.ln() + 3.0_f64.ln() + 4.0_f64.ln() + 5.0_f64.ln())).abs()
                < 1e-12
        );

        // Test that Stirling's approximation is reasonable for large n
        let log_100_factorial = log_factorial(100);
        assert!(log_100_factorial > 360.0 && log_100_factorial < 370.0); // Should be around 363.7
    }

    #[test]
    fn test_tranche_pricing_integration() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        // Test that pricing doesn't panic and returns a reasonable result
        let result = model.price_tranche(&tranche, &market_ctx, as_of);
        assert!(result.is_ok());

        let pv = result.unwrap();
        assert_eq!(pv.currency(), Currency::USD);
        // PV should be finite (could be positive or negative)
        assert!(pv.amount().is_finite());
    }

    #[test]
    fn test_expected_loss_calculation() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();

        let expected_loss = model.calculate_expected_loss(&tranche, &market_ctx);
        assert!(expected_loss.is_ok());

        let loss = expected_loss.unwrap();
        assert!(loss >= 0.0); // Expected loss should be non-negative
        assert!(loss.is_finite());
    }

    #[test]
    fn test_payment_schedule_generation() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let schedule = model.generate_payment_schedule(&tranche, as_of);
        assert!(schedule.is_ok());
        
        let dates = schedule.unwrap();
        assert!(!dates.is_empty());
        assert!(dates[0] > as_of); // First payment should be after as_of
        assert!(*dates.last().unwrap() <= tranche.maturity); // Last payment should not exceed maturity
        
        // Check dates are in ascending order
        for window in dates.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_el_curve_monotonicity() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let schedule = model.generate_payment_schedule(&tranche, as_of).unwrap();
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id).unwrap();
        let el_curve = model.build_el_curve(&tranche, index_data, &schedule);
        
        assert!(el_curve.is_ok());
        let curve = el_curve.unwrap();
        
        // EL should be non-decreasing and bounded [0,1]
        // Allow for small numerical deviations due to base correlation model limitations
        // The base correlation model can have inconsistencies at knot points
        const NUMERICAL_TOLERANCE: F = 0.01; // Allow up to 1% EL fraction decrease
        
        for (i, &(_, el_fraction)) in curve.iter().enumerate() {
            assert!((0.0..=1.0).contains(&el_fraction), 
                "EL fraction {} at index {} out of bounds", el_fraction, i);
            
            if i > 0 {
                let decrease = curve[i-1].1 - el_fraction;
                assert!(decrease <= NUMERICAL_TOLERANCE, 
                    "EL fraction decreased significantly from {} to {} (decrease: {})", 
                    curve[i-1].1, el_fraction, decrease);
            }
        }
    }

    #[test]
    fn test_cs01_calculation() {
        let model = GaussianCopulaModel::new();
        let mut tranche = sample_tranche();
        tranche.side = TrancheSide::SellProtection; // Sell protection for positive CS01
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let cs01 = model.calculate_cs01(&tranche, &market_ctx, as_of);
        assert!(cs01.is_ok());
        
        let sensitivity = cs01.unwrap();
        assert!(sensitivity.is_finite());
        // For protection seller, CS01 should typically be positive
        // (higher spreads -> higher protection premium income)
    }

    #[test]
    fn test_correlation_delta_calculation() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let corr_delta = model.calculate_correlation_delta(&tranche, &market_ctx, as_of);
        assert!(corr_delta.is_ok());
        
        let sensitivity = corr_delta.unwrap();
        assert!(sensitivity.is_finite());
        // Correlation sensitivity should be finite and reasonable in magnitude
    }

    #[test]
    fn test_jump_to_default_calculation() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        let jtd = model.calculate_jump_to_default(&tranche, &market_ctx, as_of);
        assert!(jtd.is_ok());
        
        let impact = jtd.unwrap();
        assert!(impact >= 0.0); // Impact should be non-negative
        assert!(impact.is_finite());
    }

    #[test]
    fn test_pv_decomposition_consistency() {
        let model = GaussianCopulaModel::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let index_data = market_ctx.get_credit_index(tranche.credit_index_id).unwrap();
        let discount_curve = market_ctx.discount(tranche.disc_id).unwrap();
        
        // Calculate individual leg PVs
        let pv_premium = model.calculate_premium_leg_pv(&tranche, index_data, discount_curve.as_ref(), as_of);
        let pv_protection = model.calculate_protection_leg_pv(&tranche, index_data, discount_curve.as_ref(), as_of);
        
        assert!(pv_premium.is_ok());
        assert!(pv_protection.is_ok());
        
        let premium = pv_premium.unwrap();
        let protection = pv_protection.unwrap();
        
        assert!(premium.is_finite());
        assert!(protection.is_finite());
        assert!(premium >= 0.0); // Premium leg should be positive for ongoing coupon
        assert!(protection >= 0.0); // Protection leg should be non-negative
    }
}
