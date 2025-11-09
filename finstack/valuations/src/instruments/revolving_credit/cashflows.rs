//! Cashflow generation for revolving credit facilities.
//!
//! Provides deterministic cashflow generation with:
//! - Interest on drawn amounts (fixed or floating)
//! - Commitment fees on undrawn amounts
//! - Usage fees on drawn amounts
//! - Facility fees on total commitment
//! - Upfront fees

use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::builder::CashFlowSchedule;
use crate::cashflow::primitives::{CFKind, CashFlow};

use super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCredit};
use finstack_core::config::{RoundingContext, ZeroKind};

// Use centralized rounding context thresholds instead of magic numbers.

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
/// # Arguments and as_of rules
/// * `facility` - The revolving credit facility
/// * `as_of` - Valuation date
///   - Non‑principal cashflows (interest/fees) are included only if `payment_end > as_of`
///   - Principal events (draw/repay) are included only if `event.date > as_of`
///   - Initial draw is included only if `commitment_date > as_of`
///   - Terminal repayment is included only if `maturity_date > as_of`
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
            return Err(finstack_core::Error::Validation(
                "Deterministic cashflows require DrawRepaySpec::Deterministic".to_string(),
            ));
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
        return Err(finstack_core::error::InputError::TooFewPoints.into());
    }

    // Step 2: Build reset date grid for floating rates (if applicable)
    let reset_dates: Option<Vec<Date>> = match &facility.base_rate_spec {
        BaseRateSpec::Floating { reset_freq, .. } => {
            // Build reset schedule from commitment_date to maturity_date
            let mut reset_builder = ScheduleBuilder::new(facility.commitment_date, facility.maturity_date)
                .frequency(*reset_freq)
                .stub_rule(finstack_core::dates::StubKind::None);
            
            if let Some(cal_code) = facility
                .attributes
                .get_meta("calendar_id")
                .or_else(|| facility.attributes.get_meta("calendar"))
            {
                if let Some(cal) = finstack_core::dates::CalendarRegistry::global().resolve_str(cal_code) {
                    reset_builder = reset_builder.adjust_with(
                        finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                        cal,
                    );
                }
            }
            
            Some(reset_builder.build()?.into_iter().collect())
        }
        BaseRateSpec::Fixed { .. } => None,
    };

    // Step 3: Generate cashflows based on actual balances with intra-period event slicing
    let mut flows = Vec::new();
    // Rounding context for zero checks and currency scale
    let rc = RoundingContext::default();
    let ccy = facility.commitment_amount.currency();

    // Add initial draw at commitment_date (from lender perspective: negative cashflow - capital deployment)
    // Include if commitment_date is after as_of (future cashflow). If commitment_date == as_of,
    // the draw happens "today" and should be handled separately in the pricer.
    if facility.commitment_date > as_of
        && !rc.is_effectively_zero(facility.drawn_amount.amount(), ZeroKind::Money(ccy))
    {
        flows.push(CashFlow {
            date: facility.commitment_date,
            reset_date: None,
            amount: facility.drawn_amount * -1.0,
            kind: CFKind::Notional,
            accrual_factor: 0.0,
        });
    }

    // Step 4: Generate interest and fee cashflows with intra-period event slicing
    // Reserve a reasonable capacity: 4 flow types per period + events + terminal
    flows.reserve((payment_dates.len().saturating_sub(1)) * 4 + draw_repay_events.len() + 2);
    
    for i in 0..(payment_dates.len() - 1) {
        let period_start = payment_dates[i];
        let period_end = payment_dates[i + 1];

        // Apply as_of filtering for non-principal cashflows: only future-dated payments
        if period_end <= as_of {
            continue;
        }

        // Build sub-period timeline: [period_start, events in (start, end], period_end]
        let mut timeline = vec![period_start];
        
        // Add events that occur strictly within (period_start, period_end]
        for event in draw_repay_events {
            if event.date > period_start && event.date <= period_end {
                timeline.push(event.date);
            }
        }
        timeline.push(period_end);
        timeline.sort();
        timeline.dedup();

        // Track balance through sub-periods
        let mut current_balance = if i == 0 {
            facility.drawn_amount
        } else {
            // Apply all events up to period_start to get starting balance
            let mut balance = facility.drawn_amount;
            for event in draw_repay_events {
                if event.date <= period_start {
                    balance = if event.is_draw {
                        balance.checked_add(event.amount)?
                    } else {
                        balance.checked_sub(event.amount)?
                    };
                }
            }
            balance
        };

        // Accumulators for aggregated accruals
        let mut total_interest = Money::new(0.0, ccy);
        let mut total_commitment_fee = Money::new(0.0, ccy);
        let mut total_usage_fee = Money::new(0.0, ccy);
        let mut total_facility_fee = Money::new(0.0, ccy);
        let mut total_accrual = 0.0;
        let mut reset_date_opt: Option<Date> = None;

        // Process each sub-period
        for window in timeline.windows(2) {
            let sub_start = window[0];
            let sub_end = window[1];

            // Calculate sub-period accrual
            let dt = facility.day_count.year_fraction(
                sub_start,
                sub_end,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            total_accrual += dt;

            let current_undrawn = facility.commitment_amount.checked_sub(current_balance)?;
            let utilization = if facility.commitment_amount.amount() > 0.0 {
                current_balance.amount() / facility.commitment_amount.amount()
            } else {
                0.0
            };

            // Determine reset date for floating rates
            let sub_reset_date = match &facility.base_rate_spec {
                BaseRateSpec::Floating { .. } => {
                    if let Some(ref reset_grid) = reset_dates {
                        // Find most recent reset date <= sub_start
                        reset_grid
                            .iter()
                            .rev()
                            .find(|&&d| d <= sub_start)
                            .copied()
                            .or(Some(period_start))
                    } else {
                        Some(period_start)
                    }
                }
                BaseRateSpec::Fixed { .. } => None,
            };
            
            // Track reset date for cashflow (use first sub-period's reset date)
            if reset_date_opt.is_none() {
                reset_date_opt = sub_reset_date;
            }

            // Calculate interest for this sub-period
            match &facility.base_rate_spec {
                BaseRateSpec::Fixed { rate } => {
                    let interest = current_balance * (*rate * dt);
                    total_interest = total_interest.checked_add(interest)?;
                }
                BaseRateSpec::Floating { margin_bp, index_id, reset_freq, floor_bp, .. } => {
                    let mut coupon_rate = *margin_bp * 1e-4;
                    if let Some(market) = market_opt {
                        if let Ok(fwd) = market.get_forward_ref(index_id.as_str()) {
                            if let Some(reset_d) = sub_reset_date {
                                // Compute period forward for the reset window using forward curve basis
                                let fwd_dc = fwd.day_count();
                                let fwd_base = fwd.base_date();
                                let t0 = fwd_dc.year_fraction(
                                    fwd_base,
                                    reset_d,
                                    finstack_core::dates::DayCountCtx::default(),
                                )?;
                                
                                // Compute reset period end date using reset_freq and facility calendar
                                let mut reset_end = reset_d;
                                match reset_freq {
                                    Frequency::Months(m) => {
                                        reset_end = finstack_core::dates::utils::add_months(reset_d, *m as i32);
                                    }
                                    Frequency::Days(d) => {
                                        reset_end = reset_d + time::Duration::days(*d as i64);
                                    }
                                    _ => {}
                                }
                                
                                // Apply calendar adjustment if configured
                                if let Some(cal_code) = facility.attributes.get_meta("calendar_id")
                                    .or_else(|| facility.attributes.get_meta("calendar")) {
                                    if let Some(cal) = finstack_core::dates::CalendarRegistry::global().resolve_str(cal_code) {
                                        reset_end = finstack_core::dates::adjust(
                                            reset_end,
                                            finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                                            cal,
                                        )?;
                                    }
                                }
                                
                                let t1 = fwd_dc.year_fraction(
                                    fwd_base,
                                    reset_end,
                                    finstack_core::dates::DayCountCtx::default(),
                                )?;
                                
                                // Get period forward rate
                                let mut base_rate = fwd.rate_period(t0, t1);
                                
                                // Apply floor to base rate only (before adding margin)
                                if let Some(floor) = floor_bp {
                                    let floor_rate = floor * 1e-4;
                                    base_rate = base_rate.max(floor_rate);
                                }
                                
                                coupon_rate += base_rate;
                            }
                        }
                    }
                    let interest = current_balance * (coupon_rate * dt);
                    total_interest = total_interest.checked_add(interest)?;
                }
            }

            // Calculate fees for this sub-period (evaluating tiers based on utilization)
            let commitment_fee_bps = facility.fees.commitment_fee_bps(utilization);
            if commitment_fee_bps > 0.0 {
                let commitment_fee = current_undrawn * (commitment_fee_bps * 1e-4 * dt);
                total_commitment_fee = total_commitment_fee.checked_add(commitment_fee)?;
            }

            let usage_fee_bps = facility.fees.usage_fee_bps(utilization);
            if usage_fee_bps > 0.0 {
                let usage_fee = current_balance * (usage_fee_bps * 1e-4 * dt);
                total_usage_fee = total_usage_fee.checked_add(usage_fee)?;
            }

            if facility.fees.facility_fee_bp > 0.0 {
                let facility_fee = facility.commitment_amount * (facility.fees.facility_fee_bp * 1e-4 * dt);
                total_facility_fee = total_facility_fee.checked_add(facility_fee)?;
            }

            // Apply events that occur at sub_end to update balance for next sub-period
            for event in draw_repay_events {
                if event.date == sub_end {
                    current_balance = if event.is_draw {
                        let new_balance = current_balance.checked_add(event.amount)?;
                        // Validate draw does not exceed commitment
                        if new_balance.amount() > facility.commitment_amount.amount() {
                            return Err(finstack_core::Error::Validation(format!(
                                "Draw on {} would exceed commitment: {} > {}",
                                event.date, new_balance, facility.commitment_amount
                            )));
                        }
                        new_balance
                    } else {
                        current_balance.checked_sub(event.amount)?
                    };
                }
            }
        }

        // Post aggregated cashflows at period_end
        if !rc.is_effectively_zero_money(total_interest.amount(), ccy) {
            flows.push(CashFlow {
                date: period_end,
                reset_date: reset_date_opt,
                amount: total_interest,
                kind: match &facility.base_rate_spec {
                    BaseRateSpec::Fixed { .. } => CFKind::Fixed,
                    BaseRateSpec::Floating { .. } => CFKind::FloatReset,
                },
                accrual_factor: total_accrual,
            });
        }

        if !rc.is_effectively_zero_money(total_commitment_fee.amount(), ccy) {
            flows.push(CashFlow {
                date: period_end,
                reset_date: None,
                amount: total_commitment_fee,
                kind: CFKind::Fee,
                accrual_factor: total_accrual,
            });
        }

        if !rc.is_effectively_zero_money(total_usage_fee.amount(), ccy) {
            flows.push(CashFlow {
                date: period_end,
                reset_date: None,
                amount: total_usage_fee,
                kind: CFKind::Fee,
                accrual_factor: total_accrual,
            });
        }

        if !rc.is_effectively_zero_money(total_facility_fee.amount(), ccy) {
            flows.push(CashFlow {
                date: period_end,
                reset_date: None,
                amount: total_facility_fee,
                kind: CFKind::Fee,
                accrual_factor: total_accrual,
            });
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
    // Calculate final balance by applying all events strictly before maturity
    let mut final_balance = facility.drawn_amount;
    for event in draw_repay_events {
        if event.date < facility.maturity_date {
            final_balance = if event.is_draw {
                final_balance.checked_add(event.amount)?
            } else {
                final_balance.checked_sub(event.amount)?
            };
        }
    }
    // Apply same-day maturity events to the final balance for terminal calculation context only.
    // This adjusts the balance that would be outstanding after maturity-day events are processed.
    let mut final_balance_for_terminal = final_balance;
    for event in draw_repay_events {
        if event.date == facility.maturity_date {
            final_balance_for_terminal = if event.is_draw {
                final_balance_for_terminal.checked_add(event.amount)?
            } else {
                final_balance_for_terminal.checked_sub(event.amount)?
            };
        }
    }
    // Only add terminal repayment if there's a balance remaining after all maturity-day events
    if facility.maturity_date > as_of
        && !rc.is_effectively_zero(final_balance_for_terminal.amount(), ZeroKind::Money(ccy))
    {
        flows.push(CashFlow {
            date: facility.maturity_date,
            reset_date: None,
            amount: final_balance_for_terminal,
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
        notional: Notional::par(
            facility.drawn_amount.amount(),
            facility.drawn_amount.currency(),
        ),
        day_count: facility.day_count,
        meta: Default::default(),
    })
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
            return Err(finstack_core::Error::Validation(
                "calculate_drawn_balance_at_date requires DrawRepaySpec::Deterministic".to_string(),
            ));
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
            .fees({
                let mut fees = super::super::types::RevolvingCreditFees::flat(25.0, 10.0, 5.0);
                fees.upfront_fee = Some(Money::new(50_000.0, Currency::USD));
                fees
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

        // Should have 3 principal flows: draw event (negative), repay (positive), terminal (positive)
        // Note: Initial draw at commitment_date is not included when commitment_date == as_of
        // (it happens "today" and is handled separately in the pricer)
        assert_eq!(principal_flows.len(), 3);

        // Draw should be negative (lender deploys capital)
        let draw_flow = principal_flows
            .iter()
            .find(|cf| cf.date == draw_date)
            .unwrap();
        assert!(
            draw_flow.amount.amount() < 0.0,
            "Draw should be negative (lender deploys capital)"
        );
        assert_eq!(draw_flow.amount.amount(), -2_000_000.0);

        // Repay should be positive (lender receives capital back)
        let repay_flow = principal_flows
            .iter()
            .find(|cf| cf.date == repay_date)
            .unwrap();
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
            .fees({
                let mut fees = super::super::types::RevolvingCreditFees::flat(25.0, 10.0, 5.0);
                fees.upfront_fee = Some(Money::new(50_000.0, Currency::USD));
                fees
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

        assert!(
            !has_upfront,
            "Upfront fee should not be in cashflow schedule"
        );
    }

    #[test]
    fn test_as_of_filters_non_principal_flows() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let q1_end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-004".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(Default::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        // As-of at Q1 end: no interest/fee cashflows should have date <= as_of
        let schedule = generate_deterministic_cashflows(&facility, q1_end).unwrap();
        assert!(
            schedule
                .flows
                .iter()
                .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::FloatReset | CFKind::Fee))
                .all(|cf| cf.date > q1_end),
            "Non-principal flows should be strictly after as_of"
        );
    }

    #[test]
    fn test_negative_forward_rates_are_respected() {
        use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
        use finstack_core::market_data::MarketContext;

        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Build a simple forward curve with small positive rates
        let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
            .base_date(start)
            .knots([(0.0, 0.0010), (1.0, 0.0010)])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_forward(fwd);

        let facility = RevolvingCredit::builder()
            .id("RC-005".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Floating {
                index_id: "USD-SOFR-3M".into(),
                // Negative margin big enough to make net coupon negative
                margin_bp: -20.0,
                reset_freq: Frequency::quarterly(),
                floor_bp: None,
            })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(Default::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let schedule =
            generate_deterministic_cashflows_with_curves(&facility, &market, start).unwrap();

        // Expect at least one negative interest cashflow (FloatReset)
        let has_negative_interest = schedule.flows.iter().any(|cf| {
            cf.kind == CFKind::FloatReset && cf.amount.amount() < 0.0
        });
        assert!(
            has_negative_interest,
            "Negative net coupons (from margin + forward) should produce negative interest flows"
        );
    }

    #[test]
    fn test_maturity_day_event_no_double_count_terminal() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Start drawn 5M, repay full 5M on maturity date → terminal should be suppressed
        let facility = RevolvingCredit::builder()
            .id("RC-006".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(Default::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![DrawRepayEvent {
                date: end,
                amount: Money::new(5_000_000.0, Currency::USD),
                is_draw: false,
            }]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let schedule = generate_deterministic_cashflows(&facility, start).unwrap();

        // Count notional flows at maturity: should be only the repayment event, no extra terminal
        let maturity_flows: Vec<_> = schedule
            .flows
            .iter()
            .filter(|cf| cf.date == end && cf.kind == CFKind::Notional)
            .collect();
        assert_eq!(
            maturity_flows.len(),
            1,
            "Should not double-count terminal repayment when full repay occurs on maturity date"
        );
        assert!(
            maturity_flows[0].amount.amount() > 0.0,
            "Maturity notional flow should be the explicit repayment (positive to lender)"
        );
    }
}
