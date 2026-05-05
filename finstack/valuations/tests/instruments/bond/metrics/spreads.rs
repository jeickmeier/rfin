//! Z-spread and I-spread calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::{Error, InputError};
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::instruments::fixed_income::bond::pricing::quote_conversions::price_from_oas;
use finstack_valuations::instruments::fixed_income::bond::ZSpreadCalculator;
use finstack_valuations::instruments::fixed_income::bond::{
    Bond, BondSettlementConvention, CallPut, CallPutSchedule,
};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_z_spread_discount_bond() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ZSPR1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(95.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::ZSpread],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .unwrap();
    let z = *result.measures.get("z_spread").unwrap();
    assert!(z > 0.0); // Discount bond has positive spread
}

#[test]
fn test_z_spread_reports_bond_compounding_spread() {
    use finstack_core::dates::{DayCount, DayCountContext};
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let maturity = date!(2030 - 01 - 01);
    let notional = Money::new(1_000_000.0, Currency::USD);
    let base_zero_rate = 0.03_f64;
    let target_z = 0.01_f64;
    let years = DayCount::Act365F
        .year_fraction(as_of, maturity, DayCountContext::default())
        .unwrap();
    let mut bond = Bond::fixed(
        "ZSPR-COMPOUNDING",
        notional,
        0.0,
        as_of,
        maturity,
        "USD-OIS",
    )
    .expect("bond");
    bond.cashflow_spec =
        finstack_valuations::instruments::fixed_income::bond::CashflowSpec::fixed_rate(
            0.0.into(),
            finstack_core::dates::Tenor::annual(),
            DayCount::Act365F,
        );
    bond.settlement_convention = None;
    let base_df = (1.0 + base_zero_rate).powf(-years);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (years, base_df)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);
    let target_dirty = bond
        .cashflow_schedule(&market, as_of)
        .unwrap()
        .into_iter()
        .filter(|flow| flow.date > as_of)
        .map(|flow| {
            let t = DayCount::Act365F
                .year_fraction(as_of, flow.date, DayCountContext::default())
                .unwrap();
            let df_base = market
                .get_discount("USD-OIS")
                .unwrap()
                .df_between_dates(as_of, flow.date)
                .unwrap();
            let base_rate = df_base.powf(-1.0 / t) - 1.0;
            flow.amount.amount() * (1.0 + base_rate + target_z).powf(-t)
        })
        .sum::<f64>();
    bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(100.0 * target_dirty / notional.amount());

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::ZSpread],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("Z-spread should solve");
    let z = *result.measures.get("z_spread").unwrap();

    assert!(
        (z - target_z).abs() < 2e-4,
        "Z-spread should be reported in the bond's annual compounding convention: target={target_z}, got={z}",
    );
}

#[test]
fn test_i_spread_uses_quote_date_for_settlement_based_curve() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 06);
    let quote_date = date!(2025 - 01 - 08);
    let mut bond = Bond::fixed(
        "ISPR-QUOTE-DATE",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 08),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(99.0);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(quote_date)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Ytm, MetricId::ISpread],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("I-spread should use settlement/quote date when the curve is quote-date based");

    assert!(result.measures["i_spread"].is_finite());
}

#[test]
fn test_asw_market_price_adjustment_has_correct_economic_sign() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ASW-DISCOUNT-SIGN",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(98.0);
    let discount_result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::ASWPar, MetricId::ASWMarket],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("discount bond ASW metrics should compute");
    assert!(
        discount_result.measures["asw_market"] > discount_result.measures["asw_par"],
        "discount bond ASW market spread should exceed par ASW: par={}, market={}",
        discount_result.measures["asw_par"],
        discount_result.measures["asw_market"]
    );

    bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(102.0);
    let premium_result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::ASWPar, MetricId::ASWMarket],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("premium bond ASW metrics should compute");
    assert!(
        premium_result.measures["asw_market"] < premium_result.measures["asw_par"],
        "premium bond ASW market spread should be below par ASW: par={}, market={}",
        premium_result.measures["asw_par"],
        premium_result.measures["asw_market"]
    );
}

#[test]
fn test_asw_market_uses_configured_forward_curve() {
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ASW-FORWARD-CURVE",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(98.0)
        .with_asw_forward_curve_id("USD-SOFR-6M");

    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();
    let low_forward_curve = ForwardCurve::builder("USD-SOFR-6M", 0.5)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.01), (5.0, 0.01)])
        .build()
        .unwrap();
    let market_with_forward = finstack_core::market_data::context::MarketContext::new()
        .insert(discount_curve.clone())
        .insert(low_forward_curve);
    let market_without_forward =
        finstack_core::market_data::context::MarketContext::new().insert(discount_curve);

    let with_forward = bond
        .price_with_metrics(
            &market_with_forward,
            as_of,
            &[MetricId::ASWMarket],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("ASW with configured forward curve should compute")
        .measures["asw_market"];

    bond.pricing_overrides.model_config.asw_forward_curve_id = None;
    let discount_proxy = bond
        .price_with_metrics(
            &market_without_forward,
            as_of,
            &[MetricId::ASWMarket],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("discount-proxy ASW should compute")
        .measures["asw_market"];

    assert!(
        (with_forward - discount_proxy).abs() > 1e-3,
        "ASW market should use the configured forward curve: with_forward={with_forward}, discount_proxy={discount_proxy}"
    );
}

#[test]
fn test_asw_market_falls_back_to_bond_forward_curve_id() {
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ASW-BOND-FORWARD-FALLBACK",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.forward_curve_id = Some("USD-SOFR-6M".into());
    bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(98.0);

    let discount_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();
    let low_forward_curve = ForwardCurve::builder("USD-SOFR-6M", 0.5)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.01), (5.0, 0.01)])
        .build()
        .unwrap();
    let high_forward_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.04), (5.0, 0.04)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new()
        .insert(discount_curve.clone())
        .insert(low_forward_curve)
        .insert(high_forward_curve);

    let fallback = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::ASWMarket],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("ASW with bond forward curve fallback should compute")
        .measures["asw_market"];

    bond.pricing_overrides.model_config.asw_forward_curve_id = Some("USD-SOFR-3M".into());
    let explicit_override = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::ASWMarket],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("ASW explicit forward override should compute")
        .measures["asw_market"];

    assert!(
        (fallback - explicit_override).abs() > 1e-3,
        "ASW should use bond.forward_curve_id only when asw_forward_curve_id is absent: fallback={fallback}, explicit_override={explicit_override}"
    );
}

#[test]
fn test_oas_metric_uses_bond_tree_pricing_overrides() {
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_valuations::instruments::fixed_income::bond::{CallPut, CallPutSchedule};

    let as_of = date!(2025 - 01 - 01);
    let mut base_bond = Bond::fixed(
        "OAS-CONFIG",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    base_bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.78)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let mut low_vol_bond = base_bond.clone();
    low_vol_bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(99.0)
        .with_implied_vol(0.001);

    let mut high_vol_bond = base_bond;
    high_vol_bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(99.0)
        .with_implied_vol(0.05);

    let low = low_vol_bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("low-vol OAS should price")
        .measures["oas"];
    let high = high_vol_bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("high-vol OAS should price")
        .measures["oas"];

    assert!(
        (low - high).abs() > 1e-6,
        "OAS metric should respond to bond tree volatility overrides: low={low}, high={high}"
    );
}

#[test]
fn test_oas_metric_uses_tree_discount_curve_override() {
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_valuations::instruments::fixed_income::bond::{CallPut, CallPutSchedule};

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "OAS-TREE-CURVE",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.settlement_convention = None;
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 04 - 01),
            end_date: date!(2028 - 04 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(99.0)
        .with_implied_vol(0.01)
        .with_tree_discount_curve_id("USD-TREE");

    let pricing_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.78)])
        .build()
        .unwrap();
    let tree_curve = DiscountCurve::builder("USD-TREE")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.92)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new()
        .insert(pricing_curve)
        .insert(tree_curve);

    let with_override = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("OAS with tree curve override should price")
        .measures["oas"];

    bond.pricing_overrides.model_config.tree_discount_curve_id = None;
    let without_override = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("OAS without tree curve override should price")
        .measures["oas"];

    assert!(
        (with_override - without_override).abs() > 1e-4,
        "OAS should use the configured tree discount curve: with_override={with_override}, without_override={without_override}"
    );
}

#[test]
fn test_embedded_option_value_uses_solved_oas_and_holder_sign() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "EMBEDDED-OAS-BASIS",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.settlement_convention = None;
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(103.0)
        .with_implied_vol(0.02);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas, MetricId::EmbeddedOptionValue],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("embedded option value should compute");
    let oas = result.measures["oas"];
    let actual = result.measures["embedded_option_value"];

    let mut straight_bond = bond.clone();
    straight_bond.call_put = Some(CallPutSchedule::default());
    let expected = price_from_oas(&bond, &market, as_of, oas).expect("callable OAS price")
        - price_from_oas(&straight_bond, &market, as_of, oas).expect("straight OAS price");

    assert!(
        (actual - expected).abs() < 1e-6,
        "embedded option value should be holder-view model price difference at solved OAS: actual={actual}, expected={expected}, oas={oas}"
    );
    assert!(
        actual < 0.0,
        "callable bond embedded option value should be negative from holder perspective, got {actual}"
    );
}

#[test]
fn test_embedded_option_value_uses_settlement_date_oas_pricing_basis() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 02);
    let quote_date = date!(2025 - 01 - 07);
    let quoted_oas = 0.0065;
    let mut bond = Bond::fixed(
        "EMBEDDED-QUOTE-DATE-BASIS",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 02),
        "USD-OIS",
    )
    .unwrap();
    bond.settlement_convention = Some(BondSettlementConvention {
        settlement_days: 3,
        ex_coupon_days: 0,
        ex_coupon_calendar_id: None,
    });
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 02),
            end_date: date!(2028 - 01 - 02),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = serde_json::from_value(serde_json::json!({
        "quoted_oas": quoted_oas,
        "implied_volatility": 0.20,
        "tree_steps": 80,
        "vol_model": "black",
        "mean_reversion": 0.0
    }))
    .expect("BDT pricing overrides should deserialize");

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(quote_date)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::EmbeddedOptionValue],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("embedded option value should compute");
    let actual = result.measures["embedded_option_value"];

    let mut straight_bond = bond.clone();
    straight_bond.call_put = Some(CallPutSchedule::default());
    let expected = price_from_oas(&bond, &market, quote_date, quoted_oas)
        .expect("callable quote-date OAS price")
        - price_from_oas(&straight_bond, &market, quote_date, quoted_oas)
            .expect("straight quote-date OAS price");

    assert!(
        (actual - expected).abs() < 1e-6,
        "embedded option value should use quote-date OAS pricing basis: actual={actual}, expected={expected}"
    );
}

#[test]
fn test_callable_bond_vega_is_registered_and_bumps_implied_volatility() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CALLABLE-VEGA",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = PricingOverrides::default()
        .with_quoted_clean_price(103.0)
        .with_implied_vol(0.02);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Vega],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("callable bond vega should compute");
    let vega = *result
        .measures
        .get("vega")
        .expect("bond vega should be registered");

    assert!(vega.is_finite(), "vega should be finite, got {vega}");
    assert!(
        vega < 0.0,
        "callable bond holder-view vega should be negative because higher volatility increases issuer call value, got {vega}"
    );
}

#[test]
fn test_callable_bond_oas_and_vega_use_explicit_bdt_tree_path() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CALLABLE-BDT-OAS-VEGA",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = serde_json::from_value(serde_json::json!({
        "quoted_clean_price": 103.0,
        "implied_volatility": 0.20,
        "tree_steps": 40,
        "vol_model": "black",
        "mean_reversion": 0.0
    }))
    .expect("BDT pricing overrides should deserialize");

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas, MetricId::Vega],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("explicit BDT callable OAS and vega should compute");

    let oas = result.measures["oas"];
    let vega = result.measures["vega"];
    assert!(oas.is_finite(), "BDT OAS should be finite, got {oas}");
    assert!(vega.is_finite(), "BDT vega should be finite, got {vega}");
    assert!(
        vega < 0.0,
        "holder-view callable BDT vega should be negative, got {vega}"
    );
}

#[test]
fn test_callable_bond_vega_is_invariant_to_vol_bump_size() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    fn vega_with_bump(
        base_bond: &Bond,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        bump: f64,
    ) -> f64 {
        let mut bond = base_bond.clone();
        bond.pricing_overrides = bond.pricing_overrides.with_vol_bump(bump);
        bond.price_with_metrics(
            market,
            as_of,
            &[MetricId::Vega],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("callable bond vega should compute")
        .measures["vega"]
    }

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CALLABLE-VEGA-BUMP-INVARIANT",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = serde_json::from_value(serde_json::json!({
        "quoted_clean_price": 103.0,
        "implied_volatility": 0.20,
        "implied_volatility": 0.20,
        "tree_steps": 40,
        "vol_model": "black",
        "mean_reversion": 0.0
    }))
    .expect("BDT pricing overrides should deserialize");

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let vega_half_point = vega_with_bump(&bond, &market, as_of, 0.005);
    let vega_one_point = vega_with_bump(&bond, &market, as_of, 0.01);
    let scale = vega_one_point.abs().max(1e-12);

    assert!(
        (vega_half_point - vega_one_point).abs() / scale < 0.05,
        "vega should be normalized per one vol point, not scale with finite-difference bump: half_point={vega_half_point}, one_point={vega_one_point}"
    );
}

#[test]
fn test_callable_bdt_oas_recovers_settlement_date_clean_price() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 02);
    let quote_date = date!(2025 - 01 - 07);
    let target_oas = 0.0065;
    let notional = Money::new(1_000_000.0, Currency::USD);
    let mut bond = Bond::fixed(
        "CALLABLE-BDT-QUOTE-DATE-OAS",
        notional,
        0.05,
        as_of,
        date!(2032 - 01 - 02),
        "USD-OIS",
    )
    .unwrap();
    bond.settlement_convention = Some(BondSettlementConvention {
        settlement_days: 3,
        ex_coupon_days: 0,
        ex_coupon_calendar_id: None,
    });
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 02),
            end_date: date!(2028 - 01 - 02),
            price_pct_of_par: 150.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = serde_json::from_value(serde_json::json!({
        "implied_volatility": 0.20,
        "tree_steps": 80,
        "vol_model": "black",
        "mean_reversion": 0.0
    }))
    .expect("BDT pricing overrides should deserialize");

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(quote_date)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);
    let dirty_at_quote =
        price_from_oas(&bond, &market, quote_date, target_oas).expect("quote-date OAS price");
    let schedule = bond
        .cashflow_schedule(&market, quote_date)
        .expect("cashflow schedule");
    let accrued_at_quote = finstack_valuations::cashflow::accrued_interest_amount(
        &schedule,
        quote_date,
        &bond.accrual_config(),
    )
    .expect("quote-date accrued");
    let quoted_clean_price = (dirty_at_quote - accrued_at_quote) / notional.amount() * 100.0;
    bond.pricing_overrides.market_quotes.quoted_clean_price = Some(quoted_clean_price);

    let result = bond
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Oas],
            finstack_valuations::instruments::PricingOptions::default(),
        )
        .expect("OAS metric should solve from quote-date clean price");
    let actual_oas = result.measures["oas"];

    assert!(
        (actual_oas - target_oas).abs() < 1e-6,
        "OAS should recover quote-date target: actual={actual_oas}, target={target_oas}, clean={quoted_clean_price}"
    );
}

#[test]
fn test_callable_bond_value_uses_same_bdt_tree_dispatch_as_oas_pricer() {
    use finstack_core::market_data::term_structures::DiscountCurve;

    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "CALLABLE-BDT-VALUE",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        as_of,
        date!(2032 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.call_put = Some(CallPutSchedule {
        calls: vec![CallPut {
            start_date: date!(2028 - 01 - 01),
            end_date: date!(2028 - 01 - 01),
            price_pct_of_par: 100.0,
            make_whole: None,
        }],
        puts: vec![],
    });
    bond.pricing_overrides = serde_json::from_value(serde_json::json!({
        "implied_volatility": 0.20,
        "tree_steps": 40,
        "vol_model": "black",
        "mean_reversion": 0.0
    }))
    .expect("BDT pricing overrides should deserialize");

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (7.0, 0.82)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert(curve);

    let direct_value = bond
        .value(&market, as_of)
        .expect("direct value should price");
    let canonical_tree_value =
        price_from_oas(&bond, &market, as_of, 0.0).expect("canonical tree value should price");

    assert!(
        (direct_value.amount() - canonical_tree_value).abs() < 1e-6,
        "direct callable value should use the same tree dispatch as price_from_oas: direct={}, canonical={}",
        direct_value.amount(),
        canonical_tree_value
    );
}

/// Z-spread should surface a missing discount curve error instead of silently returning 0.0
/// when pricing fails inside the root-finding objective (e.g., missing discount curve).
#[test]
fn test_z_spread_missing_discount_curve_returns_error() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ZSPR-MISSING-DC",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(95.0);

    // Market context with NO discount curves – any attempt to build a Z-spread PV should fail
    let market = finstack_core::market_data::context::MarketContext::new();

    // Minimal metric context: base value is arbitrary since Z-spread uses quoted clean price
    let base_value = Money::new(100.0, Currency::USD);
    let mut mctx = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        base_value,
        MetricContext::default_config(),
    );

    // Pre-populate accrued to bypass the metric dependency and force the failure into
    // the Z-spread pricing helper (missing discount curve), not missing accrued.
    mctx.computed.insert(MetricId::Accrued, 0.0);

    let calc = ZSpreadCalculator::default();
    let result = calc.calculate(&mut mctx);

    // Expect a propagated input error (missing discount curve), never an apparent "perfect fit" z=0.0.
    match result {
        Err(Error::Input(InputError::MissingCurve { requested, .. })) => {
            assert!(
                requested.contains("USD-OIS"),
                "expected missing discount curve id to mention USD-OIS, got {}",
                requested
            );
        }
        Err(e) => panic!(
            "expected InputError::MissingCurve for missing discount curve, got {}",
            e
        ),
        Ok(z) => panic!(
            "expected Z-spread calculation to fail for missing discount curve, but got z={}",
            z
        ),
    }
}

/// Z-spread solver should converge for IG, HY, and distressed fixed-rate bonds
/// with realistic spreads up to ~3000 bp and maintain tight price residuals.
#[test]
fn test_z_spread_solver_convergence_across_spread_regimes() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_valuations::instruments::PricingOverrides;

    let as_of = date!(2025 - 01 - 01);
    let maturity_ig = date!(2028 - 01 - 01); // shorter IG
    let maturity_hy = date!(2032 - 01 - 01); // medium HY
    let maturity_distressed = date!(2035 - 01 - 01); // longer distressed
    let notional = Money::new(1_000_000.0, Currency::USD);

    // Simple discount curve; Z-spread will be applied as an exponential shift.
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.7)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let market = MarketContext::new().insert(disc);

    let bond_ig = Bond::fixed(
        "ZSPR-CONV-IG",
        notional,
        0.03,
        as_of,
        maturity_ig,
        "USD-OIS",
    )
    .unwrap();
    let bond_hy = Bond::fixed(
        "ZSPR-CONV-HY",
        notional,
        0.06,
        as_of,
        maturity_hy,
        "USD-OIS",
    )
    .unwrap();
    let bond_distressed = Bond::fixed(
        "ZSPR-CONV-DIST",
        notional,
        0.10,
        as_of,
        maturity_distressed,
        "USD-OIS",
    )
    .unwrap();

    // (target z-spread, bond) scenarios from IG through distressed.
    let scenarios: Vec<(f64, Bond)> = vec![
        (0.01, bond_ig),         // 100 bp IG
        (0.07, bond_hy),         // 700 bp HY
        (0.30, bond_distressed), // 3000 bp distressed
    ];

    for (target_z, base_bond) in scenarios {
        let settlement_days = base_bond.settlement_days().unwrap_or(0) as i64;
        let quote_date = as_of + time::Duration::days(settlement_days);

        // Price the bond at the target Z-spread to obtain a dirty price.
        let dirty_target =
            finstack_valuations::instruments::fixed_income::bond::pricing::quote_conversions::price_from_z_spread(
                &base_bond, &market, quote_date, target_z,
            )
            .expect("pricing with target Z-spread should succeed");

        // Convert to a clean price (% of par) at the quote/settlement date
        // (dirty = clean + accrued at quote_date).
        // Accrued must be computed at the quote/settlement date, not `as_of`.
        let schedule = base_bond
            .cashflow_schedule(&market, quote_date)
            .expect("build full schedule");
        let accrued = finstack_valuations::cashflow::accrued_interest_amount(
            &schedule,
            quote_date,
            &base_bond.accrual_config(),
        )
        .expect("accrued at quote date");
        let clean_ccy = dirty_target - accrued;
        let clean_px = clean_ccy / notional.amount() * 100.0;

        let mut bond = base_bond.clone();
        bond.pricing_overrides = PricingOverrides::default().with_quoted_clean_price(clean_px);

        // Run Z-spread metric via the normal pipeline.
        let result = bond
            .price_with_metrics(
                &market,
                as_of,
                &[MetricId::ZSpread],
                finstack_valuations::instruments::PricingOptions::default(),
            )
            .expect("Z-spread metric should converge for realistic spreads");
        let z = *result
            .measures
            .get("z_spread")
            .expect("z_spread measure should be present");

        assert!(
            (z - target_z).abs() < 5e-8,
            "Z-spread solver should recover target z (target={}, got={})",
            target_z,
            z
        );

        // Re-price with solved z and verify price residual is tiny.
        let dirty_repriced =
            finstack_valuations::instruments::fixed_income::bond::pricing::quote_conversions::price_from_z_spread(
                &bond, &market, quote_date, z,
            )
            .expect("repricing with solved Z-spread should succeed");
        let price_error = (dirty_repriced - dirty_target).abs() / notional.amount();

        assert!(
            price_error < 1e-6,
            "Price residual should be < 1e-6 * notional, got {}",
            price_error
        );
    }
}
