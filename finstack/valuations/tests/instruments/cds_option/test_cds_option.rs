#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::dates::utils::add_months;
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::money::Money;
use finstack_core::F;
use finstack_valuations::instruments::cds::CreditParams;
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::cds_option::pricing::engine::CdsOptionPricer;
use finstack_valuations::instruments::cds_option::CdsOption;
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use time::Month;

fn flat_discount(id: &'static str, base: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.70)])
        .build()
        .unwrap()
}

fn flat_hazard(id: &'static str, base: Date, rec: F, hz: F) -> HazardCurve {
    let par = hz * 10000.0 * (1.0 - rec);
    HazardCurve::builder(id)
        .base_date(base)
        .recovery_rate(rec)
        .knots(vec![(1.0, hz), (5.0, hz), (10.0, hz)])
        .par_spreads(vec![(1.0, par), (5.0, par), (10.0, par)])
        .build()
        .unwrap()
}

fn build_option_single(
    as_of: Date,
    strike_bp: F,
    expiry_years: F,
    cds_years: F,
) -> (CdsOption, MarketContext) {
    let disc = flat_discount("USD-OIS", as_of);
    let rec = finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
    let hz = 0.02; // 2% hazard
    let credit_id = "HZ-SN";
    let credit = flat_hazard(credit_id, as_of, rec, hz);

    let expiry = add_months(as_of, (expiry_years * 12.0).round() as i32);
    let cds_maturity = add_months(as_of, (cds_years * 12.0).round() as i32);

    let option_params = CdsOptionParams::call(strike_bp, expiry, cds_maturity, Money::new(10_000_000.0, Currency::USD));
    let credit_params = CreditParams::senior_unsecured("SN", credit_id);
    let mut option = CdsOption::new(
        "CDSOPT-SN",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    option.pricing_overrides.implied_volatility = Some(0.30);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    (option, ctx)
}

fn build_option_index(
    as_of: Date,
    strike_bp: F,
    expiry_years: F,
    cds_years: F,
    index_factor: F,
    fwd_adjust_bp: F,
) -> (CdsOption, MarketContext) {
    let disc = flat_discount("USD-OIS", as_of);
    let rec = finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
    let hz = 0.02; // 2% hazard
    let credit_id = "HZ-IDX";
    let credit = flat_hazard(credit_id, as_of, rec, hz);

    let expiry = add_months(as_of, (expiry_years * 12.0).round() as i32);
    let cds_maturity = add_months(as_of, (cds_years * 12.0).round() as i32);

    let option_params = CdsOptionParams::call(strike_bp, expiry, cds_maturity, Money::new(10_000_000.0, Currency::USD))
        .as_index(index_factor)
        .with_forward_spread_adjust_bp(fwd_adjust_bp);
    let credit_params = CreditParams::senior_unsecured("IDX", credit_id);
    let mut option = CdsOption::new(
        "CDSOPT-IDX",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    option.pricing_overrides.implied_volatility = Some(0.30);

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    (option, ctx)
}

#[test]
fn single_name_pv_positive_for_itm_call() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (opt, ctx) = build_option_single(as_of, 100.0, 1.0, 5.0);
    let pricer = CdsOptionPricer::default();
    let pv = pricer.npv(&opt, &ctx, as_of).unwrap();
    assert!(pv.amount().is_finite());
    assert!(pv.amount() > 0.0);
}

#[test]
fn index_factor_scales_pv() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (mut opt_base, ctx) = build_option_index(as_of, 100.0, 1.0, 5.0, 1.0, 0.0);
    let pricer = CdsOptionPricer::default();
    let pv_base = pricer.npv(&opt_base, &ctx, as_of).unwrap().amount();

    let (opt_scaled, _) = build_option_index(as_of, 100.0, 1.0, 5.0, 0.8, 0.0);
    let pv_scaled = pricer.npv(&opt_scaled, &ctx, as_of).unwrap().amount();

    let ratio = pv_scaled / pv_base;
    assert!((ratio - 0.8).abs() < 5e-6, "index factor scaling failed: {}", ratio);
}

#[test]
fn index_forward_adjust_increases_call_value() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (opt_base, ctx) = build_option_index(as_of, 120.0, 1.0, 5.0, 1.0, 0.0);
    let pricer = CdsOptionPricer::default();
    let pv_base = pricer.npv(&opt_base, &ctx, as_of).unwrap().amount();

    let (opt_adj, _) = build_option_index(as_of, 120.0, 1.0, 5.0, 1.0, 25.0);
    let pv_adj = pricer.npv(&opt_adj, &ctx, as_of).unwrap().amount();

    assert!(pv_adj > pv_base, "forward adjustment should increase call PV");
}

#[test]
fn implied_vol_round_trip() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (mut opt, ctx) = build_option_single(as_of, 100.0, 1.5, 5.0);
    let pricer = CdsOptionPricer::default();
    opt.pricing_overrides.implied_volatility = Some(0.35);
    let pv = pricer.npv(&opt, &ctx, as_of).unwrap();

    // Invert
    let iv = pricer
        .implied_vol(&opt, &ctx, as_of, pv.amount(), None)
        .unwrap();
    assert!((iv - 0.35).abs() < 1e-6, "IV round-trip failed: {}", iv);
}

#[test]
fn metrics_implied_vol_matches_pricer() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let (mut opt, mut ctx) = build_option_single(as_of, 100.0, 1.0, 5.0);
    let pricer = CdsOptionPricer::default();
    opt.pricing_overrides.implied_volatility = Some(0.28);
    let pv = pricer.npv(&opt, &ctx, as_of).unwrap();

    // Metric registry implied vol
    let mut mctx = MetricContext::new(std::sync::Arc::new(opt.clone()), std::sync::Arc::new(ctx.clone()), as_of, pv);
    let reg = standard_registry();
    let res = reg.compute(&[MetricId::ImpliedVol], &mut mctx).unwrap();
    let iv_metric = *res.get(&MetricId::ImpliedVol).unwrap();

    // Compute from pricer solver too
    let iv_pricer = pricer.implied_vol(&opt, &ctx, as_of, pv.amount(), None).unwrap();
    assert!((iv_metric - iv_pricer).abs() < 1e-8, "metric/pricer IV mismatch: {} vs {}", iv_metric, iv_pricer);
}


