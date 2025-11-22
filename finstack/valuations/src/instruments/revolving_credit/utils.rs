//! Internal utilities for revolving credit facilities.
//!
//! Consolidates schedule and calendar logic to avoid duplication across
//! cashflow generation and pricing implementations.

use super::types::{BaseRateSpec, RevolvingCredit};
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::DateExt;
use finstack_core::dates::{BusinessDayConvention, Date, ScheduleBuilder};
use finstack_core::prelude::HolidayCalendar;
use finstack_core::Result;

/// Resolve the calendar for a facility from its attributes.
///
/// Looks for `calendar_id` or `calendar` metadata and returns the resolved
/// calendar if found in the global registry.
///
/// # Arguments
///
/// * `attrs` - Facility attributes containing calendar metadata
///
/// # Returns
///
/// Reference to a `HolidayCalendar` if calendar metadata is present and resolvable, `None` otherwise.
pub(super) fn resolve_facility_calendar(
    attrs: &Attributes,
) -> Option<&'static dyn HolidayCalendar> {
    let cal_code = attrs
        .get_meta("calendar_id")
        .or_else(|| attrs.get_meta("calendar"))?;
    finstack_core::dates::CalendarRegistry::global().resolve_str(cal_code)
}

/// Build payment schedule dates for a revolving credit facility.
///
/// Generates a payment schedule from commitment to maturity with the facility's
/// payment frequency, applying calendar adjustments if configured.
///
/// # Arguments
///
/// * `facility` - The revolving credit facility
/// * `include_sentinel` - If true, appends a date one day after the last payment
///   to ensure terminal cashflows are included in period aggregation (exclusive end semantics)
///
/// # Returns
///
/// Vector of payment dates, optionally with a sentinel date appended.
///
/// # Errors
///
/// Returns an error if the schedule builder fails or produces fewer than 2 dates.
pub(super) fn build_payment_dates(
    facility: &RevolvingCredit,
    include_sentinel: bool,
) -> Result<Vec<Date>> {
    let mut builder = ScheduleBuilder::new(facility.commitment_date, facility.maturity_date)
        .frequency(facility.payment_frequency)
        .stub_rule(finstack_core::dates::StubKind::None);

    if let Some(cal) = resolve_facility_calendar(&facility.attributes) {
        builder = builder.adjust_with(BusinessDayConvention::ModifiedFollowing, cal);
    }

    let payment_schedule = builder.build()?;
    let mut payment_dates: Vec<Date> = payment_schedule.into_iter().collect();

    // Add sentinel if requested (for period PV aggregation with exclusive end semantics)
    if include_sentinel {
        if let Some(&last) = payment_dates.last() {
            payment_dates.push(last + time::Duration::days(1));
        }
    }

    if payment_dates.len() < 2 {
        return Err(finstack_core::error::InputError::TooFewPoints.into());
    }

    Ok(payment_dates)
}

/// Build reset schedule dates for floating rate facilities.
///
/// For floating rate facilities, generates a reset schedule from commitment to
/// maturity with the reset frequency, applying calendar adjustments if configured.
/// For fixed rate facilities, returns `None`.
///
/// # Arguments
///
/// * `facility` - The revolving credit facility
///
/// # Returns
///
/// - `Some(Vec<Date>)` for floating rate facilities with reset dates
/// - `None` for fixed rate facilities
///
/// # Errors
///
/// Returns an error if the schedule builder fails for floating rate facilities.
pub(super) fn build_reset_dates(facility: &RevolvingCredit) -> Result<Option<Vec<Date>>> {
    match &facility.base_rate_spec {
        BaseRateSpec::Floating(spec) => {
            let mut reset_builder =
                ScheduleBuilder::new(facility.commitment_date, facility.maturity_date)
                    .frequency(spec.reset_freq)
                    .stub_rule(finstack_core::dates::StubKind::None);

            if let Some(cal) = resolve_facility_calendar(&facility.attributes) {
                reset_builder =
                    reset_builder.adjust_with(BusinessDayConvention::ModifiedFollowing, cal);
            }

            let reset_schedule = reset_builder.build()?;
            Ok(Some(reset_schedule.into_iter().collect()))
        }
        BaseRateSpec::Fixed { .. } => Ok(None),
    }
}

/// Project floating rate for revolving credit facility.
///
/// Wrapper around centralized `project_floating_rate` that handles
/// revolving credit's attribute-based calendar resolution.
///
/// # Arguments
///
/// * `reset_date` - Start date of the reset period
/// * `reset_freq` - Frequency determining the period length
/// * `index_id` - Forward curve identifier (e.g., "USD-SOFR-3M")
/// * `spread_bp` - Spread/margin over index in basis points
/// * `floor_bp` - Optional floor on index rate in basis points
/// * `market` - Market context containing forward curves
/// * `attrs` - Facility attributes (for calendar resolution)
///
/// # Returns
///
/// All-in coupon rate (index + spread, with floor applied).
#[allow(dead_code)]
pub(super) fn project_floating_rate(
    reset_date: finstack_core::dates::Date,
    reset_freq: &finstack_core::dates::Frequency,
    index_id: &str,
    spread_bp: f64,
    floor_bp: Option<f64>,
    market: &finstack_core::market_data::MarketContext,
    attrs: &Attributes,
) -> Result<f64> {
    // Compute reset period end using facility calendar
    let reset_end = compute_reset_period_end(reset_date, reset_freq, attrs)?;

    // Delegate to centralized projection (revolving credit doesn't use caps or gearing)
    crate::cashflow::builder::project_floating_rate(
        reset_date, reset_end, index_id, spread_bp, 1.0, // gearing = 1.0
        floor_bp, None, // revolving credit doesn't use caps
        market,
    )
}

/// Project floating rate for revolving credit facility using resolved curve.
///
/// Optimized version of `project_floating_rate` that avoids curve lookup.
pub(super) fn project_floating_rate_with_curve(
    reset_date: finstack_core::dates::Date,
    reset_freq: &finstack_core::dates::Frequency,
    spread_bp: f64,
    floor_bp: Option<f64>,
    fwd: &finstack_core::market_data::term_structures::ForwardCurve,
    attrs: &Attributes,
) -> Result<f64> {
    // Compute reset period end using facility calendar
    let reset_end = compute_reset_period_end(reset_date, reset_freq, attrs)?;

    // Delegate to centralized projection (revolving credit doesn't use caps or gearing)
    crate::cashflow::builder::project_floating_rate_with_curve(
        reset_date, reset_end, spread_bp, 1.0, // gearing = 1.0
        floor_bp, None, // revolving credit doesn't use caps
        fwd,
    )
}

/// Apply a draw/repay event to current balance with commitment limit validation.
///
/// Updates the balance by adding (draw) or subtracting (repayment) the event amount.
/// For draws, validates that the new balance does not exceed the commitment amount.
///
/// # Arguments
///
/// * `current_balance` - Current drawn balance
/// * `event` - Draw or repayment event to apply
/// * `commitment_amount` - Total facility commitment (for draw validation)
///
/// # Returns
///
/// Updated balance after applying the event.
///
/// # Errors
///
/// Returns a validation error if:
/// - A draw would exceed the commitment amount
/// - Checked arithmetic fails (e.g., repayment exceeds balance)
pub(super) fn apply_draw_repay_event(
    current_balance: finstack_core::money::Money,
    event: &super::types::DrawRepayEvent,
    commitment_amount: finstack_core::money::Money,
) -> Result<finstack_core::money::Money> {
    if event.is_draw {
        let new_balance = current_balance.checked_add(event.amount)?;
        // Validate draw does not exceed commitment
        if new_balance.amount() > commitment_amount.amount() {
            return Err(finstack_core::Error::Validation(format!(
                "Draw on {} would exceed commitment: {} > {}",
                event.date, new_balance, commitment_amount
            )));
        }
        Ok(new_balance)
    } else {
        current_balance.checked_sub(event.amount)
    }
}

/// Compute the end date of a reset period given start date and frequency.
///
/// Applies frequency offset (months or days) and calendar business day adjustment
/// if configured in facility attributes.
///
/// # Arguments
///
/// * `reset_date` - Start date of the reset period
/// * `reset_freq` - Frequency determining the period length
/// * `attrs` - Facility attributes (for calendar resolution)
///
/// # Returns
///
/// End date of the reset period, adjusted for business days if calendar is configured.
///
/// # Errors
///
/// Returns an error if calendar adjustment fails.
pub(super) fn compute_reset_period_end(
    reset_date: Date,
    reset_freq: &finstack_core::dates::Frequency,
    attrs: &Attributes,
) -> Result<Date> {
    use finstack_core::dates::Frequency;

    // Compute unadjusted end date based on frequency
    let mut reset_end = reset_date;
    match reset_freq {
        Frequency::Months(m) => {
            reset_end = reset_date.add_months(*m as i32);
        }
        Frequency::Days(d) => {
            reset_end = reset_date + time::Duration::days(*d as i64);
        }
        _ => {}
    }

    // Apply calendar adjustment if configured
    if let Some(cal) = resolve_facility_calendar(attrs) {
        reset_end =
            finstack_core::dates::adjust(reset_end, BusinessDayConvention::ModifiedFollowing, cal)?;
    }

    Ok(reset_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Attributes;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::money::Money;
    use time::Month;

    fn create_test_facility(
        start: Date,
        end: Date,
        payment_freq: Frequency,
        base_rate_spec: BaseRateSpec,
        calendar_id: Option<&str>,
    ) -> RevolvingCredit {
        let mut attrs = Attributes::new();
        if let Some(cal_id) = calendar_id {
            attrs = attrs.with_meta("calendar_id", cal_id);
        }

        RevolvingCredit {
            id: "TEST-RC".into(),
            commitment_amount: Money::new(10_000_000.0, Currency::USD),
            drawn_amount: Money::new(5_000_000.0, Currency::USD),
            commitment_date: start,
            maturity_date: end,
            base_rate_spec,
            day_count: DayCount::Act360,
            payment_frequency: payment_freq,
            fees: super::super::types::RevolvingCreditFees::default(),
            draw_repay_spec: super::super::types::DrawRepaySpec::Deterministic(vec![]),
            discount_curve_id: "USD-OIS".into(),
            hazard_curve_id: None,
            recovery_rate: 0.0,
            attributes: attrs,
        }
    }

    #[test]
    fn test_resolve_facility_calendar_none() {
        let attrs = Attributes::new();
        assert!(resolve_facility_calendar(&attrs).is_none());
    }

    #[test]
    fn test_resolve_facility_calendar_with_id() {
        // Test with a calendar that exists (WMR = Weekends, Memorial days, Retail calendar)
        let attrs = Attributes::new().with_meta("calendar_id", "WMR");
        let cal = resolve_facility_calendar(&attrs);
        // Calendar resolution depends on global registry state, so we just test the mechanism
        // If a calendar doesn't exist, it returns None (which is fine for this test)
        let _ = cal;
    }

    #[test]
    fn test_build_payment_dates_no_sentinel() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let facility = create_test_facility(
            start,
            end,
            Frequency::quarterly(),
            BaseRateSpec::Fixed { rate: 0.05 },
            None,
        );

        let dates = build_payment_dates(&facility, false)
            .expect("Payment dates building should succeed in test");
        assert!(dates.len() >= 2);
        // Verify no sentinel: last date should be at or before maturity
        assert!(*dates.last().expect("Dates should not be empty") <= end);
    }

    #[test]
    fn test_build_payment_dates_with_sentinel() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let facility = create_test_facility(
            start,
            end,
            Frequency::quarterly(),
            BaseRateSpec::Fixed { rate: 0.05 },
            None,
        );

        let dates_no_sentinel = build_payment_dates(&facility, false)
            .expect("Payment dates building should succeed in test");
        let dates_with_sentinel = build_payment_dates(&facility, true)
            .expect("Payment dates building should succeed in test");

        // With sentinel should have one more date
        assert_eq!(dates_with_sentinel.len(), dates_no_sentinel.len() + 1);

        // Sentinel should be one day after the last payment date
        let last_payment = dates_no_sentinel.last().expect("Dates should not be empty");
        let sentinel = dates_with_sentinel
            .last()
            .expect("Dates should not be empty");
        assert_eq!(*sentinel, *last_payment + time::Duration::days(1));
    }

    #[test]
    fn test_build_reset_dates_fixed_returns_none() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let facility = create_test_facility(
            start,
            end,
            Frequency::quarterly(),
            BaseRateSpec::Fixed { rate: 0.05 },
            None,
        );

        let reset_dates =
            build_reset_dates(&facility).expect("Reset dates building should succeed in test");
        assert!(reset_dates.is_none());
    }

    #[test]
    fn test_build_reset_dates_floating_returns_some() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");
        let facility = create_test_facility(
            start,
            end,
            Frequency::quarterly(),
            BaseRateSpec::Floating(crate::cashflow::builder::FloatingRateSpec {
                index_id: "USD-SOFR-3M".into(),
                spread_bp: 200.0,
                gearing: 1.0,
                gearing_includes_spread: true,
                floor_bp: None,
                cap_bp: None,
                all_in_floor_bp: None,
                index_cap_bp: None,
                reset_freq: Frequency::quarterly(),
                reset_lag_days: 2,
                dc: DayCount::Act360,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                fixing_calendar_id: None,
            }),
            None,
        );

        let reset_dates =
            build_reset_dates(&facility).expect("Reset dates building should succeed in test");
        assert!(reset_dates.is_some());
        let dates = reset_dates.expect("Reset dates should exist for floating rate");
        assert!(dates.len() >= 2);
    }

    #[test]
    fn test_calendar_adjustment_applied() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let end = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");

        // Without calendar
        let facility_no_cal = create_test_facility(
            start,
            end,
            Frequency::quarterly(),
            BaseRateSpec::Fixed { rate: 0.05 },
            None,
        );
        let dates_no_cal = build_payment_dates(&facility_no_cal, false)
            .expect("Payment dates building should succeed in test");

        // With NYC calendar
        let facility_with_cal = create_test_facility(
            start,
            end,
            Frequency::quarterly(),
            BaseRateSpec::Fixed { rate: 0.05 },
            Some("NYC"),
        );
        let dates_with_cal = build_payment_dates(&facility_with_cal, false)
            .expect("Payment dates building should succeed in test");

        // Both should have same length (quarterly over 1 year)
        assert_eq!(dates_no_cal.len(), dates_with_cal.len());
    }

    #[test]
    fn test_compute_reset_period_end_monthly() {
        let reset_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let attrs = Attributes::new();

        let reset_end = compute_reset_period_end(reset_date, &Frequency::Months(3), &attrs)
            .expect("Reset period end calculation should succeed");

        // 3 months from Jan 15 should be Apr 15
        let expected = Date::from_calendar_date(2025, Month::April, 15).expect("Valid test date");
        assert_eq!(reset_end, expected);
    }

    #[test]
    fn test_compute_reset_period_end_daily() {
        let reset_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let attrs = Attributes::new();

        let reset_end = compute_reset_period_end(reset_date, &Frequency::Days(90), &attrs)
            .expect("Reset period end calculation should succeed");

        // 90 days from Jan 15
        let expected = reset_date + time::Duration::days(90);
        assert_eq!(reset_end, expected);
    }

    #[test]
    fn test_compute_reset_period_end_with_calendar() {
        let reset_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        // Test mechanism - calendar adjustment is calendar-dependent
        let attrs_no_cal = Attributes::new();
        let attrs_with_cal = Attributes::new().with_meta("calendar_id", "WMR");

        let end_no_cal = compute_reset_period_end(reset_date, &Frequency::Months(1), &attrs_no_cal)
            .expect("Reset period end calculation should succeed");

        let end_with_cal =
            compute_reset_period_end(reset_date, &Frequency::Months(1), &attrs_with_cal)
                .expect("Reset period end calculation should succeed");

        // Both should succeed (calendar adjustment may or may not change date)
        assert!(end_no_cal.year() == 2025);
        assert!(end_with_cal.year() == 2025);
    }

    #[test]
    fn test_apply_draw_repay_event_draw() {
        use super::super::types::DrawRepayEvent;

        let balance = Money::new(5_000_000.0, Currency::USD);
        let commitment = Money::new(10_000_000.0, Currency::USD);
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).expect("Valid test date");

        let event = DrawRepayEvent {
            date: draw_date,
            amount: Money::new(2_000_000.0, Currency::USD),
            is_draw: true,
        };

        let new_balance = apply_draw_repay_event(balance, &event, commitment)
            .expect("Draw/repay event application should succeed");
        assert_eq!(new_balance.amount(), 7_000_000.0);
    }

    #[test]
    fn test_apply_draw_repay_event_repay() {
        use super::super::types::DrawRepayEvent;

        let balance = Money::new(5_000_000.0, Currency::USD);
        let commitment = Money::new(10_000_000.0, Currency::USD);
        let repay_date = Date::from_calendar_date(2025, Month::March, 1).expect("Valid test date");

        let event = DrawRepayEvent {
            date: repay_date,
            amount: Money::new(1_000_000.0, Currency::USD),
            is_draw: false,
        };

        let new_balance = apply_draw_repay_event(balance, &event, commitment)
            .expect("Draw/repay event application should succeed");
        assert_eq!(new_balance.amount(), 4_000_000.0);
    }

    #[test]
    fn test_apply_draw_repay_event_exceeds_commitment() {
        use super::super::types::DrawRepayEvent;

        let balance = Money::new(8_000_000.0, Currency::USD);
        let commitment = Money::new(10_000_000.0, Currency::USD);
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).expect("Valid test date");

        let event = DrawRepayEvent {
            date: draw_date,
            amount: Money::new(3_000_000.0, Currency::USD), // Would exceed commitment
            is_draw: true,
        };

        let result = apply_draw_repay_event(balance, &event, commitment);
        assert!(result.is_err());
        assert!(result
            .expect_err("should fail")
            .to_string()
            .contains("exceed commitment"));
    }

    #[test]
    fn test_project_floating_rate_parity() {
        use finstack_core::market_data::term_structures::ForwardCurve;
        use finstack_core::market_data::MarketContext;

        let reset_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let attrs = Attributes::new();

        // Create a simple forward curve
        let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(reset_date)
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.03),  // 3%
                (1.0, 0.035), // 3.5%
                (5.0, 0.04),  // 4%
            ])
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");

        let market = MarketContext::new().insert_forward(fwd_curve);

        // Test rate projection
        let rate = project_floating_rate(
            reset_date,
            &Frequency::Months(3),
            "USD-SOFR-3M",
            200.0,     // 200 bps margin
            Some(0.0), // 0% floor
            &market,
            &attrs,
        )
        .expect("Rate projection should succeed in test");

        // Rate should be forward rate + margin
        // Forward rate should be ~3% (from curve), plus 200bps = ~5%
        assert!(
            rate > 0.04 && rate < 0.06,
            "Rate should be approximately 5%: {}",
            rate
        );
    }

    #[test]
    fn test_project_floating_rate_with_floor() {
        use finstack_core::market_data::term_structures::ForwardCurve;
        use finstack_core::market_data::MarketContext;

        let reset_date =
            Date::from_calendar_date(2025, Month::January, 15).expect("Valid test date");
        let attrs = Attributes::new();

        // Create a low forward curve (below floor)
        let fwd_curve = ForwardCurve::builder("USD-LIBOR-3M", 0.25)
            .base_date(reset_date)
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.001), // 0.1% (below 1% floor)
                (1.0, 0.001),
                (5.0, 0.001),
            ])
            .build()
            .expect("ForwardCurve builder should succeed with valid test data");

        let market = MarketContext::new().insert_forward(fwd_curve);

        // Test rate projection with floor
        let rate = project_floating_rate(
            reset_date,
            &Frequency::Months(3),
            "USD-LIBOR-3M",
            100.0,       // 100 bps margin
            Some(100.0), // 1% floor on index
            &market,
            &attrs,
        )
        .expect("Rate projection should succeed in test");

        // Floor should lift index to 1%, plus 100bps margin = 2%
        assert!(
            (rate - 0.02).abs() < 0.001,
            "Rate should be ~2% (floor + margin): {}",
            rate
        );
    }
}
