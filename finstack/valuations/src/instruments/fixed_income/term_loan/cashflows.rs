//! Cashflow generation for Term Loans using the shared cashflow builder.
//!
//! Builds deterministic schedules (including DDTL draws, OID handling, PIK toggles,
//! amortization, and fees) via the unified `CashFlowBuilder` so date logic and
//! floating-rate conventions stay consistent across instruments.

use crate::cashflow::builder::schedule::CashFlowSchedule;
use crate::cashflow::builder::specs::{CouponType, FeeBase, FeeSpec, FixedCouponSpec};
use crate::cashflow::builder::{
    CashFlowBuilder, FloatCouponParams, PrincipalEvent, ScheduleParams,
};
use crate::cashflow::primitives::{CFKind, CashFlow};
use crate::cashflow::traits::DatedFlows;
use crate::instruments::term_loan::types::TermLoan;
use finstack_core::cashflow::xirr::InternalRateOfReturn;
use finstack_core::dates::Date;
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::BTreeMap;

/// Compute total margin (base spread + covenant step-ups + pricing overrides) at a given date.
pub(super) fn margin_bp_at(loan: &TermLoan, d: Date) -> f64 {
    let base_margin = match &loan.rate {
        super::types::RateSpec::Fixed { .. } => 0.0,
        super::types::RateSpec::Floating(spec) => spec.spread_bp.to_f64().unwrap_or(0.0),
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
    let draw_stop = effective_draw_stop(loan);

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
    let mut builder = CashFlowBuilder::new();
    let _ = builder
        .principal(Money::new(0.0, loan.currency), loan.issue, loan.maturity)
        .amortization(crate::cashflow::builder::AmortizationSpec::None)
        .principal_events(&principal_events)
        .strict_schedules(true);

    match &loan.rate {
        super::types::RateSpec::Fixed { rate_bp } => {
            // Convert rate from basis points to decimal, then to Decimal
            let rate_decimal = Decimal::try_from((*rate_bp as f64) * 1e-4).unwrap_or(Decimal::ZERO);
            let spec = FixedCouponSpec {
                coupon_type: loan.coupon_type,
                rate: rate_decimal,
                freq: loan.pay_freq,
                dc: loan.day_count,
                bdc: loan.bdc,
                calendar_id: loan.calendar_id.clone(),
                stub: loan.stub,
            };
            let _ = builder.fixed_cf(spec);
        }
        super::types::RateSpec::Floating(spec) => {
            // Convert Decimal spread_bp to f64 for calculations
            let spread_bp_f64 = spec.spread_bp.to_f64().unwrap_or(0.0);
            // Build cumulative margin steps (base + step-ups + overrides)
            let mut margin_events: Vec<(Date, f64)> = vec![(loan.issue, spread_bp_f64)];
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
                let last = steps.last().map(|(_, m)| *m).unwrap_or(spread_bp_f64);
                steps.push((loan.maturity, last));
            }

            let gearing_f64 = spec.gearing.to_f64().unwrap_or(1.0);
            let base_params = FloatCouponParams {
                index_id: spec.index_id.clone(),
                margin_bp: Decimal::ZERO,
                gearing: Decimal::try_from(gearing_f64).unwrap_or(Decimal::ONE),
                reset_lag_days: spec.reset_lag_days,
            };
            let sched_params = ScheduleParams {
                freq: loan.pay_freq,
                dc: loan.day_count,
                bdc: loan.bdc,
                calendar_id: loan.calendar_id.clone(),
                stub: loan.stub,
            };
            let _ =
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
        let _ = builder.payment_split_program(&payment_steps);
    }

    // Add upfront/OID fees
    for fee in fees {
        let _ = builder.fee(fee);
    }
    if let Some(ddtl) = &loan.ddtl {
        if ddtl.usage_fee_bp != 0 {
            let _ = builder.fee(FeeSpec::PeriodicBps {
                base: FeeBase::Drawn,
                bps: Decimal::try_from(ddtl.usage_fee_bp as f64).unwrap_or(Decimal::ZERO),
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

    if let Some(ddtl) = &loan.ddtl {
        if ddtl.commitment_fee_bp != 0 {
            let commitment_fees = build_commitment_fee_flows(loan, ddtl, draw_stop, &schedule)?;
            if !commitment_fees.is_empty() {
                schedule.flows.extend(commitment_fees);
                schedule.flows.sort_by(|a, b| {
                    use core::cmp::Ordering;
                    match a.date.cmp(&b.date) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Equal => crate::cashflow::builder::schedule::kind_rank(a.kind)
                            .cmp(&crate::cashflow::builder::schedule::kind_rank(b.kind)),
                    }
                });
            }
        }
    }

    // Note: We no longer filter flows by as_of here. The full schedule is returned
    // so that build_full_schedule() can compute outstanding paths correctly.
    // The holder-view filtering in build_schedule() handles date-based exclusion
    // for pricing purposes (it filters to inflows only).
    schedule.day_count = loan.day_count;
    Ok(schedule)
}

fn effective_draw_stop(loan: &TermLoan) -> Option<Date> {
    let cov_stop = loan
        .covenants
        .as_ref()
        .and_then(|c| c.draw_stop_dates.iter().min().copied());
    let override_stop = loan
        .pricing_overrides
        .term_loan
        .as_ref()
        .and_then(|ov| ov.draw_stop_date);

    match (cov_stop, override_stop) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn build_commitment_fee_flows(
    loan: &TermLoan,
    ddtl: &super::spec::DdtlSpec,
    draw_stop: Option<Date>,
    schedule: &CashFlowSchedule,
) -> finstack_core::Result<Vec<CashFlow>> {
    use finstack_core::dates::DayCountCtx;

    let fee_start = ddtl.availability_start;
    let mut fee_end = ddtl.availability_end;
    if let Some(ds) = draw_stop {
        if ds < fee_end {
            fee_end = ds;
        }
    }
    if fee_start >= fee_end {
        return Ok(Vec::new());
    }

    let mut schedule_builder = finstack_core::dates::ScheduleBuilder::new(fee_start, fee_end)
        .frequency(loan.pay_freq)
        .stub_rule(loan.stub);
    if let Some(ref cal_id) = loan.calendar_id {
        schedule_builder = schedule_builder.adjust_with_id(loan.bdc, cal_id);
    }
    let sched = schedule_builder.build()?;
    let mut dates: Vec<Date> = sched.into_iter().collect();
    if dates.first().copied() != Some(fee_start) {
        dates.insert(0, fee_start);
    }
    if dates.last().copied() != Some(fee_end) {
        dates.push(fee_end);
    }
    for sd in &ddtl.commitment_step_downs {
        if sd.date > fee_start && sd.date < fee_end {
            dates.push(sd.date);
        }
    }
    dates.sort();
    dates.dedup();

    let out_path = schedule.outstanding_by_date()?;
    let outstanding_at = |target: Date| -> Money {
        let mut last = Money::new(0.0, loan.currency);
        for (d, amt) in &out_path {
            if *d <= target {
                last = *amt;
            } else {
                break;
            }
        }
        last
    };

    let mut flows = Vec::new();
    let mut prev = dates[0];
    for &d in dates.iter().skip(1) {
        let yf = loan
            .day_count
            .year_fraction(prev, d, DayCountCtx::default())?;
        let limit = commitment_limit_at(ddtl, d);
        if limit.currency() != loan.currency {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        let base = match ddtl.fee_base {
            super::spec::CommitmentFeeBase::Undrawn => {
                let drawn = cumulative_drawn_at(ddtl, draw_stop, d);
                (limit.amount() - drawn).max(0.0)
            }
            super::spec::CommitmentFeeBase::CommitmentMinusOutstanding => {
                (limit.amount() - outstanding_at(d).amount()).max(0.0)
            }
        };
        if base > 0.0 {
            let fee_amt = base * (ddtl.commitment_fee_bp as f64) * 1e-4 * yf;
            if fee_amt > 0.0 {
                flows.push(CashFlow {
                    date: d,
                    reset_date: None,
                    amount: Money::new(fee_amt, loan.currency),
                    kind: CFKind::Fee,
                    accrual_factor: 0.0,
                    rate: Some((ddtl.commitment_fee_bp as f64) * 1e-4),
                });
            }
        }
        prev = d;
    }

    Ok(flows)
}

fn commitment_limit_at(ddtl: &super::spec::DdtlSpec, date: Date) -> Money {
    let mut limit = ddtl.commitment_limit;
    for sd in &ddtl.commitment_step_downs {
        if sd.date <= date {
            limit = sd.new_limit;
        }
    }
    limit
}

fn cumulative_drawn_at(ddtl: &super::spec::DdtlSpec, draw_stop: Option<Date>, date: Date) -> f64 {
    let mut total = 0.0;
    for ev in &ddtl.draws {
        if ev.date < ddtl.availability_start || ev.date > ddtl.availability_end {
            continue;
        }
        if let Some(ds) = draw_stop {
            if ev.date >= ds {
                continue;
            }
        }
        if ev.date <= date {
            total += ev.amount.amount();
        }
    }
    total
}

/// Convenience: build simple dated flows (no CFKind) from full schedule.
pub fn build_dated_flows(schedule: &CashFlowSchedule) -> DatedFlows {
    schedule
        .flows
        .iter()
        .map(|cf| (cf.date, cf.amount))
        .collect()
}

/// Period-level EIR amortization outputs for reporting.
#[derive(Clone, Debug)]
pub struct OidEirPeriod {
    /// Period end date.
    pub date: Date,
    /// Opening balance for the period.
    pub opening_balance: Money,
    /// Interest income recognized under EIR.
    pub interest_income: Money,
    /// Cash interest received during the period.
    pub cash_interest: Money,
    /// Cash principal received during the period.
    pub cash_principal: Money,
    /// OID amortization for the period.
    pub oid_amortization: Money,
    /// Closing balance for the period.
    pub closing_balance: Money,
}

/// EIR amortization schedule output.
#[derive(Clone, Debug)]
pub struct OidEirSchedule {
    /// Effective interest rate.
    pub effective_rate: f64,
    /// Period-by-period amortization details.
    pub periods: Vec<OidEirPeriod>,
}

/// Build an effective interest rate (EIR) amortization schedule from cashflows.
pub fn build_oid_eir_schedule(
    loan: &TermLoan,
    market: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<OidEirSchedule> {
    let schedule = generate_cashflows(loan, market, as_of)?;
    let spec = loan.oid_eir.clone().unwrap_or_default();

    let mut buckets: BTreeMap<Date, CashBuckets> = BTreeMap::new();
    for cf in &schedule.flows {
        match cf.kind {
            CFKind::Fixed | CFKind::FloatReset | CFKind::Stub => {
                buckets
                    .entry(cf.date)
                    .or_default()
                    .add_interest(cf.amount.amount());
            }
            CFKind::Fee if spec.include_fees => {
                buckets
                    .entry(cf.date)
                    .or_default()
                    .add_interest(cf.amount.amount());
            }
            CFKind::Amortization | CFKind::Notional => {
                buckets
                    .entry(cf.date)
                    .or_default()
                    .add_principal(cf.amount.amount());
            }
            _ => {}
        }
    }

    let flows: Vec<(Date, f64)> = buckets
        .iter()
        .map(|(d, b)| (*d, b.total))
        .filter(|(_, amt)| amt.abs() > 0.0)
        .collect();

    let effective_rate = flows.as_slice().irr_with_daycount(loan.day_count, None)?;

    let mut periods = Vec::new();
    let mut iter = buckets.iter();
    let (start_date, start_bucket) = iter
        .next()
        .ok_or(finstack_core::error::InputError::TooFewPoints)?;
    let mut opening_balance = -start_bucket.total;
    let mut prev = *start_date;

    for (date, bucket) in iter {
        let yf = loan
            .day_count
            .year_fraction(prev, *date, DayCountCtx::default())?;
        let interest_income = opening_balance * effective_rate * yf;
        let cash_interest = bucket.interest;
        let cash_principal = bucket.principal;
        let closing_balance = opening_balance + interest_income - bucket.total;
        let oid_amortization = interest_income - cash_interest;

        periods.push(OidEirPeriod {
            date: *date,
            opening_balance: Money::new(opening_balance, loan.currency),
            interest_income: Money::new(interest_income, loan.currency),
            cash_interest: Money::new(cash_interest, loan.currency),
            cash_principal: Money::new(cash_principal, loan.currency),
            oid_amortization: Money::new(oid_amortization, loan.currency),
            closing_balance: Money::new(closing_balance, loan.currency),
        });

        opening_balance = closing_balance;
        prev = *date;
    }

    Ok(OidEirSchedule {
        effective_rate,
        periods,
    })
}

#[derive(Default)]
struct CashBuckets {
    total: f64,
    interest: f64,
    principal: f64,
}

impl CashBuckets {
    fn add_interest(&mut self, amount: f64) {
        self.total += amount;
        self.interest += amount;
    }

    fn add_principal(&mut self, amount: f64) {
        self.total += amount;
        self.principal += amount;
    }
}
