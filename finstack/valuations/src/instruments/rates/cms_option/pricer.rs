//! Convexity-adjusted Black pricer for CMS options.
//!
//! Implements the standard market model for CMS caps/floors:
//! 1. Calculate forward swap rate for each fixing.
//! 2. Apply convexity adjustment using Hagan (2003) methodology.
//! 3. Price the option on the adjusted rate using Black-76.
//!
//! # Convexity Adjustment
//!
//! The convexity adjustment accounts for the difference between the CMS rate
//! (which is a martingale under the payment measure) and the forward swap rate
//! (martingale under the annuity measure). Per Hagan (2003), the adjustment
//! depends on the annuity sensitivity to rate changes:
//!
//! ```text
//! CMS_Rate ≈ Forward_Swap_Rate + Convexity_Adjustment
//! Convexity_Adjustment = 0.5 * σ² * T * G(S)
//! where G(S) ≈ swap_tenor / (1 + S * swap_tenor)²
//! ```
//!
//! # Accuracy Limitations
//!
//! This pricer uses the simplified Hagan (2003) first-order convexity adjustment. It is
//! accurate for short-to-medium tenors (< 10Y) and moderate volatility. For long-dated
//! CMS (> 10Y) or high-volatility environments, consider replication-based pricing.
//!
//! # Reference
//!
//! - Hagan, P. (2003). "Convexity Conundrums: Pricing CMS Swaps, Caps, and Floors."
//!   Wilmott Magazine, March, 38-44.
//! - Hull, J. (2018). "Options, Futures, and Other Derivatives."

use crate::instruments::common_impl::models::d1_d2_black76;
use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cms_option::types::CmsOption;
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Vol surface is not provided (required for CMS option pricing)
    /// - Forward swap rate is non-positive (would cause NaN in Black-76)
    pub(crate) fn price_internal_with_convexity(
        &self,
        inst: &CmsOption,
        curves: &MarketContext,
        as_of: Date,
        convexity_scale: f64,
    ) -> Result<Money> {
        use crate::instruments::common_impl::pricing::time::relative_df_discount_curve;

        let mut total_pv = 0.0;
        let strike = inst.strike_f64()?;
        let discount_curve = curves.get_discount(inst.discount_curve_id.as_ref())?;

        let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;

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

            // Validate forward rate for Black-76 (must be positive for log calculation)
            if forward_swap_rate <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "Forward swap rate {} is non-positive for fixing date {}; \
                     Black-76 requires positive forward rates",
                    forward_swap_rate, fixing_date
                )));
            }

            // 2. Calculate Convexity Adjustment
            // Time to fixing uses instrument's day_count for vol surface lookup
            let time_to_fixing =
                inst.day_count
                    .year_fraction(as_of, fixing_date, DayCountCtx::default())?;

            // Get volatility from surface
            let vol = vol_surface.value_clamped(time_to_fixing.max(0.0), strike);

            // Convexity adjustment using Hagan (2003) formula with forward rate
            let raw_convexity_adj = if time_to_fixing > 0.0 {
                convexity_adjustment(vol, time_to_fixing, inst.cms_tenor, forward_swap_rate)
            } else {
                0.0
            };

            let convexity_adj = raw_convexity_adj * convexity_scale;
            let adjusted_rate = forward_swap_rate + convexity_adj;

            // 3. Black Price
            let option_val = if time_to_fixing <= 0.0 {
                match inst.option_type {
                    crate::instruments::OptionType::Call => (forward_swap_rate - strike).max(0.0),
                    crate::instruments::OptionType::Put => (strike - forward_swap_rate).max(0.0),
                }
            } else {
                self.black_price(adjusted_rate, strike, vol, time_to_fixing, inst.option_type)
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
    /// - Accrual fractions use `swap_day_count` for the fixed leg and
    ///   `swap_float_day_count` (if provided) for the floating leg.
    pub(crate) fn calculate_forward_swap_rate(
        &self,
        inst: &CmsOption,
        market: &MarketContext,
        as_of: Date,
        start: Date,
        end: Date,
    ) -> Result<(f64, f64)> {
        use crate::instruments::common_impl::pricing::time::{
            rate_period_on_dates, relative_df_discount_curve,
        };

        // Returns (rate, annuity)
        let disc = market.get_discount(inst.discount_curve_id.as_ref())?;

        // Calculate Annuity (Fixed Leg)
        let swap_fixed_freq = inst.resolved_swap_fixed_freq();
        let swap_day_count = inst.resolved_swap_day_count();
        let sched_fixed = crate::cashflow::builder::build_dates(
            start,
            end,
            swap_fixed_freq,
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing,
            false,
            0,
            crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID,
        )?;

        let mut annuity = 0.0;
        let mut prev_date = start;
        // Skip the first date by index (not value equality) to handle BDC-adjusted dates
        for &d in sched_fixed.dates.iter().skip(1) {
            let accrual = swap_day_count.year_fraction(prev_date, d, DayCountCtx::default())?;
            // DF uses curve-consistent relative DF
            let df = relative_df_discount_curve(disc.as_ref(), as_of, d)?;
            annuity += accrual * df;
            prev_date = d;
        }

        if annuity.abs() < 1e-10 {
            return Err(finstack_core::Error::Validation(format!(
                "Annuity is near-zero ({}) for swap from {} to {}; \
                 check curve or schedule configuration",
                annuity, start, end
            )));
        }

        // Check if single curve or dual curve
        if inst.forward_curve_id == inst.discount_curve_id {
            // Single Curve Optimization: S = (DF_start - DF_end) / Annuity
            let df_start = relative_df_discount_curve(disc.as_ref(), as_of, start)?;
            let df_end = relative_df_discount_curve(disc.as_ref(), as_of, end)?;
            let rate = (df_start - df_end) / annuity;
            Ok((rate, annuity))
        } else {
            // Dual Curve: Calculate Float Leg PV
            let fwd_curve = market.get_forward(inst.forward_curve_id.as_ref())?;
            let float_day_count = inst.resolved_swap_float_day_count();
            let swap_float_freq = inst.resolved_swap_float_freq();
            let sched_float = crate::cashflow::builder::build_dates(
                start,
                end,
                swap_float_freq,
                StubKind::None,
                BusinessDayConvention::ModifiedFollowing,
                false,
                0,
                crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID,
            )?;

            let mut pv_float = 0.0;
            let mut prev_date = start;
            for &d in &sched_float.dates {
                if d == start {
                    continue;
                }
                // Floating accrual uses float day count when provided
                let accrual =
                    float_day_count.year_fraction(prev_date, d, DayCountCtx::default())?;

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

        // Use combined d1_d2_black76 for efficiency (computes shared intermediates once)
        let (d1, d2) = d1_d2_black76(forward, strike, vol, t);

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
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;

        Ok(ValuationResult::stamped(cms.id(), as_of, pv))
    }
}

/// Present value using Convexity Adjusted Black.
pub(crate) fn compute_pv(inst: &CmsOption, curves: &MarketContext, as_of: Date) -> Result<Money> {
    let pricer = CmsOptionPricer::new();
    pricer.price_internal(inst, curves, as_of)
}

/// Compute convexity adjustment for CMS rate using Hagan (2003) methodology.
///
/// The convexity adjustment accounts for the measure change from the annuity
/// measure (where the forward swap rate is a martingale) to the payment measure
/// (where the CMS rate is a martingale).
///
/// # Formula
///
/// Per Hagan (2003) "Convexity Conundrums", the adjustment is:
///
/// ```text
/// Convexity_Adjustment = 0.5 * σ² * T * G(S)
/// where G(S) = ∂²(1/A(S))/∂S² ≈ swap_tenor / (1 + S * swap_tenor)²
/// ```
///
/// The G(S) term represents the sensitivity of the inverse annuity to the
/// swap rate. Using the actual forward rate rather than a hardcoded value
/// ensures the adjustment is state-dependent and more accurate.
///
/// # Arguments
///
/// * `volatility` - Swap rate volatility (annualized, decimal form e.g. 0.20 for 20%)
/// * `time_to_fixing` - Time to fixing date in years
/// * `swap_tenor` - Tenor of the underlying CMS swap in years (e.g., 10.0 for 10Y)
/// * `forward_rate` - Current forward swap rate (decimal form e.g. 0.03 for 3%)
///
/// # Returns
///
/// Convexity adjustment to add to forward swap rate (in decimal form)
///
/// # References
///
/// - Hagan, P. S. (2003). "Convexity Conundrums: Pricing CMS Swaps, Caps, and Floors."
///   Wilmott Magazine, March, 38-44.
pub fn convexity_adjustment(
    volatility: f64,
    time_to_fixing: f64,
    swap_tenor: f64,
    forward_rate: f64,
) -> f64 {
    // G(S) = swap_tenor / (1 + S * swap_tenor)²
    // This approximates the second derivative of 1/Annuity with respect to swap rate
    let denominator = 1.0 + forward_rate * swap_tenor;
    let annuity_sensitivity = swap_tenor / (denominator * denominator);

    // Convexity adjustment = 0.5 * σ² * T * G(S)
    0.5 * volatility * volatility * time_to_fixing * annuity_sensitivity
}
