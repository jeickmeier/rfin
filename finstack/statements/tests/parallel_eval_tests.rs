//! Tests for parallel evaluation.

use finstack_statements::prelude::*;
use std::time::Instant;

#[test]
fn test_parallel_evaluation_correctness() {
    // Create a model without time-series dependencies
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
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .compute("gross_profit", "revenue - cogs")
        .unwrap()
        .compute("gross_margin", "gross_profit / revenue")
        .unwrap()
        .build()
        .unwrap();

    // Evaluate sequentially
    let mut evaluator_seq = Evaluator::new();
    let results_seq = evaluator_seq.evaluate(&model, false).unwrap();

    // Evaluate in parallel
    let mut evaluator_par = Evaluator::new();
    let results_par = evaluator_par.evaluate(&model, true).unwrap();

    // Results should be identical
    for period in [1, 2, 3, 4] {
        let period_id = PeriodId::quarter(2025, period);

        // Check revenue
        assert_eq!(
            results_seq.get("revenue", &period_id),
            results_par.get("revenue", &period_id),
            "Revenue mismatch for period {:?}",
            period_id
        );

        // Check cogs
        assert_eq!(
            results_seq.get("cogs", &period_id),
            results_par.get("cogs", &period_id),
            "COGS mismatch for period {:?}",
            period_id
        );

        // Check gross_profit
        assert_eq!(
            results_seq.get("gross_profit", &period_id),
            results_par.get("gross_profit", &period_id),
            "Gross profit mismatch for period {:?}",
            period_id
        );

        // Check gross_margin
        assert_eq!(
            results_seq.get("gross_margin", &period_id),
            results_par.get("gross_margin", &period_id),
            "Gross margin mismatch for period {:?}",
            period_id
        );
    }
}

#[test]
fn test_parallel_disabled_for_time_series() {
    // Create a model with time-series dependencies
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
        .compute("qoq_growth", "pct_change(revenue, 1)")
        .unwrap()
        .compute("lagged_revenue", "lag(revenue, 1)")
        .unwrap()
        .build()
        .unwrap();

    // Even with parallel=true, should use sequential for time-series deps
    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model, true).unwrap();

    // Check that time-series functions work correctly
    // Q1: No prior period
    assert!(results
        .get("qoq_growth", &PeriodId::quarter(2025, 1))
        .unwrap()
        .is_nan());
    assert!(results
        .get("lagged_revenue", &PeriodId::quarter(2025, 1))
        .unwrap()
        .is_nan());

    // Q2: Should have correct lag and pct_change
    assert_eq!(
        results
            .get("qoq_growth", &PeriodId::quarter(2025, 2))
            .unwrap(),
        0.10 // (110k - 100k) / 100k
    );
    assert_eq!(
        results
            .get("lagged_revenue", &PeriodId::quarter(2025, 2))
            .unwrap(),
        100_000.0
    );

    // Q3: Should have correct lag and pct_change
    let q3_growth = results
        .get("qoq_growth", &PeriodId::quarter(2025, 3))
        .unwrap();
    assert!((q3_growth - 0.0909).abs() < 0.001); // (120k - 110k) / 110k
    assert_eq!(
        results
            .get("lagged_revenue", &PeriodId::quarter(2025, 3))
            .unwrap(),
        110_000.0
    );
}

#[cfg(feature = "parallel")]
#[test]
#[ignore] // Run with --ignored to test performance
fn test_parallel_performance() {
    // Create a large model without time-series dependencies
    let mut builder = ModelBuilder::new("large_test")
        .periods("2020Q1..2030Q4", None) // 44 quarters
        .unwrap();

    // Add many nodes
    for i in 0..100 {
        let node_id = format!("node_{}", i);
        if i == 0 {
            // Base node with values
            let values: Vec<_> = (2020..=2030)
                .flat_map(|year| {
                    (1..=4).map(move |q| {
                        (
                            PeriodId::quarter(year, q),
                            AmountOrScalar::scalar(1000.0 + i as f64),
                        )
                    })
                })
                .collect();
            builder = builder.value(&node_id, &values);
        } else {
            // Computed nodes that depend on previous nodes
            let formula = if i % 3 == 0 {
                format!("node_{} * 1.1", i - 1)
            } else if i % 3 == 1 {
                format!("node_{} + 100", i - 1)
            } else {
                format!("node_{} / 2", i - 1)
            };
            builder = builder.compute(&node_id, &formula).unwrap();
        }
    }

    let model = builder.build().unwrap();

    // Time sequential evaluation
    let mut evaluator_seq = Evaluator::new();
    let start_seq = Instant::now();
    let _results_seq = evaluator_seq.evaluate(&model, false).unwrap();
    let time_seq = start_seq.elapsed();

    // Time parallel evaluation
    let mut evaluator_par = Evaluator::new();
    let start_par = Instant::now();
    let _results_par = evaluator_par.evaluate(&model, true).unwrap();
    let time_par = start_par.elapsed();

    println!("Sequential evaluation time: {:?}", time_seq);
    println!("Parallel evaluation time: {:?}", time_par);
    println!(
        "Speedup: {:.2}x",
        time_seq.as_secs_f64() / time_par.as_secs_f64()
    );

    // Parallel should be faster (though actual speedup depends on core count)
    // We just check that it doesn't make things worse
    assert!(
        time_par.as_millis() <= time_seq.as_millis() * 2,
        "Parallel shouldn't be more than 2x slower than sequential"
    );
}
