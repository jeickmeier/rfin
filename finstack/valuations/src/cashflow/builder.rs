use crate::cashflow::amortization::AmortizationSpec;
use crate::cashflow::notional::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency, StubKind};
use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::dates::{adjust, BusinessDayConvention};
use finstack_core::error::InputError;
use finstack_core::money::Money;
use time::Duration;
use std::collections::BTreeSet;

type FixedSchedule = (FixedCouponSpec, Vec<Date>, hashbrown::HashMap<Date, Date>, hashbrown::HashSet<Date>);
type FloatSchedule = (FloatingCouponSpec, Vec<Date>, hashbrown::HashMap<Date, Date>);

/// Periodic fee schedule prepared from fee specs.
#[derive(Debug, Clone)]
struct PeriodicFee {
    base: FeeBase,
    bps: f64,
    dc: DayCount,
    dates: Vec<Date>,
    prev: hashbrown::HashMap<Date, Date>,
}

type PeriodicFees = Vec<PeriodicFee>;
type FixedFees = Vec<(Date, Money)>;

/// Collect all relevant dates from components into a naturally ordered set.
#[inline]
fn collect_dates(
    issue: Date,
    maturity: Date,
    fixed_schedules: &[FixedSchedule],
    float_schedules: &[FloatSchedule],
    periodic_fee_date_slices: &[&[Date]],
    fixed_fees: &[(Date, Money)],
    notional: &Notional,
) -> Vec<Date> {
    let mut set: BTreeSet<Date> = BTreeSet::new();
    set.insert(issue);
    set.insert(maturity);

    for (_, ds, _, _) in fixed_schedules {
        for &d in ds.iter() { set.insert(d); }
    }
    for (_, ds, _) in float_schedules {
        for &d in ds.iter() { set.insert(d); }
    }
    for dates in periodic_fee_date_slices {
        for &d in dates.iter() { set.insert(d); }
    }
    for (d, _) in fixed_fees { set.insert(*d); }
    if let AmortizationSpec::CustomPrincipal { items } = &notional.amort {
        for (d, _) in items { set.insert(*d); }
    }

    set.into_iter().collect()
}

// -------------------------------------------------------------------------
// Helper builders - schedule building
// -------------------------------------------------------------------------

#[inline]
fn build_fixed_schedules(
    issue: Date,
    maturity: Date,
    fixed: &[FixedCouponSpec],
) -> finstack_core::Result<Vec<FixedSchedule>> {
    let mut fixed_schedules: Vec<FixedSchedule> = Vec::new();
    for spec in fixed {
        let sched = crate::cashflow::schedule::build_dates(issue, maturity, spec.freq, spec.stub, spec.bdc, spec.calendar_id);
        let dates = sched.dates;
        if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }
        fixed_schedules.push((*spec, dates.clone(), sched.prev, sched.first_or_last));
    }
    Ok(fixed_schedules)
}

#[inline]
fn build_float_schedules(
    issue: Date,
    maturity: Date,
    floating: &[FloatingCouponSpec],
) -> finstack_core::Result<Vec<FloatSchedule>> {
    let mut float_schedules: Vec<FloatSchedule> = Vec::new();
    for spec in floating {
        let sched = crate::cashflow::schedule::build_dates(issue, maturity, spec.freq, spec.stub, spec.bdc, spec.calendar_id);
        let dates = sched.dates;
        if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }
        float_schedules.push((*spec, dates.clone(), sched.prev));
    }
    Ok(float_schedules)
}

#[inline]
fn build_fee_schedules(
    issue: Date,
    maturity: Date,
    fees: &[FeeSpec],
) -> finstack_core::Result<(PeriodicFees, FixedFees)> {
    let mut periodic_fees: PeriodicFees = Vec::new();
    let mut fixed_fees: FixedFees = Vec::new();
    for fee in fees {
        match fee {
            FeeSpec::Fixed { date, amount } => fixed_fees.push((*date, *amount)),
            FeeSpec::PeriodicBps { base, bps, freq, dc, bdc, calendar_id, stub } => {
                let sched = crate::cashflow::schedule::build_dates(issue, maturity, *freq, *stub, *bdc, *calendar_id);
                let dates = sched.dates;
                if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }
                periodic_fees.push(PeriodicFee { base: base.clone(), bps: *bps, dc: *dc, dates, prev: sched.prev });
            }
        }
    }
    Ok((periodic_fees, fixed_fees))
}

// -------------------------------------------------------------------------
// Program compiler — compile coupon/payment windows into schedules
// -------------------------------------------------------------------------

#[inline]
fn compute_coupon_schedules(
    builder: &CashflowBuilder,
    issue: Date,
    maturity: Date,
) -> finstack_core::Result<CompiledSchedules> {
    use std::collections::BTreeSet;

    // Fallback to legacy if no segmented input at all
    let no_programs = builder.coupon_program.is_empty() && builder.payment_program.is_empty();
    if no_programs {
        let fixed_schedules = build_fixed_schedules(issue, maturity, &builder.fixed)?;
        let float_schedules = build_float_schedules(issue, maturity, &builder.floating)?;
        return Ok(CompiledSchedules {
            fixed_schedules,
            float_schedules,
            used_fixed_specs: builder.fixed.clone(),
            used_float_specs: builder.floating.clone(),
        });
    }

    // Build coupon pieces: if coupon program empty but legacy specs exist, derive full-span pieces
    let mut coupon_pieces: Vec<CouponProgramPiece> = builder.coupon_program.clone();
    if coupon_pieces.is_empty() {
        for s in &builder.fixed {
            coupon_pieces.push(CouponProgramPiece {
                window: DateWindow { start: issue, end: maturity },
                schedule: ScheduleParams { freq: s.freq, dc: s.dc, bdc: s.bdc, calendar_id: s.calendar_id, stub: s.stub },
                coupon: CouponSpec::Fixed { rate: s.rate },
            });
        }
        for s in &builder.floating {
            coupon_pieces.push(CouponProgramPiece {
                window: DateWindow { start: issue, end: maturity },
                schedule: ScheduleParams { freq: s.freq, dc: s.dc, bdc: s.bdc, calendar_id: s.calendar_id, stub: s.stub },
                coupon: CouponSpec::Float { index_id: s.index_id, margin_bp: s.margin_bp, gearing: s.gearing, reset_lag_days: s.reset_lag_days },
            });
        }
    }

    // Payment pieces (PIK toggles) — may be sparse; missing windows default to Cash
    let payment_pieces: Vec<PaymentProgramPiece> = builder.payment_program.clone();

    // Validate windows are within [issue, maturity] and build boundary grid
    let within = |w: &DateWindow| -> bool { w.start >= issue && w.end <= maturity && w.start < w.end };
    let mut bounds: BTreeSet<Date> = BTreeSet::new();
    bounds.insert(issue);
    bounds.insert(maturity);
    for p in &coupon_pieces {
        if !within(&p.window) { return Err(InputError::Invalid.into()); }
        bounds.insert(p.window.start);
        bounds.insert(p.window.end);
    }
    for p in &payment_pieces {
        if !within(&p.window) { return Err(InputError::Invalid.into()); }
        bounds.insert(p.window.start);
        bounds.insert(p.window.end);
    }
    let grid: Vec<Date> = bounds.into_iter().collect();
    if grid.len() < 2 { return Err(InputError::TooFewPoints.into()); }

    let mut fixed_schedules: Vec<FixedSchedule> = Vec::new();
    let mut float_schedules: Vec<FloatSchedule> = Vec::new();
    let mut used_fixed_specs: Vec<FixedCouponSpec> = Vec::new();
    let mut used_float_specs: Vec<FloatingCouponSpec> = Vec::new();

    for w in grid.windows(2) {
        let s = w[0];
        let e = w[1];
        if s >= e { continue; }

        // Select single covering coupon piece
        let mut chosen_coupon: Option<&CouponProgramPiece> = None;
        for p in &coupon_pieces {
            if p.window.start <= s && e <= p.window.end {
                if chosen_coupon.is_some() { return Err(InputError::Invalid.into()); }
                chosen_coupon = Some(p);
            }
        }
        let chosen_coupon = chosen_coupon.ok_or(InputError::Invalid)?;

        // Select payment split (if multiple overlap -> invalid; if none -> default Cash)
        let mut chosen_split: Option<CouponType> = None;
        for p in &payment_pieces {
            if p.window.start <= s && e <= p.window.end {
                if chosen_split.is_some() { return Err(InputError::Invalid.into()); }
                chosen_split = Some(p.split);
            }
        }
        let split = chosen_split.unwrap_or(CouponType::Cash);

        let sched = crate::cashflow::schedule::build_dates(
            s,
            e,
            chosen_coupon.schedule.freq,
            chosen_coupon.schedule.stub,
            chosen_coupon.schedule.bdc,
            chosen_coupon.schedule.calendar_id,
        );
        let dates = sched.dates;
        if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }

        match chosen_coupon.coupon {
            CouponSpec::Fixed { rate } => {
                let spec = FixedCouponSpec {
                    coupon_type: split,
                    rate,
                    freq: chosen_coupon.schedule.freq,
                    dc: chosen_coupon.schedule.dc,
                    bdc: chosen_coupon.schedule.bdc,
                    calendar_id: chosen_coupon.schedule.calendar_id,
                    stub: chosen_coupon.schedule.stub,
                };
                used_fixed_specs.push(spec);
                fixed_schedules.push((spec, dates.clone(), sched.prev, sched.first_or_last));
            }
            CouponSpec::Float { index_id, margin_bp, gearing, reset_lag_days } => {
                let spec = FloatingCouponSpec {
                    index_id,
                    margin_bp,
                    gearing,
                    coupon_type: split,
                    freq: chosen_coupon.schedule.freq,
                    dc: chosen_coupon.schedule.dc,
                    bdc: chosen_coupon.schedule.bdc,
                    calendar_id: chosen_coupon.schedule.calendar_id,
                    stub: chosen_coupon.schedule.stub,
                    reset_lag_days,
                };
                used_float_specs.push(spec);
                float_schedules.push((spec, dates.clone(), sched.prev));
            }
        }
    }

    Ok(CompiledSchedules {
        fixed_schedules,
        float_schedules,
        used_fixed_specs,
        used_float_specs,
    })
}

// -------------------------------------------------------------------------
// Helper emitters - cashflow emission
// -------------------------------------------------------------------------

#[inline]
fn emit_fixed_coupons_on(
    d: Date,
    fixed_schedules: &[FixedSchedule],
    outstanding_after: &hashbrown::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    
) -> finstack_core::Result<(f64, Vec<CashFlow>)> {
    let mut pik_to_add = 0.0;
    let mut new_flows: Vec<CashFlow> = Vec::new();
    for (spec, _ds, prev_map, first_last) in fixed_schedules {
        if let Some(&prev) = prev_map.get(&d) {
            let base_out = *outstanding_after.get(&prev).unwrap_or(&outstanding_fallback);
            let yf = spec.dc.year_fraction(prev, d)?;
            let coupon_total = base_out * (spec.rate * yf);
            let (cash_pct, pik_pct) = spec.coupon_type.split_parts();
            let cash_amt = coupon_total * cash_pct;
            let pik_amt = coupon_total * pik_pct;
            if cash_amt > 0.0 {
                let kind = if first_last.contains(&d) { CFKind::Stub } else { CFKind::Fixed };
                new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(cash_amt, ccy), kind, accrual_factor: yf });
            }
            if pik_amt > 0.0 {
                new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(pik_amt, ccy), kind: CFKind::PIK, accrual_factor: 0.0 });
                pik_to_add += pik_amt;
            }
        }
    }
    Ok((pik_to_add, new_flows))
}

#[inline]
fn emit_float_coupons_on(
    d: Date,
    float_schedules: &[FloatSchedule],
    outstanding_after: &hashbrown::HashMap<Date, f64>,
    outstanding_fallback: f64,
    ccy: Currency,
    
) -> finstack_core::Result<(f64, Vec<CashFlow>)> {
    let mut pik_to_add = 0.0;
    let mut new_flows: Vec<CashFlow> = Vec::new();
    for (spec, _ds, prev_map) in float_schedules {
        if let Some(&prev) = prev_map.get(&d) {
            let base_out = *outstanding_after.get(&prev).unwrap_or(&outstanding_fallback);
            let yf = spec.dc.year_fraction(prev, d)?;
            let margin_rate = (spec.margin_bp * 1e-4) * spec.gearing;
            let coupon_total = base_out * (margin_rate * yf);
            let (cash_pct, pik_pct) = spec.coupon_type.split_parts();
            let cash_amt = coupon_total * cash_pct;
            let pik_amt = coupon_total * pik_pct;
            let mut reset_date = d - Duration::days(spec.reset_lag_days as i64);
            if let Some(id) = spec.calendar_id {
                if let Some(cal) = calendar_by_id(id) {
                    reset_date = adjust(reset_date, spec.bdc, cal);
                }
            }
            if cash_amt > 0.0 {
                new_flows.push(CashFlow { date: d, reset_date: Some(reset_date), amount: Money::new(cash_amt, ccy), kind: CFKind::FloatReset, accrual_factor: yf });
            }
            if pik_amt > 0.0 {
                new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(pik_amt, ccy), kind: CFKind::PIK, accrual_factor: 0.0 });
                pik_to_add += pik_amt;
            }
        }
    }
    Ok((pik_to_add, new_flows))
}

#[derive(Debug, Clone)]
struct AmortizationParams<'a> {
    ccy: Currency,
    amort_dates: &'a hashbrown::HashSet<Date>,
    linear_delta: Option<f64>,
    percent_per: Option<f64>,
    step_remaining_map: &'a Option<hashbrown::HashMap<Date, Money>>,
}

#[inline]
fn emit_amortization_on(
    d: Date,
    notional: &Notional,
    outstanding: &mut f64,
    params: &AmortizationParams,
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut new_flows: Vec<CashFlow> = Vec::new();
    match &notional.amort {
        AmortizationSpec::None => {}
        AmortizationSpec::LinearTo { .. } => {
            if params.amort_dates.contains(&d) {
                if let Some(delta) = params.linear_delta {
                    let pay = delta.min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, params.ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                        *outstanding -= pay;
                    }
                }
            }
        }
        AmortizationSpec::StepRemaining { .. } => {
            if let Some(map) = params.step_remaining_map {
                if let Some(rem_after) = map.get(&d) {
                    let target = rem_after.amount();
                    let pay = (*outstanding - target).max(0.0).min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, params.ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                        *outstanding -= pay;
                    }
                }
            }
        }
        AmortizationSpec::PercentPerPeriod { .. } => {
            if params.amort_dates.contains(&d) {
                if let Some(per) = params.percent_per {
                    let pay = per.min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, params.ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                        *outstanding -= pay;
                    }
                }
            }
        }
        AmortizationSpec::CustomPrincipal { items } => {
            for (dd, amt) in items {
                if *dd == d {
                    let pay = amt.amount().max(0.0).min(*outstanding);
                    if pay > 0.0 {
                        new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, params.ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                        *outstanding -= pay;
                    }
                }
            }
        }
    }
    Ok(new_flows)
}

#[inline]
fn emit_fees_on(
    d: Date,
    periodic_fees: &[PeriodicFee],
    fixed_fees: &[(Date, Money)],
    outstanding: f64,
    ccy: Currency,
    
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut new_flows: Vec<CashFlow> = Vec::new();
    for pf in periodic_fees {
        if let Some(&prev) = pf.prev.get(&d) {
            let yf = pf.dc.year_fraction(prev, d)?;
            let base_amt = match &pf.base {
                FeeBase::Drawn => outstanding,
                FeeBase::Undrawn { facility_limit } => {
                    if facility_limit.currency() != ccy { return Err(InputError::Invalid.into()); }
                    (facility_limit.amount() - outstanding).max(0.0)
                }
            };
            let fee_amt = base_amt * (pf.bps * 1e-4 * yf);
            if fee_amt > 0.0 {
                new_flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(fee_amt, ccy), kind: CFKind::Fee, accrual_factor: 0.0 });
            }
        }
    }

    for (fd, amt) in fixed_fees { if *fd == d { new_flows.push(CashFlow { date: d, reset_date: None, amount: *amt, kind: CFKind::Fee, accrual_factor: 0.0 }); } }
    Ok(new_flows)
}

#[inline]
fn finalize_flows(
    mut flows: Vec<CashFlow>,
    fixed: &[FixedCouponSpec],
    floating: &[FloatingCouponSpec],
) -> (Vec<CashFlow>, CashflowMeta, DayCount) {
    flows.sort_by(|a, b| {
        use core::cmp::Ordering;
        match a.date.cmp(&b.date) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => kind_rank(a.kind).cmp(&kind_rank(b.kind)),
        }
    });

    let mut cals: Vec<&'static str> = Vec::new();
    for s in fixed { if let Some(id) = s.calendar_id { cals.push(id); } }
    for s in floating { if let Some(id) = s.calendar_id { cals.push(id); } }
    cals.sort_unstable();
    cals.dedup();
    let meta = CashflowMeta { calendar_ids: cals };

    let out_dc = if let Some(s) = fixed.first() { s.dc } else if let Some(s) = floating.first() { s.dc } else { DayCount::Act365F };
    (flows, meta, out_dc)
}

/// Cashflow schedule output from the composable builder.
#[derive(Debug, Clone)]
pub struct CashFlowSchedule {
    pub flows: Vec<CashFlow>,
    pub notional: Notional,
    pub day_count: DayCount,
    pub meta: CashflowMeta,
}

impl CashFlowSchedule {
    #[inline]
    pub fn dates(&self) -> Vec<Date> {
        self.flows.iter().map(|cf| cf.date).collect()
    }

    #[inline]
    pub fn flows_of_kind(&self, kind: CFKind) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(move |cf| cf.kind == kind)
    }

    /// Outstanding principal path computed from principal/PIK/amortization flows.
    /// Assumes economic signs: amortization negative, PIK positive, final notional positive redemption.
    pub fn outstanding_path(&self) -> Vec<(Date, Money)> {
        let mut out = Vec::new();
        let mut outstanding = self.notional.initial.amount();
        let ccy = self.notional.initial.currency();
        for cf in &self.flows {
            match cf.kind {
                CFKind::Amortization => {
                    outstanding += cf.amount.amount(); // amount is negative
                }
                CFKind::PIK => {
                    outstanding += cf.amount.amount(); // adds to outstanding
                }
                _ => {}
            }
            out.push((cf.date, Money::new(outstanding, ccy)));
        }
        out
    }

    // Convenience iterators for callers to avoid ad-hoc filtering.
    #[inline]
    pub fn coupons(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
    }

    #[inline]
    pub fn amortizations(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(|cf| cf.kind == CFKind::Amortization)
    }

    #[inline]
    pub fn redemptions(&self) -> impl Iterator<Item = &CashFlow> {
        self.flows.iter().filter(|cf| cf.kind == CFKind::Notional && cf.amount.amount() > 0.0)
    }

    /// End-of-date outstanding path: one entry per unique date after applying Amortization/PIK on that date.
    /// Does not change on coupon/fee/notional flows; redemption does not reduce outstanding here.
    #[inline]
    pub fn outstanding_by_date(&self) -> Vec<(Date, Money)> {
        let mut result: Vec<(Date, Money)> = Vec::new();
        if self.flows.is_empty() {
            return result;
        }

        let ccy = self.notional.initial.currency();
        let mut outstanding = self.notional.initial.amount();

        let mut i = 0usize;
        while i < self.flows.len() {
            let d = self.flows[i].date;
            // Process all flows on this date in their deterministic order
            let mut j = i;
            while j < self.flows.len() && self.flows[j].date == d {
                match self.flows[j].kind {
                    CFKind::Amortization => {
                        outstanding += self.flows[j].amount.amount();
                    }
                    CFKind::PIK => {
                        outstanding += self.flows[j].amount.amount();
                    }
                    _ => {}
                }
                j += 1;
            }
            result.push((d, Money::new(outstanding, ccy)));
            i = j;
        }

        result
    }
}

/// Minimal schedule metadata (phase 1).
#[derive(Debug, Clone, Default)]
pub struct CashflowMeta {
    pub calendar_ids: Vec<&'static str>,
}

/// Coupon cashflow type for fixed/floating coupons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CouponType {
    Cash,
    PIK,
    Split { cash_pct: f64, pik_pct: f64 },
}

impl CouponType {
    #[inline]
    fn split_parts(self) -> (f64, f64) {
        match self {
            CouponType::Cash => (1.0, 0.0),
            CouponType::PIK => (0.0, 1.0),
            CouponType::Split { cash_pct, pik_pct } => (cash_pct, pik_pct),
        }
    }
}

/// Fixed coupon specification.
#[derive(Debug, Clone, Copy)]
pub struct FixedCouponSpec {
    pub coupon_type: CouponType,
    pub rate: f64,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
}

/// Floating coupon specification (scaffold for phase 2).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct FloatingCouponSpec {
    pub index_id: &'static str,
    pub margin_bp: f64,
    pub gearing: f64,
    pub coupon_type: CouponType,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
    pub reset_lag_days: i32,
}

/// Fee specification (scaffold for phase 2).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FeeSpec {
    Fixed { date: Date, amount: Money },
    PeriodicBps { base: FeeBase, bps: f64, freq: Frequency, dc: DayCount, bdc: BusinessDayConvention, calendar_id: Option<&'static str>, stub: StubKind },
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone)]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit − outstanding, 0).
    Undrawn { facility_limit: Money },
}

// -------------------------------------------------------------------------
// Segmented coupon program primitives (coupon windows and payment toggles)
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct ScheduleParams {
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
}

#[derive(Debug, Clone, Copy)]
struct DateWindow {
    start: Date,
    end: Date, // exclusive
}

#[derive(Debug, Clone, Copy)]
enum CouponSpec {
    Fixed { rate: f64 },
    Float { index_id: &'static str, margin_bp: f64, gearing: f64, reset_lag_days: i32 },
}

#[derive(Debug, Clone)]
struct CouponProgramPiece {
    window: DateWindow,
    schedule: ScheduleParams,
    coupon: CouponSpec,
}

#[derive(Debug, Clone)]
struct PaymentProgramPiece {
    window: DateWindow,
    split: CouponType, // Cash | PIK | Split
}

#[derive(Debug, Clone, Copy)]
pub struct FloatCouponParams {
    pub index_id: &'static str,
    pub margin_bp: f64,
    pub gearing: f64,
    pub reset_lag_days: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct FixedWindow {
    pub rate: f64,
    pub schedule: ScheduleParams,
}

#[derive(Debug, Clone, Copy)]
pub struct FloatWindow {
    pub params: FloatCouponParams,
    pub schedule: ScheduleParams,
}

#[derive(Debug, Clone)]
struct CompiledSchedules {
    fixed_schedules: Vec<FixedSchedule>,
    float_schedules: Vec<FloatSchedule>,
    used_fixed_specs: Vec<FixedCouponSpec>,
    used_float_specs: Vec<FloatingCouponSpec>,
}

/// Entry-point for the composable cashflow builder.
pub fn cf() -> CashflowBuilder {
    CashflowBuilder::default()
}

/// Composable cashflow builder (phase 1: principal, amortization, fixed coupons).
#[derive(Debug, Default, Clone)]
pub struct CashflowBuilder {
    notional: Option<Notional>,
    issue: Option<Date>,
    maturity: Option<Date>,
    fixed: Vec<FixedCouponSpec>,
    floating: Vec<FloatingCouponSpec>,
    fees: Vec<FeeSpec>,
    // Segmented programs (optional): coupon program and payment/PIK program
    coupon_program: Vec<CouponProgramPiece>,
    payment_program: Vec<PaymentProgramPiece>,
}

impl CashflowBuilder {
    /// Set principal details.
    pub fn principal(&mut self, initial: Money, issue_date: Date, maturity: Date) -> &mut Self {
        self.notional = Some(Notional { initial, amort: AmortizationSpec::None });
        self.issue = Some(issue_date);
        self.maturity = Some(maturity);
        self
    }

    /// Sugar helper for principal by amount and currency.
    pub fn principal_amount(&mut self, amount: f64, currency: Currency, issue_date: Date, maturity: Date) -> &mut Self {
        self.principal(Money::new(amount, currency), issue_date, maturity)
    }

    /// Configure amortization on the current notional.
    pub fn amortization(&mut self, spec: AmortizationSpec) -> &mut Self {
        if let Some(n) = &mut self.notional { n.amort = spec; }
        self
    }

    /// Add a fixed coupon spec.
    pub fn fixed_cf(&mut self, spec: FixedCouponSpec) -> &mut Self {
        self.fixed.push(spec);
        self
    }

    /// Add a floating coupon spec (not yet implemented in phase 1).
    pub fn floating_cf(&mut self, spec: FloatingCouponSpec) -> &mut Self {
        self.floating.push(spec);
        self
    }

    /// Add a fee spec (not yet implemented in phase 1).
    pub fn fee(&mut self, spec: FeeSpec) -> &mut Self {
        self.fees.push(spec);
        self
    }

    /// Add a fixed coupon window with its own schedule and payment split (cash/PIK/split).
    pub fn add_fixed_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        rate: f64,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow { start, end },
            schedule,
            coupon: CouponSpec::Fixed { rate },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Add a floating coupon window with its own schedule and payment split.
    pub fn add_float_coupon_window(
        &mut self,
        start: Date,
        end: Date,
        params: FloatCouponParams,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> &mut Self {
        self.coupon_program.push(CouponProgramPiece {
            window: DateWindow { start, end },
            schedule,
            coupon: CouponSpec::Float { index_id: params.index_id, margin_bp: params.margin_bp, gearing: params.gearing, reset_lag_days: params.reset_lag_days },
        });
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Add/override a payment split (cash/PIK/split) over a window (PIK toggle support).
    pub fn add_payment_window(&mut self, start: Date, end: Date, split: CouponType) -> &mut Self {
        self.payment_program.push(PaymentProgramPiece {
            window: DateWindow { start, end },
            split,
        });
        self
    }

    /// Convenience: fixed step-up program using boundary dates.
    /// steps: ordered by boundary end date, last tuple's date should equal maturity.
    pub fn fixed_stepup(
        &mut self,
        steps: &[(Date, f64)],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let issue = self.issue.expect("issue must be set before stepup");
        let maturity = self.maturity.expect("maturity must be set before stepup");
        let mut prev = issue;
        for &(end, rate) in steps {
            self.add_fixed_coupon_window(prev, end, rate, schedule, default_split);
            prev = end;
        }
        if prev != maturity {
            // If the last step didn't reach maturity, extend using last rate
            if let Some(&(_, rate)) = steps.last() {
                self.add_fixed_coupon_window(prev, maturity, rate, schedule, default_split);
            }
        }
        self
    }

    /// Convenience: floating margin step-up program.
    /// steps: ordered by boundary end date, last tuple's date should equal maturity.
    pub fn float_margin_stepup(
        &mut self,
        steps: &[(Date, f64)],
        base_params: FloatCouponParams,
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> &mut Self {
        let issue = self.issue.expect("issue must be set before stepup");
        let maturity = self.maturity.expect("maturity must be set before stepup");
        let mut prev = issue;
        for &(end, margin_bp) in steps {
            let mut params = base_params;
            params.margin_bp = margin_bp;
            self.add_float_coupon_window(prev, end, params, schedule, default_split);
            prev = end;
        }
        if prev != maturity {
            let mut params = base_params;
            if let Some(&(_, margin_bp)) = steps.last() { params.margin_bp = margin_bp; }
            self.add_float_coupon_window(prev, maturity, params, schedule, default_split);
        }
        self
    }

    /// Convenience: fixed-to-float switch at `switch` date.
    pub fn fixed_to_float(
        &mut self,
        switch: Date,
        fixed_win: FixedWindow,
        float_win: FloatWindow,
        default_split: CouponType,
    ) -> &mut Self {
        let issue = self.issue.expect("issue must be set before fixed_to_float");
        let maturity = self.maturity.expect("maturity must be set before fixed_to_float");
        self.add_fixed_coupon_window(issue, switch, fixed_win.rate, fixed_win.schedule, default_split);
        self.add_float_coupon_window(switch, maturity, float_win.params, float_win.schedule, default_split);
        self
    }

    /// Convenience: payment split program with boundary dates (PIK toggle windows).
    /// steps: list of (end, split); default outside windows is Cash.
    pub fn payment_split_program(&mut self, steps: &[(Date, CouponType)]) -> &mut Self {
        let issue = self.issue.expect("issue must be set before payment program");
        let maturity = self.maturity.expect("maturity must be set before payment program");
        let mut prev = issue;
        for &(end, split) in steps {
            self.add_payment_window(prev, end, split);
            prev = end;
        }
        self.add_payment_window(prev, maturity, CouponType::Cash);
        self
    }

    /// Build the complete cashflow schedule.
    pub fn build(&self) -> finstack_core::Result<CashFlowSchedule> {
        // Validate principal
        let notional = self.notional.clone().ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
        let issue = self.issue.ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
        let maturity = self.maturity.ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;

        // Build coupon schedules: segmented (if any program provided) or legacy full-span
        let CompiledSchedules { fixed_schedules, float_schedules, used_fixed_specs, used_float_specs } =
            compute_coupon_schedules(self, issue, maturity)?;
        let (periodic_fees, fixed_fees) = build_fee_schedules(issue, maturity, &self.fees)?;

        // Collect all relevant dates using an ordered set for deterministic traversal
        let periodic_date_slices: Vec<&[Date]> = periodic_fees.iter().map(|pf| pf.dates.as_slice()).collect();
        let dates: Vec<Date> = collect_dates(
            issue,
            maturity,
            &fixed_schedules,
            &float_schedules,
            &periodic_date_slices,
            &fixed_fees,
            &notional,
        );
        if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }

        // Determine amortization cadence when needed (linear/percent)
        let amort_base_schedule: Option<Vec<Date>> = if matches!(notional.amort, AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentPerPeriod { .. }) {
            if let Some((_, ds, _, _)) = fixed_schedules.first() { Some(ds.clone()) }
            else if let Some((_, ds, _)) = float_schedules.first() { Some(ds.clone()) }
            else { None }
        } else { None };

        if amort_base_schedule.is_none() && matches!(notional.amort, AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentPerPeriod { .. }) {
            return Err(InputError::Invalid.into());
        }

        // Precompute amortization helpers
        let step_remaining_map: Option<hashbrown::HashMap<Date, Money>> = match &notional.amort {
            AmortizationSpec::StepRemaining { schedule } => {
                let mut m = hashbrown::HashMap::with_capacity(schedule.len());
                for (d, mny) in schedule { m.insert(*d, *mny); }
                Some(m)
            }
            _ => None,
        };
        let (linear_delta, percent_per) = match &notional.amort {
            AmortizationSpec::LinearTo { final_notional } => {
                let base = amort_base_schedule.as_ref().unwrap();
                let steps = (base.len() - 1) as f64;
                (Some(((notional.initial.amount() - final_notional.amount()) / steps).max(0.0)), None)
            }
            AmortizationSpec::PercentPerPeriod { pct } => (None, Some((notional.initial.amount() * *pct).max(0.0))),
            _ => (None, None),
        };

        // Start with initial principal exchange (outflow)
        let mut flows: Vec<CashFlow> = Vec::new();
        let ccy = notional.initial.currency();
        flows.push(CashFlow { date: issue, reset_date: None, amount: notional.initial * -1.0, kind: CFKind::Notional, accrual_factor: 0.0 });

        // Track outstanding after each processed date
        let mut outstanding_after: hashbrown::HashMap<Date, f64> = hashbrown::HashMap::new();
        outstanding_after.insert(issue, notional.initial.amount());
        let mut outstanding = notional.initial.amount();

        // For amortization cadence, create a set of amort dates (excluding start)
        let amort_dates: hashbrown::HashSet<Date> = amort_base_schedule
            .as_ref()
            .map(|v| v.iter().copied().skip(1).collect())
            .unwrap_or_default();

        for &d in dates.iter().skip(1) {
            // Sum PIK to add after amortization this date
            let mut pik_to_add = 0.0;

            // Fixed and floating coupon emissions
            let (pik_f, fixed_new) = emit_fixed_coupons_on(d, &fixed_schedules, &outstanding_after, outstanding, ccy)?;
            pik_to_add += pik_f;
            flows.extend(fixed_new);
            let (pik_fl, float_new) = emit_float_coupons_on(d, &float_schedules, &outstanding_after, outstanding, ccy)?;
            pik_to_add += pik_fl;
            flows.extend(float_new);

            // Amortization
            let amort_params = AmortizationParams { ccy, amort_dates: &amort_dates, linear_delta, percent_per, step_remaining_map: &step_remaining_map };
            let amort_flows = emit_amortization_on(d, &notional, &mut outstanding, &amort_params)?;
            flows.extend(amort_flows);

            // Apply PIK capitalization after amortization
            if pik_to_add > 0.0 { outstanding += pik_to_add; }

            // Fees (periodic and fixed)
            let fee_flows = emit_fees_on(d, &periodic_fees, &fixed_fees, outstanding, ccy)?;
            flows.extend(fee_flows);

            // Final principal redemption if maturity
            if d == maturity && outstanding > 0.0 {
                flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(outstanding, ccy), kind: CFKind::Notional, accrual_factor: 0.0 });
                outstanding = 0.0;
            }

            // Record outstanding after this date
            outstanding_after.insert(d, outstanding);
        }

        // Finalize flows and produce meta/day count (use actual specs used)
        let (flows, meta, out_dc) = finalize_flows(flows, &used_fixed_specs, &used_float_specs);
        Ok(CashFlowSchedule { flows, notional, day_count: out_dc, meta })
    }
}



#[inline]
fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
    }
}

// -------------------------------------------------------------------------
// Tests (Phase 1)
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::discountable::Discountable;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve as CoreDiscCurve;
    use finstack_core::market_data::traits::Discount as _;
    use finstack_core::dates::ScheduleBuilder;
    use time::Month;

    #[test]
    fn linear_vs_step_parity() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let init = Money::new(1_000.0, Currency::USD);

        // Linear
        let mut b1 = cf();
        b1.principal(init, issue, maturity)
            .amortization(AmortizationSpec::LinearTo { final_notional: Money::new(0.0, Currency::USD) })
            .fixed_cf(fixed);
        let s1 = b1.build().unwrap();

        // Step schedule equivalent
        let sched: Vec<Date> = ScheduleBuilder::new(issue, maturity)
            .frequency(Frequency::quarterly())
            .build_raw()
            .collect();
        let delta = init.amount() / (sched.len() - 1) as f64;
        let mut remaining = init.amount();
        let mut pairs: Vec<(Date, Money)> = Vec::new();
        for &d in sched.iter().skip(1) {
            remaining = (remaining - delta).max(0.0);
            pairs.push((d, Money::new(remaining, Currency::USD)));
        }

        let mut b2 = cf();
        b2.principal(init, issue, maturity)
            .amortization(AmortizationSpec::StepRemaining { schedule: pairs })
            .fixed_cf(fixed);
        let s2 = b2.build().unwrap();

        assert_eq!(s1.flows.len(), s2.flows.len());
        for (a, b) in s1.flows.iter().zip(s2.flows.iter()) {
            assert_eq!(a.date, b.date);
            assert_eq!(a.kind, b.kind);
            assert!((a.amount.amount() - b.amount.amount()).abs() < 1e-9);
        }
    }

    #[test]
    fn pik_capitalization_increases_outstanding() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
        let init = Money::new(1_000.0, Currency::USD);

        let fixed = FixedCouponSpec {
            coupon_type: CouponType::PIK,
            rate: 0.10,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let mut b = cf();
        b.principal(init, issue, maturity).fixed_cf(fixed);
        let s = b.build().unwrap();
        let path = s.outstanding_path();
        // Find last outstanding before redemption
        let last_before = path.iter().rev().find(|(d, _)| *d < maturity).unwrap().1.amount();
        assert!(last_before > init.amount());
    }

    #[test]
    fn ordering_invariants_within_date() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2025, Month::July, 15).unwrap();
        let init = Money::new(1_000.0, Currency::USD);
        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Split { cash_pct: 0.5, pik_pct: 0.5 },
            rate: 0.10,
            freq: Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        // Percent-per-period amortization to force amort on coupon dates
        let mut b = cf();
        b.principal(init, issue, maturity)
            .amortization(AmortizationSpec::PercentPerPeriod { pct: 0.25 })
            .fixed_cf(fixed);
        let s = b.build().unwrap();

        // On coupon dates where multiple flows exist, enforce order: Fixed/Stub -> Amortization -> PIK -> Notional
        let mut by_date: hashbrown::HashMap<Date, Vec<CFKind>> = hashbrown::HashMap::new();
        for cf in &s.flows { by_date.entry(cf.date).or_default().push(cf.kind); }

        for (_d, kinds) in by_date {
            let mut sorted = kinds.clone();
            sorted.sort_by_key(|k| kind_rank(*k));
            assert_eq!(kinds, sorted);
        }
    }

    #[test]
    fn fixed_schedule_npv_equals_sum_cashflows() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.05,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let init = Money::new(1_000_000.0, Currency::USD);

        let mut b = cf();
        b.principal(init, issue, maturity).fixed_cf(fixed);
        let schedule = b.build().unwrap();

        let curve = CoreDiscCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (5.0, 1.0)])
            .linear_df()
            .build()
            .unwrap();

        let pv = schedule.npv(&curve, curve.base_date(), schedule.day_count).unwrap();

        // PV with flat curve 1.0 should equal sum of coupon amounts
        let expected = schedule
            .flows
            .iter()
            .fold(0.0, |sum, cf| sum + cf.amount.amount());
        assert!((pv.amount() - expected).abs() < 1e-9);
    }

    #[test]
    fn detects_stub_periods() {
        let issue = Date::from_calendar_date(2025, Month::January, 10).unwrap(); // irregular
        let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();

        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: 0.04,
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let init = Money::new(1_000_000.0, Currency::USD);

        let mut b = cf();
        b.principal(init, issue, maturity).fixed_cf(fixed);
        let schedule = b.build().unwrap();

        // Find coupon flows (not notional)
        let coupon_flows: Vec<&CashFlow> = schedule.flows.iter().filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub).collect();
        
        // At least one should be a stub due to irregular start date
        let has_stub = coupon_flows.iter().any(|cf| cf.kind == CFKind::Stub);
        assert!(has_stub, "Should detect stub period with irregular start date");
    }

    #[test]
    fn outstanding_by_date_dedup_and_values() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2025, Month::July, 15).unwrap();
        let init = Money::new(10_000.0, Currency::USD);

        // Force multiple flows per date: split coupon (cash + PIK) and amortization on coupon dates
        let fixed = FixedCouponSpec {
            coupon_type: CouponType::Split { cash_pct: 0.5, pik_pct: 0.5 },
            rate: 0.12,
            freq: Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        };

        let mut b = cf();
        b.principal(init, issue, maturity)
            .amortization(AmortizationSpec::PercentPerPeriod { pct: 0.25 })
            .fixed_cf(fixed);
        let s = b.build().unwrap();

        let end_by_date = s.outstanding_by_date();

        // 1) One entry per unique date
        let unique_dates: std::collections::BTreeSet<Date> = s.flows.iter().map(|cf| cf.date).collect();
        assert_eq!(end_by_date.len(), unique_dates.len());
        // Dates are ordered
        for ((d1, _), d2) in end_by_date.iter().zip(unique_dates.iter()) {
            assert_eq!(d1, d2);
        }

        // 2) Values match the final outstanding on each date from outstanding_path()
        let path = s.outstanding_path();
        let mut last_by_date: hashbrown::HashMap<Date, f64> = hashbrown::HashMap::new();
        for (d, m) in path { last_by_date.insert(d, m.amount()); }

        for (d, m) in end_by_date {
            let expected = *last_by_date.get(&d).unwrap();
            assert!((m.amount() - expected).abs() < 1e-9);
        }
    }
}


