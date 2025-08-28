use crate::cashflow::amortization::AmortizationSpec;
use crate::cashflow::notional::Notional;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency, ScheduleBuilder, StubKind};
use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::dates::{adjust, BusinessDayConvention};
use finstack_core::error::InputError;
use finstack_core::money::Money;
use time::Duration;

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
    pub fn by_kind(&self, kind: CFKind) -> Vec<CashFlow> {
        self.flows.iter().copied().filter(|cf| cf.kind == kind).collect()
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
}

/// Minimal schedule metadata (phase 1).
#[derive(Debug, Clone, Default)]
pub struct CashflowMeta {
    pub calendar_ids: Vec<&'static str>,
}

/// Coupon cashflow type for fixed/floating coupons.
#[derive(Debug, Clone, Copy)]
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
    pub bdc: finstack_core::dates::BusinessDayConvention,
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
    pub bdc: finstack_core::dates::BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
    pub reset_lag_days: i32,
}

/// Fee specification (scaffold for phase 2).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FeeSpec {
    Fixed { date: Date, amount: Money },
    PeriodicBps { base: FeeBase, bps: f64, freq: Frequency, dc: DayCount, bdc: finstack_core::dates::BusinessDayConvention, calendar_id: Option<&'static str>, stub: StubKind },
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone)]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit − outstanding, 0).
    Undrawn { facility_limit: Money },
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

    /// Build the complete cashflow schedule.
    pub fn build(&self) -> finstack_core::Result<CashFlowSchedule> {
        // Validate principal
        let notional = self.notional.clone().ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
        let issue = self.issue.ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;
        let maturity = self.maturity.ok_or_else(|| finstack_core::Error::from(InputError::Invalid))?;


        // Build schedules for fixed coupons
        let mut fixed_schedules: Vec<(FixedCouponSpec, Vec<Date>, hashbrown::HashMap<Date, Date>)> = Vec::new();
        for spec in &self.fixed {
            let builder = ScheduleBuilder::new(issue, maturity)
                .frequency(spec.freq)
                .stub_rule(spec.stub);
            let dates: Vec<Date> = if let Some(id) = spec.calendar_id {
                if let Some(cal) = calendar_by_id(id) {
                    builder.adjust_with(spec.bdc, cal).build().collect()
                } else {
                    builder.build_raw().collect()
                }
            } else {
                builder.build_raw().collect()
            };
            if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }
            let mut prev_map = hashbrown::HashMap::with_capacity(dates.len());
            let mut prev = dates[0];
            for &d in dates.iter().skip(1) { prev_map.insert(d, prev); prev = d; }
            fixed_schedules.push((*spec, dates, prev_map));
        }

        // Build schedules for floating coupons
        let mut float_schedules: Vec<(FloatingCouponSpec, Vec<Date>, hashbrown::HashMap<Date, Date>)> = Vec::new();
        for spec in &self.floating {
            let builder = ScheduleBuilder::new(issue, maturity)
                .frequency(spec.freq)
                .stub_rule(spec.stub);
            let dates: Vec<Date> = if let Some(id) = spec.calendar_id {
                if let Some(cal) = calendar_by_id(id) {
                    builder.adjust_with(spec.bdc, cal).build().collect()
                } else { builder.build_raw().collect() }
            } else { builder.build_raw().collect() };
            if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }
            let mut prev_map = hashbrown::HashMap::with_capacity(dates.len());
            let mut prev = dates[0];
            for &d in dates.iter().skip(1) { prev_map.insert(d, prev); prev = d; }
            float_schedules.push((*spec, dates, prev_map));
        }

        // Periodic fee schedules (with previous map for yf)
        struct PeriodicFee {
            base: FeeBase,
            bps: f64,
            dc: DayCount,
            dates: Vec<Date>,
            prev: hashbrown::HashMap<Date, Date>,
        }

        let mut periodic_fees: Vec<PeriodicFee> = Vec::new();
        let mut fixed_fees: Vec<(Date, Money)> = Vec::new();
        for fee in &self.fees {
            match fee {
                FeeSpec::Fixed { date, amount } => fixed_fees.push((*date, *amount)),
                FeeSpec::PeriodicBps { base, bps, freq, dc, stub, calendar_id, .. } => {
                    let builder = ScheduleBuilder::new(issue, maturity)
                        .frequency(*freq)
                        .stub_rule(*stub);
                    let dates: Vec<Date> = if let Some(id) = calendar_id {
                        if let Some(cal) = calendar_by_id(id) {
                            // Use Following by default for fees if bdc not specified for fee
                            builder.adjust_with(BusinessDayConvention::Following, cal).build().collect()
                        } else { builder.build_raw().collect() }
                    } else { builder.build_raw().collect() };
                    if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }
                    let mut prev_map = hashbrown::HashMap::with_capacity(dates.len());
                    let mut prev = dates[0];
                    for &d in dates.iter().skip(1) { prev_map.insert(d, prev); prev = d; }
                    periodic_fees.push(PeriodicFee { base: base.clone(), bps: *bps, dc: *dc, dates, prev: prev_map });
                }
            }
        }

        // Union of all relevant dates
        let mut union: hashbrown::HashSet<Date> = hashbrown::HashSet::new();
        union.insert(issue);
        union.insert(maturity);
        for (_, ds, _) in &fixed_schedules { for &d in ds.iter().skip(1) { union.insert(d); } union.insert(ds[0]); }
        for (_, ds, _) in &float_schedules { for &d in ds.iter().skip(1) { union.insert(d); } union.insert(ds[0]); }
        for pf in &periodic_fees { for &d in pf.dates.iter().skip(1) { union.insert(d); } union.insert(pf.dates[0]); }
        for (d, _) in &fixed_fees { union.insert(*d); }
        if let AmortizationSpec::CustomPrincipal { items } = &notional.amort { for (d, _) in items { union.insert(*d); } }

        let mut dates: Vec<Date> = union.into_iter().collect();
        dates.sort();
        if dates.len() < 2 { return Err(InputError::TooFewPoints.into()); }

        // Determine amortization cadence when needed (linear/percent)
        let amort_base_schedule: Option<Vec<Date>> = if matches!(notional.amort, AmortizationSpec::LinearTo { .. } | AmortizationSpec::PercentPerPeriod { .. }) {
            if let Some((_, ds, _)) = fixed_schedules.first() { Some(ds.clone()) }
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
            // Sum PIK added this date to adjust outstanding after amort
            let mut pik_to_add = 0.0;

            // Fixed coupons on this date
            for (spec, _ds, prev_map) in &fixed_schedules {
                if let Some(&prev) = prev_map.get(&d) {
                    let base_out = *outstanding_after.get(&prev).unwrap_or(&outstanding);
                    let yf = spec.dc.year_fraction(prev, d)?;
                    let coupon_total = base_out * (spec.rate * yf);
                    let (cash_pct, pik_pct) = spec.coupon_type.split_parts();
                    let cash_amt = coupon_total * cash_pct;
                    let pik_amt = coupon_total * pik_pct;
                    if cash_amt > 0.0 {
                        flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(cash_amt, ccy), kind: if is_stub_date(prev_map, d) { CFKind::Stub } else { CFKind::Fixed }, accrual_factor: yf });
                    }
                    if pik_amt > 0.0 {
                        flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(pik_amt, ccy), kind: CFKind::PIK, accrual_factor: 0.0 });
                        pik_to_add += pik_amt;
                    }
                }
            }

            // Floating coupons on this date (margin-only)
            for (spec, _ds, prev_map) in &float_schedules {
                if let Some(&prev) = prev_map.get(&d) {
                    let base_out = *outstanding_after.get(&prev).unwrap_or(&outstanding);
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
                        flows.push(CashFlow { date: d, reset_date: Some(reset_date), amount: Money::new(cash_amt, ccy), kind: CFKind::FloatReset, accrual_factor: yf });
                    } else {
                        // emit zero-amount resets? Skip for determinism/compactness
                    }
                    if pik_amt > 0.0 {
                        flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(pik_amt, ccy), kind: CFKind::PIK, accrual_factor: 0.0 });
                        pik_to_add += pik_amt;
                    }
                }
            }

            // Amortization on this date
            match &notional.amort {
                AmortizationSpec::None => {}
                AmortizationSpec::LinearTo { .. } => {
                    if amort_dates.contains(&d) {
                        if let Some(delta) = linear_delta {
                            let pay = delta.min(outstanding);
                            if pay > 0.0 {
                                flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                                outstanding -= pay;
                            }
                        }
                    }
                }
                AmortizationSpec::StepRemaining { .. } => {
                    if let Some(map) = &step_remaining_map {
                        if let Some(rem_after) = map.get(&d) {
                            let target = rem_after.amount();
                            let pay = (outstanding - target).max(0.0).min(outstanding);
                            if pay > 0.0 {
                                flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                                outstanding -= pay;
                            }
                        }
                    }
                }
                AmortizationSpec::PercentPerPeriod { .. } => {
                    if amort_dates.contains(&d) {
                        if let Some(per) = percent_per {
                            let pay = per.min(outstanding);
                            if pay > 0.0 {
                                flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                                outstanding -= pay;
                            }
                        }
                    }
                }
                AmortizationSpec::CustomPrincipal { items } => {
                    for (dd, amt) in items {
                        if *dd == d {
                            let pay = amt.amount().max(0.0).min(outstanding);
                            if pay > 0.0 {
                                flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(-pay, ccy), kind: CFKind::Amortization, accrual_factor: 0.0 });
                                outstanding -= pay;
                            }
                        }
                    }
                }
            }

            // Apply PIK capitalization after amortization
            if pik_to_add > 0.0 { outstanding += pik_to_add; }

            // Now that outstanding reflects post-amort & post-PIK, compute periodic fees on this date
            for pf in &periodic_fees {
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
                        flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(fee_amt, ccy), kind: CFKind::Fee, accrual_factor: 0.0 });
                    }
                }
            }

            // Fixed one-off fees on this date
            for (fd, amt) in &fixed_fees { if *fd == d { flows.push(CashFlow { date: d, reset_date: None, amount: *amt, kind: CFKind::Fee, accrual_factor: 0.0 }); } }

            // Final principal redemption if maturity
            if d == maturity && outstanding > 0.0 {
                flows.push(CashFlow { date: d, reset_date: None, amount: Money::new(outstanding, ccy), kind: CFKind::Notional, accrual_factor: 0.0 });
                outstanding = 0.0;
            }

            // Record outstanding after this date
            outstanding_after.insert(d, outstanding);
        }

        // Deterministic ordering: by date then within-date kind order
        flows.sort_by(|a, b| {
            use core::cmp::Ordering;
            match a.date.cmp(&b.date) {
                Ordering::Less => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
                Ordering::Equal => kind_rank(a.kind).cmp(&kind_rank(b.kind)),
            }
        });

        // Meta: collect calendar ids used
        let mut cals: Vec<&'static str> = Vec::new();
        for s in &self.fixed { if let Some(id) = s.calendar_id { cals.push(id); } }
        for s in &self.floating { if let Some(id) = s.calendar_id { cals.push(id); } }
        let meta = CashflowMeta { calendar_ids: cals };

        // Choose representative day_count
        let out_dc = if let Some(s) = self.fixed.first() { s.dc } else if let Some(s) = self.floating.first() { s.dc } else { DayCount::Act365F };
        Ok(CashFlowSchedule { flows, notional, day_count: out_dc, meta })
    }
}

#[inline]
#[allow(dead_code)]
fn is_stub(idx: usize, total_dates: usize) -> bool {
    idx == 1 || idx == total_dates - 1
}

#[inline]
fn is_stub_date(prev_map: &hashbrown::HashMap<Date, Date>, d: Date) -> bool {
    // Consider stub if either first or last period in this schedule (by absence of prev/next context we approximate):
    if let Some(&prev) = prev_map.get(&d) {
        // If prev has no predecessor, it's the first period; if d is the max key, it's last.
        // Approximate: mark only first period as stub here; full detection would compare yf vs typical.
        !prev_map.values().any(|&v| v == prev)
    } else { false }
}

#[inline]
fn kind_rank(kind: CFKind) -> u8 {
    match kind {
        CFKind::Fixed | CFKind::Stub => 0,
        CFKind::Fee => 1,
        CFKind::Amortization => 2,
        CFKind::PIK => 3,
        CFKind::Notional => 4,
        _ => 5,
    }
}

// -------------------------------------------------------------------------
// Tests (Phase 1)
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
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
            bdc: finstack_core::dates::BusinessDayConvention::Following,
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
            bdc: finstack_core::dates::BusinessDayConvention::Following,
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
            bdc: finstack_core::dates::BusinessDayConvention::Following,
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
}


