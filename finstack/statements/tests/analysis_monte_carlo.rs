//! Monte Carlo analysis integration tests.
#![allow(clippy::expect_used)]

use finstack_core::dates::PeriodId;
use finstack_statements::analysis::monte_carlo::MonteCarloConfig;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::{AmountOrScalar, ForecastSpec};

#[test]
fn evaluate_monte_carlo_produces_deterministic_results() {
    let model = ModelBuilder::new("mc-test")
        .periods("2025Q1..Q4", Some("2025Q2"))
        .expect("valid periods")
        .mixed("revenue")
        .values(&[
            (
                PeriodId::quarter(2025, 1),
                AmountOrScalar::scalar(100_000.0),
            ),
            (
                PeriodId::quarter(2025, 2),
                AmountOrScalar::scalar(110_000.0),
            ),
        ])
        .forecast(ForecastSpec::normal(120_000.0, 10_000.0, 42))
        .build()
        .build()
        .expect("valid model");

    let config = MonteCarloConfig::new(32, 7);

    let mut eval1 = finstack_statements::evaluator::Evaluator::new();
    let mut eval2 = finstack_statements::evaluator::Evaluator::new();

    let res1 = eval1
        .evaluate_monte_carlo(&model, &config)
        .expect("mc eval 1");
    let res2 = eval2
        .evaluate_monte_carlo(&model, &config)
        .expect("mc eval 2");

    assert_eq!(res1.n_paths, res2.n_paths);
    assert_eq!(res1.percentiles, res2.percentiles);
    assert_eq!(res1.percentile_results.len(), res2.percentile_results.len());

    let p95_series_1 = res1
        .get_percentile_series("revenue", 0.95)
        .expect("p95 series");
    let p95_series_2 = res2
        .get_percentile_series("revenue", 0.95)
        .expect("p95 series");
    assert_eq!(p95_series_1, p95_series_2);
}
