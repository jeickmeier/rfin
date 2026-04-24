//! Tests covering structured credit instrument-level stochastic helpers and loss math.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::PricingMode;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    CorrelationStructure, DealType, Pool, PoolAsset, StochasticDefaultSpec, StochasticPrepaySpec,
    StructuredCredit, Tranche, TrancheCoupon, TrancheStructure,
};
use time::Month;

fn closing_date() -> Date {
    Date::from_calendar_date(2024, Month::January, 1).unwrap()
}

fn legal_maturity() -> Date {
    Date::from_calendar_date(2030, Month::January, 1).unwrap()
}

fn simple_pool(balance: f64) -> Pool {
    let mut pool = Pool::new("POOL", DealType::ABS, Currency::USD);
    if balance > 0.0 {
        pool.assets.push(PoolAsset::fixed_rate_bond(
            "A1",
            Money::new(balance, Currency::USD),
            0.06,
            Date::from_calendar_date(2029, Month::January, 1).unwrap(),
            finstack_core::dates::DayCount::Thirty360,
        ));
    }
    pool
}

fn single_tranche_structure(balance: f64) -> TrancheStructure {
    let tranche = Tranche::new(
        "SENIOR",
        0.0,
        100.0,
        finstack_valuations::instruments::fixed_income::structured_credit::Seniority::Senior,
        Money::new(balance, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        legal_maturity(),
    )
    .unwrap();
    TrancheStructure::new(vec![tranche]).unwrap()
}

fn discount_curve(base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (5.0, 0.95)])
        .build()
        .expect("discount curve")
}

fn build_sc(id: &str, pool_balance: f64) -> StructuredCredit {
    let pool = simple_pool(pool_balance);
    let tranches = single_tranche_structure(pool_balance);
    StructuredCredit::new_abs(
        id,
        pool,
        tranches,
        closing_date(),
        legal_maturity(),
        "USD-OIS",
    )
    .with_payment_calendar("nyse")
}

#[test]
fn stochastic_pricing_zero_notional_returns_zero_result() {
    let sc = build_sc("ABS-ZERO", 0.0);
    let mut market = MarketContext::new();
    market = market.insert(discount_curve(closing_date()));

    let result = sc
        .price_stochastic_with_mode(&market, closing_date(), PricingMode::Tree)
        .expect("stochastic pricing");

    assert_eq!(result.npv.amount(), 0.0);
    assert_eq!(result.expected_loss.amount(), 0.0);
    assert!(
        result.tranche_results.is_empty(),
        "zero notional should skip tranche pricing"
    );
    assert_eq!(result.num_paths, 0);
}

#[test]
fn stochastic_pricing_is_deterministic_and_returns_tranche_results() {
    let sc = build_sc("ABS-DETERMINISTIC", 1_000_000.0);
    let mut market = MarketContext::new();
    market = market.insert(discount_curve(closing_date()));

    let as_of = closing_date();
    let first = sc
        .price_stochastic_with_mode(
            &market,
            as_of,
            PricingMode::MonteCarlo {
                num_paths: 1,
                antithetic: false,
            },
        )
        .expect("stochastic pricing");
    let second = sc
        .price_stochastic_with_mode(
            &market,
            as_of,
            PricingMode::MonteCarlo {
                num_paths: 1,
                antithetic: false,
            },
        )
        .expect("stochastic pricing");

    assert!(first.npv.amount().is_finite());
    assert_eq!(first.tranche_results.len(), 1);
    assert_eq!(first.pricing_mode, "MonteCarlo(1)");
    assert_eq!(first.npv.amount(), second.npv.amount());
    assert_eq!(first.tranche_results.len(), second.tranche_results.len());
}

#[test]
fn stochastic_pricing_rejects_invalid_correlation_structure() {
    let mut sc = build_sc("ABS-BAD-CORR", 1_000_000.0);
    sc.with_correlation(CorrelationStructure::matrix(
        vec![1.0, 0.2, 0.2],
        vec!["A".to_string(), "B".to_string()],
    ));
    let mut market = MarketContext::new();
    market = market.insert(discount_curve(closing_date()));

    let err = sc
        .price_stochastic_with_mode(&market, closing_date(), PricingMode::Tree)
        .expect_err("invalid correlation should fail before pricing");

    assert!(format!("{err:?}").contains("Correlation matrix size mismatch"));
}

#[test]
fn current_loss_percentage_respects_defaults_and_recoveries() {
    let mut sc = build_sc("ABS-LOSS", 1_000_000.0);
    sc.pool.cumulative_defaults = Money::new(100_000.0, Currency::USD);
    sc.pool.cumulative_recoveries = Money::new(25_000.0, Currency::USD);

    let loss_pct = sc.current_loss_percentage().expect("loss percentage");
    // Original balance ≈ current(1M) + defaults(100k) + prepays(0) = 1.1M
    // Net loss = 100k - 25k = 75k => 75k / 1.1M * 100 ≈ 6.818%
    let expected = (100_000.0 - 25_000.0) / 1_100_000.0 * 100.0;
    assert!(
        (loss_pct - expected).abs() < 1e-9,
        "expected {expected}%, got {loss_pct}"
    );
}

#[test]
fn stochastic_helper_methods_toggle_flags_and_preserve_chainability() {
    let mut sc = build_sc("ABS-STOCHASTIC", 1_000_000.0);
    assert!(!sc.is_stochastic());

    let chained = sc
        .with_stochastic_prepay(StochasticPrepaySpec::clo_standard())
        .with_stochastic_default(StochasticDefaultSpec::clo_standard())
        .with_correlation(CorrelationStructure::clo_standard());

    assert!(std::ptr::eq(chained, &sc));
    assert!(sc.is_stochastic());
    assert!(sc.credit_model.stochastic_prepay_spec.is_some());
    assert!(sc.credit_model.stochastic_default_spec.is_some());
    assert!(sc.credit_model.correlation_structure.is_some());

    sc.disable_stochastic();
    assert!(!sc.is_stochastic());
    assert!(sc.credit_model.stochastic_prepay_spec.is_none());
    assert!(sc.credit_model.stochastic_default_spec.is_none());
    assert!(sc.credit_model.correlation_structure.is_none());
}

#[test]
fn enable_stochastic_defaults_populates_specs_for_each_deal_family() {
    let mut abs = build_sc("ABS-DEFAULTS", 1_000_000.0);
    abs.enable_stochastic_defaults();
    assert!(abs.is_stochastic());

    let make = |deal_type| {
        let pool = Pool::new("POOL", deal_type, Currency::USD);
        let tranches = single_tranche_structure(1_000_000.0);
        StructuredCredit::apply_deal_defaults(
            format!("TEST-{deal_type:?}"),
            deal_type,
            pool,
            tranches,
            closing_date(),
            legal_maturity(),
            "USD-OIS",
        )
    };

    for mut sc in [
        make(DealType::RMBS),
        make(DealType::CLO),
        make(DealType::CMBS),
        make(DealType::Card),
    ] {
        sc.enable_stochastic_defaults();
        assert!(sc.credit_model.stochastic_prepay_spec.is_some());
        assert!(sc.credit_model.stochastic_default_spec.is_some());
        assert!(sc.credit_model.correlation_structure.is_some());
    }
}

#[test]
fn price_with_metrics_standalone_returns_base_value_when_no_metrics_or_hedges() {
    let sc = build_sc("ABS-STANDALONE", 1_000_000.0).with_payment_calendar("nyse");
    let mut market = MarketContext::new();
    market = market.insert(discount_curve(closing_date()));

    let result = sc
        .price_with_metrics_standalone(&market, closing_date(), &[])
        .expect("standalone pricing");

    assert_eq!(result.instrument_id, "ABS-STANDALONE");
    assert!(result.value.amount().is_finite());
    assert_eq!(result.value.currency(), Currency::USD);
    assert!(result.measures.is_empty());
}

#[test]
fn hedge_helpers_track_attached_swaps() {
    let swap = finstack_valuations::instruments::rates::irs::InterestRateSwap::example()
        .expect("example hedge swap");
    let mut sc = build_sc("ABS-HEDGED", 1_000_000.0);
    assert!(!sc.has_hedges());
    assert_eq!(sc.hedge_count(), 0);

    sc.add_hedge_swap(swap.clone());
    assert!(sc.has_hedges());
    assert_eq!(sc.hedge_count(), 1);

    sc.add_hedge_swaps(vec![swap.clone()]);
    assert_eq!(sc.hedge_count(), 2);

    let chained = build_sc("ABS-HEDGED-BUILDER", 1_000_000.0)
        .with_hedge_swap(swap.clone())
        .with_hedge_swaps(vec![swap]);
    assert!(chained.has_hedges());
    assert_eq!(chained.hedge_count(), 2);
}

#[test]
fn hedge_valuation_helpers_return_zero_when_no_swaps_are_attached() {
    let sc = build_sc("ABS-UNHEDGED", 1_000_000.0).with_payment_calendar("nyse");
    let mut market = MarketContext::new();
    market = market.insert(discount_curve(closing_date()));

    let hedge_npv = sc.hedge_npv(&market, closing_date()).expect("hedge npv");
    let (deal_npv, hedges, total) = sc
        .price_with_hedges(&market, closing_date())
        .expect("combined hedge pricing");

    assert_eq!(hedge_npv.amount(), 0.0);
    assert_eq!(hedges.amount(), 0.0);
    assert_eq!(deal_npv, total);
}
