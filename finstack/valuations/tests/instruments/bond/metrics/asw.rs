//! Asset swap (ASW) forward helper tests.
//!
//! These tests focus on:
//! - Explicit erroring when dirty market price is missing for market ASW.
//! - Basic golden checks for the forward-based ASW helpers against a
//!   hand-computed implementation using the same curves and schedule.
//! - Verifying the fixed-leg day-count override is respected.

use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::error::{Error, InputError};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::bond::metrics::price_yield_spread::asw::{
    asw_market_with_forward, asw_par_with_forward, asw_par_with_forward_config,
};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use std::collections::BTreeSet;
use time::macros::date;

fn build_test_market_and_bond() -> (Bond, MarketContext) {
    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);

    let notional = Money::new(100.0, Currency::USD);

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.03), (5.0, 0.035)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    // Fixed-rate bond; coupon and day-count intentionally differ from the
    // forward curve's day-count so that fixed-leg DC overrides matter.
    let bond = Bond::fixed(
        "ASW-FIXED",
        notional,
        0.05,
        as_of,
        maturity,
        "USD-OIS",
    );

    (bond, market)
}

#[test]
fn test_asw_market_forward_missing_dirty_price_returns_error() {
    let (bond, market) = build_test_market_and_bond();
    let as_of = bond.issue;

    let result = asw_market_with_forward(
        &bond,
        &market,
        as_of,
        "USD-SOFR-3M",
        25.0,
        None,
    );

    match result {
        Err(Error::Input(InputError::NotFound { id })) => {
            assert!(
                id.contains("dirty_price_ccy"),
                "expected missing dirty price id to mention dirty_price_ccy, got {}",
                id
            );
        }
        Err(e) => panic!(
            "expected InputError::NotFound for missing dirty price, got {}",
            e
        ),
        Ok(v) => panic!(
            "expected ASW market calculation to fail for missing dirty price, got {}",
            v
        ),
    }
}

#[test]
fn test_asw_forward_matches_manual_for_fixed_bond() {
    let (bond, market) = build_test_market_and_bond();
    let as_of = bond.issue;

    // Re-build curves from the market so manual and helper paths share state.
    let disc = market
        .get_discount_ref(bond.discount_curve_id.as_str())
        .expect("discount curve should be present for test");
    let fwd = market
        .get_forward_ref("USD-SOFR-3M")
        .expect("forward curve should be present for test");

    // Build the simplified holder schedule (dates only) mirroring ASW helpers.
    let flows = bond
        .build_schedule(&market, as_of)
        .expect("schedule building should succeed in test");

    let mut date_set: BTreeSet<finstack_core::dates::Date> = BTreeSet::new();
    for (d, _) in &flows {
        if *d > as_of {
            date_set.insert(*d);
        }
    }
    let mut sched: Vec<finstack_core::dates::Date> = Vec::with_capacity(date_set.len() + 1);
    sched.push(as_of);
    sched.extend(date_set);

    assert!(
        sched.len() >= 2,
        "test schedule should contain at least one future payment date"
    );

    // Fixed-leg annuity using the bond's coupon day-count.
    let fixed_dc = bond.cashflow_spec.day_count();
    let mut ann = 0.0;
    let mut prev = sched[0];
    for &d in &sched[1..] {
        let alpha = fixed_dc
            .year_fraction(
                prev,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("year fraction should be computable in test");
        let p = disc.df_on_date_curve(d);
        ann += alpha * p;
        prev = d;
    }
    assert!(ann > 0.0, "annuity should be positive in test setup");

    let notional = bond.notional.amount();

    // Floating-leg PV using forward rates plus margin, mirroring ASW helper.
    let f_base = fwd.base_date();
    let f_dc = fwd.day_count();
    let float_spread_bp = 25.0;
    let spread = float_spread_bp * 1e-4;

    let mut pv_float = 0.0;
    let mut prev_d = sched[0];
    for &d in &sched[1..] {
        let t1 = f_dc
            .year_fraction(
                f_base,
                prev_d,
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("year fraction t1 should be computable");
        let t2 = f_dc
            .year_fraction(
                f_base,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("year fraction t2 should be computable");
        let yf = f_dc
            .year_fraction(
                prev_d,
                d,
                finstack_core::dates::DayCountCtx::default(),
            )
            .expect("year fraction over period should be computable");
        let rate = fwd.rate_period(t1, t2) + spread;
        let coupon_flt = notional * rate * yf;
        let df = disc.df_on_date_curve(d);
        pv_float += coupon_flt * df;
        prev_d = d;
    }

    let par_rate_manual = pv_float / (notional * ann);
    let coupon = match &bond.cashflow_spec {
        finstack_valuations::instruments::bond::CashflowSpec::Fixed(spec) => spec.rate,
        _ => panic!("test bond should be fixed-rate"),
    };
    let par_asw_manual = coupon - par_rate_manual;

    // Choose a dirty price above par (e.g., 101.25%) for the market ASW leg.
    let dirty_price_ccy = 1.0125 * notional;
    let price_pct = dirty_price_ccy / notional;
    let mkt_asw_manual = par_asw_manual + (price_pct - 1.0) / ann;

    // Library helpers under test.
    let par = asw_par_with_forward(
        &bond,
        &market,
        as_of,
        "USD-SOFR-3M",
        float_spread_bp,
    )
    .expect("par ASW helper should succeed");
    let mkt = asw_market_with_forward(
        &bond,
        &market,
        as_of,
        "USD-SOFR-3M",
        float_spread_bp,
        Some(dirty_price_ccy),
    )
    .expect("market ASW helper should succeed");

    // Golden check: helper output should be within 1–2 bp of the manual
    // implementation when conventions are aligned.
    let bp_tol = 2e-4; // 2 basis points in decimal
    assert!(
        (par - par_asw_manual).abs() <= bp_tol,
        "par ASW helper should match manual calculation within 2bp; helper={} manual={}",
        par,
        par_asw_manual
    );
    assert!(
        (mkt - mkt_asw_manual).abs() <= bp_tol,
        "market ASW helper should match manual calculation within 2bp; helper={} manual={}",
        mkt,
        mkt_asw_manual
    );
}

#[test]
fn test_asw_par_forward_respects_fixed_leg_daycount_override() {
    let (bond, market) = build_test_market_and_bond();
    let as_of = bond.issue;

    // Baseline ASW using bond's coupon day-count on the fixed leg.
    let par_default = asw_par_with_forward(
        &bond,
        &market,
        as_of,
        "USD-SOFR-3M",
        25.0,
    )
    .expect("par ASW with default conventions should succeed");

    // Override fixed-leg day-count to a different convention (Act/365F)
    // to mimic a swap fixed-leg convention differing from the bond.
    let par_override = asw_par_with_forward_config(
        &bond,
        &market,
        as_of,
        "USD-SOFR-3M",
        25.0,
        Some(DayCount::Act365F),
    )
    .expect("par ASW with explicit fixed-leg day-count should succeed");

    // For this test setup, using a different fixed-leg day-count should
    // produce a non-zero difference.
    assert!(
        (par_default - par_override).abs() > 1e-8,
        "expected fixed-leg day-count override to change par ASW; default={} override={}",
        par_default,
        par_override
    );
}

#[test]
fn test_asw_par_finite() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "ASW1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::ASWPar])
        .unwrap();
    let asw = *result.measures.get("asw_par").unwrap();
    assert!(asw.is_finite() && asw.abs() < 0.5);
}
