//! Cashflow generation for Term Loans.
//!
//! This module generates deterministic cashflow schedules for term loans including:
//! - DDTL draws and commitment/usage fees
//! - Interest (fixed or floating with floor/cap)
//! - Amortization (linear, custom, or percent-per-period)
//! - PIK capitalization and cash sweeps
//! - Covenant-driven events
//!
//! # Conventions
//!
//! ## Internal Engine View vs Holder View
//!
//! This module produces a **full internal schedule** (`CashFlowSchedule`) that includes:
//! - Funding legs (draws) as **negative** `CFKind::Notional` flows
//! - Redemptions (principal repayments) as **positive** `CFKind::Notional` flows
//! - Amortization as **positive** `CFKind::Amortization` flows (economically reduce outstanding)
//! - PIK as **positive** `CFKind::PIK` flows (economically increase outstanding)
//! - Interest as `CFKind::Fixed` or `CFKind::FloatReset`
//! - Fees as `CFKind::Fee`
//!
//! The **holder view** (via `CashflowProvider::build_schedule`) filters this schedule
//! to expose only contractual inflows to a long lender: coupons, amortization, and
//! positive redemptions, excluding funding legs and PIK capitalization.
//!
//! ## Sign Conventions
//!
//! - `Notional.initial` is set to **0** for term loans (funding-leg modelling).
//! - Draws (funding) are **negative** notional flows (cash out from lender's perspective).
//! - Redemptions are **positive** notional flows (cash in to lender).
//! - Outstanding principal is computed via `outstanding_by_date_including_notional()`.

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::builder::Notional;
use crate::cashflow::traits::DatedFlows;
use finstack_core::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::types::TermLoan;

/// Compute total margin (base spread + covenant step-ups + pricing overrides) at a given date.
///
/// This helper unifies margin calculation across cashflow generation and metrics.
pub(super) fn margin_bp_at(loan: &TermLoan, d: Date) -> f64 {
    let base_margin = match &loan.rate {
        super::types::RateSpec::Fixed { .. } => 0.0,
        super::types::RateSpec::Floating(spec) => spec.spread_bp,
    };
    let step = loan
        .covenants
        .as_ref()
        .map(|c| {
            c.margin_stepups
                .iter()
                .filter(|m| m.date <= d)
                .map(|m| m.delta_bp as f64)
                .sum::<f64>()
        })
        .unwrap_or(0.0);
    let override_add = loan
        .pricing_overrides
        .term_loan
        .as_ref()
        .map(|ov| {
            ov.margin_add_bp_by_date
                .iter()
                .filter(|(dt, _)| *dt <= d)
                .map(|(_, bp)| *bp as f64)
                .sum::<f64>()
        })
        .unwrap_or(0.0);
    base_margin + step + override_add
}

/// Internal representation of a principal event (draw, amortization, PIK, sweep).
#[derive(Clone, Debug)]
struct PrincipalEvent {
    date: Date,
    /// Delta to outstanding principal: positive = increase, negative = decrease
    delta: Money,
}

/// Compute outstanding principal at a given date by folding all principal events up to that date.
///
/// This function processes draws, amortization, PIK, and sweeps in chronological order
/// to determine the outstanding balance at the target date. This is a **forward-looking**
/// calculation that applies all principal events up to `target_date`, regardless of `as_of`.
///
/// The `as_of` date is used only for filtering which cashflows are included in PV calculations,
/// not for truncating the principal evolution path.
fn compute_outstanding_at(
    events: &[PrincipalEvent],
    target_date: Date,
    currency: finstack_core::currency::Currency,
) -> finstack_core::Result<Money> {
    let mut outstanding = Money::new(0.0, currency);
    
    for event in events {
        // Apply all events up to target_date (forward-looking)
        if event.date <= target_date {
            outstanding = outstanding.checked_add(event.delta)?;
        }
    }
    
    Ok(outstanding)
}

/// Generate the full internal cashflow schedule for a term loan.
///
/// Returns a `CashFlowSchedule` with all flows including funding legs (negative notional),
/// interest, fees, amortization, PIK, and redemptions. Use `build_dated_flows` or
/// `TermLoan::build_schedule` (via `CashflowProvider`) for holder-view cashflows.
pub fn generate_cashflows(
    loan: &TermLoan,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<CashFlowSchedule> {
    let mut flows: Vec<CashFlow> = Vec::new();
    
    // Step 1: Build list of all principal events (draws, sweeps, amortization schedule)
    let mut principal_events: Vec<PrincipalEvent> = Vec::new();

    // Step 1a: Collect draws and emit funding leg flows
    let draw_stop = loan
        .covenants
        .as_ref()
        .and_then(|c| c.draw_stop_dates.iter().min().copied());
    
    if let Some(ddtl) = &loan.ddtl {
        for ev in ddtl.draws.iter() {
            if ev.date < ddtl.availability_start || ev.date > ddtl.availability_end {
                continue;
            }
            if let Some(ds) = draw_stop {
                if ev.date >= ds {
                    continue;
                }
            }
            
            // Apply OID policy to determine cash inflow
            let mut cash_inflow = ev.amount;
            if let Some(oid) = &ddtl.oid_policy {
                match oid {
                    super::spec::OidPolicy::WithheldPct(bp) => {
                        let pct = (*bp as f64) * 1e-4;
                        cash_inflow =
                            Money::new(ev.amount.amount() * (1.0 - pct), ev.amount.currency());
                    }
                    super::spec::OidPolicy::WithheldAmount(m) => {
                        cash_inflow = ev.amount.checked_sub(*m)?;
                    }
                    super::spec::OidPolicy::SeparatePct(bp) => {
                        let pct = (*bp as f64) * 1e-4;
                        let fee_amt = Money::new(ev.amount.amount() * pct, ev.amount.currency());
                        if fee_amt.amount() > 0.0 {
                            flows.push(CashFlow {
                                date: ev.date,
                                reset_date: None,
                                amount: fee_amt,
                                kind: CFKind::Fee,
                                accrual_factor: 0.0,
                                rate: None,
                            });
                        }
                    }
                    super::spec::OidPolicy::SeparateAmount(m) => {
                        if m.amount() > 0.0 {
                            flows.push(CashFlow {
                                date: ev.date,
                                reset_date: None,
                                amount: *m,
                                kind: CFKind::Fee,
                                accrual_factor: 0.0,
                                rate: None,
                            });
                        }
                    }
                }
            }
            
            // Funding outflow from lender (negative notional = draw)
            if cash_inflow.amount() != 0.0 {
                flows.push(CashFlow {
                    date: ev.date,
                    reset_date: None,
                    amount: Money::new(-cash_inflow.amount(), cash_inflow.currency()),
                    kind: CFKind::Notional,
                    accrual_factor: 0.0,
                    rate: None,
                });
            }
            
            // Record principal event: draw increases outstanding
            principal_events.push(PrincipalEvent {
                date: ev.date,
                delta: ev.amount,
            });
        }
    } else {
        // Plain term loan: funding at issue (negative notional = draw)
        if loan.notional_limit.amount() != 0.0 {
            flows.push(CashFlow {
                date: loan.issue,
                reset_date: None,
                amount: Money::new(-loan.notional_limit.amount(), loan.notional_limit.currency()),
                kind: CFKind::Notional,
                accrual_factor: 0.0,
                rate: None,
            });
            
            // Record principal event: initial draw
            principal_events.push(PrincipalEvent {
                date: loan.issue,
                delta: loan.notional_limit,
            });
        }
    }
    
    // Upfront fee at issue (if any)
    if let Some(fee) = loan.upfront_fee {
        if fee.amount() > 0.0 {
            flows.push(CashFlow {
                date: loan.issue,
                reset_date: None,
                amount: fee,
                kind: CFKind::Fee,
                accrual_factor: 0.0,
                rate: None,
            });
        }
    }
    
    // Step 1b: Add cash sweeps and covenant sweeps to principal events
    if let Some(cov) = &loan.covenants {
        for sweep in &cov.cash_sweeps {
            if sweep.amount.amount() > 0.0 {
                principal_events.push(PrincipalEvent {
                    date: sweep.date,
                    delta: Money::new(-sweep.amount.amount(), sweep.amount.currency()),
                });
            }
        }
    }
    if let Some(ov) = &loan.pricing_overrides.term_loan {
        for (dt, amt) in &ov.extra_cash_sweeps {
            if amt.amount() > 0.0 {
                principal_events.push(PrincipalEvent {
                    date: *dt,
                    delta: Money::new(-amt.amount(), amt.currency()),
                });
            }
        }
    }
    
    // Step 1c: Sort principal events by date for correct ordering
    principal_events.sort_by_key(|e| e.date);

    // Build coupon dates using payment frequency, BDC, and calendar
    let mut schedule_builder = finstack_core::dates::ScheduleBuilder::new(loan.issue, loan.maturity)
        .frequency(loan.pay_freq)
        .stub_rule(loan.stub);
    
    // Apply business-day adjustment if calendar is specified
    if let Some(ref cal_id) = loan.calendar_id {
        schedule_builder = schedule_builder.adjust_with_id(loan.bdc, cal_id);
    }
    
    let schedule = schedule_builder.build()?;
    let mut dates: Vec<Date> = schedule.into_iter().collect();
    if dates.first().copied() != Some(loan.issue) {
        dates.insert(0, loan.issue);
    }

    // Step 2: Interest and fees per period end (using time-dependent outstanding)
    let dc = loan.day_count;
    let mut prev = dates[0];
    for &d in dates.iter().skip(1) {
        if d <= as_of {
            prev = d;
            continue;
        }
        let yf = dc.year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;

        // Compute outstanding at the start of this period (respecting draw timing)
        let outstanding = compute_outstanding_at(&principal_events, prev, loan.currency)?;

        // Period rate with centralized projection for floating rates
        let period_rate = match &loan.rate {
            super::types::RateSpec::Fixed { rate_bp } => (*rate_bp as f64) * 1e-4,
            super::types::RateSpec::Floating(spec) => {
                // Use centralized projection with total margin (base + step-ups + overrides)
                let total_spread = margin_bp_at(loan, d);
                crate::cashflow::builder::project_floating_rate_simple(
                    prev,
                    yf,
                    spec.index_id.as_str(),
                    total_spread,
                    spec.gearing,
                    spec.floor_bp,
                    spec.cap_bp,
                    market,
                )?
            }
        };

        // Interest on outstanding (using time-dependent balance)
        let interest_amt = Money::new(outstanding.amount() * period_rate * yf, loan.currency);

        // PIK split using coupon_type and any covenant toggles
        let mut cash_interest = interest_amt;
        let mut pik_interest = Money::new(0.0, loan.currency);
        let force_pik_cov = loan
            .covenants
            .as_ref()
            .map(|c| c.pik_toggles.iter().any(|t| t.date <= d && t.enable_pik))
            .unwrap_or(false);
        let force_pik_ov = loan
            .pricing_overrides
            .term_loan
            .as_ref()
            .map(|ov| ov.pik_toggle_by_date.iter().any(|(dt, en)| *dt <= d && *en))
            .unwrap_or(false);
        let force_pik = force_pik_cov || force_pik_ov;
        match loan.coupon_type {
            crate::cashflow::builder::specs::CouponType::PIK => {
                pik_interest = interest_amt;
                cash_interest = Money::new(0.0, loan.currency);
            }
            crate::cashflow::builder::specs::CouponType::Split { cash_pct, pik_pct } => {
                pik_interest = Money::new(interest_amt.amount() * pik_pct, loan.currency);
                cash_interest = Money::new(interest_amt.amount() * cash_pct, loan.currency);
            }
            _ => {
                if force_pik {
                    pik_interest = interest_amt;
                    cash_interest = Money::new(0.0, loan.currency);
                }
            }
        }
        if cash_interest.amount() != 0.0 {
            flows.push(CashFlow {
                date: d,
                reset_date: None,
                amount: cash_interest,
                kind: CFKind::Fixed,
                accrual_factor: yf,
                rate: Some(period_rate),
            });
        }
        if pik_interest.amount() != 0.0 {
            flows.push(CashFlow {
                date: d,
                reset_date: None,
                amount: pik_interest,
                kind: CFKind::PIK,
                accrual_factor: yf,
                rate: Some(period_rate),
            });
            // Record PIK as a principal event (increases outstanding)
            principal_events.push(PrincipalEvent {
                date: d,
                delta: pik_interest,
            });
        }

        // Fees if DDTL
        if let Some(ddtl) = &loan.ddtl {
            // Commitment limit with step-downs
            let mut limit = ddtl.commitment_limit;
            for sd in &ddtl.commitment_step_downs {
                if sd.date <= d {
                    limit = sd.new_limit;
                }
            }
            
            // Calculate fee base according to CommitmentFeeBase enum
            // For Undrawn: sum all draws up to this date (not implemented here, simplified to use outstanding)
            // For CommitmentMinusOutstanding: use limit - outstanding
            // Note: Both converge to the same value when all draws are fully captured in outstanding
            let undrawn = match ddtl.fee_base {
                super::spec::CommitmentFeeBase::Undrawn => {
                    // Simplified: use limit - outstanding
                    // In a full implementation, this would track total drawn vs outstanding separately
                    (limit.amount() - outstanding.amount()).max(0.0)
                }
                super::spec::CommitmentFeeBase::CommitmentMinusOutstanding => {
                    (limit.amount() - outstanding.amount()).max(0.0)
                }
            };

            // Emit commitment fee using centralized function
            if ddtl.commitment_fee_bp != 0 {
                flows.extend(crate::cashflow::builder::emit_commitment_fee_on(
                    d,
                    undrawn,
                    ddtl.commitment_fee_bp as f64,
                    yf,
                    loan.currency,
                ));
            }

            // Emit usage fee using centralized function
            if ddtl.usage_fee_bp != 0 {
                flows.extend(crate::cashflow::builder::emit_usage_fee_on(
                    d,
                    outstanding.amount(),
                    ddtl.usage_fee_bp as f64,
                    yf,
                    loan.currency,
                ));
            }
        }

        // Cash sweeps (already in principal_events; just emit flows)
        if let Some(cov) = &loan.covenants {
            for sweep in cov.cash_sweeps.iter().filter(|s| s.date == d) {
                if sweep.amount.amount() > 0.0 {
                    flows.push(CashFlow {
                        date: d,
                        reset_date: None,
                        amount: sweep.amount,
                        kind: CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    });
                }
            }
        }
        if let Some(ov) = &loan.pricing_overrides.term_loan {
            for (dt, amt) in ov.extra_cash_sweeps.iter().filter(|(dt, _)| *dt == d) {
                if amt.amount() > 0.0 {
                    flows.push(CashFlow {
                        date: *dt,
                        reset_date: None,
                        amount: *amt,
                        kind: CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    });
                }
            }
        }

        // Amortization spec (compute using current outstanding including PIK just added)
        let current_outstanding = compute_outstanding_at(&principal_events, d, loan.currency)?;
        
        match &loan.amortization {
            super::spec::AmortizationSpec::None => {}
            super::spec::AmortizationSpec::Custom(items) => {
                for (adt, amt) in items.iter().filter(|(adt, _)| *adt == d) {
                    let pay = Money::new(amt.amount().min(current_outstanding.amount()), loan.currency);
                    if pay.amount() > 0.0 {
                        flows.push(CashFlow {
                            date: *adt,
                            reset_date: None,
                            amount: pay,
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        // Record as principal event (decreases outstanding)
                        principal_events.push(PrincipalEvent {
                            date: *adt,
                            delta: Money::new(-pay.amount(), pay.currency()),
                        });
                    }
                }
            }
            super::spec::AmortizationSpec::PercentPerPeriod { bp } => {
                let pct = (*bp as f64) * 1e-4;
                let pay = Money::new(
                    (loan.notional_limit.amount() * pct).min(current_outstanding.amount()),
                    loan.currency,
                );
                if pay.amount() > 0.0 {
                    flows.push(CashFlow {
                        date: d,
                        reset_date: None,
                        amount: pay,
                        kind: CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    });
                    // Record as principal event
                    principal_events.push(PrincipalEvent {
                        date: d,
                        delta: Money::new(-pay.amount(), pay.currency()),
                    });
                }
            }
            super::spec::AmortizationSpec::Linear { start, end } => {
                if d >= *start && d <= *end {
                    // Count remaining coupon dates including current within [start,end]
                    let remaining = dates
                        .iter()
                        .filter(|&&dt| dt >= d && dt <= *end)
                        .count()
                        .max(1);
                    let pay_amt =
                        (current_outstanding.amount() / (remaining as f64)).min(current_outstanding.amount());
                    let pay = Money::new(pay_amt, loan.currency);
                    if pay.amount() > 0.0 {
                        flows.push(CashFlow {
                            date: d,
                            reset_date: None,
                            amount: pay,
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        // Record as principal event
                        principal_events.push(PrincipalEvent {
                            date: d,
                            delta: Money::new(-pay.amount(), pay.currency()),
                        });
                    }
                }
            }
        }

        prev = d;
    }

    // Final redemption of remaining principal at maturity (positive = inflow to lender)
    let final_outstanding = compute_outstanding_at(&principal_events, loan.maturity, loan.currency)?;
    if final_outstanding.amount() > 0.0 {
        flows.push(CashFlow {
            date: loan.maturity,
            reset_date: None,
            amount: final_outstanding,
            kind: CFKind::Notional,
            accrual_factor: 0.0,
            rate: None,
        });
    }

    // Sort by date then kind rank (match builder's ordering)
    let rank = |k: CFKind| match k {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
        _ => 5,
    };
    flows.sort_by(|a, b| a.date.cmp(&b.date).then(rank(a.kind).cmp(&rank(b.kind))));

    let schedule = CashFlowSchedule {
        flows,
        notional: Notional::par(0.0, loan.currency), // Funding-leg modelling: initial = 0
        day_count: loan.day_count, // Use instrument's day-count convention
        meta: crate::cashflow::builder::schedule::CashflowMeta::default(),
    };
    Ok(schedule)
}

/// Convenience: build simple dated flows (no CFKind) from full schedule.
pub fn build_dated_flows(schedule: &CashFlowSchedule) -> DatedFlows {
    schedule
        .flows
        .iter()
        .map(|cf| (cf.date, cf.amount))
        .collect()
}
