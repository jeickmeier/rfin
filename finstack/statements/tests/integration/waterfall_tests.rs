//! Integration tests for Cash Flow Waterfall & Sweep Mechanics

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, PeriodId};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::capital_structure::{EcfSweepSpec, WaterfallSpec};
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::AmountOrScalar;
use time::Month;

#[test]
fn test_ecf_sweep_basic() {
    // Create a simple model with a term loan and ECF sweep
    let issue = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
    let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("valid date");

    let model = ModelBuilder::new("ecf_test")
        .periods("2025Q1..2025Q2", None)
        .expect("valid periods")
        .value(
            "ebitda",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(1_000_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(1_100_000.0),
                ),
            ],
        )
        .value(
            "taxes",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(200_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(220_000.0),
                ),
            ],
        )
        .value(
            "capex",
            &[
                (
                    PeriodId::quarter(2025, 1),
                    AmountOrScalar::scalar(100_000.0),
                ),
                (
                    PeriodId::quarter(2025, 2),
                    AmountOrScalar::scalar(100_000.0),
                ),
            ],
        )
        .add_bond(
            "BOND-001",
            Money::new(10_000_000.0, Currency::USD),
            0.05,
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("valid bond")
        .waterfall(WaterfallSpec {
            ecf_sweep: Some(EcfSweepSpec {
                ebitda_node: "ebitda".to_string(),
                taxes_node: Some("taxes".to_string()),
                capex_node: Some("capex".to_string()),
                working_capital_node: None,
                cash_interest_node: None,
                sweep_percentage: 0.5,      // 50% sweep
                target_instrument_id: None, // Apply to all
            }),
            ..WaterfallSpec::default()
        })
        .build()
        .expect("model should build");

    // Create market context
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(issue)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build()
        .expect("curve should build");
    let market_ctx = MarketContext::new().insert_discount(disc_curve);

    // Evaluate model
    let mut evaluator = Evaluator::new();
    let results = evaluator
        .evaluate_with_market_context(&model, Some(&market_ctx), Some(issue))
        .expect("evaluation should succeed");

    // Verify that EBITDA values are present
    assert!(results.get("ebitda", &PeriodId::quarter(2025, 1)).is_some());
    assert_eq!(
        results.get("ebitda", &PeriodId::quarter(2025, 1)),
        Some(1_000_000.0)
    );
}
