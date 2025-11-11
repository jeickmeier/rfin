//! Cashflow generation for Term Loans (placeholder for v1 wiring).

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::primitives::{CFKind, CashFlow, Notional};
use crate::cashflow::traits::DatedFlows;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::types::TermLoan;
// use crate::instruments::pricing_overrides::TermLoanOverrides; // not directly used here

/// Build a minimal deterministic schedule for a Term Loan.
///
/// This stub returns an empty schedule for now; subsequent tasks will implement
/// draws, interest, amortization, fees, and PIK per the consolidated plan.
pub fn generate_cashflows(
    loan: &TermLoan,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<CashFlowSchedule> {
    let mut flows: Vec<CashFlow> = Vec::new();

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

    // Simple DDTL draw handling (availability window + optional step-down enforcement)
    let mut outstanding = Money::new(0.0, loan.currency);
    let mut _commitment_limit_opt = None;
    if let Some(ddtl) = &loan.ddtl {
        // Earliest draw-stop date from covenants, if any
        let draw_stop = loan
            .covenants
            .as_ref()
            .and_then(|c| c.draw_stop_dates.iter().min().copied());

        for ev in ddtl.draws.iter() {
            if ev.date < ddtl.availability_start || ev.date > ddtl.availability_end {
                continue;
            }
            if let Some(ds) = draw_stop {
                if ev.date >= ds {
                    continue;
                }
            }
            // Apply OID policy
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
            // Funding inflow to borrower
            if cash_inflow.amount() != 0.0 {
                flows.push(CashFlow {
                    date: ev.date,
                    reset_date: None,
                    amount: cash_inflow,
                    kind: CFKind::Notional,
                    accrual_factor: 0.0,
                    rate: None,
                });
            }
            // Principal increases by full draw amount
            outstanding = outstanding.checked_add(ev.amount)?;
        }
        _commitment_limit_opt = Some(ddtl.commitment_limit);
    } else {
        // Plain term loan: treat as fully funded at issue
        if loan.notional_limit.amount() != 0.0 {
            flows.push(CashFlow {
                date: loan.issue,
                reset_date: None,
                amount: loan.notional_limit,
                kind: CFKind::Notional,
                accrual_factor: 0.0,
                rate: None,
            });
        }
        outstanding = outstanding.checked_add(loan.notional_limit)?;
    }

    // Build coupon dates using payment frequency
    let schedule = finstack_core::dates::ScheduleBuilder::new(loan.issue, loan.maturity)
        .frequency(loan.pay_freq)
        .stub_rule(loan.stub)
        .build()?;
    let mut dates: Vec<Date> = schedule.into_iter().collect();
    if dates.first().copied() != Some(loan.issue) {
        dates.insert(0, loan.issue);
    }

    // Helper to compute margin including step-ups and overrides
    let margin_bp_at = |d: Date| -> f64 {
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
    };

    // Interest and fees per period end
    let dc = loan.day_count;
    let mut prev = dates[0];
    for &d in dates.iter().skip(1) {
        if d <= as_of {
            prev = d;
            continue;
        }
        let yf = dc.year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;

        // Period rate with centralized projection for floating rates
        let period_rate = match &loan.rate {
            super::types::RateSpec::Fixed { rate_bp } => (*rate_bp as f64) * 1e-4,
            super::types::RateSpec::Floating(spec) => {
                // Use centralized projection with total margin (base + step-ups + overrides)
                let total_spread = margin_bp_at(d);
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

        // Interest on outstanding
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
            outstanding = outstanding.checked_add(pik_interest)?;
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
            let undrawn = (limit.amount() - outstanding.amount()).max(0.0);

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

        // Cash sweeps
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
                    outstanding = outstanding.checked_sub(sweep.amount)?;
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
                    outstanding = outstanding.checked_sub(*amt)?;
                }
            }
        }

        // Amortization spec
        match &loan.amortization {
            super::spec::AmortizationSpec::None => {}
            super::spec::AmortizationSpec::Custom(items) => {
                for (adt, amt) in items.iter().filter(|(adt, _)| *adt == d) {
                    let pay = Money::new(amt.amount().min(outstanding.amount()), loan.currency);
                    if pay.amount() > 0.0 {
                        flows.push(CashFlow {
                            date: *adt,
                            reset_date: None,
                            amount: pay,
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                            rate: None,
                        });
                        outstanding = outstanding.checked_sub(pay)?;
                    }
                }
            }
            super::spec::AmortizationSpec::PercentPerPeriod { bp } => {
                let pct = (*bp as f64) * 1e-4;
                let pay = Money::new(
                    (loan.notional_limit.amount() * pct).min(outstanding.amount()),
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
                    outstanding = outstanding.checked_sub(pay)?;
                }
            }
            super::spec::AmortizationSpec::Linear { start, end } => {
                if d >= *start && d <= *end {
                    // Count remaining coupon dates including current within [start,end]
                    // For simplicity, assume regular spacing as in dates vector
                    let remaining = dates
                        .iter()
                        .filter(|&&dt| dt >= d && dt <= *end)
                        .count()
                        .max(1);
                    let pay_amt =
                        (outstanding.amount() / (remaining as f64)).min(outstanding.amount());
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
                        outstanding = outstanding.checked_sub(pay)?;
                    }
                }
            }
        }

        prev = d;
    }

    // Final redemption of remaining principal at maturity (outflow)
    if outstanding.amount() != 0.0 {
        let redemption = Money::new(-outstanding.amount(), outstanding.currency());
        flows.push(CashFlow {
            date: loan.maturity,
            reset_date: None,
            amount: redemption,
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
        notional: Notional::par(0.0, loan.currency),
        day_count: DayCount::Act360,
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
