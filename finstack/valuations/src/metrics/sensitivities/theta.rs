//! Shared utilities for theta (time decay) calculations.
//!
//! Provides period parsing, date rolling, and a generic theta calculator that
//! works for any instrument implementing the `Instrument` trait.
//!
//! # Quick Start
//!
//! ## Example 1: Computing 1-Day Theta for an Equity Option
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::{EquityOption, Instrument, PricingOptions};
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let expiry = create_date(2024, Month::July, 1)?; // 6 months to expiry
//!
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     expiry,
//!     Money::new(100.0, Currency::USD),
//! )?;
//!
//! // Setup market (abbreviated)
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! let result = option.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Option value: ${:.2}", result.value.amount());
//!     println!("1-day theta: ${:.2}", theta);
//!     // Negative theta indicates time decay (option loses value each day)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 2: Computing Custom Period Theta (1 Week)
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::{EquityOption, Instrument, PricingOptions};
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     create_date(2024, Month::July, 1)?,
//!     Money::new(100.0, Currency::USD),
//! )?;
//!
//! // Setup market
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Customize theta period - supported formats:
//! // "1D", "2D", ... (days)
//! // "1W", "2W", ... (weeks)
//! // "1M", "3M", "6M", ... (months)
//! // "1Y", "2Y", ... (years)
//! let result = option.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("1-week theta: ${:.2}", theta);
//!     println!("This is the expected P&L from holding the option for 1 week");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 3: Bond Carry (Theta with Coupon Accrual)
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::{Bond, Instrument, PricingOptions, PricingOverrides};
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::January, 1)?;
//! let bond = Bond::example().unwrap();
//!
//! // Setup market
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Measure 1-month carry
//! let result = bond.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Bond value: ${:.2}", result.value.amount());
//!     println!("1-month carry: ${:.2}", theta);
//!     // Theta includes both:
//!     // 1. PV change (pull-to-par effect)
//!     // 2. Coupon payments during the period
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Example 4: Computing Theta Near Expiry
//!
//! When an instrument expires before the theta period ends, theta is automatically
//! capped at the expiry date:
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::{EquityOption, Instrument, PricingOptions};
//! use finstack_valuations::metrics::{standard_registry, MetricId};
//! use finstack_core::currency::Currency;
//! use finstack_core::dates::create_date;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::money::Money;
//! use time::Month;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let as_of = create_date(2024, Month::June, 25)?;
//! let expiry = create_date(2024, Month::July, 1)?; // Only 6 days to expiry
//!
//! let option = EquityOption::european_call(
//!     "OPT-001",
//!     "SPX",
//!     4500.0,
//!     expiry,
//!     Money::new(100.0, Currency::USD),
//! )?;
//!
//! // Setup market
//! # let market = MarketContext::new();
//!
//! let _registry = standard_registry();
//! let metrics = vec![MetricId::Theta];
//!
//! // Request 1-week theta, but only 6 days remain
//! let result = option.price_with_metrics(&market, as_of, &metrics, PricingOptions::default())?;
//!
//! if let Some(theta) = result.measures.get(MetricId::Theta.as_str()) {
//!     println!("Theta to expiry (6 days): ${:.2}", theta);
//!     // Theta is computed only to expiry, not the full 7-day period
//!     // This equals: PV(expiry) - PV(today)
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # How Theta is Calculated
//!
//! Theta represents the total carry (profit/loss) from holding an instrument over a time period:
//!
//! ```text
//! Theta = PV(t + period) - PV(t) + Cashflows(t, t + period)
//! ```
//!
//! Where:
//! - `PV(t)` = present value at valuation date (base value)
//! - `PV(t + period)` = present value at rolled forward date
//! - `Cashflows(t, t + period)` = sum of net cashflows during the period (signed canonical schedule)
//!
//! ## Components
//!
//! 1. **Pull-to-par effect**: Change in present value due to passage of time
//!    - For bonds: Price converges to par as maturity approaches
//!    - For options: Time value decays (typically negative theta)
//!
//! 2. **Cashflows**: Interest, coupons, or other payments during the period
//!    - Bonds: Accrued interest, coupon payments
//!    - Swaps: Net interest payments
//!    - Options: Usually zero (no interim cashflows)
//!
//! ## Sign Convention
//!
//! - **Negative theta**: Instrument loses value over time (e.g., long options)
//! - **Positive theta**: Instrument gains value over time (e.g., short options, carry trades)
//! - **Zero theta**: No time-dependent value change (rare)

use crate::instruments::common_impl::traits::Instrument;
use finstack_core::cashflow::CFKind;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DateExt};
use finstack_core::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThetaPeriod {
    Days(i64),
    Months(i32),
    Years(i32),
}

fn parse_theta_period(period: &str) -> Result<ThetaPeriod> {
    let period = period.trim().to_uppercase();
    if period.is_empty() {
        return Err(finstack_core::Error::from(
            finstack_core::InputError::Invalid,
        ));
    }

    let (num_str, unit) = if let Some(pos) = period.find(|c: char| c.is_alphabetic()) {
        (&period[..pos], &period[pos..])
    } else {
        return Err(finstack_core::Error::from(
            finstack_core::InputError::Invalid,
        ));
    };

    let num_i64: i64 = num_str
        .parse()
        .map_err(|_| finstack_core::Error::from(finstack_core::InputError::Invalid))?;

    if num_i64 < 0 {
        return Err(finstack_core::Error::Validation(format!(
            "Theta period must be non-negative, got '{period}'"
        )));
    }

    match unit {
        "D" => Ok(ThetaPeriod::Days(num_i64)),
        "W" => Ok(ThetaPeriod::Days(num_i64 * 7)),
        "M" => Ok(ThetaPeriod::Months(i32::try_from(num_i64).map_err(
            |_| finstack_core::Error::from(finstack_core::InputError::Invalid),
        )?)),
        "Y" => Ok(ThetaPeriod::Years(i32::try_from(num_i64).map_err(
            |_| finstack_core::Error::from(finstack_core::InputError::Invalid),
        )?)),
        _ => Err(finstack_core::Error::from(
            finstack_core::InputError::Invalid,
        )),
    }
}

/// Calculate the rolled forward date for theta calculation.
///
/// Advances the base date by the specified period, but caps at the expiry date if the
/// instrument expires before the period ends.
///
/// Notes:
/// - `"D"` and `"W"` are treated as fixed day increments.
/// - `"M"` and `"Y"` are treated as **calendar** month/year rolls (EOM-aware).
///
/// # Arguments
/// * `base_date` - Starting valuation date
/// * `period_str` - Period string (e.g., "1D", "1W", "1M")
/// * `expiry_date` - Optional instrument expiry date
///
/// # Returns
/// The rolled forward date, capped at expiry if applicable
pub(crate) fn calculate_theta_date(
    base_date: Date,
    period_str: &str,
    expiry_date: Option<Date>,
) -> Result<Date> {
    let rolled_date = match parse_theta_period(period_str)? {
        ThetaPeriod::Days(n) => base_date + time::Duration::days(n),
        ThetaPeriod::Months(n) => base_date.add_months(n),
        ThetaPeriod::Years(n) => base_date.add_months(n * 12),
    };

    // Cap at expiry if instrument expires before the rolled date
    if let Some(expiry) = expiry_date {
        if rolled_date > expiry {
            return Ok(expiry);
        }
    }

    Ok(rolled_date)
}

/// Collect income cashflows that occur during a time period.
///
/// Uses the full `cashflow_schedule()` so that each flow's [`CFKind`] is
/// available for filtering.  Only flows representing economic income to the
/// holder are included; negative notional flows (initial draws / funding
/// legs) are excluded because they are not discounted receipt flows and are
/// not reflected in the instrument PV.
///
/// The half-open interval `[start_date, end_date)` aligns with the PV
/// boundary convention: `value(as_of)` includes same-day flows
/// (`date >= as_of`) with DF=1, so a flow at `start_date` is part of
/// PV(start) but not PV(end), meaning it was "received" during the period.
/// Conversely, a flow at `end_date` is still inside PV(end), so it has not
/// yet been received.
///
/// # Returns
/// Sum of cashflow amounts in the period (converted to base currency)
pub(crate) fn collect_cashflows_in_period(
    instrument: &dyn Instrument,
    curves: &finstack_core::market_data::context::MarketContext,
    start_date: Date,
    end_date: Date,
    base_currency: Currency,
) -> Result<f64> {
    let schedule = instrument.cashflow_schedule(curves, start_date)?;
    let mut sum = 0.0;
    for cf in &schedule.flows {
        if cf.date >= start_date
            && cf.date < end_date
            && !(cf.kind == CFKind::Notional && cf.amount.amount() < 0.0)
        {
            if cf.amount.currency() != base_currency {
                return Err(finstack_core::Error::Validation(format!(
                    "Theta cashflow currency mismatch: base={} but saw cashflow currency={} (instrument_id={})",
                    base_currency,
                    cf.amount.currency(),
                    instrument.id(),
                )));
            }
            sum += cf.amount.amount();
        }
    }
    Ok(sum)
}

// Note: The `get_instrument_expiry` function has been replaced by the `Instrument::expiry()` trait method.
// Instruments now implement `expiry()` directly, returning `Some(date)` for instruments with expiry/maturity
// or `None` for instruments without a clear expiry concept (e.g., equity spot positions).
// See the `Instrument` trait in `instruments/common/traits.rs`.

/// Theta decomposition calculator.
///
/// Decomposes total theta into three additive components:
///
/// - **Carry**: net cashflows during the period (coupons, interest, fees; signed canonical schedule)
/// - **Roll-down**: PV change from time passing along the *same* curve (no curve movement)
/// - **Decay**: residual time-value / optionality decay (`total_theta - carry - roll_down`)
///
/// With static (T0) curves, `total_theta = carry + roll_down` by construction, so
/// decay is identically zero. Non-zero decay arises only when the total theta is
/// computed with distinct T1 curves (e.g., in daily P&L attribution).
///
/// The calculator is registered under [`MetricId::ThetaCarry`] and stores all three
/// components as side-effects in [`MetricContext::computed`].
pub(crate) struct GenericThetaDecomposed;

impl Default for GenericThetaDecomposed {
    fn default() -> Self {
        Self
    }
}

impl crate::metrics::MetricCalculator for GenericThetaDecomposed {
    fn calculate(&self, context: &mut crate::metrics::MetricContext) -> Result<f64> {
        let period_str = context
            .metric_overrides
            .as_ref()
            .and_then(|po| po.theta_period.as_deref())
            .unwrap_or("1D");

        let expiry_date = context.instrument.expiry();
        let rolled_date = calculate_theta_date(context.as_of, period_str, expiry_date)?;

        if rolled_date <= context.as_of {
            context
                .computed
                .insert(crate::metrics::MetricId::ThetaCarry, 0.0);
            context
                .computed
                .insert(crate::metrics::MetricId::ThetaRollDown, 0.0);
            context
                .computed
                .insert(crate::metrics::MetricId::ThetaDecay, 0.0);
            return Ok(0.0);
        }

        let base_pv = context
            .instrument_value_with_scenario(&context.curves, context.as_of)?
            .amount();
        let base_ccy = context.base_value.currency();

        let rolled_pv = context
            .instrument_value_with_scenario(&context.curves, rolled_date)?
            .amount();

        let carry = collect_cashflows_in_period(
            context.instrument.as_ref(),
            &context.curves,
            context.as_of,
            rolled_date,
            base_ccy,
        )?;

        let roll_down = rolled_pv - base_pv;

        // Total theta = carry + roll-down with T0-only curves.
        // Decay would be non-zero if a separate total theta (with T1 curves) were available,
        // but with static curves the decomposition is exact: total = carry + roll_down.
        let total_theta = carry + roll_down;
        let decay = total_theta - carry - roll_down;

        context
            .computed
            .insert(crate::metrics::MetricId::ThetaCarry, carry);
        context
            .computed
            .insert(crate::metrics::MetricId::ThetaRollDown, roll_down);
        context
            .computed
            .insert(crate::metrics::MetricId::ThetaDecay, decay);

        Ok(total_theta)
    }

    fn dependencies(&self) -> &[crate::metrics::MetricId] {
        &[]
    }
}

/// Lookup calculator for theta sub-components stored by [`GenericThetaDecomposed`].
///
/// Returns a value previously inserted into [`MetricContext::computed`] by the
/// decomposition calculator, avoiding redundant re-computation.
pub(crate) struct ThetaComponentLookup(pub crate::metrics::MetricId);

impl crate::metrics::MetricCalculator for ThetaComponentLookup {
    fn calculate(&self, context: &mut crate::metrics::MetricContext) -> Result<f64> {
        context.computed.get(&self.0).copied().ok_or_else(|| {
            finstack_core::InputError::NotFound {
                id: format!("metric:{}", self.0),
            }
            .into()
        })
    }

    fn dependencies(&self) -> &[crate::metrics::MetricId] {
        static DEPS: &[crate::metrics::MetricId] = &[crate::metrics::MetricId::ThetaCarry];
        DEPS
    }
}

/// Universal theta calculator that works with any instrument via the Instrument trait.
///
/// Computes theta as the total carry from rolling the valuation date forward:
///   Theta = PV(end_date) - PV(start_date) + Sum(Cashflows from start to end)
///
/// This calculator works with `dyn Instrument` directly, using the trait's `value()` method,
/// and is registered as the default theta calculator for all instruments.
pub(crate) struct GenericThetaAny;

impl Default for GenericThetaAny {
    fn default() -> Self {
        Self
    }
}

impl crate::metrics::MetricCalculator for GenericThetaAny {
    fn calculate(&self, context: &mut crate::metrics::MetricContext) -> Result<f64> {
        // Get theta period from pricing overrides, default to "1D"
        let period_str = context
            .metric_overrides
            .as_ref()
            .and_then(|po| po.theta_period.as_deref())
            .unwrap_or("1D");

        // Get expiry date if available (via Instrument trait method)
        let expiry_date = context.instrument.expiry();

        // Calculate rolled date
        let rolled_date = calculate_theta_date(context.as_of, period_str, expiry_date)?;

        // If already expired or rolling to same date, theta is zero
        if rolled_date <= context.as_of {
            tracing::warn!(
                as_of = %context.as_of,
                rolled_date = %rolled_date,
                "GenericThetaAny: Instrument already expired or rolling to same date, returning 0.0"
            );
            return Ok(0.0);
        }

        // Theta uses value() (instrument-economics-signed PV) for both base and rolled dates.
        // See GenericTheta for rationale on why value_raw() is not appropriate here.
        let base_pv = context
            .instrument_value_with_scenario(&context.curves, context.as_of)?
            .amount();
        let base_ccy = context.base_value.currency();

        let bumped_pv = context
            .instrument_value_with_scenario(&context.curves, rolled_date)?
            .amount();
        let pv_change = bumped_pv - base_pv;

        let cashflows_during_period = collect_cashflows_in_period(
            context.instrument.as_ref(),
            &context.curves,
            context.as_of,
            rolled_date,
            base_ccy,
        )?;

        Ok(pv_change + cashflows_during_period)
    }

    fn dependencies(&self) -> &[crate::metrics::MetricId] {
        &[]
    }
}

// ================================================================================================
// Unit tests (internal helpers)
// ================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use time::macros::date;
    use time::Month;

    fn test_date() -> Date {
        date!(2025 - 01 - 01)
    }

    // -------------------------------------------------------------------------
    // Theta date calculation
    // -------------------------------------------------------------------------

    #[test]
    fn calculate_theta_date_no_expiry() {
        let base = test_date();
        let rolled = calculate_theta_date(base, "1D", None).expect("roll 1D");
        let expected = Date::from_calendar_date(2025, Month::January, 2).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_one_week() {
        let base = test_date();
        let rolled = calculate_theta_date(base, "1W", None).expect("roll 1W");
        let expected = Date::from_calendar_date(2025, Month::January, 8).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_one_month() {
        let base = test_date();
        let rolled = calculate_theta_date(base, "1M", None).expect("roll 1M");
        let expected = Date::from_calendar_date(2025, Month::February, 1).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_with_expiry_cap() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::January, 5).expect("expiry date");

        let rolled = calculate_theta_date(base, "1W", Some(expiry)).expect("roll 1W");
        assert_eq!(rolled, expiry);
    }

    #[test]
    fn calculate_theta_date_before_expiry() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::February, 1).expect("expiry date");

        let rolled = calculate_theta_date(base, "1D", Some(expiry)).expect("roll 1D");
        let expected = Date::from_calendar_date(2025, Month::January, 2).expect("expected date");
        assert_eq!(rolled, expected);
    }

    #[test]
    fn calculate_theta_date_exactly_at_expiry() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::January, 31).expect("expiry date");

        let rolled = calculate_theta_date(base, "30D", Some(expiry)).expect("roll 30D");
        assert_eq!(rolled, expiry);
    }

    #[test]
    fn calculate_theta_date_already_past_expiry() {
        let base = Date::from_calendar_date(2025, Month::February, 1).expect("base date");
        let expiry = test_date();

        let rolled = calculate_theta_date(base, "1D", Some(expiry)).expect("roll 1D");
        assert_eq!(rolled, expiry);
    }

    #[test]
    fn calculate_theta_date_various_periods() {
        let base = test_date();

        let rolled_3m = calculate_theta_date(base, "3M", None).expect("roll 3M");
        assert_eq!(
            rolled_3m,
            Date::from_calendar_date(2025, Month::April, 1).expect("expected date")
        );

        let rolled_1y = calculate_theta_date(base, "1Y", None).expect("roll 1Y");
        assert_eq!(
            rolled_1y,
            Date::from_calendar_date(2026, Month::January, 1).expect("expected date")
        );
    }

    #[test]
    fn theta_workflow_short_dated_expiry_capped() {
        let base = test_date();
        let expiry = Date::from_calendar_date(2025, Month::January, 6).expect("expiry date");

        let theta_date_1d = calculate_theta_date(base, "1D", Some(expiry)).expect("roll 1D");
        assert_eq!(
            theta_date_1d,
            Date::from_calendar_date(2025, Month::January, 2).expect("expected date")
        );

        let theta_date_1w = calculate_theta_date(base, "1W", Some(expiry)).expect("roll 1W");
        assert_eq!(theta_date_1w, expiry);
    }
}
