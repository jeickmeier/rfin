//! Tests for instrument-level shock adapters.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_scenarios::{
    ExecutionContext, InstrumentType, OperationSpec, ScenarioEngine, ScenarioSpec,
};
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::common::traits::{Attributes, Instrument};
use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
use finstack_valuations::instruments::Bond;
use time::Month;

#[test]
fn test_instrument_type_price_shock_matching() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Create test instruments
    let mut instruments: Vec<Box<dyn Instrument>> = vec![
        Box::new(
            Bond::builder()
                .id("BOND1".into())
                .notional(finstack_core::money::Money::new(100.0, Currency::USD))
                .coupon(0.05)
                .issue(base_date)
                .maturity(base_date + time::Duration::days(365))
                .freq(finstack_core::dates::Frequency::annual())
                .dc(finstack_core::dates::DayCount::Thirty360)
                .bdc(finstack_core::dates::BusinessDayConvention::Following)
                .calendar_id_opt(None)
                .stub(finstack_core::dates::StubKind::None)
                .discount_curve_id(finstack_core::types::CurveId::new("USD-OIS"))
                .credit_curve_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .unwrap(),
        ),
        Box::new(
            Bond::builder()
                .id("BOND2".into())
                .notional(finstack_core::money::Money::new(100.0, Currency::USD))
                .coupon(0.04)
                .issue(base_date)
                .maturity(base_date + time::Duration::days(730))
                .freq(finstack_core::dates::Frequency::annual())
                .dc(finstack_core::dates::DayCount::Thirty360)
                .bdc(finstack_core::dates::BusinessDayConvention::Following)
                .calendar_id_opt(None)
                .stub(finstack_core::dates::StubKind::None)
                .discount_curve_id(finstack_core::types::CurveId::new("USD-OIS"))
                .credit_curve_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .unwrap(),
        ),
    ];

    let scenario = ScenarioSpec {
        id: "bond_price_shock".into(),
        name: Some("Bond Price Shock".into()),
        description: None,
        operations: vec![OperationSpec::InstrumentPricePctByType {
            instrument_types: vec![InstrumentType::Bond],
            pct: -5.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 2, "Should shock 2 bonds");

    // Verify shock was applied to metadata
    for instrument in &instruments {
        let meta = &instrument.attributes().meta;
        assert!(meta.contains_key("scenario_price_shock_pct"));
        assert_eq!(meta.get("scenario_price_shock_pct").unwrap(), "-5.0000");
    }
}

#[test]
fn test_instrument_type_spread_shock_matching() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let mut instruments: Vec<Box<dyn Instrument>> = vec![Box::new(
        Bond::builder()
            .id("BOND1".into())
            .notional(finstack_core::money::Money::new(100.0, Currency::USD))
            .coupon(0.05)
            .issue(base_date)
            .maturity(base_date + time::Duration::days(365))
            .freq(finstack_core::dates::Frequency::annual())
            .dc(finstack_core::dates::DayCount::Thirty360)
            .bdc(finstack_core::dates::BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(finstack_core::dates::StubKind::None)
            .discount_curve_id(finstack_core::types::CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap(),
    )];

    let scenario = ScenarioSpec {
        id: "bond_spread_shock".into(),
        name: Some("Bond Spread Shock".into()),
        description: None,
        operations: vec![OperationSpec::InstrumentSpreadBpByType {
            instrument_types: vec![InstrumentType::Bond],
            bp: 100.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify shock metadata
    let meta = &instruments[0].attributes().meta;
    assert!(meta.contains_key("scenario_spread_shock_bp"));
    assert_eq!(meta.get("scenario_spread_shock_bp").unwrap(), "100.00");
}

#[test]
fn test_instrument_shock_empty_list() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let mut instruments: Vec<Box<dyn Instrument>> = vec![];

    let scenario = ScenarioSpec {
        id: "empty_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::InstrumentPricePctByType {
            instrument_types: vec![InstrumentType::Bond],
            pct: -5.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 0, "No instruments to shock");
}

#[test]
fn test_instrument_shock_no_matching_types() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let mut instruments: Vec<Box<dyn Instrument>> = vec![Box::new(
        Bond::builder()
            .id("BOND1".into())
            .notional(finstack_core::money::Money::new(100.0, Currency::USD))
            .coupon(0.05)
            .issue(base_date)
            .maturity(base_date + time::Duration::days(365))
            .freq(finstack_core::dates::Frequency::annual())
            .dc(finstack_core::dates::DayCount::Thirty360)
            .bdc(finstack_core::dates::BusinessDayConvention::Following)
            .calendar_id_opt(None)
            .stub(finstack_core::dates::StubKind::None)
            .discount_curve_id(finstack_core::types::CurveId::new("USD-OIS"))
            .credit_curve_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap(),
    )];

    let scenario = ScenarioSpec {
        id: "no_match_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::InstrumentPricePctByType {
            instrument_types: vec![InstrumentType::CDS], // Looking for CDS, have Bond
            pct: -5.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 0, "No CDS instruments to shock");
}

#[test]
fn test_instrument_shock_without_instruments_provided() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "no_instruments".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::InstrumentPricePctByType {
            instrument_types: vec![InstrumentType::Bond],
            pct: -5.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None, // No instruments provided
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 0);
    assert!(!report.warnings.is_empty(), "Should have warning");
    assert!(report.warnings[0].contains("no instruments provided"));
}

#[test]
fn test_instrument_shock_multiple_types() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let mut instruments: Vec<Box<dyn Instrument>> = vec![
        Box::new(
            Bond::builder()
                .id("BOND1".into())
                .notional(finstack_core::money::Money::new(100.0, Currency::USD))
                .coupon(0.05)
                .issue(base_date)
                .maturity(base_date + time::Duration::days(365))
                .freq(finstack_core::dates::Frequency::annual())
                .dc(finstack_core::dates::DayCount::Thirty360)
                .bdc(finstack_core::dates::BusinessDayConvention::Following)
                .calendar_id_opt(None)
                .stub(finstack_core::dates::StubKind::None)
                .discount_curve_id(finstack_core::types::CurveId::new("USD-OIS"))
                .credit_curve_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .unwrap(),
        ),
        Box::new(
            Bond::builder()
                .id("BOND2".into())
                .notional(finstack_core::money::Money::new(100.0, Currency::USD))
                .coupon(0.04)
                .issue(base_date)
                .maturity(base_date + time::Duration::days(730))
                .freq(finstack_core::dates::Frequency::annual())
                .dc(finstack_core::dates::DayCount::Thirty360)
                .bdc(finstack_core::dates::BusinessDayConvention::Following)
                .calendar_id_opt(None)
                .stub(finstack_core::dates::StubKind::None)
                .discount_curve_id(finstack_core::types::CurveId::new("USD-OIS"))
                .credit_curve_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .unwrap(),
        ),
    ];

    let scenario = ScenarioSpec {
        id: "multi_type_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::InstrumentPricePctByType {
            instrument_types: vec![InstrumentType::Bond, InstrumentType::Loan],
            pct: -10.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 2, "Both bonds should be shocked");
}
