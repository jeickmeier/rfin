#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::traits::Discount;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use crate::cashflow::amortization::AmortizationSpec;
use crate::cashflow::leg::CashFlowLeg;
use crate::cashflow::primitives::CFKind;
use crate::cashflow::notional::Notional;
use crate::pricing::discountable::Discountable;


// No local amortization types; use AmortizationSpec from cashflow.

/// Parameters for present valuing a fixed-rate leg with optional amortization
/// and principal/redemption handling.
#[derive(Clone, Debug)]
pub struct FixedLegParams<'a> {
    pub base: Date,
    pub dc: DayCount,
    pub notional: Money,
    pub coupon_rate: F,
    pub schedule: &'a [Date],
    pub amortization: Option<&'a AmortizationSpec>,
    pub include_principal_flows: bool,
    pub include_final_redemption: bool,
}

/// Run a closure for each consecutive pair of dates in `schedule`,
/// threading an accumulator through calls.
#[inline]
pub fn fold_periods<T, FAcc>(schedule: &[Date], init: T, mut f: FAcc) -> T
where
    FAcc: FnMut(Date, Date, T) -> T,
{
    if schedule.len() < 2 {
        return init;
    }
    let mut acc = init;
    let mut prev = schedule[0];
    for &d in &schedule[1..] {
        acc = f(prev, d, acc);
        prev = d;
    }
    acc
}

/// Fixed-leg annuity: sum over periods of yf(prev, d) * df(d).
#[inline]
pub fn annuity(disc: &dyn Discount, base: Date, dc: DayCount, schedule: &[Date]) -> F {
    fold_periods(schedule, 0.0, |prev, d, acc| {
        let yf = DiscountCurve::year_fraction(prev, d, dc);
        let df = DiscountCurve::df_on(disc, base, d, dc);
        acc + yf * df
    })
}

/// Present value of a level fixed leg (coupon-only; no redemption) with `rate`.
#[inline]
pub fn pv_fixed_leg(
    disc: &dyn Discount,
    base: Date,
    dc: DayCount,
    notional: Money,
    rate: F,
    schedule: &[Date],
) -> finstack_core::Result<Money> {
    if schedule.len() < 2 { return Ok(Money::new(0.0, notional.currency())); }
    let leg = CashFlowLeg::fixed_rate(
        Notional { initial: notional, amort: AmortizationSpec::None },
        rate,
        schedule.iter().copied(),
        dc,
    )?;
    leg.npv(disc, base, dc)
}

/// Present value of a fixed-rate leg with optional amortization and principal/redemption handling.
#[inline]
pub fn pv_fixed_leg_amortized(
    disc: &dyn Discount,
    params: FixedLegParams,
) -> finstack_core::Result<Money> {
    if params.schedule.len() < 2 { return Ok(Money::new(0.0, params.notional.currency())); }
    let amort = params.amortization.cloned().unwrap_or(AmortizationSpec::None);
    let leg = CashFlowLeg::fixed_rate(
        Notional { initial: params.notional, amort },
        params.coupon_rate,
        params.schedule.iter().copied(),
        params.dc,
    )?;

    // Optionally drop principal flows
    let flows: Vec<(Date, Money)> = if params.include_principal_flows {
        leg.flows
            .iter()
            .filter_map(|cf| match cf.kind {
                CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                CFKind::Amortization => Some((cf.date, Money::new(-cf.amount.amount(), cf.amount.currency()))),
                _ => None,
            })
            .collect()
    } else {
        leg.flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub)
            .map(|cf| (cf.date, cf.amount))
            .collect()
    };

    // Optionally add final redemption of remaining principal
    let mut flows = flows;
    if params.include_final_redemption {
        let paid_principal = leg
            .flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Amortization)
            .fold(0.0, |acc, cf| acc + (-cf.amount.amount()).max(0.0));
        let remaining = (params.notional.amount() - paid_principal).max(0.0);
        if remaining > 0.0 {
            let last_date = *params.schedule.last().unwrap();
            flows.push((last_date, Money::new(remaining, params.notional.currency())));
        }
    }

    flows.npv(disc, params.base, params.dc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::id::CurveId;
    use finstack_core::market_data::traits::TermStructure;
    use time::Month;

    struct FlatCurve {
        id: CurveId,
    }

    impl FlatCurve {
        fn new(id: &'static str) -> Self { Self { id: CurveId::new(id) } }
    }

    impl TermStructure for FlatCurve {
        fn id(&self) -> &CurveId { &self.id }
    }

    impl Discount for FlatCurve {
        fn base_date(&self) -> Date {
            Date::from_calendar_date(2025, Month::January, 1).unwrap()
        }
        fn df(&self, _t: F) -> F { 1.0 }
    }

    fn quarterly_schedule(start: Date, end: Date) -> Vec<Date> {
        finstack_core::dates::ScheduleBuilder::new(start, end)
            .frequency(finstack_core::dates::Frequency::quarterly())
            .build_raw()
            .collect()
    }

    #[test]
    fn annuity_with_unit_df_equals_sum_of_yfs() {
        let curve = FlatCurve::new("USD-OIS");
        let base = curve.base_date();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let sched = quarterly_schedule(base, end);
        let a = annuity(&curve, base, DayCount::Act365F, &sched);
        // With DF=1, annuity == sum of year fractions over the periods (~1.0)
        assert!((a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pv_fixed_leg_matches_notional_rate_times_annuity() {
        let curve = FlatCurve::new("USD-OIS");
        let base = curve.base_date();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let sched = quarterly_schedule(base, end);
        let notional = Money::new(1_000.0, Currency::USD);
        let rate = 0.06;
        let pv = pv_fixed_leg(&curve, base, DayCount::Act365F, notional, rate, &sched).unwrap();
        let a = annuity(&curve, base, DayCount::Act365F, &sched);
        assert!((pv.amount() - notional.amount() * rate * a).abs() < 1e-10);
    }
}


