//! Forward swap rate and annuity calculation tests

use crate::swaption::common::*;
use finstack_core::dates::DayCountContext;
use finstack_valuations::instruments::rates::swaption::Swaption;
use finstack_valuations::instruments::rates::swaption::{CashSettlementMethod, SwaptionSettlement};

fn expected_forward_rate(
    swaption: &Swaption,
    market: &finstack_core::market_data::context::MarketContext,
    as_of: finstack_core::dates::Date,
) -> f64 {
    let disc = market
        .get_discount(swaption.discount_curve_id.as_ref())
        .unwrap();
    let fwd = market
        .get_forward(swaption.forward_curve_id.as_ref())
        .unwrap();
    let annuity = swaption.swap_annuity(disc.as_ref(), as_of).unwrap();

    let sched = finstack_valuations::cashflow::builder::build_dates(
        swaption.swap_start,
        swaption.swap_end,
        swaption.float_freq,
        finstack_core::dates::StubKind::None,
        finstack_core::dates::BusinessDayConvention::Following,
        false,
        0,
        finstack_valuations::cashflow::builder::calendar::WEEKENDS_ONLY_ID,
    )
    .unwrap();

    let mut pv_float = 0.0;
    let mut prev = swaption.swap_start;
    for &d in sched.dates.iter().skip(1) {
        let t_prev = fwd
            .day_count()
            .year_fraction(
                fwd.base_date(),
                prev,
                finstack_core::dates::DayCountContext::default(),
            )
            .unwrap();
        let t_next = fwd
            .day_count()
            .year_fraction(
                fwd.base_date(),
                d,
                finstack_core::dates::DayCountContext::default(),
            )
            .unwrap();
        let accrual = fwd
            .day_count()
            .year_fraction(prev, d, finstack_core::dates::DayCountContext::default())
            .unwrap();
        let forward = fwd.rate_period(t_prev, t_next);
        let df = disc.df_between_dates(as_of, d).unwrap();
        pv_float += accrual * forward * df;
        prev = d;
    }

    pv_float / annuity
}

#[test]
fn test_forward_swap_rate_calculation() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

    let market = create_flat_market(as_of, 0.05, 0.30);

    let forward = swaption.forward_swap_rate(&market, as_of).unwrap();

    let expected = expected_forward_rate(&swaption, &market, as_of);
    assert_approx_eq(forward, expected, 1e-8, "Forward swap rate");
}

#[test]
fn test_swap_annuity_positive() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount("USD_OIS").unwrap();

    let annuity = swaption.swap_annuity(disc.as_ref(), as_of).unwrap();

    // 5Y semi-annual swap should have annuity around 4.0-5.0
    assert_reasonable(annuity, 3.0, 6.0, "Swap annuity");
}

#[test]
fn test_annuity_increases_with_tenor() {
    let (as_of, expiry, swap_start, _) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount("USD_OIS").unwrap();

    // 2Y swap
    let swap_end_2y = time::macros::date!(2027 - 01 - 01);
    let swaption_2y = create_standard_payer_swaption(expiry, swap_start, swap_end_2y, 0.05);
    let annuity_2y = swaption_2y.swap_annuity(disc.as_ref(), as_of).unwrap();

    // 5Y swap
    let swap_end_5y = time::macros::date!(2030 - 01 - 01);
    let swaption_5y = create_standard_payer_swaption(expiry, swap_start, swap_end_5y, 0.05);
    let annuity_5y = swaption_5y.swap_annuity(disc.as_ref(), as_of).unwrap();

    // 10Y swap
    let swap_end_10y = time::macros::date!(2035 - 01 - 01);
    let swaption_10y = create_standard_payer_swaption(expiry, swap_start, swap_end_10y, 0.05);
    let annuity_10y = swaption_10y.swap_annuity(disc.as_ref(), as_of).unwrap();

    assert!(annuity_2y < annuity_5y, "5Y annuity should exceed 2Y");
    assert!(annuity_5y < annuity_10y, "10Y annuity should exceed 5Y");
}

#[test]
fn test_year_fraction_act360() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);

    let yf = swaption
        .day_count
        .year_fraction(as_of, expiry, DayCountContext::default())
        .unwrap();

    // 1 year ≈ 1.0 under Act/360
    assert_approx_eq(yf, 1.0, 0.02, "Year fraction for 1Y");
}

#[test]
fn test_forward_rate_consistency() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);

    // Test with different curve levels
    for rate in [0.02, 0.05, 0.08] {
        let market = create_flat_market(as_of, rate, 0.30);

        let forward = swaption.forward_swap_rate(&market, as_of).unwrap();

        let expected = expected_forward_rate(&swaption, &market, as_of);
        assert_approx_eq(
            forward,
            expected,
            1e-8,
            &format!("Forward rate at {}%", rate * 100.0),
        );
    }
}

#[test]
fn test_annuity_dispatch_matches_selected_settlement_method() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount("USD_OIS").unwrap();
    let base = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let forward = base.forward_swap_rate(&market, as_of).unwrap();

    let physical = base.annuity(disc.as_ref(), as_of, forward).unwrap();
    let expected_physical = base.swap_annuity(disc.as_ref(), as_of).unwrap();
    assert_approx_eq(
        physical,
        expected_physical,
        1e-12,
        "physical settlement annuity",
    );

    let par_yield = base
        .clone()
        .with_settlement(SwaptionSettlement::Cash)
        .with_cash_settlement_method(CashSettlementMethod::ParYield);
    let par_yield_annuity = par_yield.annuity(disc.as_ref(), as_of, forward).unwrap();
    let expected_par_yield = par_yield.cash_annuity_par_yield(forward).unwrap();
    assert_approx_eq(
        par_yield_annuity,
        expected_par_yield,
        1e-12,
        "par-yield cash annuity",
    );

    let isda = base
        .clone()
        .with_settlement(SwaptionSettlement::Cash)
        .with_cash_settlement_method(CashSettlementMethod::IsdaParPar);
    let isda_annuity = isda.annuity(disc.as_ref(), as_of, forward).unwrap();
    assert_approx_eq(isda_annuity, expected_physical, 1e-12, "isda cash annuity");

    let zero_coupon = base
        .clone()
        .with_settlement(SwaptionSettlement::Cash)
        .with_cash_settlement_method(CashSettlementMethod::ZeroCoupon);
    let zero_coupon_annuity = zero_coupon.annuity(disc.as_ref(), as_of, forward).unwrap();
    let expected_zero_coupon = zero_coupon
        .cash_annuity_zero_coupon(disc.as_ref(), as_of)
        .unwrap();
    assert_approx_eq(
        zero_coupon_annuity,
        expected_zero_coupon,
        1e-12,
        "zero-coupon cash annuity",
    );
}

#[test]
fn test_zero_coupon_cash_annuity_matches_tenor_times_maturity_df() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);
    let disc = market.get_discount("USD_OIS").unwrap();
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05)
        .with_settlement(SwaptionSettlement::Cash)
        .with_cash_settlement_method(CashSettlementMethod::ZeroCoupon);

    let tenor = swaption
        .day_count
        .year_fraction(swap_start, swap_end, DayCountContext::default())
        .unwrap();
    let expected = tenor * disc.df_between_dates(as_of, swap_end).unwrap();
    let actual = swaption
        .cash_annuity_zero_coupon(disc.as_ref(), as_of)
        .unwrap();
    assert_approx_eq(actual, expected, 1e-12, "zero coupon annuity formula");
}

#[test]
fn test_forward_swap_rate_single_curve_matches_discount_factor_ratio() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);
    let mut swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    swaption.forward_curve_id = swaption.discount_curve_id.clone();

    let forward = swaption.forward_swap_rate(&market, as_of).unwrap();
    let disc = market.get_discount("USD_OIS").unwrap();
    let annuity = swaption.swap_annuity(disc.as_ref(), as_of).unwrap();
    let df_start = disc.df_between_dates(as_of, swap_start).unwrap();
    let df_end = disc.df_between_dates(as_of, swap_end).unwrap();
    let expected = (df_start - df_end) / annuity;
    assert_approx_eq(forward, expected, 1e-12, "single-curve forward swap rate");
}
