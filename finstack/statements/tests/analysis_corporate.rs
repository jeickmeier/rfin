//! Corporate analysis integration tests.
#![allow(clippy::expect_used)]

use finstack_core::currency::Currency;
use finstack_core::dates::PeriodId;
use finstack_statements::analysis::corporate::evaluate_dcf;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::AmountOrScalar;
use finstack_valuations::instruments::TerminalValueSpec;

#[test]
fn test_dcf_evaluation_gordon_growth() {
    let model = ModelBuilder::new("test-corp")
        .periods("2025Q1..Q4", None)
        .expect("valid periods")
        .value(
            "ufcf",
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
        .build()
        .expect("valid model");

    let result = evaluate_dcf(
        &model,
        0.10,
        TerminalValueSpec::GordonGrowth { growth_rate: 0.02 },
        "ufcf",
        Some(50_000.0),
    )
    .expect("DCF evaluation should succeed");

    assert!(result.equity_value.amount() > 0.0);
    assert_eq!(result.equity_value.currency(), Currency::USD);
}
