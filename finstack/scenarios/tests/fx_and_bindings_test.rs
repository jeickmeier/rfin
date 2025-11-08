//! Tests for FX shocks and rate bindings.

use finstack_core::currency::Currency;
use finstack_core::dates::{build_periods, Date};
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec,
};
use finstack_statements::{AmountOrScalar, FinancialModelSpec, NodeSpec, NodeType};
use indexmap::{indexmap, IndexMap};
use std::sync::Arc;
use time::Month;

#[test]
fn test_fx_shock() {
    // Setup FX provider
    let fx_provider = Arc::new(SimpleFxProvider::new());
    fx_provider.set_quote(Currency::EUR, Currency::USD, 1.1);

    let fx_matrix = FxMatrix::new(fx_provider);
    let mut market = MarketContext::new().insert_fx(fx_matrix);

    // Setup empty model
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Create FX shock scenario
    let scenario = ScenarioSpec {
        id: "fx_shock".into(),
        name: Some("FX Shock".into()),
        description: None,
        operations: vec![OperationSpec::MarketFxPct {
            base: Currency::EUR,
            quote: Currency::USD,
            pct: 10.0, // EUR strengthens by 10%
        }],
        priority: 0,
    };

    // Apply scenario
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify shocked FX rate
    let fx = market.fx.as_ref().unwrap();
    let query = finstack_core::money::fx::FxQuery::new(Currency::EUR, Currency::USD, base_date);
    let rate = fx.rate(query).unwrap().rate;

    let expected = 1.1 * 1.1; // 10% increase
    assert!(
        (rate - expected).abs() < 1e-6,
        "Expected {}, got {}",
        expected,
        rate
    );
}

#[test]
fn test_fx_shock_preserves_other_quotes() {
    let fx_provider = Arc::new(SimpleFxProvider::new());
    fx_provider.set_quote(Currency::EUR, Currency::USD, 1.1);
    fx_provider.set_quote(Currency::GBP, Currency::USD, 1.25);

    let fx_matrix = FxMatrix::new(fx_provider);
    let mut market = MarketContext::new().insert_fx(fx_matrix);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "fx_shock".into(),
        name: Some("FX Shock".into()),
        description: None,
        operations: vec![OperationSpec::MarketFxPct {
            base: Currency::EUR,
            quote: Currency::USD,
            pct: 5.0,
        }],
        priority: 0,
    };

    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    engine.apply(&scenario, &mut ctx).unwrap();

    let fx = market.fx.as_ref().unwrap();
    let gbp_query = finstack_core::money::fx::FxQuery::new(Currency::GBP, Currency::USD, base_date);
    let gbp_rate = fx.rate(gbp_query).unwrap().rate;
    assert!((gbp_rate - 1.25).abs() < 1e-6, "Expected unchanged GBP/USD quote");
}

#[test]
fn test_rate_binding() {
    // Setup market with discount curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD_SOFR")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98), // ~2% rate
            (5.0, 0.90),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_discount(curve);

    // Setup model with interest rate node
    let period_plan = build_periods("2025Q1..Q4", None).unwrap();
    let periods = period_plan.periods;
    let mut model = FinancialModelSpec::new("test", periods.clone());

    // Add interest rate node with initial values
    let mut rate_values = IndexMap::new();
    for period in &periods {
        rate_values.insert(period.id, AmountOrScalar::Scalar(0.015)); // 1.5% initial
    }

    let rate_node = NodeSpec::new("InterestRate", NodeType::Value).with_values(rate_values);
    model.add_node(rate_node);

    // Configure rate binding
    let rate_bindings = Some(indexmap! {
        "InterestRate".to_string() => "USD_SOFR".to_string(),
    });

    // Create scenario with curve shock
    let scenario = ScenarioSpec {
        id: "rate_shock".into(),
        name: Some("Rate Shock".into()),
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD_SOFR".into(),
            bp: 100.0, // +100bp = +1%
        }],
        priority: 0,
    };

    // Apply scenario with rate binding
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify rate was updated from curve
    let updated_rate = model
        .get_node("InterestRate")
        .unwrap()
        .values
        .as_ref()
        .unwrap()
        .values()
        .next()
        .unwrap();

    match updated_rate {
        AmountOrScalar::Scalar(s) => {
            // Should reflect the shocked curve's rate (~3% after +100bp)
            assert!(*s > 0.02, "Expected rate > 2%, got {}", s);
        }
        _ => panic!("Expected scalar value"),
    }
}
