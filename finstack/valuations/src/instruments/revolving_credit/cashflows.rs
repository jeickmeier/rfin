//! Cashflow generation for revolving credit facilities.
//!
//! Provides deterministic cashflow generation with:
//! - Interest on drawn amounts (fixed or floating)
//! - Commitment fees on undrawn amounts
//! - Usage fees on drawn amounts
//! - Facility fees on total commitment
//! - Upfront fees

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::builder::{
    CashFlowSchedule, CouponType, FeeBase, FeeSpec, FixedCouponSpec, FloatingCouponSpec,
};
use crate::cashflow::primitives::{CFKind, CashFlow};

use super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCredit};

/// Generate deterministic cashflows for a revolving credit facility.
///
/// This function creates a complete cashflow schedule including:
/// - Interest payments on drawn amounts
/// - Periodic fees (commitment, usage, facility)
/// - Upfront fee (if applicable)
/// - Terminal repayment of drawn balance
///
/// **Note**: For the 80/20 implementation, the drawn balance is held constant
/// throughout the facility life. Draw/repayment events are reserved for future
/// enhancement or Monte Carlo simulation.
///
/// # Arguments
/// * `facility` - The revolving credit facility
/// * `as_of` - Valuation date (for filtering future cashflows)
///
/// # Returns
/// A cashflow schedule with all cashflows and their kinds
pub fn generate_deterministic_cashflows(
    facility: &RevolvingCredit,
    as_of: Date,
) -> Result<CashFlowSchedule> {
    // Validate that we have a deterministic spec
    let _draw_repay_events = match &facility.draw_repay_spec {
        DrawRepaySpec::Deterministic(_events) => _events,
        DrawRepaySpec::Stochastic(_) => {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
    };

    // Start with base cashflow builder
    let mut builder = CashFlowSchedule::builder();

    // Set principal (drawn amount) with start/end dates
    builder.principal(
        facility.drawn_amount,
        facility.commitment_date,
        facility.maturity_date,
    );

    // Add interest cashflows based on rate spec
    match &facility.base_rate_spec {
        BaseRateSpec::Fixed { rate } => {
            builder.fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: *rate,
                freq: facility.payment_frequency,
                dc: facility.day_count,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: None, // Could be extracted from attributes if needed
                stub: finstack_core::dates::StubKind::None,
            });
        }
        BaseRateSpec::Floating {
            index_id,
            margin_bp,
            reset_freq,
        } => {
            builder.floating_cf(FloatingCouponSpec {
                index_id: index_id.clone(),
                margin_bp: *margin_bp,
                gearing: 1.0,
                coupon_type: CouponType::Cash,
                freq: *reset_freq,
                dc: facility.day_count,
                bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                calendar_id: None,
                stub: finstack_core::dates::StubKind::None,
                reset_lag_days: 2, // Standard T-2 reset
            });
        }
    }

    // Add commitment fee (on undrawn amount)
    if facility.fees.commitment_fee_bp > 0.0 {
        builder.fee(FeeSpec::PeriodicBps {
            base: FeeBase::Undrawn {
                facility_limit: facility.commitment_amount,
            },
            bps: facility.fees.commitment_fee_bp,
            freq: facility.payment_frequency,
            dc: facility.day_count,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
        });
    }

    // Add usage fee (on drawn amount)
    if facility.fees.usage_fee_bp > 0.0 {
        builder.fee(FeeSpec::PeriodicBps {
            base: FeeBase::Drawn,
            bps: facility.fees.usage_fee_bp,
            freq: facility.payment_frequency,
            dc: facility.day_count,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
        });
    }

    // Build the base schedule
    let mut schedule = builder.build()?;

    // Add facility fee manually (on total commitment, not handled by standard builder)
    if facility.fees.facility_fee_bp > 0.0 {
        let facility_fee_flows =
            generate_facility_fee_flows(facility, as_of, &facility.fees.facility_fee_bp)?;
        schedule.flows.extend(facility_fee_flows);
    }

    // Add upfront fee if present
    if let Some(upfront_fee) = facility.fees.upfront_fee {
        if facility.commitment_date >= as_of {
            schedule.flows.push(CashFlow {
                date: facility.commitment_date,
                reset_date: None,
                amount: upfront_fee,
                kind: CFKind::Fee,
                accrual_factor: 0.0,
            });
        }
    }

    // Note: Draw/repayment events change the outstanding balance but don't
    // create cashflows themselves. They affect interest calculations.
    // The cashflow builder already handles the terminal repayment of the
    // final outstanding balance.

    // Sort flows by date and kind
    schedule.flows.sort_by(|a, b| {
        a.date.cmp(&b.date).then_with(|| {
            // Define kind ranking for stable ordering
            let rank_a = match a.kind {
                CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
                CFKind::Fee => 1,
                CFKind::Amortization => 2,
                CFKind::PIK => 3,
                CFKind::Notional => 4,
                _ => 5,
            };
            let rank_b = match b.kind {
                CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
                CFKind::Fee => 1,
                CFKind::Amortization => 2,
                CFKind::PIK => 3,
                CFKind::Notional => 4,
                _ => 5,
            };
            rank_a.cmp(&rank_b)
        })
    });

    Ok(schedule)
}

/// Generate facility fee flows (fee on total commitment amount).
///
/// Since the standard builder doesn't support fees based on fixed amounts
/// (only drawn/undrawn), we manually generate facility fee cashflows.
fn generate_facility_fee_flows(
    facility: &RevolvingCredit,
    _as_of: Date,
    fee_bp: &f64,
) -> Result<Vec<CashFlow>> {
    use finstack_core::dates::ScheduleBuilder;

    // Optionally adjust by calendar if provided via attributes metadata
    let mut builder = ScheduleBuilder::new(facility.commitment_date, facility.maturity_date)
        .frequency(facility.payment_frequency)
        .stub_rule(finstack_core::dates::StubKind::None);
    if let Some(cal_code) = facility
        .attributes
        .get_meta("calendar_id")
        .or_else(|| facility.attributes.get_meta("calendar"))
    {
        if let Some(cal) = finstack_core::dates::CalendarRegistry::global().resolve_str(cal_code) {
            builder = builder.adjust_with(
                finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                cal,
            );
        }
    }
    let schedule = builder.build()?;

    let dates: Vec<Date> = schedule.into_iter().collect();
    let mut flows = Vec::new();

    if dates.len() < 2 {
        return Ok(flows);
    }

    let mut prev = dates[0];
    for &current in &dates[1..] {
        let accrual = facility.day_count.year_fraction(
            prev,
            current,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let fee_amount = facility.commitment_amount * (*fee_bp * 1e-4 * accrual);

        flows.push(CashFlow {
            date: current,
            reset_date: None,
            amount: fee_amount,
            kind: CFKind::Fee,
            accrual_factor: accrual,
        });

        prev = current;
    }

    Ok(flows)
}

/// Calculate the outstanding drawn balance at a given date considering draw/repay events.
///
/// This helper function simulates the drawn balance evolution based on the
/// deterministic schedule of draws and repayments.
///
/// # Arguments
/// * `facility` - The revolving credit facility
/// * `target_date` - The date at which to calculate the balance
///
/// # Returns
/// The outstanding drawn balance at the target date
pub fn calculate_drawn_balance_at_date(
    facility: &RevolvingCredit,
    target_date: Date,
) -> Result<Money> {
    let draw_repay_events = match &facility.draw_repay_spec {
        DrawRepaySpec::Deterministic(events) => events,
        DrawRepaySpec::Stochastic(_) => {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
    };

    let mut balance = facility.drawn_amount;

    // Apply all events up to the target date
    for event in draw_repay_events {
        if event.date <= target_date {
            if event.is_draw {
                balance = balance.checked_add(event.amount)?;
            } else {
                balance = balance.checked_sub(event.amount)?;
            }
        }
    }

    Ok(balance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::revolving_credit::DrawRepayEvent;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use time::Month;

    #[test]
    fn test_generate_deterministic_cashflows_fixed() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-001".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(super::super::types::RevolvingCreditFees {
                upfront_fee: Some(Money::new(50_000.0, Currency::USD)),
                commitment_fee_bp: 25.0, // 25 bps
                usage_fee_bp: 10.0,      // 10 bps
                facility_fee_bp: 5.0,    // 5 bps
            })
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let schedule = generate_deterministic_cashflows(&facility, start).unwrap();

        // Should have flows: interest, commitment fee, usage fee, facility fee, upfront fee
        assert!(!schedule.flows.is_empty());

        // Check that we have at least some fees
        let fee_count = schedule
            .flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Fee)
            .count();
        assert!(fee_count > 0, "Should have fee cashflows");
    }

    #[test]
    fn test_calculate_drawn_balance() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let repay_date = Date::from_calendar_date(2025, Month::June, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-001".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(Default::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![
                DrawRepayEvent {
                    date: draw_date,
                    amount: Money::new(2_000_000.0, Currency::USD),
                    is_draw: true,
                },
                DrawRepayEvent {
                    date: repay_date,
                    amount: Money::new(1_000_000.0, Currency::USD),
                    is_draw: false,
                },
            ]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        // Before draw
        let balance_before = calculate_drawn_balance_at_date(&facility, start).unwrap();
        assert_eq!(balance_before.amount(), 5_000_000.0);

        // After draw
        let balance_after_draw = calculate_drawn_balance_at_date(&facility, draw_date).unwrap();
        assert_eq!(balance_after_draw.amount(), 7_000_000.0);

        // After repayment
        let balance_after_repay = calculate_drawn_balance_at_date(&facility, repay_date).unwrap();
        assert_eq!(balance_after_repay.amount(), 6_000_000.0);
    }
}
