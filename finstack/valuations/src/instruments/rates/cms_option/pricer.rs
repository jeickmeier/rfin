//! Convexity-adjusted Black pricer for CMS options.
//!
//! Implements the standard market model for CMS caps/floors:
//! 1. Calculate forward swap rate for each fixing.
//! 2. Apply convexity adjustment (approximated using Hull-White formula or similar).
//! 3. Price the option on the adjusted rate using Black-76.
//!
//! Reference:
//! - Hagan, P. S. (2003). "Convexity Conundrums: Pricing CMS Swaps, Caps, and Floors."
//! - Hull, J. (2018). "Options, Futures, and Other Derivatives."

use crate::instruments::cms_option::types::CmsOption;
use crate::instruments::common::models::{d1_black76, d2_black76};
use crate::instruments::common::traits::Instrument;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{BusinessDayConvention, Date, DateExt, DayCountCtx, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Convexity-adjusted Black pricer for CMS options.
pub struct CmsOptionPricer;

impl CmsOptionPricer {
    /// Create a new CMS option pricer.
    pub fn new() -> Self {
        Self
    }

    /// Internal pricing logic
    ///
    /// # Time Basis
    ///
    /// - Vol surface lookups use the instrument's day_count for time_to_fixing
    ///   (market convention for vol surfaces).
    /// - Discount factors use curve-consistent relative DFs via `relative_df_discount_curve`.
    pub(crate) fn price_internal_with_convexity(
        &self,
        inst: &CmsOption,
        curves: &MarketContext,
        as_of: Date,
        convexity_scale: f64,
    ) -> Result<Money> {
        use crate::instruments::common::pricing::time::relative_df_discount_curve;

        let mut total_pv = 0.0;
        let discount_curve = curves.get_discount(inst.discount_curve_id.as_ref())?;

        // Get volatility surface if present
        let vol_surface = if let Some(vol_id) = &inst.vol_surface_id {
            Some(curves.surface(vol_id.as_str())?)
        } else {
            None
        };

        for (i, &fixing_date) in inst.fixing_dates.iter().enumerate() {
            let payment_date = inst.payment_dates.get(i).copied().unwrap_or(fixing_date);
            let accrual_fraction = inst.accrual_fractions.get(i).copied().unwrap_or(0.0);

            if payment_date <= as_of {
                continue; // Period expired
            }

            // 1. Calculate Forward Swap Rate
            let swap_start = fixing_date;
            let swap_tenor_months = (inst.cms_tenor * 12.0).round() as i32;
            let swap_end = swap_start.add_months(swap_tenor_months);

            // Calculate annuity and forward rate
            let (forward_swap_rate, _) =
                self.calculate_forward_swap_rate(inst, curves, as_of, swap_start, swap_end)?;

            // 2. Calculate Convexity Adjustment
            // Time to fixing uses instrument's day_count for vol surface lookup
            let time_to_fixing =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;

            // Get volatility
            let vol = if let Some(surface) = vol_surface.as_ref() {
                surface.value_clamped(time_to_fixing.max(0.0), inst.strike_rate)
            } else {
                0.20
            };

            // Convexity adjustment
            let raw_convexity_adj = if time_to_fixing > 0.0 {
                convexity_adjustment(vol, time_to_fixing, inst.cms_tenor)
            } else {
                0.0
            };

            let convexity_adj = raw_convexity_adj * convexity_scale;
            let adjusted_rate = forward_swap_rate + convexity_adj;

            // 3. Black Price
            let option_val = if time_to_fixing <= 0.0 {
                match inst.option_type {
                    crate::instruments::OptionType::Call => {
                        (forward_swap_rate - inst.strike_rate).max(0.0)
                    }
                    crate::instruments::OptionType::Put => {
                        (inst.strike_rate - forward_swap_rate).max(0.0)
                    }
                }
            } else {
                self.black_price(
                    adjusted_rate,
                    inst.strike_rate,
                    vol,
                    time_to_fixing,
                    inst.option_type,
                )
            };

            // 4. Discount to present using curve-consistent relative DF
            let df_pay = relative_df_discount_curve(discount_curve.as_ref(), as_of, payment_date)?;

            let period_pv = option_val * accrual_fraction * df_pay;
            total_pv += period_pv;
        }

        Ok(Money::new(
            total_pv * inst.notional.amount(),
            inst.notional.currency(),
        ))
    }

    fn price_internal(
        &self,
        inst: &CmsOption,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        self.price_internal_with_convexity(inst, curves, as_of, 1.0)
    }

    /// Calculate forward swap rate and annuity.
    ///
    /// # Time Basis
    ///
    /// Uses curve-consistent time mapping:
    /// - Discount factors use `relative_df_discount_curve` (curve's own day_count/base_date)
    /// - Forward rates use `rate_period_on_dates` (forward curve's own day_count/base_date)
    /// - Accrual fractions use `swap_day_count` (correct for coupon calculation)
    ///
    /// # Note on Float Day Count
    ///
    /// Currently uses `swap_day_count` for float leg accrual. In a production system,
    /// CmsOption could have a separate `swap_float_day_count` field to correctly
    /// handle different fixed/float conventions.
    pub(crate) fn calculate_forward_swap_rate(
        &self,
        inst: &CmsOption,
        market: &MarketContext,
        as_of: Date,
        start: Date,
        end: Date,
    ) -> Result<(f64, f64)> {
        use crate::instruments::common::pricing::time::{
            rate_period_on_dates, relative_df_discount_curve,
        };

        // Returns (rate, annuity)
        let disc = market.get_discount(inst.discount_curve_id.as_ref())?;

        // Calculate Annuity (Fixed Leg)
        let sched_fixed = crate::cashflow::builder::build_dates(
            start,
            end,
            inst.swap_fixed_freq,
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing,
            None,
        )?;

        let mut annuity = 0.0;
        let mut prev_date = start;
        for &d in &sched_fixed.dates {
            if d == start {
                continue;
            }
            // Accrual uses swap_day_count (correct for coupon calculation)
            let accrual =
                inst.swap_day_count
                    .year_fraction(prev_date, d, DayCountCtx::default())?;
            // DF uses curve-consistent relative DF
            let df = relative_df_discount_curve(disc.as_ref(), as_of, d)?;
            annuity += accrual * df;
            prev_date = d;
        }

        if annuity.abs() < 1e-10 {
            return Ok((0.0, 0.0));
        }

        // Check if single curve or dual curve
        let forward_curve_id = inst
            .forward_curve_id
            .as_ref()
            .unwrap_or(&inst.discount_curve_id);

        if forward_curve_id == &inst.discount_curve_id {
            // Single Curve Optimization: S = (DF_start - DF_end) / Annuity
            let df_start = relative_df_discount_curve(disc.as_ref(), as_of, start)?;
            let df_end = relative_df_discount_curve(disc.as_ref(), as_of, end)?;
            let rate = (df_start - df_end) / annuity;
            Ok((rate, annuity))
        } else {
            // Dual Curve: Calculate Float Leg PV
            let fwd_curve = market.get_forward(forward_curve_id.as_ref())?;
            let sched_float = crate::cashflow::builder::build_dates(
                start,
                end,
                inst.swap_float_freq,
                StubKind::None,
                BusinessDayConvention::ModifiedFollowing,
                None,
            )?;

            let mut pv_float = 0.0;
            let mut prev_date = start;
            for &d in &sched_float.dates {
                if d == start {
                    continue;
                }
                // Floating accrual (using swap_day_count - see note in docstring)
                let accrual =
                    inst.swap_day_count
                        .year_fraction(prev_date, d, DayCountCtx::default())?;

                // Forward rate uses forward curve's time basis
                let fwd_rate = rate_period_on_dates(fwd_curve.as_ref(), prev_date, d)?;

                // DF uses curve-consistent relative DF
                let df = relative_df_discount_curve(disc.as_ref(), as_of, d)?;

                pv_float += fwd_rate * accrual * df;
                prev_date = d;
            }

            let rate = pv_float / annuity;
            Ok((rate, annuity))
        }
    }

    fn black_price(
        &self,
        forward: f64,
        strike: f64,
        vol: f64,
        t: f64,
        option_type: crate::instruments::OptionType,
    ) -> f64 {
        if t <= 0.0 {
            return match option_type {
                crate::instruments::OptionType::Call => (forward - strike).max(0.0),
                crate::instruments::OptionType::Put => (strike - forward).max(0.0),
            };
        }

        let d1 = d1_black76(forward, strike, vol, t);
        let d2 = d2_black76(forward, strike, vol, t);

        match option_type {
            crate::instruments::OptionType::Call => {
                forward * finstack_core::math::norm_cdf(d1)
                    - strike * finstack_core::math::norm_cdf(d2)
            }
            crate::instruments::OptionType::Put => {
                strike * finstack_core::math::norm_cdf(-d2)
                    - forward * finstack_core::math::norm_cdf(-d1)
            }
        }
    }
}

impl Default for CmsOptionPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for CmsOptionPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::CmsOption, ModelKey::Black76) // Or ConvexityAdjustedBlack
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let cms = instrument
            .as_any()
            .downcast_ref::<CmsOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::CmsOption, instrument.key())
            })?;

        let pv = self.price_internal(cms, market, as_of).map_err(|e| {
            PricingError::model_failure_ctx(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(cms.id(), as_of, pv))
    }
}

/// Present value using Convexity Adjusted Black.
pub fn npv(inst: &CmsOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = CmsOptionPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Compute convexity adjustment for CMS rate (simplified approximation).
///
/// Convexity adjustment accounts for the difference between CMS rate
/// and forward swap rate due to volatility.
///
/// # Arguments
///
/// * `volatility` - Swap rate volatility
/// * `tenor` - Time to fixing date
/// * `swap_tenor` - Tenor of the CMS swap
///
/// # Returns
///
/// Convexity adjustment to add to forward swap rate
pub(crate) fn convexity_adjustment(volatility: f64, tenor: f64, swap_tenor: f64) -> f64 {
    // Simplified convexity adjustment
    // More sophisticated: use full volatility smile and correlation
    0.5 * volatility * volatility * tenor * swap_tenor / (1.0 + 0.03 * swap_tenor)
}
