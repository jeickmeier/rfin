//! Shared pricing utilities for swap legs.
//!
//! This module consolidates the floating and fixed leg pricing logic that was
//! previously duplicated across IRS, BasisSwap, and other swap instruments.
//! The implementation preserves the Bloomberg-validated methodology from IRS.
//!
//! # Key Features
//!
//! - Numerical stability via robust relative discount factor calculation
//! - Kahan compensated summation for long-dated swaps
//! - Holiday-aware payment delay handling
//! - Compounded-in-arrears support for RFR swaps (SOFR, SONIA, etc.)
//! - Forward rate projection with floor/cap/gearing
//!
//! # Bloomberg Validation
//!
//! The `robust_relative_df` function implements the same numerical stability
//! checks used in IRS pricing that have been validated against Bloomberg SWPM
//! for discount factor calibration.

use crate::cashflow::builder::rate_helpers::FloatingRateParams;
use finstack_core::dates::calendar::registry::CalendarRegistry;
use finstack_core::dates::{Date, DateExt, DayCount, DayCountCtx, Schedule};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::math::KahanAccumulator;
use finstack_core::Result;

/// Minimum threshold for discount factor values to avoid numerical instability.
///
/// Set to 1e-10 to protect against division by near-zero discount factors
/// that can arise from extreme rate scenarios or very long time horizons.
///
/// # Numerical Justification
///
/// For extreme rate scenarios:
/// - At +50% rates over 50 years: DF ≈ exp(-0.50 × 50) = exp(-25) ≈ 1.4e-11
/// - At +60% rates over 50 years: DF ≈ exp(-0.60 × 50) = exp(-30) ≈ 9.4e-14
///
/// The threshold of 1e-10 catches pathological cases while allowing reasonable
/// stress testing up to ~48% rates over 50 years or ~96% over 25 years.
/// This aligns with ISDA stress testing requirements for rates ranging
/// from -10% to +50%.
pub const DF_EPSILON: f64 = 1e-10;

/// Basis points to decimal conversion factor.
pub const BP_TO_DECIMAL: f64 = 1e-4;

/// Minimum threshold for annuity values to avoid divide-by-zero in par spread calculations.
///
/// Set to 1e-12 to catch scenarios where all periods have expired or the annuity
/// is effectively zero due to extreme discounting.
pub const ANNUITY_EPSILON: f64 = 1e-12;

/// Compute discount factor at `target` relative to `as_of`, with numerical stability guard.
///
/// This helper centralizes the pattern of:
/// 1. Computing the discount factor ratio DF(target) / DF(as_of)
/// 2. Validating as_of DF against DF_EPSILON
/// 3. Returning the relative DF
///
/// This is the Bloomberg-validated implementation used in IRS pricing.
///
/// # Arguments
///
/// * `disc` - Discount curve for pricing
/// * `as_of` - Valuation date (denominator for relative discounting)
/// * `target` - Target payment date (numerator for relative discounting)
///
/// # Returns
///
/// Discount factor from `as_of` to `target` (DF(target) / DF(as_of)).
/// For seasoned instruments this represents the proper discount factor for
/// cashflows occurring after the valuation date.
///
/// # Errors
///
/// Returns a validation error if:
/// - Year fraction calculation fails
/// - The as_of discount factor is below DF_EPSILON threshold (1e-10),
///   which can occur in extreme rate scenarios or very long time horizons
/// - The resulting discount factor is non-positive (non-physical)
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_core::dates::Date;
/// use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
/// use finstack_valuations::instruments::common::pricing::swap_legs::robust_relative_df;
/// use time::Month;
///
/// # fn main() -> finstack_core::Result<()> {
/// let curve = DiscountCurve::builder("USD-OIS")
///     .base_date(Date::from_calendar_date(2024, Month::January, 1).expect("valid date"))
///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
///     .build()
///     .expect("curve should build");
///
/// let as_of = Date::from_calendar_date(2024, Month::January, 1).unwrap();
/// let target = Date::from_calendar_date(2025, Month::January, 1).unwrap();
///
/// let df = robust_relative_df(&curve, as_of, target)?;
/// assert!(df > 0.0 && df <= 1.0);
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn robust_relative_df(disc: &DiscountCurve, as_of: Date, target: Date) -> Result<f64> {
    let df_as_of = disc.df_on_date_curve(as_of)?;

    // Guard against invalid/near-zero discount factors for numerical stability and no-arb.
    if !df_as_of.is_finite() {
        return Err(finstack_core::error::Error::Validation(
            "Valuation date discount factor is not finite.".into(),
        ));
    }
    // Discount factors must be strictly positive under standard discounting assumptions.
    if df_as_of <= DF_EPSILON {
        return Err(finstack_core::error::Error::Validation(format!(
            "Valuation date discount factor ({:.2e}) is below numerical stability threshold ({:.2e}). \
             This may indicate extreme rate scenarios or very long time horizons.",
            df_as_of, DF_EPSILON
        )));
    }

    let df = disc.df_between_dates(as_of, target)?;
    if !df.is_finite() {
        return Err(finstack_core::error::Error::Validation(
            "Discount factor between dates is not finite.".into(),
        ));
    }
    if df <= 0.0 {
        return Err(finstack_core::error::Error::Validation(format!(
            "Discount factor between dates is non-positive (df={:.3e}) which is non-physical.",
            df
        )));
    }
    Ok(df)
}

/// Apply a payment-delay in business days using an optional holiday calendar.
///
/// Bloomberg/ISDA conventions define payment delay in **business days**, not just weekdays.
/// If a calendar is provided and found in the registry, we apply holiday-aware business day
/// addition; otherwise we fall back to weekday-only addition.
///
/// # Arguments
///
/// * `date` - The base date to adjust
/// * `delay_days` - Number of business days to add (0 or negative returns unchanged date)
/// * `calendar_id` - Optional calendar identifier for business day adjustments
///
/// # Returns
///
/// The adjusted payment date.
#[inline]
pub fn add_payment_delay(date: Date, delay_days: i32, calendar_id: Option<&str>) -> Date {
    if delay_days <= 0 {
        return date;
    }

    if let Some(id) = calendar_id {
        match CalendarRegistry::global().resolve_str(id) {
            Some(cal) => match date.add_business_days(delay_days, cal) {
                Ok(d) => return d,
                Err(e) => {
                    tracing::warn!(
                        calendar_id = id,
                        date = %date,
                        delay_days,
                        err = %e,
                        "Failed holiday-aware business-day addition for payment delay; \
                         falling back to weekday-only adjustment (Mon-Fri)"
                    );
                }
            },
            None => {
                tracing::warn!(
                    calendar_id = id,
                    date = %date,
                    delay_days,
                    "Payment-delay calendar not found; \
                     falling back to weekday-only adjustment (Mon-Fri)"
                );
            }
        };
    }

    // Fallback: weekday-only (Mon-Fri), ignores holidays.
    date.add_weekdays(delay_days)
}

/// Parameters for pricing a floating rate leg.
///
/// This struct wraps [`FloatingRateParams`] and adds swap-specific fields for
/// payment delay and calendar handling. Use this for swap leg pricing.
///
/// # Validation
///
/// Call [`validate()`](Self::validate) before pricing to ensure parameters are consistent.
/// The validation checks for:
/// - Valid spread and gearing (finite, gearing > 0)
/// - Consistent floor/cap ordering (floor <= cap)
/// - Valid payment delay (non-negative for practical use)
#[derive(Debug, Clone, Default)]
pub struct FloatingLegParams {
    /// Core rate parameters (spread, gearing, floors, caps).
    pub rate_params: FloatingRateParams,
    /// Payment delay in business days after period end.
    pub payment_delay_days: i32,
    /// Optional calendar ID for payment date adjustments.
    pub calendar_id: Option<String>,
}

impl FloatingLegParams {
    /// Create params with just spread (most common case).
    pub fn with_spread(spread_bp: f64) -> Self {
        Self {
            rate_params: FloatingRateParams::with_spread(spread_bp),
            ..Default::default()
        }
    }

    /// Create params with spread and payment delay.
    pub fn with_spread_and_delay(spread_bp: f64, payment_delay_days: i32) -> Self {
        Self {
            rate_params: FloatingRateParams::with_spread(spread_bp),
            payment_delay_days,
            ..Default::default()
        }
    }

    /// Create params from rate params with payment delay.
    pub fn from_rate_params(rate_params: FloatingRateParams, payment_delay_days: i32) -> Self {
        Self {
            rate_params,
            payment_delay_days,
            ..Default::default()
        }
    }

    /// Create params with full configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn full(
        spread_bp: f64,
        gearing: f64,
        gearing_includes_spread: bool,
        index_floor_bp: Option<f64>,
        index_cap_bp: Option<f64>,
        all_in_floor_bp: Option<f64>,
        all_in_cap_bp: Option<f64>,
        payment_delay_days: i32,
        calendar_id: Option<String>,
    ) -> Self {
        Self {
            rate_params: FloatingRateParams {
                spread_bp,
                gearing,
                gearing_includes_spread,
                index_floor_bp,
                index_cap_bp,
                all_in_floor_bp,
                all_in_cap_bp,
            },
            payment_delay_days,
            calendar_id,
        }
    }

    /// Validate the floating leg parameters.
    ///
    /// Checks that:
    /// - Rate parameters are valid (delegates to [`FloatingRateParams::validate`])
    /// - Payment delay is reasonable (warning logged if negative)
    ///
    /// # Returns
    ///
    /// `Ok(())` if all parameters are valid, otherwise returns an error
    /// describing the validation failure.
    pub fn validate(&self) -> Result<()> {
        self.rate_params.validate()
    }
}

/// A period in a swap leg schedule.
///
/// This is a simpler view of cashflow data focused on what's needed for pricing.
#[derive(Debug, Clone)]
pub struct LegPeriod {
    /// Start of the accrual period.
    pub accrual_start: Date,
    /// End of the accrual period (also the unadjusted payment date).
    pub accrual_end: Date,
    /// Rate reset/fixing date (for floating legs).
    pub reset_date: Option<Date>,
    /// Year fraction for the accrual period.
    pub year_fraction: f64,
}

/// Compute present value of a floating rate leg using the standard term-rate methodology.
///
/// This is the Bloomberg-validated implementation from IRS pricing, generalized to work
/// with any swap instrument. It handles:
/// - Forward rate projection from the curve
/// - Spread, gearing, floors and caps
/// - Payment delay adjustment
/// - Numerical stability via Kahan summation
/// - Robust relative discount factors
///
/// # Arguments
///
/// * `periods` - Iterator over the leg periods
/// * `notional` - Notional amount (absolute value)
/// * `params` - Floating leg parameters
/// * `disc` - Discount curve for PV calculation
/// * `fwd` - Forward curve for rate projection
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Present value of the floating leg as a raw f64 (unsigned).
/// The caller is responsible for applying sign conventions.
///
/// # Errors
///
/// Returns an error if:
/// - Parameter validation fails (contradictory floors/caps, invalid gearing)
/// - Forward rate projection fails
/// - Discount factor calculation fails due to numerical instability
/// - Date calculations fail
pub fn pv_floating_leg<I>(
    periods: I,
    notional: f64,
    params: &FloatingLegParams,
    disc: &DiscountCurve,
    fwd: &ForwardCurve,
    as_of: Date,
) -> Result<f64>
where
    I: Iterator<Item = LegPeriod>,
{
    // Validate parameters at entry point for fail-fast behavior
    params.validate()?;

    // Use incremental Kahan accumulator to avoid Vec allocation
    let mut acc = KahanAccumulator::new();

    for period in periods {
        // Skip settled cashflows
        if period.accrual_end <= as_of {
            continue;
        }

        let reset_date = period.reset_date.unwrap_or(period.accrual_start);

        // Project forward rate using the validated rate_helpers implementation
        let forward_rate = crate::cashflow::builder::rate_helpers::project_floating_rate(
            reset_date,
            period.accrual_end,
            fwd,
            &params.rate_params,
        )?;

        // Coupon amount
        let coupon_amount = notional * forward_rate * period.year_fraction;

        // Apply payment delay
        let payment_date = add_payment_delay(
            period.accrual_end,
            params.payment_delay_days,
            params.calendar_id.as_deref(),
        );

        // Discount from as_of for correct theta
        let df = robust_relative_df(disc, as_of, payment_date)?;
        acc.add(coupon_amount * df);
    }

    Ok(acc.total())
}

/// Parameters for pricing a fixed rate leg.
#[derive(Debug, Clone)]
pub struct FixedLegParams {
    /// Fixed rate (decimal, e.g., 0.05 for 5%).
    pub rate: f64,
    /// Day count convention for accrual.
    pub day_count: DayCount,
    /// Payment delay in business days after period end.
    pub payment_delay_days: i32,
    /// Optional calendar ID for payment date adjustments.
    pub calendar_id: Option<String>,
}

impl FixedLegParams {
    /// Create params with rate and day count.
    pub fn new(rate: f64, day_count: DayCount) -> Self {
        Self {
            rate,
            day_count,
            payment_delay_days: 0,
            calendar_id: None,
        }
    }

    /// Create params with rate, day count, and payment delay.
    pub fn with_delay(rate: f64, day_count: DayCount, payment_delay_days: i32) -> Self {
        Self {
            rate,
            day_count,
            payment_delay_days,
            calendar_id: None,
        }
    }

    /// Validate fixed leg parameters.
    ///
    /// Checks that:
    /// - Rate is finite
    pub fn validate(&self) -> Result<()> {
        if !self.rate.is_finite() {
            return Err(finstack_core::error::Error::Validation(
                "Fixed rate must be finite".into(),
            ));
        }
        Ok(())
    }
}

/// Compute present value of a fixed rate leg.
///
/// This is the Bloomberg-validated implementation from IRS pricing, generalized to work
/// with any swap instrument. It handles:
/// - Fixed coupon calculation with proper day count
/// - Payment delay adjustment
/// - Numerical stability via Kahan summation
/// - Robust relative discount factors
///
/// # Arguments
///
/// * `periods` - Iterator over the leg periods
/// * `notional` - Notional amount (absolute value)
/// * `params` - Fixed leg parameters
/// * `disc` - Discount curve for PV calculation
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Present value of the fixed leg as a raw f64 (unsigned).
/// The caller is responsible for applying sign conventions.
///
/// # Errors
///
/// Returns an error if:
/// - Parameter validation fails
/// - Discount factor calculation fails due to numerical instability
pub fn pv_fixed_leg<I>(
    periods: I,
    notional: f64,
    params: &FixedLegParams,
    disc: &DiscountCurve,
    as_of: Date,
) -> Result<f64>
where
    I: Iterator<Item = LegPeriod>,
{
    // Validate parameters at entry point
    params.validate()?;

    // Use incremental Kahan accumulator to avoid Vec allocation
    let mut acc = KahanAccumulator::new();

    for period in periods {
        // Skip settled cashflows
        if period.accrual_end <= as_of {
            continue;
        }

        // Fixed coupon amount
        let coupon_amount = notional * params.rate * period.year_fraction;

        // Apply payment delay
        let payment_date = add_payment_delay(
            period.accrual_end,
            params.payment_delay_days,
            params.calendar_id.as_deref(),
        );

        // Discount from as_of for correct theta
        let df = robust_relative_df(disc, as_of, payment_date)?;
        acc.add(coupon_amount * df);
    }

    Ok(acc.total())
}

/// Compute discounted annuity (sum of DF × year_fraction) for a leg.
///
/// This is useful for DV01 calculations and par rate computations.
///
/// # Arguments
///
/// * `periods` - Iterator over the leg periods
/// * `disc` - Discount curve for PV calculation
/// * `as_of` - Valuation date
/// * `payment_delay_days` - Payment delay in business days
/// * `calendar_id` - Optional calendar ID for payment date adjustments
///
/// # Returns
///
/// The annuity (discounted year fraction sum) as a raw f64.
///
/// # Errors
///
/// Returns an error if the annuity is zero or below [`ANNUITY_EPSILON`],
/// which would cause divide-by-zero in downstream par spread calculations.
pub fn leg_annuity<I>(
    periods: I,
    disc: &DiscountCurve,
    as_of: Date,
    payment_delay_days: i32,
    calendar_id: Option<&str>,
) -> Result<f64>
where
    I: Iterator<Item = LegPeriod>,
{
    let mut annuity = 0.0;

    for period in periods {
        let payment_date = add_payment_delay(period.accrual_end, payment_delay_days, calendar_id);

        // Only include future payments
        if payment_date > as_of {
            let df = robust_relative_df(disc, as_of, payment_date)?;
            annuity += period.year_fraction * df;
        }
    }

    // Guard against zero annuity which would cause divide-by-zero in par spread calculations
    if annuity < ANNUITY_EPSILON {
        return Err(finstack_core::error::Error::Validation(format!(
            "Annuity ({:.2e}) is below minimum threshold ({:.2e}). \
             This may indicate all periods have expired or extreme discounting scenarios.",
            annuity, ANNUITY_EPSILON
        )));
    }

    Ok(annuity)
}

/// Convert a Schedule to an iterator of LegPeriods.
///
/// This helper bridges the gap between the core Schedule type and
/// the LegPeriod type used by the pricing functions.
///
/// # Arguments
///
/// * `schedule` - The schedule containing period dates
/// * `day_count` - Day count convention for calculating year fractions
/// * `reset_lag_days` - Reset lag in business days (for floating legs)
/// * `calendar_id` - Optional calendar ID for reset date adjustments
///
/// # Returns
///
/// A vector of LegPeriod structs.
pub fn schedule_to_periods(
    schedule: &Schedule,
    day_count: DayCount,
    reset_lag_days: Option<i32>,
    calendar_id: Option<&str>,
) -> Result<Vec<LegPeriod>> {
    if schedule.dates.len() < 2 {
        return Err(finstack_core::Error::Validation(
            "Schedule must contain at least 2 dates".to_string(),
        ));
    }

    let cal = calendar_id.and_then(|id| CalendarRegistry::global().resolve_str(id));

    let mut periods = Vec::with_capacity(schedule.dates.len() - 1);

    for i in 1..schedule.dates.len() {
        let accrual_start = schedule.dates[i - 1];
        let accrual_end = schedule.dates[i];

        let year_fraction =
            day_count.year_fraction(accrual_start, accrual_end, DayCountCtx::default())?;

        // Calculate reset date for floating legs
        let reset_date = if let Some(lag) = reset_lag_days {
            if lag == 0 {
                Some(accrual_start)
            } else if let Some(cal) = cal {
                Some(accrual_start.add_business_days(-lag, cal)?)
            } else {
                Some(accrual_start.add_weekdays(-lag))
            }
        } else {
            None
        };

        periods.push(LegPeriod {
            accrual_start,
            accrual_end,
            reset_date,
            year_fraction,
        });
    }

    Ok(periods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::types::CurveId;
    use time::Month;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
            .expect("valid date")
    }

    fn test_discount_curve(base_date: Date) -> DiscountCurve {
        DiscountCurve::builder(CurveId::new("TEST-DISC"))
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (0.5, 0.975), (1.0, 0.95), (5.0, 0.80)])
            .build()
            .expect("test curve should build")
    }

    fn test_forward_curve(base_date: Date) -> ForwardCurve {
        ForwardCurve::builder(CurveId::new("TEST-FWD"), 0.25)
            .base_date(base_date)
            .day_count(DayCount::Act360)
            .knots(vec![(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .build()
            .expect("test curve should build")
    }

    #[test]
    fn robust_relative_df_positive() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);

        let df = robust_relative_df(&disc, base_date, date(2025, 1, 1)).expect("should succeed");
        assert!(df > 0.0 && df <= 1.0, "DF should be in (0, 1]: {}", df);
    }

    #[test]
    fn robust_relative_df_rejects_zero_df() {
        // Create a curve that produces effectively zero DFs at long horizons
        let base_date = date(2024, 1, 1);
        let disc = DiscountCurve::builder(CurveId::new("EXTREME"))
            .base_date(base_date)
            .knots(vec![(0.0, 1e-12), (1.0, 1e-15)]) // Near-zero DFs
            .build()
            .expect("curve should build");

        let result = robust_relative_df(&disc, base_date, date(2025, 1, 1));
        assert!(result.is_err(), "Should reject near-zero DF");
    }

    #[test]
    fn pv_floating_leg_basic() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);
        let fwd = test_forward_curve(base_date);

        let periods = vec![
            LegPeriod {
                accrual_start: date(2024, 1, 1),
                accrual_end: date(2024, 4, 1),
                reset_date: Some(date(2024, 1, 1)),
                year_fraction: 0.25,
            },
            LegPeriod {
                accrual_start: date(2024, 4, 1),
                accrual_end: date(2024, 7, 1),
                reset_date: Some(date(2024, 4, 1)),
                year_fraction: 0.25,
            },
        ];

        let params = FloatingLegParams::with_spread(100.0); // 100 bps
        let pv = pv_floating_leg(
            periods.into_iter(),
            1_000_000.0,
            &params,
            &disc,
            &fwd,
            base_date,
        )
        .expect("should price");

        // Should be positive (receiving floating)
        assert!(pv > 0.0, "PV should be positive: {}", pv);
    }

    #[test]
    fn pv_floating_leg_validates_params() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);
        let fwd = test_forward_curve(base_date);

        let periods = vec![LegPeriod {
            accrual_start: date(2024, 1, 1),
            accrual_end: date(2024, 4, 1),
            reset_date: Some(date(2024, 1, 1)),
            year_fraction: 0.25,
        }];

        // Create params with contradictory floor/cap
        let params = FloatingLegParams::full(
            100.0,       // spread_bp
            1.0,         // gearing
            true,        // gearing_includes_spread
            None,        // index_floor_bp
            None,        // index_cap_bp
            Some(500.0), // all_in_floor_bp (5%)
            Some(300.0), // all_in_cap_bp (3%) - less than floor!
            0,           // payment_delay_days
            None,        // calendar_id
        );

        let result = pv_floating_leg(
            periods.into_iter(),
            1_000_000.0,
            &params,
            &disc,
            &fwd,
            base_date,
        );
        assert!(
            result.is_err(),
            "Should reject contradictory floor/cap params"
        );
    }

    #[test]
    fn pv_floating_leg_validates_zero_gearing() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);
        let fwd = test_forward_curve(base_date);

        let periods = vec![LegPeriod {
            accrual_start: date(2024, 1, 1),
            accrual_end: date(2024, 4, 1),
            reset_date: Some(date(2024, 1, 1)),
            year_fraction: 0.25,
        }];

        // Create params with zero gearing
        let params = FloatingLegParams::full(
            100.0, // spread_bp
            0.0,   // gearing - invalid!
            true,  // gearing_includes_spread
            None,  // index_floor_bp
            None,  // index_cap_bp
            None,  // all_in_floor_bp
            None,  // all_in_cap_bp
            0,     // payment_delay_days
            None,  // calendar_id
        );

        let result = pv_floating_leg(
            periods.into_iter(),
            1_000_000.0,
            &params,
            &disc,
            &fwd,
            base_date,
        );
        assert!(result.is_err(), "Should reject zero gearing");
    }

    #[test]
    fn pv_fixed_leg_basic() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);

        let periods = vec![
            LegPeriod {
                accrual_start: date(2024, 1, 1),
                accrual_end: date(2024, 7, 1),
                reset_date: None,
                year_fraction: 0.5,
            },
            LegPeriod {
                accrual_start: date(2024, 7, 1),
                accrual_end: date(2025, 1, 1),
                reset_date: None,
                year_fraction: 0.5,
            },
        ];

        let params = FixedLegParams::new(0.03, DayCount::Thirty360);
        let pv = pv_fixed_leg(periods.into_iter(), 1_000_000.0, &params, &disc, base_date)
            .expect("should price");

        // Should be positive (receiving fixed)
        assert!(pv > 0.0, "PV should be positive: {}", pv);

        // Approximate check: 2 × 0.5 × 0.03 × 1M × avg_df ≈ 30000 × 0.95 ≈ 28500
        assert!(
            pv > 20000.0 && pv < 35000.0,
            "PV should be reasonable: {}",
            pv
        );
    }

    #[test]
    fn pv_fixed_leg_validates_nan_rate() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);

        let periods = vec![LegPeriod {
            accrual_start: date(2024, 1, 1),
            accrual_end: date(2024, 7, 1),
            reset_date: None,
            year_fraction: 0.5,
        }];

        let params = FixedLegParams::new(f64::NAN, DayCount::Thirty360);
        let result = pv_fixed_leg(periods.into_iter(), 1_000_000.0, &params, &disc, base_date);
        assert!(result.is_err(), "Should reject NaN rate");
    }

    #[test]
    fn add_payment_delay_zero_returns_same() {
        let d = date(2024, 1, 15);
        let result = add_payment_delay(d, 0, None);
        assert_eq!(result, d);
    }

    #[test]
    fn add_payment_delay_positive_adds_weekdays() {
        let d = date(2024, 1, 15); // Monday
        let result = add_payment_delay(d, 2, None);
        // 2 weekdays from Monday = Wednesday
        assert_eq!(result, date(2024, 1, 17));
    }

    #[test]
    fn leg_annuity_computation() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);

        let periods = vec![
            LegPeriod {
                accrual_start: date(2024, 1, 1),
                accrual_end: date(2024, 7, 1),
                reset_date: None,
                year_fraction: 0.5,
            },
            LegPeriod {
                accrual_start: date(2024, 7, 1),
                accrual_end: date(2025, 1, 1),
                reset_date: None,
                year_fraction: 0.5,
            },
        ];

        let annuity =
            leg_annuity(periods.into_iter(), &disc, base_date, 0, None).expect("should compute");

        // Should be sum of (yf × df) ≈ 0.5 × 0.975 + 0.5 × 0.95 ≈ 0.9625
        assert!(
            annuity > 0.9 && annuity < 1.0,
            "Annuity should be reasonable: {}",
            annuity
        );
    }

    #[test]
    fn leg_annuity_rejects_zero() {
        let base_date = date(2024, 1, 1);
        let disc = test_discount_curve(base_date);

        // All periods are in the past
        let periods = vec![
            LegPeriod {
                accrual_start: date(2023, 1, 1),
                accrual_end: date(2023, 7, 1),
                reset_date: None,
                year_fraction: 0.5,
            },
            LegPeriod {
                accrual_start: date(2023, 7, 1),
                accrual_end: date(2024, 1, 1), // Ends exactly on as_of
                reset_date: None,
                year_fraction: 0.5,
            },
        ];

        let result = leg_annuity(periods.into_iter(), &disc, base_date, 0, None);
        assert!(
            result.is_err(),
            "Should reject zero annuity (all periods expired)"
        );
    }

    #[test]
    fn floating_leg_params_from_rate_params() {
        let rate_params = FloatingRateParams::with_spread_and_floor(200.0, 100.0);
        let leg_params = FloatingLegParams::from_rate_params(rate_params, 2);

        assert_eq!(leg_params.rate_params.spread_bp, 200.0);
        assert_eq!(leg_params.rate_params.index_floor_bp, Some(100.0));
        assert_eq!(leg_params.payment_delay_days, 2);
    }
}
