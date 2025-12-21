//! Comprehensive tests for all repo metrics calculators.

use super::fixtures::*;
use finstack_core::prelude::*;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::repo::{CollateralSpec, Repo};
use finstack_valuations::metrics::*;
use std::sync::Arc;

fn create_test_repo() -> Repo {
    let collateral = treasury_collateral();
    Repo::term(
        "METRICS_TEST",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    )
}

fn create_metric_context(repo: Repo, context: MarketContext, as_of: Date) -> MetricContext {
    let pv = repo.value(&context, as_of).unwrap();
    MetricContext::new(Arc::new(repo), Arc::new(context), as_of, pv)
}

#[test]
fn test_collateral_value_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::CollateralValue], &mut mctx)
        .unwrap();

    let collateral_value = results.get(&MetricId::CollateralValue).unwrap();

    // 1M * 1.02 = 1,020,000
    assert_approx_eq(*collateral_value, 1_020_000.0, 1.0);
}

#[test]
fn test_required_collateral_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::RequiredCollateral], &mut mctx)
        .unwrap();

    let required = results.get(&MetricId::RequiredCollateral).unwrap();

    // 1M / (1 - 0.02) = 1,020,408.16
    assert_approx_eq(*required, 1_020_408.16, 1.0);
}

#[test]
fn test_collateral_coverage_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::CollateralCoverage], &mut mctx)
        .unwrap();

    let coverage = results.get(&MetricId::CollateralCoverage).unwrap();

    // 1,020,000 / 1,020,000 = 1.0
    assert_approx_eq(*coverage, 1.0, 0.01);
}

#[test]
fn test_collateral_coverage_overcollateralized() {
    // Use enough collateral to be over-collateralized
    // Collateral: 2M units at 105% = 2,100,000
    // Required: 1M * 1.02 = 1,020,000
    // Coverage: 2,100,000 / 1,020,000 = 2.06
    let collateral = CollateralSpec::new("SPECIAL_BOND", 2_000_000.0, "SPECIAL_BOND_PRICE");
    let repo = Repo::term(
        "OVERCOLLATERALIZED",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::CollateralCoverage], &mut mctx)
        .unwrap();

    let coverage = results.get(&MetricId::CollateralCoverage).unwrap();

    // Should be > 1.0 (expecting ~2.06)
    assert!(
        *coverage > 1.5,
        "Coverage should exceed 1.5 for overcollateralized repo, got {}",
        coverage
    );
}

#[test]
fn test_repo_interest_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo.clone(), context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::RepoInterest], &mut mctx)
        .unwrap();

    let metric_interest = results.get(&MetricId::RepoInterest).unwrap();
    let direct_interest = repo.interest_amount().unwrap().amount();

    assert_approx_eq(*metric_interest, direct_interest, 1e-9);
}

#[test]
fn test_effective_rate_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo.clone(), context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::EffectiveRate], &mut mctx)
        .unwrap();

    let effective_rate = results.get(&MetricId::EffectiveRate).unwrap();

    assert_approx_eq(*effective_rate, repo.effective_rate(), 1e-9);
}

#[test]
fn test_time_to_maturity_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let as_of = date(2025, 1, 10);
    let mut mctx = create_metric_context(repo.clone(), context, as_of);

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::TimeToMaturity], &mut mctx)
        .unwrap();

    let ttm = results.get(&MetricId::TimeToMaturity).unwrap();

    // Should be positive and reasonable (roughly 0.25 years for 3 months)
    assert!(*ttm > 0.0);
    assert!(*ttm < 1.0);
}

#[test]
fn test_time_to_maturity_at_maturity() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let maturity = repo.maturity;
    let mut mctx = create_metric_context(repo, context, maturity);

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::TimeToMaturity], &mut mctx)
        .unwrap();

    let ttm = results.get(&MetricId::TimeToMaturity).unwrap();

    assert_approx_eq(*ttm, 0.0, 1e-6);
}

#[test]
fn test_implied_collateral_return_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::ImpliedCollateralReturn], &mut mctx)
        .unwrap();

    let implied_return = results.get(&MetricId::ImpliedCollateralReturn).unwrap();

    // For adequately collateralized repo, should be near zero
    assert!(implied_return.abs() < 0.1);
}

#[test]
fn test_dv01_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry.compute(&[MetricId::Dv01], &mut mctx).unwrap();

    let dv01 = results.get(&MetricId::Dv01).unwrap();

    // DV01 = PV(bumped) - PV(base); when rates rise, PV falls, so DV01 is negative
    assert!(*dv01 <= 0.0);

    // Should be reasonable magnitude for 1M notional, 3-month repo
    assert!(dv01.abs() < 1000.0);
}

#[test]
fn test_funding_risk_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::FundingRisk], &mut mctx)
        .unwrap();

    let funding_risk = results.get(&MetricId::FundingRisk).unwrap();

    // Increasing repo rate typically increases PV (more interest earned)
    // So funding risk (base - bumped) should be negative
    assert!(*funding_risk <= 0.0);
}

#[test]
fn test_theta_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry.compute(&[MetricId::Theta], &mut mctx).unwrap();

    let theta = results.get(&MetricId::Theta).unwrap();

    // Theta measures time decay; should be non-zero for mid-life repo
    assert!(theta.abs() > 0.0);
}

#[test]
fn test_accrued_interest_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();

    // Value mid-term to have accrued interest
    let mid_date = date(2025, 2, 15);
    let mut mctx = create_metric_context(repo.clone(), context, mid_date);

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::AccruedInterest], &mut mctx)
        .unwrap();

    let accrued = results.get(&MetricId::AccruedInterest).unwrap();

    // Should have accrued some interest by mid-term
    assert!(*accrued > 0.0);

    // Should be less than total interest
    let total_interest = repo.interest_amount().unwrap().amount();
    assert!(*accrued < total_interest);
}

#[test]
fn test_accrued_interest_before_start() {
    let repo = create_test_repo();
    let context = create_standard_market_context();

    // Value before start
    let before_start = date(2025, 1, 10);
    let mut mctx = create_metric_context(repo, context, before_start);

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::AccruedInterest], &mut mctx)
        .unwrap();

    let accrued = results.get(&MetricId::AccruedInterest).unwrap();

    // No accrual before start
    assert_approx_eq(*accrued, 0.0, 1e-6);
}

#[test]
fn test_accrued_interest_at_maturity() {
    let repo = create_test_repo();
    let context = create_standard_market_context();

    let maturity = repo.maturity;
    let mut mctx = create_metric_context(repo.clone(), context, maturity);

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::AccruedInterest], &mut mctx)
        .unwrap();

    let accrued = results.get(&MetricId::AccruedInterest).unwrap();
    let total_interest = repo.interest_amount().unwrap().amount();

    // At maturity, accrued should equal total interest
    assert_approx_eq(*accrued, total_interest, 1.0);
}

#[test]
fn test_bucketed_dv01_metric() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();
    let results = registry
        .compute(&[MetricId::BucketedDv01], &mut mctx)
        .unwrap();

    // Should successfully compute
    assert!(results.contains_key(&MetricId::BucketedDv01));
}

#[test]
fn test_multiple_metrics_computed_together() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let metrics = vec![
        MetricId::CollateralValue,
        MetricId::RequiredCollateral,
        MetricId::CollateralCoverage,
        MetricId::RepoInterest,
        MetricId::EffectiveRate,
        MetricId::TimeToMaturity,
    ];

    let registry = standard_registry();
    let results = registry.compute(&metrics, &mut mctx).unwrap();

    // All metrics should be present
    for metric in &metrics {
        assert!(results.contains_key(metric), "Missing metric: {:?}", metric);
    }
}

#[test]
fn test_metric_dependencies_resolved() {
    let repo = create_test_repo();
    let context = create_standard_market_context();
    let mut mctx = create_metric_context(repo, context, date(2025, 1, 10));

    let registry = standard_registry();

    // CollateralCoverage depends on CollateralValue and RequiredCollateral
    let results = registry
        .compute(&[MetricId::CollateralCoverage], &mut mctx)
        .unwrap();

    // The metric should be computed
    assert!(
        results.contains_key(&MetricId::CollateralCoverage),
        "CollateralCoverage should be computed"
    );

    // Note: The current metrics framework may not return dependencies in results
    // Just verify the main metric works
    let coverage = results.get(&MetricId::CollateralCoverage).unwrap();
    assert!(*coverage > 0.0, "Coverage should be positive");
}

#[test]
fn test_metrics_with_price_with_metrics() {
    let repo = create_test_repo();
    let context = create_standard_market_context();

    let metrics = vec![
        MetricId::CollateralValue,
        MetricId::RequiredCollateral,
        MetricId::RepoInterest,
        MetricId::Dv01,
    ];

    let result = repo
        .price_with_metrics(&context, date(2025, 1, 10), &metrics)
        .unwrap();

    // Check base valuation
    assert_eq!(result.value.currency(), Currency::USD);

    // Check metrics are present in results
    assert!(result.measures.contains_key("collateral_value"));
    assert!(result.measures.contains_key("required_collateral"));
    assert!(result.measures.contains_key("repo_interest"));
    assert!(result.measures.contains_key("dv01"));
}

#[test]
fn test_metrics_registry_has_repo_metrics() {
    let registry = standard_registry();

    // Verify all repo metrics are registered
    assert!(registry.has_metric(MetricId::CollateralValue));
    assert!(registry.has_metric(MetricId::RequiredCollateral));
    assert!(registry.has_metric(MetricId::CollateralCoverage));
    assert!(registry.has_metric(MetricId::RepoInterest));
    assert!(registry.has_metric(MetricId::EffectiveRate));
    assert!(registry.has_metric(MetricId::Dv01));
    assert!(registry.has_metric(MetricId::FundingRisk));
    assert!(registry.has_metric(MetricId::TimeToMaturity));
    assert!(registry.has_metric(MetricId::ImpliedCollateralReturn));
    assert!(registry.has_metric(MetricId::Theta));
    assert!(registry.has_metric(MetricId::AccruedInterest));
    assert!(registry.has_metric(MetricId::BucketedDv01));
}

#[test]
fn test_metrics_applicable_to_repo() {
    let registry = standard_registry();

    // Check applicability to Repo instrument type
    assert!(registry.is_applicable(
        &MetricId::CollateralValue,
        finstack_valuations::pricer::InstrumentType::Repo
    ));
    assert!(registry.is_applicable(
        &MetricId::RequiredCollateral,
        finstack_valuations::pricer::InstrumentType::Repo
    ));
    assert!(registry.is_applicable(
        &MetricId::Dv01,
        finstack_valuations::pricer::InstrumentType::Repo
    ));
    assert!(registry.is_applicable(
        &MetricId::Theta,
        finstack_valuations::pricer::InstrumentType::Repo
    ));
}
