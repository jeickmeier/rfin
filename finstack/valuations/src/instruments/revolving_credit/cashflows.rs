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

use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::{CFKind, CashFlow};

use super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCredit};

/// Generate deterministic cashflows for a revolving credit facility.
///
/// This function creates a complete cashflow schedule including:
/// - Interest payments on drawn amounts (adjusted for draw/repay events)
/// - Periodic fees (commitment, usage, facility) based on actual balances
/// - Principal flows from draw/repay events
/// - Terminal repayment of outstanding balance
///
/// **Key Feature**: Properly processes draw/repay events to adjust outstanding
/// balance over time, affecting interest and fee calculations.
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
    // Backward-compatible entry point without market curves: floating interest
    // is approximated using margin only. Prefer using
    // generate_deterministic_cashflows_with_curves when available.
    generate_deterministic_cashflows_internal(facility, None, as_of)
}

/// Generate deterministic cashflows for a revolving credit facility using market curves.
///
/// This variant includes floating rate projections from the provided MarketContext.
pub fn generate_deterministic_cashflows_with_curves(
    facility: &RevolvingCredit,
    market: &finstack_core::market_data::MarketContext,
    as_of: Date,
) -> Result<CashFlowSchedule> {
    generate_deterministic_cashflows_internal(facility, Some(market), as_of)
}

fn generate_deterministic_cashflows_internal(
    facility: &RevolvingCredit,
    market_opt: Option<&finstack_core::market_data::MarketContext>,
    as_of: Date,
) -> Result<CashFlowSchedule> {
    // Validate that we have a deterministic spec
    let draw_repay_events = match &facility.draw_repay_spec {
        DrawRepaySpec::Deterministic(events) => events,
        DrawRepaySpec::Stochastic(_) => {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
    };

    // Step 1: Build payment schedule dates
    use finstack_core::dates::ScheduleBuilder;
    
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
    
    let payment_schedule = builder.build()?;
    let payment_dates: Vec<Date> = payment_schedule.into_iter().collect();
    
    if payment_dates.len() < 2 {
        return Err(finstack_core::error::InputError::Invalid.into());
    }

    // Step 2: Calculate balance schedule (balance at start of each period)
    let balance_schedule = calculate_balance_schedule_internal(
        facility.drawn_amount,
        draw_repay_events,
        &payment_dates,
    )?;

    // Step 3: Generate cashflows based on actual balances
    let mut flows = Vec::new();
    
    // Add interest and fee cashflows for each period
    for i in 0..(payment_dates.len() - 1) {
        let period_start = payment_dates[i];
        let period_end = payment_dates[i + 1];
        let balance_start = balance_schedule[i];
        let undrawn_start = facility.commitment_amount.checked_sub(balance_start)?;

        // Calculate accrual factor
        let accrual = facility.day_count.year_fraction(
            period_start,
            period_end,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        // Interest on drawn amount (based on balance at period start)
        match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => {
                let interest = balance_start * (*rate * accrual);
                if interest.amount().abs() > 1e-10 {
                    flows.push(CashFlow {
                        date: period_end,
                        reset_date: None,
                        amount: interest,
                        kind: CFKind::Fixed,
                        accrual_factor: accrual,
                    });
                }
            }
            BaseRateSpec::Floating { margin_bp, .. } => {
                // For floating, include forward-looking base rate if market is provided,
                // otherwise fall back to margin-only (legacy behavior).
                let mut coupon_rate = (*margin_bp * 1e-4);
                if let Some(market) = market_opt {
                    // Resolve forward curve from index_id
                    if let BaseRateSpec::Floating { index_id, .. } = &facility.base_rate_spec {
                        if let Ok(fwd) = market.get_forward_ref(index_id.as_str()) {
                            // Use period start as reset date (reset frequency is typically aligned
                            // with payment frequency in this deterministic path).
                            let t_reset = fwd
                                .day_count()
                                .year_fraction(
                                    fwd.base_date(),
                                    period_start,
                                    finstack_core::dates::DayCountCtx::default(),
                                )
                                .unwrap_or(0.0);
                            let base_rate = fwd.rate(t_reset).max(0.0);
                            coupon_rate += base_rate;
                        }
                    }
                }
                let interest = balance_start * (coupon_rate * accrual);
                if interest.amount().abs() > 1e-10 {
                    flows.push(CashFlow {
                        date: period_end,
                        reset_date: Some(period_start),
                        amount: interest,
                        kind: CFKind::FloatReset,
                        accrual_factor: accrual,
                    });
                }
            }
        }

        // Commitment fee on undrawn amount
        if facility.fees.commitment_fee_bp > 0.0 {
            let commitment_fee = undrawn_start * (facility.fees.commitment_fee_bp * 1e-4 * accrual);
            if commitment_fee.amount().abs() > 1e-10 {
                flows.push(CashFlow {
                    date: period_end,
                    reset_date: None,
                    amount: commitment_fee,
                    kind: CFKind::Fee,
                    accrual_factor: accrual,
                });
            }
        }

        // Usage fee on drawn amount
        if facility.fees.usage_fee_bp > 0.0 {
            let usage_fee = balance_start * (facility.fees.usage_fee_bp * 1e-4 * accrual);
            if usage_fee.amount().abs() > 1e-10 {
                flows.push(CashFlow {
                    date: period_end,
                    reset_date: None,
                    amount: usage_fee,
                    kind: CFKind::Fee,
                    accrual_factor: accrual,
                });
            }
        }

        // Facility fee on total commitment
        if facility.fees.facility_fee_bp > 0.0 {
            let facility_fee = facility.commitment_amount * (facility.fees.facility_fee_bp * 1e-4 * accrual);
            if facility_fee.amount().abs() > 1e-10 {
                flows.push(CashFlow {
                    date: period_end,
                    reset_date: None,
                    amount: facility_fee,
                    kind: CFKind::Fee,
                    accrual_factor: accrual,
                });
            }
        }
    }

    // Step 4: Add principal flows from draw/repay events
    for event in draw_repay_events {
        if event.date > as_of {
            flows.push(CashFlow {
                date: event.date,
                reset_date: None,
                // From lender perspective: draw is negative (deployment), repay is positive (receipt)
                amount: if event.is_draw {
                    event.amount * -1.0
                } else {
                    event.amount
                },
                kind: CFKind::Notional,
                accrual_factor: 0.0,
            });
        }
    }

    // Step 5: Add terminal repayment (if balance outstanding at maturity)
    let final_balance = balance_schedule.last().copied().unwrap_or(facility.drawn_amount);
    if final_balance.amount() > 1e-6 {
        flows.push(CashFlow {
            date: facility.maturity_date,
            reset_date: None,
            amount: final_balance,
            kind: CFKind::Notional,
            accrual_factor: 0.0,
        });
    }

    // Sort flows by date and kind
    flows.sort_by(|a, b| {
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

    // Create schedule with flows
    // For revolving credit, notional represents the initial drawn amount (not commitment)
    use crate::cashflow::primitives::Notional;
    Ok(CashFlowSchedule {
        flows,
        notional: Notional::par(facility.drawn_amount.amount(), facility.drawn_amount.currency()),
        day_count: facility.day_count,
        meta: Default::default(),
    })
}

/// Calculate the outstanding balance schedule at each payment date.
///
/// Processes draw/repay events chronologically to determine the balance
/// at the start of each period.
///
/// # Arguments
/// * `initial_drawn` - Initial drawn amount
/// * `events` - Chronologically ordered draw/repay events
/// * `payment_dates` - Payment schedule dates
///
/// # Returns
/// Vector of balances at the start of each period (same length as payment_dates)
fn calculate_balance_schedule_internal(
    initial_drawn: Money,
    events: &[super::types::DrawRepayEvent],
    payment_dates: &[Date],
) -> Result<Vec<Money>> {
    let mut balances = Vec::with_capacity(payment_dates.len());
    let mut current_balance = initial_drawn;
    
    // Sort events by date (should already be sorted, but ensure it)
    let mut sorted_events: Vec<_> = events.iter().collect();
    sorted_events.sort_by_key(|e| e.date);
    
    let mut event_idx = 0;
    
    for &date in payment_dates {
        // Apply all events that occur before this date
        while event_idx < sorted_events.len() && sorted_events[event_idx].date < date {
            let event = sorted_events[event_idx];
            if event.is_draw {
                current_balance = current_balance.checked_add(event.amount)?;
            } else {
                current_balance = current_balance.checked_sub(event.amount)?;
            }
            event_idx += 1;
        }
        
        balances.push(current_balance);
    }
    
    Ok(balances)
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

    #[test]
    fn test_balance_schedule_with_events() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let q1_end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
        let q2_end = Date::from_calendar_date(2025, Month::July, 1).unwrap();
        let q3_end = Date::from_calendar_date(2025, Month::October, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let initial_drawn = Money::new(5_000_000.0, Currency::USD);
        let draw_amount = Money::new(2_000_000.0, Currency::USD);
        let repay_amount = Money::new(1_000_000.0, Currency::USD);

        let events = vec![
            DrawRepayEvent {
                date: Date::from_calendar_date(2025, Month::February, 15).unwrap(),
                amount: draw_amount,
                is_draw: true,
            },
            DrawRepayEvent {
                date: Date::from_calendar_date(2025, Month::May, 15).unwrap(),
                amount: repay_amount,
                is_draw: false,
            },
        ];

        let payment_dates = vec![start, q1_end, q2_end, q3_end, end];

        let balances =
            calculate_balance_schedule_internal(initial_drawn, &events, &payment_dates).unwrap();

        // Q1 (start): initial = 5M
        assert_eq!(balances[0].amount(), 5_000_000.0);

        // Q1 end (Apr 1): draw occurred Feb 15, so 5M + 2M = 7M
        assert_eq!(balances[1].amount(), 7_000_000.0);

        // Q2 end (Jul 1): repay occurred May 15, so 7M - 1M = 6M
        assert_eq!(balances[2].amount(), 6_000_000.0);

        // Q3 end and maturity: no more events, stays at 6M
        assert_eq!(balances[3].amount(), 6_000_000.0);
        assert_eq!(balances[4].amount(), 6_000_000.0);
    }

    #[test]
    fn test_principal_flows_have_correct_signs() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();
        let repay_date = Date::from_calendar_date(2025, Month::June, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-002".into())
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

        let schedule = generate_deterministic_cashflows(&facility, start).unwrap();

        // Find principal flows
        let principal_flows: Vec<_> = schedule
            .flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Notional)
            .collect();

        // Should have 3 principal flows: draw (negative), repay (positive), terminal (positive)
        assert_eq!(principal_flows.len(), 3);

        // Draw should be negative (lender deploys capital)
        let draw_flow = principal_flows.iter().find(|cf| cf.date == draw_date).unwrap();
        assert!(
            draw_flow.amount.amount() < 0.0,
            "Draw should be negative (lender deploys capital)"
        );
        assert_eq!(draw_flow.amount.amount(), -2_000_000.0);

        // Repay should be positive (lender receives capital back)
        let repay_flow = principal_flows.iter().find(|cf| cf.date == repay_date).unwrap();
        assert!(
            repay_flow.amount.amount() > 0.0,
            "Repayment should be positive (lender receives capital)"
        );
        assert_eq!(repay_flow.amount.amount(), 1_000_000.0);

        // Terminal repayment should be positive (remaining balance)
        let terminal_flow = principal_flows.iter().find(|cf| cf.date == end).unwrap();
        assert!(terminal_flow.amount.amount() > 0.0);
        // Balance at maturity: 5M initial + 2M draw - 1M repay = 6M
        assert_eq!(terminal_flow.amount.amount(), 6_000_000.0);
    }

    #[test]
    fn test_no_upfront_fee_in_schedule() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-003".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(super::super::types::RevolvingCreditFees {
                upfront_fee: Some(Money::new(50_000.0, Currency::USD)),
                commitment_fee_bp: 25.0,
                usage_fee_bp: 10.0,
                facility_fee_bp: 5.0,
            })
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let schedule = generate_deterministic_cashflows(&facility, start).unwrap();

        // Upfront fee should NOT be in the schedule (handled at pricer level)
        let has_upfront = schedule
            .flows
            .iter()
            .any(|cf| cf.amount == Money::new(50_000.0, Currency::USD));
        
        assert!(!has_upfront, "Upfront fee should not be in cashflow schedule");
    }
}
