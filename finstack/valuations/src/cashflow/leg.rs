use crate::cashflow::notional::Notional;
use crate::cashflow::amortization::AmortizationSpec;
use crate::cashflow::primitives::{CFKind, CashFlow};
use finstack_core::dates::{Date, DayCount};
use finstack_core::error::InputError;
use finstack_core::money::Money;

/// Collection of ordered cash-flows plus leg-level metadata.
#[derive(Debug, Clone)]
pub struct CashFlowLeg {
    /// Cash-flows in chronological order.
    pub flows: Vec<CashFlow>,
    /// Notional amount (currency tags PV results).
    pub notional: Notional,
    /// Day-count convention used for accrual calculations.
    pub day_count: DayCount,
}

impl CashFlowLeg {
    /// Build a fixed-rate coupon leg from a **schedule iterator**.
    ///
    /// * `notional` – principal amount (currency of the leg).
    /// * `rate` – fixed coupon rate (e.g. 0.025 for 2.5 %).
    /// * `schedule` – iterator yielding **inclusive** date sequence (start, …, end).
    /// * `day_count` – convention to derive accrual fractions.
    pub fn fixed_rate<I>(
        notional: Notional,
        rate: f64,
        schedule: I,
        day_count: DayCount,
    ) -> finstack_core::Result<Self>
    where
        I: IntoIterator<Item = Date>,
    {
        let dates: Vec<Date> = schedule.into_iter().collect();
        if dates.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }

        let mut flows = Vec::with_capacity(dates.len() - 1);
        let init_notional_amt = notional.initial;
        // For step amortisation, we need to track outstanding through the loop
        // and apply the provided "remaining after date" map on coupon dates.
        let mut step_remaining_after: Option<hashbrown::HashMap<Date, Money>> = None;
        if let AmortizationSpec::StepRemaining { schedule } = &notional.amort {
            let mut map = hashbrown::HashMap::with_capacity(schedule.len());
            for (d, m) in schedule {
                map.insert(*d, *m);
            }
            step_remaining_after = Some(map);
        }
        let mut current_outstanding_step = init_notional_amt;
        let mut prev = dates[0];
        for (idx, &curr) in dates[1..].iter().enumerate() {
            let yf = day_count.year_fraction(prev, curr)?;
            // Determine current outstanding notional amount
            let outstanding = match &notional.amort {
                AmortizationSpec::None => init_notional_amt,
                AmortizationSpec::LinearTo { final_notional } => {
                    let steps = (dates.len() - 1) as f64;
                    let delta = (init_notional_amt.amount() - final_notional.amount()) / steps;
                    let current = init_notional_amt.amount() - delta * idx as f64;
                    Money::new(current, init_notional_amt.currency())
                }
                AmortizationSpec::StepRemaining { .. } => current_outstanding_step,
                AmortizationSpec::PercentPerPeriod { .. } => init_notional_amt,
                AmortizationSpec::CustomPrincipal { .. } => init_notional_amt,
            };

            let amt = outstanding * (rate * yf);
            // Determine if this period is an irregular stub (first or last)
            let kind = if idx == 0 || idx == dates.len() - 2 {
                // compare yf to typical period (second period if exists)
                let norm_yf = if dates.len() > 2 {
                    day_count.year_fraction(dates[1], dates[2]).unwrap_or(yf)
                } else {
                    yf
                };
                if (yf - norm_yf).abs() > 1e-6 {
                    CFKind::Stub
                } else {
                    CFKind::Fixed
                }
            } else {
                CFKind::Fixed
            };

            let cf = CashFlow { date: curr, reset_date: None, amount: amt, kind, accrual_factor: yf };
            flows.push(cf);
            prev = curr;

            // Update step outstanding AFTER the coupon date based on provided schedule
            if let Some(map) = &step_remaining_after {
                if let Some(rem_after) = map.get(&curr) {
                    current_outstanding_step = *rem_after;
                }
            }
        }

        // Build leg struct first
        let mut leg = Self {
            flows,
            notional,
            day_count,
        };

        // Apply amortisation principal exchanges if needed
        leg.apply_amortisation();

        Ok(leg)
    }

    /// Apply Payment-in-Kind (PIK) capitalization at a constant rate on each period.
    ///
    /// Inserts `CFKind::PIK` flows dated at coupon dates, increases outstanding principal accordingly.
    pub fn apply_pik_rate(&mut self, rate: f64) {
        if self.flows.is_empty() || rate == 0.0 {
            return;
        }
        let currency = self.notional.initial.currency();
        let mut outstanding = self.notional.initial.amount();
        let mut merged: Vec<CashFlow> = Vec::with_capacity(self.flows.len() * 2);
        for cf in self.flows.iter().copied() {
            // Keep existing flow first
            merged.push(cf);
            // PIK uses accrual factor from coupon flows only
            if cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub {
                let pik_amt = (outstanding * rate * cf.accrual_factor).max(0.0);
                if pik_amt > 0.0 {
                    let pik = CashFlow { date: cf.date, reset_date: None, amount: Money::new(pik_amt, currency), kind: CFKind::PIK, accrual_factor: 0.0 };
                    merged.push(pik);
                    // Capitalize into principal
                    outstanding += pik_amt;
                }
            }
        }
        self.flows = merged;
    }

    /// Build a floating-rate leg with spread-only cashflows for schedule transparency.
    ///
    /// This generates `CFKind::FloatReset` flows with amount = notional * (spread_bp * 1e-4 * gearing * yf),
    /// and sets `reset_date = payment_date - reset_lag_days` (calendar days).
    /// Full forward-rate dependent PV remains the responsibility of pricing.
    pub fn floating_spread<I>(
        notional: Money,
        spread_bp: f64,
        gearing: f64,
        _reset_lag_days: i32,
        schedule: I,
        day_count: DayCount,
    ) -> finstack_core::Result<Self>
    where
        I: IntoIterator<Item = Date>,
    {
        let dates: Vec<Date> = schedule.into_iter().collect();
        if dates.len() < 2 {
            return Err(InputError::TooFewPoints.into());
        }
        let mut flows = Vec::with_capacity(dates.len() - 1);
        let mut prev = dates[0];
        for &curr in &dates[1..] {
            let yf = day_count.year_fraction(prev, curr)?;
            let amt = notional * ((spread_bp * 1e-4 * gearing) * yf);
            let reset_date = curr;
            let cf = CashFlow { date: curr, reset_date: Some(reset_date), amount: amt, kind: CFKind::FloatReset, accrual_factor: yf };
            flows.push(cf);
            prev = curr;
        }
        Ok(Self { flows, notional: Notional { initial: notional, amort: AmortizationSpec::None }, day_count })
    }

    /// Apply amortisation rule – inserts principal flows where applicable.
    ///
    /// Ordering guarantees:
    /// - Flows remain sorted by date.
    /// - On the same date, coupon flows (`CFKind::Fixed`/`CFKind::Stub`) precede
    ///   principal exchanges (`CFKind::Amortization`) to keep accrual logic intact.
    fn apply_amortisation(&mut self) {
        match &self.notional.amort {
            AmortizationSpec::None => {}
            AmortizationSpec::LinearTo { final_notional } => {
                let n = self.flows.len();
                if n == 0 {
                    return;
                }
                let init_amt = self.notional.initial.amount();
                let final_amt = final_notional.amount();
                let delta = (init_amt - final_amt) / n as f64;
                if delta <= 0.0 {
                    return;
                }
                let currency = self.notional.initial.currency();

                let mut remaining = init_amt;
                let mut merged: Vec<CashFlow> = Vec::with_capacity(self.flows.len() + n);
                for cf in self.flows.iter().copied() {
                    // Keep coupon first on the date
                    merged.push(cf);

                    // Principal exchange at coupon date
                    let pay = (delta).min(remaining - final_amt);
                    if pay > 0.0 {
                        let princ = CashFlow {
                            date: cf.date,
                            reset_date: None,
                            amount: Money::new(-pay, currency),
                            kind: CFKind::Amortization,
                            accrual_factor: 0.0,
                        };
                        merged.push(princ);
                        remaining -= pay;
                    }
                }
                self.flows = merged;
            }
            AmortizationSpec::StepRemaining { schedule } => {
                if self.flows.is_empty() || schedule.is_empty() {
                    return;
                }
                // Validate chronological order (non-decreasing strictly)
                let mut last = schedule[0].0;
                for (d, _) in schedule.iter().skip(1) {
                    if *d <= last { last = *d; /* keep moving; schedule may contain dupes */ } else { last = *d; }
                }

                // Map date -> remaining after that date
                let mut map: hashbrown::HashMap<Date, Money> = hashbrown::HashMap::with_capacity(schedule.len());
                for (d, m) in schedule {
                    map.insert(*d, *m);
                }

                let currency = self.notional.initial.currency();
                let mut remaining = self.notional.initial.amount();
                let mut merged: Vec<CashFlow> = Vec::with_capacity(self.flows.len() + schedule.len());
                for cf in self.flows.iter().copied() {
                    // Push coupon first
                    merged.push(cf);
                    if let Some(rem_after) = map.get(&cf.date) {
                        if rem_after.currency() == currency {
                            let target = rem_after.amount();
                            let mut pay = (remaining - target).max(0.0);
                            if pay > 0.0 {
                                // Cap by current remaining
                                if pay > remaining { pay = remaining; }
                                let princ = CashFlow {
                                    date: cf.date,
                                    reset_date: None,
                                    amount: Money::new(-pay, currency),
                                    kind: CFKind::Amortization,
                                    accrual_factor: 0.0,
                                };
                                merged.push(princ);
                                remaining -= pay;
                            }
                        }
                    }
                }
                self.flows = merged;
            }
            AmortizationSpec::PercentPerPeriod { pct } => {
                let n = self.flows.len();
                if n == 0 { return; }
                let init_amt = self.notional.initial.amount();
                let pay_per = (init_amt * *pct).max(0.0);
                if pay_per == 0.0 { return; }
                let currency = self.notional.initial.currency();
                let mut remaining = init_amt;
                let mut merged: Vec<CashFlow> = Vec::with_capacity(self.flows.len() + n);
                for cf in self.flows.iter().copied() {
                    merged.push(cf);
                    if remaining > 0.0 {
                        let pay = pay_per.min(remaining);
                        if pay > 0.0 {
                            let princ = CashFlow { date: cf.date, reset_date: None, amount: Money::new(-pay, currency), kind: CFKind::Amortization, accrual_factor: 0.0 };
                            merged.push(princ);
                            remaining -= pay;
                        }
                    }
                }
                self.flows = merged;
            }
            AmortizationSpec::CustomPrincipal { items } => {
                if self.flows.is_empty() || items.is_empty() { return; }
                let currency = self.notional.initial.currency();
                let mut map: hashbrown::HashMap<Date, Money> = hashbrown::HashMap::with_capacity(items.len());
                for (d, m) in items { map.insert(*d, *m); }
                let mut remaining = self.notional.initial.amount();
                let mut merged: Vec<CashFlow> = Vec::with_capacity(self.flows.len() + items.len());
                for cf in self.flows.iter().copied() {
                    merged.push(cf);
                    if let Some(amt) = map.get(&cf.date) {
                        let mut pay = amt.amount().max(0.0);
                        if pay > 0.0 {
                            if pay > remaining { pay = remaining; }
                            let princ = CashFlow { date: cf.date, reset_date: None, amount: Money::new(-pay, currency), kind: CFKind::Amortization, accrual_factor: 0.0 };
                            merged.push(princ);
                            remaining -= pay;
                        }
                    }
                }
                self.flows = merged;
            }
        }
    }

    /// Return accrued interest up to (but excluding) `val_date`.
    pub fn accrued(&self, val_date: Date) -> Money {

        // No accrual before first period
        if val_date <= self.flows.first().unwrap().date {
            return Money::new(0.0, self.notional.initial.currency());
        }

        // Find index of first flow after valuation date
        let idx = match self.flows.iter().position(|cf| cf.date > val_date) {
            Some(i) => i,
            None => return Money::new(0.0, self.notional.initial.currency()), // past last payment
        };

        let prev_date = if idx == 0 {
            return Money::new(0.0, self.notional.initial.currency());
        } else {
            self.flows[idx - 1].date
        };

        let curr_flow = &self.flows[idx];

        // Derive coupon rate from stored amount and accrual factor
        let coupon_rate =
            curr_flow.amount.amount() / (self.notional.initial.amount() * curr_flow.accrual_factor);

        let elapsed_yf = self
            .day_count
            .year_fraction(prev_date, val_date)
            .expect("day_count calculation should not fail");

        self.notional.initial * (coupon_rate * elapsed_yf)
    }
}

#[cfg(feature = "index")]
/// Builder for floating-rate cash-flow legs (index feature).
pub struct FloatingRateBuilder<I> {
    notional: Notional,
    index: Option<I>,
    spread_bp: f64,
    gearing: f64,
    reset_lag: i32,
    schedule: Vec<Date>,
    day_count: DayCount,
}

#[cfg(feature = "index")]
impl<I> FloatingRateBuilder<I> {
    pub fn notional(mut self, notional: Notional) -> Self {
        self.notional = notional;
        self
    }
    pub fn index(mut self, index: I) -> Self {
        self.index = Some(index);
        self
    }
    pub fn spread_bp(mut self, bp: f64) -> Self {
        self.spread_bp = bp;
        self
    }
    pub fn gearing(mut self, g: f64) -> Self {
        self.gearing = g;
        self
    }
    pub fn reset_lag(mut self, lag: i32) -> Self {
        self.reset_lag = lag;
        self
    }
    pub fn schedule<S>(mut self, sched: S) -> Self
    where
        S: IntoIterator<Item = Date>,
    {
        self.schedule = sched.into_iter().collect();
        self
    }
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }
    pub fn build(self) -> finstack_core::Result<CashFlowLeg> {
        // Minimal placeholder: generate empty flows, to be filled later.
        Ok(CashFlowLeg {
            flows: Vec::new(),
            notional: self.notional,
            day_count: self.day_count,
        })
    }
}

#[cfg(feature = "index")]
impl CashFlowLeg {
    /// Entry-point to start building a floating-rate leg.
    pub fn floating_rate<I>() -> FloatingRateBuilder<I> {
        FloatingRateBuilder {
            notional: Notional::par(0.0, finstack_core::currency::Currency::USD),
            index: None,
            spread_bp: 0.0,
            gearing: 1.0,
            reset_lag: 2,
            schedule: Vec::new(),
            day_count: DayCount::Act360,
        }
    }
}

// -------------------------------------------------------------------------
// Additional helpers for cashflow generation outside of legs
// -------------------------------------------------------------------------

/// Build dated flows for a simple money-market deposit.
///
/// Principal out at start (negative), principal+simple interest back at end (positive).
pub fn deposit_dated_flows(
    notional: Money,
    start: Date,
    end: Date,
    day_count: DayCount,
    simple_rate: Option<f64>,
) -> finstack_core::Result<Vec<(Date, Money)>> {
    let principal_out = (start, notional * -1.0);
    let yf = day_count.year_fraction(start, end)?;
    let interest = match simple_rate {
        Some(r) => notional * (r * yf),
        None => Money::new(0.0, notional.currency()),
    };
    let redemption = (end, (notional + interest)?);
    Ok(vec![principal_out, redemption])
}

/// Return floating-rate periods (prev_date, pay_date, year_fraction) from a schedule.
#[inline]
pub fn floating_periods(schedule: &[Date], day_count: DayCount) -> finstack_core::Result<Vec<(Date, Date, f64)>> {
    if schedule.len() < 2 { return Ok(Vec::new()); }
    let mut out = Vec::with_capacity(schedule.len() - 1);
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        let yf = day_count.year_fraction(prev, d)?;
        out.push((prev, d, yf));
        prev = d;
    }
    Ok(out)
}

// -------------------------------------------------------------------------
// Tests – basic fixed leg generation & NPV
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::discountable::Discountable;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Frequency, ScheduleBuilder};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve as CoreDiscCurve;
    use finstack_core::market_data::traits::Discount as _;
    use time::Month;

    #[test]
    fn fixed_leg_npv_equals_sum_cashflows() {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 15).unwrap();
        let sched = ScheduleBuilder::new(start, end)
            .frequency(Frequency::semi_annual())
            .build_raw();

        let notional = Notional::par(1_000_000.0, Currency::USD);
        let rate = 0.05; // 5 %
        let leg = CashFlowLeg::fixed_rate(notional, rate, sched, DayCount::Act365F).unwrap();

        let curve = CoreDiscCurve::builder("USD-OIS")
            .base_date(start)
            .knots([(0.0, 1.0), (5.0, 1.0)])
            .linear_df()
            .build()
            .unwrap();

        let pv = leg.npv(&curve, curve.base_date(), leg.day_count).unwrap();

        // PV with flat curve 1.0 should equal sum of coupon amounts
        let expected = leg
            .flows
            .iter()
            .fold(0.0, |sum, cf| sum + cf.amount.amount());
        assert!((pv.amount() - expected).abs() < 1e-9);
    }

    #[test]
    fn detects_stub_periods() {
        let start = Date::from_calendar_date(2025, Month::January, 10).unwrap(); // irregular
        let end = Date::from_calendar_date(2026, Month::January, 15).unwrap();
        let sched = ScheduleBuilder::new(start, end)
            .frequency(Frequency::semi_annual())
            .build_raw();

        let notional = Notional::par(1_000_000.0, Currency::USD);
        let leg = CashFlowLeg::fixed_rate(notional, 0.04, sched, DayCount::Act365F).unwrap();

        // First flow should be stub
        assert_eq!(leg.flows.first().unwrap().kind, CFKind::Stub);
    }

    #[test]
    fn linear_amort_inserts_principal_flows_and_pv_sums() {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 15).unwrap();
        let sched: Vec<Date> = ScheduleBuilder::new(start, end)
            .frequency(Frequency::quarterly())
            .build_raw()
            .collect();

        let init = Money::new(1_000.0, Currency::USD);
        let notional = Notional { initial: init, amort: AmortizationSpec::LinearTo { final_notional: Money::new(0.0, Currency::USD) } };
        // Zero coupon rate to isolate principal flows
        let leg = CashFlowLeg::fixed_rate(notional, 0.0, sched.clone(), DayCount::Act365F).unwrap();

        // Expect one principal flow per period, each of -250
        let principal: Vec<&CashFlow> = leg.flows.iter().filter(|cf| cf.kind == CFKind::Amortization).collect();
        let coupons: Vec<&CashFlow> = leg.flows.iter().filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub).collect();
        assert_eq!(principal.len(), coupons.len());
        let delta = 250.0;
        for (i, cf) in principal.iter().enumerate() {
            let expected_date = sched[i + 1];
            assert_eq!(cf.date, expected_date);
            assert!((cf.amount.amount() + delta).abs() < 1e-9);
        }

        // PV with flat curve at 1 equals sum of all flows
        let curve = CoreDiscCurve::builder("USD-OIS")
            .base_date(start)
            .knots([(0.0, 1.0), (5.0, 1.0)])
            .linear_df()
            .build()
            .unwrap();
        let pv = leg.npv(&curve, curve.base_date(), leg.day_count).unwrap();
        let expected = leg
            .flows
            .iter()
            .fold(0.0, |sum, cf| sum + cf.amount.amount());
        assert!((pv.amount() - expected).abs() < 1e-9);
    }

    #[test]
    fn step_amortization_matches_linear_parity() {
        let start = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 15).unwrap();
        let sched: Vec<Date> = ScheduleBuilder::new(start, end)
            .frequency(Frequency::quarterly())
            .build_raw()
            .collect();
        // 4 periods (5 dates)
        let init = Money::new(1_000.0, Currency::USD);
        let final_notional = Money::new(0.0, Currency::USD);
        let linear = Notional { initial: init, amort: AmortizationSpec::LinearTo { final_notional } };
        let rate = 0.05;
        let leg_linear = CashFlowLeg::fixed_rate(linear, rate, sched.clone(), DayCount::Act365F).unwrap();

        // Build step schedule with remaining-after-date matching linear path: 750, 500, 250, 0
        let dates_only: Vec<Date> = sched.clone();
        let mut remaining = init.amount();
        let delta = (init.amount() - final_notional.amount()) / ((dates_only.len() - 1) as f64);
        let mut step_pairs: Vec<(Date, Money)> = Vec::new();
        for &d in dates_only.iter().skip(1) {
            remaining = (remaining - delta).max(0.0);
            step_pairs.push((d, Money::new(remaining, Currency::USD)));
        }
        let step = Notional { initial: init, amort: AmortizationSpec::StepRemaining { schedule: step_pairs } };
        let leg_step = CashFlowLeg::fixed_rate(step, rate, sched.clone(), DayCount::Act365F).unwrap();

        assert_eq!(leg_linear.flows.len(), leg_step.flows.len());
        for (a, b) in leg_linear.flows.iter().zip(leg_step.flows.iter()) {
            assert_eq!(a.date, b.date);
            assert_eq!(a.kind, b.kind);
            assert!((a.amount.amount() - b.amount.amount()).abs() < 1e-9);
        }
    }
}
