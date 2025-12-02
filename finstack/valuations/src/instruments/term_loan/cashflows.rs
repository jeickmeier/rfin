//! Cashflow generation for Term Loans using the shared cashflow builder.
//!
//! Builds deterministic schedules (including DDTL draws, OID handling, PIK toggles,
//! amortization, and fees) via the unified `CashflowBuilder` so date logic and
//! floating-rate conventions stay consistent across instruments.

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::builder::specs::{CouponType, FeeBase, FeeSpec, FixedCouponSpec};
use crate::cashflow::builder::{
    CashflowBuilder, FloatCouponParams, PrincipalEvent, ScheduleParams,
};
use crate::cashflow::primitives::CFKind;
use crate::instruments::term_loan::types::TermLoan;
use crate::cashflow::traits::DatedFlows;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

/// Compute total margin (base spread + covenant step-ups + pricing overrides) at a given date.
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

/// Generate the full internal cashflow schedule for a term loan using the shared builder.
pub fn generate_cashflows(
    loan: &TermLoan,
    market: &MarketContext,
    _as_of: Date,
) -> finstack_core::Result<CashFlowSchedule> {
    let mut principal_events: Vec<PrincipalEvent> = Vec::new();
    let mut fees: Vec<FeeSpec> = Vec::new();

    // Draw stop date (if any)
    let draw_stop = loan
        .covenants
        .as_ref()
        .and_then(|c| c.draw_stop_dates.iter().min().copied());

    // DDTL draws or upfront funding
    if let Some(ddtl) = &loan.ddtl {
        for ev in &ddtl.draws {
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
                            fees.push(FeeSpec::Fixed {
                                date: ev.date,
                                amount: fee_amt,
                            });
                        }
                    }
                    super::spec::OidPolicy::SeparateAmount(m) => {
                        if m.amount() > 0.0 {
                            fees.push(FeeSpec::Fixed {
                                date: ev.date,
                                amount: *m,
                            });
                        }
                    }
                }
            }

            principal_events.push(PrincipalEvent {
                date: ev.date,
                delta: ev.amount,
                cash: cash_inflow,
                kind: CFKind::Notional,
            });
        }
    } else if loan.notional_limit.amount() != 0.0 {
        principal_events.push(PrincipalEvent {
            date: loan.issue,
            delta: loan.notional_limit,
            cash: loan.notional_limit,
            kind: CFKind::Notional,
        });
    }

    // Upfront fee
    if let Some(fee) = loan.upfront_fee {
        if fee.amount() > 0.0 {
            fees.push(FeeSpec::Fixed {
                date: loan.issue,
                amount: fee,
            });
        }
    }

    // Cash sweeps
    if let Some(cov) = &loan.covenants {
        for sweep in &cov.cash_sweeps {
            if sweep.amount.amount() > 0.0 {
                principal_events.push(PrincipalEvent {
                    date: sweep.date,
                    delta: Money::new(-sweep.amount.amount(), sweep.amount.currency()),
                    cash: sweep.amount,
                    kind: CFKind::Amortization,
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
                    cash: *amt,
                    kind: CFKind::Amortization,
                });
            }
        }
    }

    // Coupon dates for amortization conversion
    let coupon_dates: Vec<Date> = {
        let mut sb = finstack_core::dates::ScheduleBuilder::new(loan.issue, loan.maturity)
            .frequency(loan.pay_freq)
            .stub_rule(loan.stub);
        if let Some(ref cal) = loan.calendar_id {
            sb = sb.adjust_with_id(loan.bdc, cal);
        }
        let mut ds: Vec<Date> = sb.build()?.into_iter().collect();
        if ds.first().copied() != Some(loan.issue) {
            ds.insert(0, loan.issue);
        }
        ds
    };

    // Amortization → principal events
    match &loan.amortization {
        super::spec::AmortizationSpec::None => {}
        super::spec::AmortizationSpec::Custom(items) => {
            for (dt, amt) in items {
                principal_events.push(PrincipalEvent {
                    date: *dt,
                    delta: Money::new(-amt.amount(), amt.currency()),
                    cash: *amt,
                    kind: CFKind::Amortization,
                });
            }
        }
        super::spec::AmortizationSpec::PercentPerPeriod { bp } => {
            let pct = (*bp as f64) * 1e-4;
            for d in coupon_dates.iter().copied().skip(1) {
                let pay = Money::new(loan.notional_limit.amount() * pct, loan.currency);
                principal_events.push(PrincipalEvent {
                    date: d,
                    delta: Money::new(-pay.amount(), pay.currency()),
                    cash: pay,
                    kind: CFKind::Amortization,
                });
            }
        }
        super::spec::AmortizationSpec::Linear { start, end } => {
            let steps: Vec<Date> = coupon_dates
                .iter()
                .copied()
                .filter(|d| *d >= *start && *d <= *end)
                .collect();
            if !steps.is_empty() {
                let per_step = loan.notional_limit.amount() / (steps.len() as f64);
                for d in steps {
                    let pay = Money::new(per_step, loan.currency);
                    principal_events.push(PrincipalEvent {
                        date: d,
                        delta: Money::new(-pay.amount(), pay.currency()),
                        cash: pay,
                        kind: CFKind::Amortization,
                    });
                }
            }
        }
    }

    principal_events.sort_by_key(|e| e.date);

    // Build coupon program via unified builder
    let mut builder = CashflowBuilder::new();
    builder
        .principal(Money::new(0.0, loan.currency), loan.issue, loan.maturity)
        .amortization(crate::cashflow::builder::AmortizationSpec::None)
        .principal_events(&principal_events)
        .strict_schedules(true);

    match &loan.rate {
        super::types::RateSpec::Fixed { rate_bp } => {
            let spec = FixedCouponSpec {
                coupon_type: loan.coupon_type,
                rate: (*rate_bp as f64) * 1e-4,
                freq: loan.pay_freq,
                dc: loan.day_count,
                bdc: loan.bdc,
                calendar_id: loan.calendar_id.clone(),
                stub: loan.stub,
            };
            builder.fixed_cf(spec);
        }
        super::types::RateSpec::Floating(spec) => {
            // Build cumulative margin steps (base + step-ups + overrides)
            let mut margin_events: Vec<(Date, f64)> = vec![(loan.issue, spec.spread_bp)];
            if let Some(cov) = &loan.covenants {
                for step in &cov.margin_stepups {
                    margin_events.push((step.date, step.delta_bp as f64));
                }
            }
            if let Some(ov) = &loan.pricing_overrides.term_loan {
                for (dt, bp) in &ov.margin_add_bp_by_date {
                    margin_events.push((*dt, *bp as f64));
                }
            }
            margin_events.sort_by_key(|(d, _)| *d);
            let mut steps: Vec<(Date, f64)> = Vec::new();
            let mut running = 0.0;
            for (d, delta) in margin_events {
                running += delta;
                steps.push((d, running));
            }
            if steps
                .last()
                .map(|(d, _)| *d != loan.maturity)
                .unwrap_or(true)
            {
                let last = steps.last().map(|(_, m)| *m).unwrap_or(spec.spread_bp);
                steps.push((loan.maturity, last));
            }

            let base_params = FloatCouponParams {
                index_id: spec.index_id.clone(),
                margin_bp: 0.0,
                gearing: spec.gearing,
                reset_lag_days: spec.reset_lag_days,
            };
            let sched_params = ScheduleParams {
                freq: loan.pay_freq,
                dc: loan.day_count,
                bdc: loan.bdc,
                calendar_id: loan.calendar_id.clone(),
                stub: loan.stub,
            };
            builder.float_margin_stepup(&steps, base_params, sched_params, loan.coupon_type);
        }
    }

    // Payment split windows for PIK toggles
    let mut payment_steps: Vec<(Date, CouponType)> = Vec::new();
    if let Some(cov) = &loan.covenants {
        for t in &cov.pik_toggles {
            if t.enable_pik {
                payment_steps.push((t.date, CouponType::PIK));
            }
        }
    }
    if let Some(ov) = &loan.pricing_overrides.term_loan {
        for (dt, en) in &ov.pik_toggle_by_date {
            if *en {
                payment_steps.push((*dt, CouponType::PIK));
            }
        }
    }
    payment_steps.sort_by_key(|(d, _)| *d);
    if !payment_steps.is_empty() {
        builder.payment_split_program(&payment_steps);
    }

    // Add upfront/OID fees
    for fee in fees {
        builder.fee(fee);
    }
    if let Some(ddtl) = &loan.ddtl {
        // Commitment fee (undrawn). Note: builder does not yet window facility limits; use current limit.
        if ddtl.commitment_fee_bp != 0 {
            builder.fee(FeeSpec::PeriodicBps {
                base: match ddtl.fee_base {
                    super::spec::CommitmentFeeBase::Undrawn => FeeBase::Undrawn {
                        facility_limit: ddtl.commitment_limit,
                    },
                    super::spec::CommitmentFeeBase::CommitmentMinusOutstanding => FeeBase::Undrawn {
                        facility_limit: ddtl.commitment_limit,
                    },
                },
                bps: ddtl.commitment_fee_bp as f64,
                freq: loan.pay_freq,
                dc: loan.day_count,
                bdc: loan.bdc,
                calendar_id: loan.calendar_id.clone(),
                stub: loan.stub,
            });
        }

        if ddtl.usage_fee_bp != 0 {
            builder.fee(FeeSpec::PeriodicBps {
                base: FeeBase::Drawn,
                bps: ddtl.usage_fee_bp as f64,
                freq: loan.pay_freq,
                dc: loan.day_count,
                bdc: loan.bdc,
                calendar_id: loan.calendar_id.clone(),
                stub: loan.stub,
            });
        }
    }

    // Build via shared builder (use market for forwards)
    let mut schedule = builder.build_with_curves(Some(market))?;

    // Note: We no longer filter flows by as_of here. The full schedule is returned
    // so that build_full_schedule() can compute outstanding paths correctly.
    // The holder-view filtering in build_schedule() handles date-based exclusion
    // for pricing purposes (it filters to inflows only).
    schedule.day_count = loan.day_count;
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
