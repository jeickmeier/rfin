//! Tests for JSON serialization stability.

use finstack_core::currency::Currency;
use finstack_scenarios::{CurveKind, OperationSpec, ScenarioSpec, VolSurfaceKind};
use indexmap::IndexMap;

#[test]
fn test_scenario_json_roundtrip() {
    let scenario = ScenarioSpec {
        id: "test_scenario".into(),
        name: Some("Test Scenario".into()),
        description: Some("For JSON testing".into()),
        operations: vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD_SOFR".into(),
                bp: 50.0,
            },
            OperationSpec::EquityPricePct {
                ids: vec!["SPY".into()],
                pct: -10.0,
            },
            OperationSpec::MarketFxPct {
                base: Currency::EUR,
                quote: Currency::USD,
                pct: 5.0,
            },
        ],
        priority: 0,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&scenario).unwrap();
    println!("Serialized scenario:\n{}", json);

    // Deserialize back
    let deserialized: ScenarioSpec = serde_json::from_str(&json).unwrap();

    // Verify equality
    assert_eq!(scenario.id, deserialized.id);
    assert_eq!(scenario.name, deserialized.name);
    assert_eq!(scenario.operations.len(), deserialized.operations.len());
    assert_eq!(scenario.priority, deserialized.priority);
}

#[test]
fn test_all_operation_types_serialize() {
    let operations = vec![
        OperationSpec::MarketFxPct {
            base: Currency::EUR,
            quote: Currency::USD,
            pct: 5.0,
        },
        OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: -10.0,
        },
        OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD_SOFR".into(),
            bp: 50.0,
        },
        OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Forecast,
            curve_id: "USD_LIBOR".into(),
            nodes: vec![("1Y".into(), 25.0), ("5Y".into(), -10.0)],
        },
        OperationSpec::BaseCorrParallelPts {
            surface_id: "CDX".into(),
            points: 0.05,
        },
        OperationSpec::VolSurfaceParallelPct {
            surface_kind: VolSurfaceKind::Equity,
            surface_id: "SPX".into(),
            pct: 20.0,
        },
        OperationSpec::StmtForecastPercent {
            node_id: "Revenue".into(),
            pct: -5.0,
        },
        OperationSpec::StmtForecastAssign {
            node_id: "Cost".into(),
            value: 100_000.0,
        },
    ];

    let scenario = ScenarioSpec {
        id: "all_ops".into(),
        name: None,
        description: None,
        operations,
        priority: 0,
    };

    // Roundtrip
    let json = serde_json::to_string(&scenario).unwrap();
    let deserialized: ScenarioSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(scenario.operations.len(), deserialized.operations.len());
}

#[test]
fn test_reject_unknown_fields() {
    let json = r#"{
        "id": "test",
        "operations": [],
        "priority": 0,
        "unknown_field": "should_fail"
    }"#;

    let result = serde_json::from_str::<ScenarioSpec>(json);
    assert!(result.is_err(), "Should reject unknown fields");
}

#[test]
fn test_attribute_selector_serde() {
    let mut attrs = IndexMap::new();
    attrs.insert("sector".into(), "Energy".into());
    attrs.insert("rating".into(), "BBB".into());

    let op = OperationSpec::InstrumentPricePctByAttr {
        attrs,
        pct: -5.0,
    };

    let scenario = ScenarioSpec {
        id: "attr_test".into(),
        name: None,
        description: None,
        operations: vec![op],
        priority: 0,
    };

    let json = serde_json::to_string_pretty(&scenario).unwrap();
    let deserialized: ScenarioSpec = serde_json::from_str(&json).unwrap();

    match &deserialized.operations[0] {
        OperationSpec::InstrumentPricePctByAttr { attrs, pct } => {
            assert_eq!(attrs.len(), 2);
            assert_eq!(attrs.get("sector").unwrap(), "Energy");
            assert_eq!(*pct, -5.0);
        }
        _ => panic!("Wrong operation type"),
    }
}

