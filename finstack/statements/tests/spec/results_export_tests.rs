//! Integration tests for results export functionality.

#![cfg(feature = "dataframes")]

use finstack_statements::prelude::*;
use indexmap::indexmap;
use serde_json::json;

#[test]
fn test_export_to_polars_long() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let df = results.to_polars_long().unwrap();

    // Should have 3 nodes × 2 periods = 6 rows
    assert_eq!(df.height(), 6);
    assert_eq!(df.width(), 3); // node_id, period_id, value

    // Check column names
    let columns = df.get_column_names();
    assert_eq!(columns.len(), 3);
    assert_eq!(columns[0].as_str(), "node_id");
    assert_eq!(columns[1].as_str(), "period_id");
    assert_eq!(columns[2].as_str(), "value");
}

#[test]
fn test_export_to_polars_long_filtered() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Filter to just revenue and cogs
    let df = results
        .to_polars_long_filtered(&["revenue", "cogs"])
        .unwrap();

    // Should have 2 nodes × 2 periods = 4 rows
    assert_eq!(df.height(), 4);
    assert_eq!(df.width(), 3);

    // Verify only revenue and cogs are present
    let node_ids = df.column("node_id").unwrap().str().unwrap();
    let unique_nodes: std::collections::HashSet<_> = node_ids.into_iter().flatten().collect();
    assert_eq!(unique_nodes.len(), 2);
    assert!(unique_nodes.contains("revenue"));
    assert!(unique_nodes.contains("cogs"));
    assert!(!unique_nodes.contains("gross_profit"));
}

#[test]
fn test_export_to_polars_wide() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let df = results.to_polars_wide().unwrap();

    // Should have 2 periods (rows)
    assert_eq!(df.height(), 2);
    // Should have 4 columns: period_id + 3 nodes
    assert_eq!(df.width(), 4);

    // Check column names
    let columns = df.get_column_names();
    let col_names: Vec<String> = columns.iter().map(|c| c.as_str().to_string()).collect();
    assert!(col_names.contains(&"period_id".to_string()));
    assert!(col_names.contains(&"revenue".to_string()));
    assert!(col_names.contains(&"cogs".to_string()));
    assert!(col_names.contains(&"gross_profit".to_string()));

    // Check first row values
    let revenue = df.column("revenue").unwrap().f64().unwrap();
    assert_eq!(revenue.get(0), Some(100_000.0));
    assert_eq!(revenue.get(1), Some(110_000.0));

    let cogs = df.column("cogs").unwrap().f64().unwrap();
    assert_eq!(cogs.get(0), Some(60_000.0));
    assert_eq!(cogs.get(1), Some(66_000.0));
}

#[test]
fn test_export_complete_pl_model() {
    let model = ModelBuilder::new("Complete P&L")
        .periods("2025Q1..2025Q4", Some("2025Q1"))
        .unwrap()
        .value(
            "revenue",
            &[(
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            )],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => json!(0.05) },
            },
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .value(
            "opex",
            &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(20_000.0))],
        )
        .forecast(
            "opex",
            ForecastSpec {
                method: ForecastMethod::ForwardFill,
                params: indexmap! {},
            },
        )
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .compute("operating_income", "gross_profit - opex")
        .unwrap()
        .compute("gross_margin", "gross_profit / revenue")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Test long format
    let df_long = results.to_polars_long().unwrap();
    assert_eq!(df_long.height(), 24); // 6 nodes × 4 periods

    // Test wide format
    let df_wide = results.to_polars_wide().unwrap();
    assert_eq!(df_wide.height(), 4); // 4 periods
    assert_eq!(df_wide.width(), 7); // period_id + 6 nodes

    // Verify some calculated values
    let gross_margin = df_wide.column("gross_margin").unwrap().f64().unwrap();
    // All quarters should have ~0.4 gross margin (40%)
    for i in 0..4 {
        let margin = gross_margin.get(i).unwrap();
        assert!((margin - 0.4).abs() < 0.001);
    }
}

#[test]
fn test_export_with_multiple_periods() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2025Q4", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
                (
                    PeriodId::quarter(2025, 3),
                    AmountOrScalar::scalar(120_000.0),
                ),
                (
                    PeriodId::quarter(2025, 4),
                    AmountOrScalar::scalar(130_000.0),
                ),
            ],
        )
        .compute("doubled", "revenue * 2")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Test long format
    let df_long = results.to_polars_long().unwrap();
    assert_eq!(df_long.height(), 8); // 2 nodes × 4 periods

    // Test wide format
    let df_wide = results.to_polars_wide().unwrap();
    assert_eq!(df_wide.height(), 4); // 4 periods
    assert_eq!(df_wide.width(), 3); // period_id + 2 nodes

    // Verify period ordering in wide format
    let period_ids = df_wide.column("period_id").unwrap().str().unwrap();
    assert_eq!(period_ids.get(0), Some("2025Q1"));
    assert_eq!(period_ids.get(1), Some("2025Q2"));
    assert_eq!(period_ids.get(2), Some("2025Q3"));
    assert_eq!(period_ids.get(3), Some("2025Q4"));
}

#[test]
fn test_export_with_builtin_metrics() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q2", None)
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(110_000.0),
                ),
            ],
        )
        .value(
            "cogs",
            &[
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(60_000.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(66_000.0)),
            ],
        )
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .compute("gross_margin", "gross_profit / revenue")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let df_wide = results.to_polars_wide().unwrap();

    // Should include revenue, cogs, and all calculated metrics
    assert!(df_wide.height() > 0);
    let columns = df_wide.get_column_names();
    let col_names: Vec<String> = columns.iter().map(|c| c.as_str().to_string()).collect();
    assert!(col_names.contains(&"revenue".to_string()));
    assert!(col_names.contains(&"cogs".to_string()));
    assert!(col_names.contains(&"gross_profit".to_string()));
    assert!(col_names.contains(&"gross_margin".to_string()));
}

#[test]
fn test_empty_results_export() {
    let results = StatementResult {
        nodes: indexmap::IndexMap::new(),
        monetary_nodes: indexmap::IndexMap::new(),
        node_value_types: indexmap::IndexMap::new(),
        cs_cashflows: None,
        meta: Default::default(),
    };

    let df_long = results.to_polars_long().unwrap();
    assert_eq!(df_long.height(), 0);
    assert_eq!(df_long.width(), 6); // Updated for new columns

    let df_wide = results.to_polars_wide().unwrap();
    assert_eq!(df_wide.height(), 0);
    assert_eq!(df_wide.width(), 1); // Just period_id column
}

#[test]
fn test_export_preserves_period_order() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2025Q4", None)
        .unwrap()
        .value(
            "value",
            &[
                (PeriodId::quarter(2025, 4), AmountOrScalar::scalar(4.0)),
                (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(2.0)),
                (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(1.0)),
                (PeriodId::quarter(2025, 3), AmountOrScalar::scalar(3.0)),
            ],
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let df_wide = results.to_polars_wide().unwrap();

    // Periods should be sorted in chronological order
    let period_ids = df_wide.column("period_id").unwrap().str().unwrap();
    assert_eq!(period_ids.get(0), Some("2025Q1"));
    assert_eq!(period_ids.get(1), Some("2025Q2"));
    assert_eq!(period_ids.get(2), Some("2025Q3"));
    assert_eq!(period_ids.get(3), Some("2025Q4"));

    // Values should match the sorted order
    let values = df_wide.column("value").unwrap().f64().unwrap();
    assert_eq!(values.get(0), Some(1.0));
    assert_eq!(values.get(1), Some(2.0));
    assert_eq!(values.get(2), Some(3.0));
    assert_eq!(values.get(3), Some(4.0));
}
