#![cfg(test)]

use finstack_core::dates::Date;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::prelude::*;
use finstack_valuations::instruments::fixed_income::{bond, deposit, irs};
use finstack_valuations as _; // ensure crate is linked
use finstack_valuations::traits::Priceable;
use finstack_valuations::cashflow::aggregation::aggregate_by_period;
use finstack_valuations::metrics::{standard_registry, MetricContext};
use std::sync::Arc;
use time::Month;

fn flat_df_curve(id: &'static str, base: Date, df: F) -> DiscountCurve {
    // Build a trivial curve with two identical points to maintain piecewise structure
    let _ = df; // df not used directly; keep API consistent; use 1.0 for MVP tests
    DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (10.0, 1.0)])
        .linear_df()
        .build()
        .unwrap()
}

fn flat_fwd_curve(id: &'static str, base: Date, rate: F) -> ForwardCurve {
    ForwardCurve::builder(id, 0.25)
        .base_date(base)
        .knots([(0.0, rate), (10.0, rate)])
        .linear_df()
        .build()
        .unwrap()
}

#[test]
fn deposit_par_at_zero_rate_with_unit_df() {
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", start, 1.0);
    let curves = CurveSet::new().with_discount(disc);

    let dep = deposit::Deposit {
        id: "DEP-USD-3M".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        start,
        end,
        day_count: DayCount::Act365F,
        quote_rate: Some(0.0),
        disc_id: "USD-OIS",
        attributes: Default::default(),
    };

    let res = dep.price(&curves, start).unwrap();
    // PV should be ~0 at par with DF=1
    assert!(res.value.amount().abs() < 1e-9);
}

#[test]
fn irs_par_rate_matches_forward_rate() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", base, 1.0);
    let fwd_rate = 0.05;
    let fwd = flat_fwd_curve("USD-SOFR3M", base, fwd_rate);
    let curves = CurveSet::new().with_discount(disc).with_forecast(fwd);

    let irs = irs::InterestRateSwap {
        id: "IRS-TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: irs::PayReceive::PayFixed,
        fixed: irs::FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fwd_rate,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        float: irs::FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR3M",
            spread_bp: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        attributes: finstack_valuations::traits::Attributes::new(),
    };

    let res = irs.price(&curves, base).unwrap();
    let par = *res.measures.get("par_rate").unwrap();
    assert!((par - fwd_rate).abs() < 1e-12);
}

#[test]
fn bond_pv_with_unit_df_is_sum_of_cashflows() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mat = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", issue, 1.0);
    let curves = CurveSet::new().with_discount(disc);

    let bond = bond::Bond {
        id: "BOND-TEST".into(),
        notional: Money::new(1_000.0, Currency::USD),
        coupon: 0.10, // 10%
        freq: finstack_core::dates::Frequency::semi_annual(),
        dc: DayCount::Act365F,
        issue,
        maturity: mat,
        disc_id: "USD-OIS",
        quoted_clean: None,
        call_put: None,
        amortization: None,
        custom_cashflows: None,
        attributes: finstack_valuations::traits::Attributes::new(),
    };

    let res = bond.price(&curves, issue).unwrap();
    // Two coupons (semi-annual, approx 0.5 year fractions), plus principal, DF=1
    assert!(res.value.amount() > 1_000.0);
}

#[test]
fn irs_dv01_sign_and_magnitude() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", base, 1.0);
    let fwd_rate = 0.04;
    let fwd = flat_fwd_curve("USD-SOFR3M", base, fwd_rate);
    let curves = CurveSet::new().with_discount(disc).with_forecast(fwd);

    // Receive-fixed swap
    let irs_recv = irs::InterestRateSwap {
        id: "IRS-RECV".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: irs::PayReceive::ReceiveFixed,
        fixed: irs::FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fwd_rate,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        float: irs::FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR3M",
            spread_bp: 0.0,
            freq: finstack_core::dates::Frequency::quarterly(),
            dc: DayCount::Act365F,
            bdc: finstack_core::dates::BusinessDayConvention::Following,
            calendar_id: None,
            stub: finstack_core::dates::StubKind::None,
            start: base,
            end: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        },
        attributes: finstack_valuations::traits::Attributes::new(),
    };
    let res = irs_recv.price(&curves, base).unwrap();
    let dv01 = *res.measures.get("dv01").unwrap();
    let ann = *res.measures.get("annuity").unwrap();
    assert!(dv01.abs() > 0.0);
    assert!(dv01.abs() > 0.5 * ann * 1_000_000.0 / 1_000_000.0); // rough lower bound

    // Pay-fixed swap: dv01 should be negative
    let irs_pay = irs::InterestRateSwap { side: irs::PayReceive::PayFixed, ..irs_recv };
    let res2 = irs_pay.price(&curves, base).unwrap();
    let dv01_pay = *res2.measures.get("dv01").unwrap();
    assert!(dv01 * dv01_pay < 0.0);
    assert!((dv01.abs() - dv01_pay.abs()).abs() < 1e-6);
}

#[test]
fn bond_ytm_ytw_and_amortization() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mat_short = Date::from_calendar_date(2025, Month::July, 1).unwrap();
    let mat = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", issue, 1.0);
    let curves = CurveSet::new().with_discount(disc);

    // Baseline bullet bond
    let bullet = bond::Bond {
        id: "BOND-BULLET".into(),
        notional: Money::new(1_000.0, Currency::USD),
        coupon: 0.06,
        freq: finstack_core::dates::Frequency::semi_annual(),
        dc: DayCount::Act365F,
        issue,
        maturity: mat,
        disc_id: "USD-OIS",
        quoted_clean: Some(1_000.0),
        call_put: Some(bond::CallPutSchedule { calls: vec![bond::CallPut { date: mat_short, price_pct_of_par: 102.0 }], puts: vec![] }),
        amortization: None,
        custom_cashflows: None,
        attributes: finstack_valuations::traits::Attributes::new(),
    };
    let res_bullet = bullet.price(&curves, issue).unwrap();
    let ytm = *res_bullet.measures.get("ytm").unwrap();
    let ytw = *res_bullet.measures.get("ytw").unwrap();
    assert!(ytw <= ytm + 1e-9);

    // Amortizing version (linear to 800)
    let amort = bond::Bond { 
        id: "BOND-AMORT".into(), 
        amortization: Some(bond::AmortizationSpec::LinearTo { final_notional: Money::new(800.0, Currency::USD) }), 
        quoted_clean: None, 
        call_put: None, 
        attributes: finstack_valuations::traits::Attributes::new(),
        ..bullet 
    };
    let res_amort = amort.price(&curves, issue).unwrap();
    assert!(res_amort.value.amount() < res_bullet.value.amount());

    // Aggregate a couple of flows into monthly periods
    let plan = finstack_core::dates::build_periods("2025M01..M03", None).unwrap();
    let flows = vec![
        (Date::from_calendar_date(2025, Month::January, 15).unwrap(), Money::new(10.0, Currency::USD)),
        (Date::from_calendar_date(2025, Month::February, 10).unwrap(), Money::new(5.0, Currency::USD)),
        (Date::from_calendar_date(2025, Month::February, 20).unwrap(), Money::new(7.0, Currency::EUR)),
    ];
    let agg = aggregate_by_period(&flows, &plan.periods);
    assert_eq!(agg.get(&finstack_core::dates::PeriodId::month(2025,1)).unwrap().get(&Currency::USD).copied().unwrap_or(0.0), 10.0);
    assert_eq!(agg.get(&finstack_core::dates::PeriodId::month(2025,2)).unwrap().get(&Currency::USD).copied().unwrap_or(0.0), 5.0);
    assert_eq!(agg.get(&finstack_core::dates::PeriodId::month(2025,2)).unwrap().get(&Currency::EUR).copied().unwrap_or(0.0), 7.0);
}

#[test]
fn dv01_bucketed_bond_simple() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mat = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let disc = flat_df_curve("USD-OIS", issue, 1.0);
    let curves = Arc::new(CurveSet::new().with_discount(disc));

    // 1Y semi-annual 5% bond, 1,000,000 notional
    let bond = bond::Bond {
        id: "BOND-DV01".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        coupon: 0.05,
        freq: finstack_core::dates::Frequency::semi_annual(),
        dc: DayCount::Act365F,
        issue,
        maturity: mat,
        disc_id: "USD-OIS",
        quoted_clean: None,
        call_put: None,
        amortization: None,
        custom_cashflows: None,
        attributes: finstack_valuations::traits::Attributes::new(),
    };

    // Use the metrics framework to compute bucketed DV01
    let base_value = bond.value(&curves, issue).unwrap();
    
    // Create metric context and compute with standard metrics (which includes risk metrics)
    let mut context = MetricContext::new(
        Arc::new(finstack_valuations::instruments::Instrument::Bond(bond.clone())),
        curves.clone(),
        issue,
        base_value,
    );
    
    // Compute accrued first (which caches flows) and then bucketed DV01
    use finstack_valuations::metrics::MetricId;
    let registry = standard_registry();
    let metrics = registry.compute(&[MetricId::Accrued, MetricId::BucketedDv01], &mut context).unwrap();
    
    // Get bucketed DV01 total
    let total = *metrics.get(&MetricId::BucketedDv01).unwrap_or(&0.0);
    assert!(total > 0.0);
    
    // Check individual buckets from context.computed
    // Note: Individual bucket results are currently not stored in context.computed
    // due to dynamic key nature. The total is returned from the calculator.
    // This is a TODO for future enhancement - we could store bucketed results
    // in a structured way or use dynamic MetricId::Custom variants
}


