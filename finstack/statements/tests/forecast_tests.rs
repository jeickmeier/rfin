//! Tests for forecast methods.

use finstack_statements::prelude::*;
use indexmap::indexmap;

#[test]
fn test_forward_fill_forecast() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q2"))
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
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::ForwardFill,
                params: indexmap! {},
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Q1-Q2 are actuals
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 1)),
        Some(100_000.0)
    );
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 2)),
        Some(110_000.0)
    );

    // Q3-Q4 should forward fill from Q2
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 3)),
        Some(110_000.0)
    );
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 4)),
        Some(110_000.0)
    );
}

#[test]
fn test_growth_pct_forecast() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q1"))
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
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Q1 is actual
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 1)),
        Some(100_000.0)
    );

    // Q2-Q4 should grow by 5% per quarter
    let q2 = results.get("revenue", &PeriodId::quarter(2025, 2)).unwrap();
    let q3 = results.get("revenue", &PeriodId::quarter(2025, 3)).unwrap();
    let q4 = results.get("revenue", &PeriodId::quarter(2025, 4)).unwrap();

    assert!((q2 - 105_000.0).abs() < 1.0);
    assert!((q3 - 110_250.0).abs() < 1.0);
    assert!((q4 - 115_762.5).abs() < 1.0);
}

#[test]
fn test_growth_pct_negative() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q3", Some("2025Q1"))
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
                params: indexmap! { "rate".into() => serde_json::json!(-0.1) }, // -10% decline
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    let q2 = results.get("revenue", &PeriodId::quarter(2025, 2)).unwrap();
    let q3 = results.get("revenue", &PeriodId::quarter(2025, 3)).unwrap();

    assert!((q2 - 90_000.0).abs() < 1.0);
    assert!((q3 - 81_000.0).abs() < 1.0);
}

#[test]
fn test_curve_pct_forecast() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q1"))
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
                method: ForecastMethod::CurvePct,
                params: indexmap! {
                    "curve".into() => serde_json::json!([0.05, 0.06, 0.05])
                },
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Q1 is actual
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 1)),
        Some(100_000.0)
    );

    // Q2-Q4 should apply curve rates
    let q2 = results.get("revenue", &PeriodId::quarter(2025, 2)).unwrap();
    let q3 = results.get("revenue", &PeriodId::quarter(2025, 3)).unwrap();
    let q4 = results.get("revenue", &PeriodId::quarter(2025, 4)).unwrap();

    assert!((q2 - 105_000.0).abs() < 1.0); // 100k * 1.05
    assert!((q3 - 111_300.0).abs() < 1.0); // 105k * 1.06
    assert!((q4 - 116_865.0).abs() < 1.0); // 111.3k * 1.05
}

#[test]
fn test_override_forecast() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q1"))
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
                method: ForecastMethod::Override,
                params: indexmap! {
                    "overrides".into() => serde_json::json!({
                        "2025Q2": 120_000.0,
                        "2025Q4": 140_000.0,
                    })
                },
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Q1 is actual
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 1)),
        Some(100_000.0)
    );

    // Q2 and Q4 have overrides, Q3 forward fills from Q2
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 2)),
        Some(120_000.0)
    );
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 3)),
        Some(120_000.0)
    ); // Forward fill
    assert_eq!(
        results.get("revenue", &PeriodId::quarter(2025, 4)),
        Some(140_000.0)
    );
}

#[test]
fn test_forecast_with_formula_fallback() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q2"))
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
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Revenue should be forecasted
    let q3_revenue = results.get("revenue", &PeriodId::quarter(2025, 3)).unwrap();
    assert!((q3_revenue - 115_500.0).abs() < 1.0);

    // COGS should use formula based on forecasted revenue
    let q3_cogs = results.get("cogs", &PeriodId::quarter(2025, 3)).unwrap();
    assert!((q3_cogs - 69_300.0).abs() < 1.0);
}

#[test]
fn test_multiple_periods_with_forecast() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..2026Q4", Some("2025Q2"))
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
                    AmountOrScalar::scalar(105_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.03) },
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Check that all periods are evaluated
    assert!(results
        .get("revenue", &PeriodId::quarter(2025, 3))
        .is_some());
    assert!(results
        .get("revenue", &PeriodId::quarter(2025, 4))
        .is_some());
    assert!(results
        .get("revenue", &PeriodId::quarter(2026, 1))
        .is_some());
    assert!(results
        .get("revenue", &PeriodId::quarter(2026, 4))
        .is_some());

    // Verify compounding over longer period
    let q1_2026 = results.get("revenue", &PeriodId::quarter(2026, 1)).unwrap();
    // Q1 2026 should be Q2 2025 (105000) * 1.03^3 = 114736.335
    assert!((q1_2026 - 114_736.335).abs() < 10.0); // Should be growing with 3% compound
}

#[test]
fn test_forecast_pl_model() {
    let model = ModelBuilder::new("P&L with Forecasts")
        .periods("2025Q1..2025Q4", Some("2025Q2"))
        .unwrap()
        .value(
            "revenue",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(10_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(11_000_000.0),
                ),
            ],
        )
        .forecast(
            "revenue",
            ForecastSpec {
                method: ForecastMethod::GrowthPct,
                params: indexmap! { "rate".into() => serde_json::json!(0.05) },
            },
        )
        .compute("cogs", "revenue * 0.6")
        .unwrap()
        .value(
            "opex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(2_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(2_100_000.0),
                ),
            ],
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
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // Check Q3 values (forecast period)
    let q3_revenue = results.get("revenue", &PeriodId::quarter(2025, 3)).unwrap();
    let q3_cogs = results.get("cogs", &PeriodId::quarter(2025, 3)).unwrap();
    let q3_opex = results.get("opex", &PeriodId::quarter(2025, 3)).unwrap();
    let q3_gross_profit = results
        .get("gross_profit", &PeriodId::quarter(2025, 3))
        .unwrap();
    let q3_operating_income = results
        .get("operating_income", &PeriodId::quarter(2025, 3))
        .unwrap();

    assert!((q3_revenue - 11_550_000.0).abs() < 10.0);
    assert!((q3_cogs - 6_930_000.0).abs() < 10.0);
    assert!((q3_opex - 2_100_000.0).abs() < 10.0);
    assert!((q3_gross_profit - 4_620_000.0).abs() < 10.0);
    assert!((q3_operating_income - 2_520_000.0).abs() < 10.0);
}

#[test]
fn test_normal_forecast_deterministic() {
    let model1 = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q1"))
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
                method: ForecastMethod::Normal,
                params: indexmap! {
                    "mean".into() => serde_json::json!(100_000.0),
                    "std_dev".into() => serde_json::json!(15_000.0),
                    "seed".into() => serde_json::json!(42),
                },
            },
        )
        .build()
        .unwrap();

    let model2 = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q1"))
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
                method: ForecastMethod::Normal,
                params: indexmap! {
                    "mean".into() => serde_json::json!(100_000.0),
                    "std_dev".into() => serde_json::json!(15_000.0),
                    "seed".into() => serde_json::json!(42),
                },
            },
        )
        .build()
        .unwrap();

    let mut evaluator1 = Evaluator::new();
    let results1 = evaluator1.evaluate(&model1).unwrap();

    let mut evaluator2 = Evaluator::new();
    let results2 = evaluator2.evaluate(&model2).unwrap();

    // Same seed should produce identical results
    assert_eq!(
        results1.get("revenue", &PeriodId::quarter(2025, 2)),
        results2.get("revenue", &PeriodId::quarter(2025, 2))
    );
    assert_eq!(
        results1.get("revenue", &PeriodId::quarter(2025, 3)),
        results2.get("revenue", &PeriodId::quarter(2025, 3))
    );
}

#[test]
fn test_lognormal_forecast_always_positive() {
    let model = ModelBuilder::new("test")
        .periods("2025Q1..Q4", Some("2025Q1"))
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
                method: ForecastMethod::LogNormal,
                params: indexmap! {
                    "mean".into() => serde_json::json!(11.5),
                    "std_dev".into() => serde_json::json!(0.15),
                    "seed".into() => serde_json::json!(42),
                },
            },
        )
        .build()
        .unwrap();

    let mut evaluator = Evaluator::new();
    let results = evaluator.evaluate(&model).unwrap();

    // All forecasted values should be positive
    assert!(results.get("revenue", &PeriodId::quarter(2025, 2)).unwrap() > 0.0);
    assert!(results.get("revenue", &PeriodId::quarter(2025, 3)).unwrap() > 0.0);
    assert!(results.get("revenue", &PeriodId::quarter(2025, 4)).unwrap() > 0.0);
}

/// Test that overflow in compound growth is detected and returns an error.
#[test]
fn test_growth_pct_overflow_error() {
    use finstack_statements::forecast::growth_pct;

    // Use extreme growth rate (10000% per period) to trigger overflow
    // With rate=100, starting at 1e10: 1e10 * 101^150 > f64::MAX (1.8e308)
    // After 100 periods we're at ~1e210, need ~50 more to overflow
    let periods: Vec<_> = (0..200)
        .map(|i| PeriodId::quarter(2025 + i / 4, ((i % 4) as u8) + 1))
        .collect();

    let mut params = indexmap::IndexMap::new();
    params.insert("rate".to_string(), serde_json::json!(100.0)); // 10000% per period

    let result = growth_pct(1e10, &periods, &params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Overflow"));
}

/// Test that high growth rates (>100%) produce a warning but don't error.
#[test]
fn test_growth_pct_high_rate_no_error() {
    use finstack_statements::forecast::growth_pct;

    // Single period with 150% growth rate
    let periods = vec![PeriodId::quarter(2025, 1)];

    let mut params = indexmap::IndexMap::new();
    params.insert("rate".to_string(), serde_json::json!(1.5)); // 150% per period

    // Should succeed (just warns, doesn't error)
    let result = growth_pct(100.0, &periods, &params);
    assert!(result.is_ok());

    // Verify the value is correct: 100 * (1 + 1.5) = 250
    let values = result.unwrap();
    assert!((values[&PeriodId::quarter(2025, 1)] - 250.0).abs() < 0.01);
}

/// Test that the normal forecast never produces NaN values.
/// This tests the Box-Muller guard against ln(0).
#[test]
fn test_normal_forecast_no_nan() {
    use finstack_statements::forecast::normal_forecast;

    // Generate many periods
    let periods: Vec<_> = (0..100)
        .map(|i| PeriodId::quarter(2025 + i / 4, ((i % 4) as u8) + 1))
        .collect();

    // Test with many different seeds to exercise the RNG
    for seed in 0..1000 {
        let mut params = indexmap::IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(100.0));
        params.insert("std_dev".to_string(), serde_json::json!(15.0));
        params.insert("seed".to_string(), serde_json::json!(seed));

        let result = normal_forecast(0.0, &periods, &params).expect("normal_forecast should succeed");
        for value in result.values() {
            assert!(!value.is_nan(), "NaN produced with seed {}", seed);
            assert!(value.is_finite(), "Non-finite value produced with seed {}", seed);
        }
    }
}

/// Test that lognormal forecast with std_dev=0.0 produces identical values.
/// This is a degenerate distribution (all values are exp(mean)).
#[test]
fn test_lognormal_zero_stddev_degenerate() {
    use finstack_statements::forecast::lognormal_forecast;

    let periods = vec![
        PeriodId::quarter(2025, 1),
        PeriodId::quarter(2025, 2),
        PeriodId::quarter(2025, 3),
    ];

    let mut params = indexmap::IndexMap::new();
    params.insert("mean".to_string(), serde_json::json!(11.5));
    params.insert("std_dev".to_string(), serde_json::json!(0.0)); // Degenerate distribution
    params.insert("seed".to_string(), serde_json::json!(42));

    // Should succeed (just warns about degenerate distribution)
    let result = lognormal_forecast(0.0, &periods, &params);
    assert!(result.is_ok());

    // All values should be identical (exp(mean) since std_dev=0)
    let values: Vec<_> = result.unwrap().values().copied().collect();
    let expected = (11.5_f64).exp();

    for value in &values {
        assert!(
            (*value - expected).abs() < 1e-10,
            "Expected {}, got {}",
            expected,
            value
        );
    }
}
