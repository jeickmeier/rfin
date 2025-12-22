use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{create_date, DayCount};
use finstack_core::money::Money;
use finstack_portfolio::builder::PortfolioBuilder;
use finstack_portfolio::optimization::{
    CandidatePosition, DefaultLpOptimizer, MetricExpr, Objective, PerPositionMetric,
    PortfolioOptimizationProblem, PortfolioOptimizer, WeightingScheme,
};
use finstack_portfolio::position::{Position, PositionUnit};
use finstack_portfolio::types::Entity;
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;
use time::Month;

// Mock market context builder (simplified)
fn build_mock_market() -> finstack_core::market_data::context::MarketContext {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;

    let as_of = create_date(2024, Month::January, 1).unwrap();
    // Build a flat 5% yield curve using knots
    // 5% continuously compounded rate roughly.
    // Discount factor at T=1 is exp(-0.05*1) = 0.9512
    let flat_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (10.0, 0.6065)]) // exp(-0.05 * 10) = 0.6065
        .build()
        .expect("Curve build failed");

    let mut market = MarketContext::new();
    market = market.insert_discount(flat_curve);
    market
}

#[test]
fn test_notional_weighting() -> Result<(), Box<dyn std::error::Error>> {
    let as_of = create_date(2024, Month::January, 1)?;

    // Deposit 1: Long 1M USD
    let dep1 = Deposit::builder()
        .id("DEP_LONG".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(create_date(2024, Month::February, 1)?)
        .day_count(DayCount::Act365F)
        .discount_curve_id("USD-OIS".into())
        .quote_rate_opt(Some(0.045))
        .build()?;

    let dep2 = Deposit::builder()
        .id("DEP_SHORT".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(create_date(2024, Month::February, 1)?)
        .day_count(DayCount::Act365F)
        .discount_curve_id("USD-OIS".into())
        .quote_rate_opt(Some(0.045))
        .build()?;

    let p1 = Position::new(
        "POS_LONG",
        "ENT_A",
        "DEP_LONG",
        Arc::new(dep1),
        1.0,
        PositionUnit::Notional(Some(Currency::USD)),
    )?;

    let p2 = Position::new(
        "POS_SHORT",
        "ENT_A",
        "DEP_SHORT",
        Arc::new(dep2),
        -1.0,
        PositionUnit::Notional(Some(Currency::USD)),
    )?;

    let portfolio = PortfolioBuilder::new("HEDGED_PORTFOLIO")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("ENT_A"))
        .position(p1)
        .position(p2)
        .build()?;

    // With NotionalWeight, Total Notional = 1M + |-1M| = 2M.
    // Weights should be 0.5 and -0.5.

    let mut problem = PortfolioOptimizationProblem::new(
        portfolio,
        Objective::Maximize(MetricExpr::WeightedSum {
            metric: PerPositionMetric::Constant(1.0),
        }),
    );
    problem.weighting = WeightingScheme::NotionalWeight;

    let market = build_mock_market();
    let config = FinstackConfig::default();
    let optimizer = DefaultLpOptimizer::default();

    let result = optimizer.optimize(&problem, &market, &config)?;

    println!("Status: {:?}", result.status);
    println!("Current Weights: {:?}", result.current_weights);

    let w_long = result.current_weights.get("POS_LONG").unwrap();
    let w_short = result.current_weights.get("POS_SHORT").unwrap();

    assert!(w_long.is_finite());
    assert!(w_short.is_finite());
    // Expect approx 0.5 and -0.5
    assert!((w_long - 0.5).abs() < 1e-4);
    assert!((w_short + 0.5).abs() < 1e-4);

    Ok(())
}

#[test]
fn test_candidate_batching() -> Result<(), Box<dyn std::error::Error>> {
    let as_of = create_date(2024, Month::January, 1)?;

    let portfolio = PortfolioBuilder::new("EMPTY_PORTFOLIO")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .build()?;

    let mut problem = PortfolioOptimizationProblem::new(
        portfolio,
        Objective::Maximize(MetricExpr::WeightedSum {
            metric: PerPositionMetric::PvBase,
        }),
    );

    // Add 10 candidate deposits
    for i in 0..10 {
        let dep = Deposit::builder()
            .id(format!("CAND_DEP_{}", i).into())
            .notional(Money::new(100_000.0, Currency::USD))
            .start(as_of)
            .end(create_date(2024, Month::February, 1)?)
            .day_count(DayCount::Act365F)
            .discount_curve_id("USD-OIS".into())
            .quote_rate_opt(Some(0.045))
            .build()?;

        let cand = CandidatePosition::new(
            format!("CAND_{}", i),
            "ENT_A",
            Arc::new(dep),
            PositionUnit::Units,
        )
        .with_max_weight(0.1);

        problem.trade_universe.candidates.push(cand);
    }

    let market = build_mock_market();
    let config = FinstackConfig::default();
    let optimizer = DefaultLpOptimizer::default();

    let result = optimizer.optimize(&problem, &market, &config)?;

    assert!(result.status.is_feasible());
    assert_eq!(result.optimal_weights.len(), 10);

    Ok(())
}
