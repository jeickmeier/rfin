//! Basic integration tests for portfolio optimization.

mod common;

use common::{base_date, market_with_usd};
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_portfolio::PortfolioOptimizer;
use finstack_portfolio::{
    optimization::{
        Constraint, DefaultLpOptimizer, MetricExpr, Objective, PerPositionMetric,
        PortfolioOptimizationProblem, WeightingScheme,
    },
    PortfolioBuilder, Position, PositionUnit,
};
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::rates::deposit::Deposit;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::MetricId;
use std::sync::Arc;
use time::{Duration, Month};

/// Build a simple two‑deposit portfolio for optimization tests.
fn build_deposit_portfolio() -> finstack_portfolio::Portfolio {
    let as_of = base_date();

    let dep1_end = as_of + Duration::days(30);
    let dep2_end = as_of + Duration::days(60);

    let dep1 = Deposit::builder()
        .id("DEP_1".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(dep1_end)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .quote_rate_opt(Some(0.045))
        .build()
        .expect("deposit 1 should build");

    let dep2 = Deposit::builder()
        .id("DEP_2".into())
        .notional(Money::new(2_000_000.0, Currency::USD))
        .start(as_of)
        .end(dep2_end)
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD".into())
        .quote_rate_opt(Some(0.045))
        .build()
        .expect("deposit 2 should build");

    let pos1 = Position::new(
        "POS_1",
        "ENTITY_A",
        "DEP_1",
        Arc::new(dep1),
        1.0,
        PositionUnit::Units,
    )
    .expect("position 1 should build");

    let pos2 = Position::new(
        "POS_2",
        "ENTITY_A",
        "DEP_2",
        Arc::new(dep2),
        1.0,
        PositionUnit::Units,
    )
    .expect("position 2 should build");

    PortfolioBuilder::new("TEST_PORTFOLIO")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(finstack_portfolio::Entity::new("ENTITY_A"))
        .position(pos1)
        .position(pos2)
        .build()
        .expect("portfolio should build")
}

#[test]
fn optimize_simple_value_weighted_portfolio() {
    let portfolio = build_deposit_portfolio();
    let market = market_with_usd();
    let config = FinstackConfig::default();

    // Maximize PV in base currency subject to budget constraint.
    let problem = PortfolioOptimizationProblem::new(
        portfolio,
        Objective::Maximize(MetricExpr::WeightedSum {
            metric: PerPositionMetric::PvBase,
        }),
    );

    let optimizer = DefaultLpOptimizer::default();
    let result = optimizer
        .optimize(&problem, &market, &config)
        .expect("optimization should succeed");

    assert!(result.status.is_feasible(), "solution should be feasible");

    let w1 = result.optimal_weights.get("POS_1").copied().unwrap_or(0.0);
    let w2 = result.optimal_weights.get("POS_2").copied().unwrap_or(0.0);

    let sum_w = w1 + w2;
    assert!(
        (sum_w - 1.0).abs() < 1.0e-6,
        "weights should sum to 1, got {}",
        sum_w
    );
    assert!(w1 >= 0.0 && w2 >= 0.0, "weights should be non-negative");
}

/// Build a small bond portfolio with rating tags for a more realistic test.
fn build_bond_portfolio() -> finstack_portfolio::Portfolio {
    let as_of = base_date();
    let issue = as_of;
    let maturity =
        Date::from_calendar_date(as_of.year() + 5, Month::January, 1).expect("valid maturity date");

    // All bonds use the same discount curve "USD" so that YTM is well-defined.
    let mut bond_aaa = Bond::fixed(
        "BOND_AAA",
        Money::new(1_000_000.0, Currency::USD),
        0.03,
        issue,
        maturity,
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    let mut bond_bbb = Bond::fixed(
        "BOND_BBB",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue,
        maturity,
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    let mut bond_ccc = Bond::fixed(
        "BOND_CCC",
        Money::new(1_000_000.0, Currency::USD),
        0.08,
        issue,
        maturity,
        "USD",
    )
    .expect("Bond::fixed should succeed with valid parameters");

    // For yield-based optimization, require explicit quoted clean prices for all bonds.
    // Use par (100.0) for simplicity so coupon ordering drives YTM ordering.
    bond_aaa.pricing_overrides = PricingOverrides::default().with_clean_price(100.0);
    bond_bbb.pricing_overrides = PricingOverrides::default().with_clean_price(100.0);
    bond_ccc.pricing_overrides = PricingOverrides::default().with_clean_price(100.0);

    let pos_aaa = Position::new(
        "POS_AAA",
        "FUND_A",
        "BOND_AAA",
        Arc::new(bond_aaa),
        1.0,
        PositionUnit::FaceValue,
    )
    .expect("AAA position should build")
    .with_tag("rating", "AAA");

    let pos_bbb = Position::new(
        "POS_BBB",
        "FUND_A",
        "BOND_BBB",
        Arc::new(bond_bbb),
        1.0,
        PositionUnit::FaceValue,
    )
    .expect("BBB position should build")
    .with_tag("rating", "BBB");

    let pos_ccc = Position::new(
        "POS_CCC",
        "FUND_A",
        "BOND_CCC",
        Arc::new(bond_ccc),
        1.0,
        PositionUnit::FaceValue,
    )
    .expect("CCC position should build")
    .with_tag("rating", "CCC");

    PortfolioBuilder::new("BOND_FUND")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(finstack_portfolio::Entity::new("FUND_A"))
        .position(pos_aaa)
        .position(pos_bbb)
        .position(pos_ccc)
        .build()
        .expect("bond portfolio should build")
}

/// Finance‑realistic test:
/// Maximize value‑weighted average yield (YTM) subject to a CCC exposure limit.
#[test]
fn optimize_max_yield_with_ccc_limit() {
    let portfolio = build_bond_portfolio();
    let market = market_with_usd();
    let config = FinstackConfig::default();

    // Objective: maximize value‑weighted average yield.
    let objective = Objective::Maximize(MetricExpr::ValueWeightedAverage {
        metric: PerPositionMetric::Metric(MetricId::Ytm),
    });

    let mut problem = PortfolioOptimizationProblem::new(portfolio, objective);
    problem.weighting = WeightingScheme::ValueWeight;

    // Constraint: CCC exposure <= 20% of portfolio.
    problem = problem.with_constraint(Constraint::TagExposureLimit {
        label: Some("ccc_limit".to_string()),
        tag_key: "rating".to_string(),
        tag_value: "CCC".to_string(),
        max_share: 0.20,
    });

    let optimizer = DefaultLpOptimizer::default();
    let result = optimizer
        .optimize(&problem, &market, &config)
        .expect("optimization should succeed");

    assert!(
        result.status.is_feasible(),
        "solution should be feasible, got {:?}",
        result.status
    );

    // Weights should be non‑negative and sum to ~1.0.
    let total_weight: f64 = result.optimal_weights.values().copied().sum();
    assert!(
        (total_weight - 1.0).abs() < 1.0e-6,
        "weights should sum to 1, got {}",
        total_weight
    );
    for (_pos_id, &w) in &result.optimal_weights {
        assert!(w >= -1.0e-9, "weights should be non-negative, got {}", w);
    }

    // Check CCC exposure constraint directly from weights and tags.
    let portfolio_ref = &result.problem.portfolio;
    let mut ccc_weight = 0.0_f64;
    for (pos_id, &w) in &result.optimal_weights {
        if let Some(position) = portfolio_ref.get_position(pos_id.as_str()) {
            if position.tags.get("rating").map(String::as_str) == Some("CCC") {
                ccc_weight += w;
            }
        }
    }
    assert!(
        ccc_weight <= 0.20 + 1.0e-6,
        "CCC weight should be <= 20%, got {}",
        ccc_weight
    );
}
