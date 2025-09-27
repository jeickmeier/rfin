//! Repo metrics module.
//!
//! Provides metric calculators specific to `Repo`, split into focused files.
//! The calculators compose with the shared metrics framework and are registered
//! via `register_repo_metrics`.
//!
//! Exposed metrics:
//! - Collateral value and required collateral
//! - Collateral coverage ratio
//! - Repo interest amount
//! - DV01 (discount curve parallel bp)
//! - Funding risk (sensitivity to repo rate)
//! - Effective rate (with special collateral adj.)
//! - Time to maturity (years)
//! - Implied collateral return

pub mod collateral_coverage;
pub mod collateral_value;
pub mod dv01;
pub mod effective_rate;
pub mod funding_risk;
pub mod implied_collateral_return;
pub mod repo_interest;
pub mod required_collateral;
// risk_bucketed_dv01 - now using generic implementation
pub mod time_to_maturity;

use crate::metrics::MetricRegistry;

/// Register all Repo metrics with the registry.
pub fn register_repo_metrics(registry: &mut MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::CollateralValue,
            Arc::new(collateral_value::CollateralValueCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::RequiredCollateral,
            Arc::new(required_collateral::RequiredCollateralCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::CollateralCoverage,
            Arc::new(collateral_coverage::CollateralCoverageCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::RepoInterest,
            Arc::new(repo_interest::RepoInterestCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::Dv01,
            Arc::new(dv01::RepoDv01Calculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::FundingRisk,
            Arc::new(funding_risk::FundingRiskCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::EffectiveRate,
            Arc::new(effective_rate::EffectiveRateCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::TimeToMaturity,
            Arc::new(time_to_maturity::TimeToMaturityCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::ImpliedCollateralReturn,
            Arc::new(implied_collateral_return::ImpliedCollateralReturnCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::BucketedDv01,
            Arc::new(crate::instruments::common::GenericBucketedDv01ForStringCurves::<crate::instruments::Repo>::default()),
            &["Repo"],
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use crate::instruments::repo::{CollateralSpec, Repo};
    use crate::metrics::{MetricCalculator, MetricContext};
    use finstack_core::currency::Currency;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::MarketContext;
    use finstack_core::prelude::*;
    use time::Month;

    fn test_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    fn create_test_repo() -> Repo {
        let collateral = CollateralSpec::new("BOND_ABC", 1000.0, "BOND_ABC_PRICE");
        Repo::term(
            "REPO_001",
            Money::new(1_000_000.0, Currency::USD),
            collateral,
            0.05,
            test_date(2025, 1, 15),
            test_date(2025, 4, 15),
            "USD-OIS",
        )
    }

    fn create_test_context() -> MarketContext {
        let as_of = test_date(2025, 1, 10);
        let disc =
            finstack_core::market_data::term_structures::discount_curve::DiscountCurve::builder(
                "USD-OIS",
            )
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (10.0, 1.0)])
            .build()
            .unwrap();
        MarketContext::new().insert_discount(disc).insert_price(
            "BOND_ABC_PRICE",
            MarketScalar::Price(Money::new(1.02, Currency::USD)),
        )
    }

    #[test]
    fn test_collateral_value_calculator() {
        let repo = create_test_repo();
        let market_context = create_test_context();
        let mut context = MetricContext::new(
            std::sync::Arc::new(repo),
            std::sync::Arc::new(market_context),
            test_date(2025, 1, 10),
            Money::new(0.0, Currency::USD),
        );

        let calculator = collateral_value::CollateralValueCalculator;
        let value = calculator.calculate(&mut context).unwrap();
        assert!((value - 1020.0).abs() < 1e-6);
    }

    #[test]
    fn test_required_collateral_calculator() {
        let repo = create_test_repo();
        let market_context = create_test_context();
        let mut context = MetricContext::new(
            std::sync::Arc::new(repo),
            std::sync::Arc::new(market_context),
            test_date(2025, 1, 10),
            Money::new(0.0, Currency::USD),
        );

        let calculator = required_collateral::RequiredCollateralCalculator;
        let value = calculator.calculate(&mut context).unwrap();
        assert!((value - 1_020_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_effective_rate_calculator() {
        let repo = create_test_repo();
        let market_context = create_test_context();
        let mut context = MetricContext::new(
            std::sync::Arc::new(repo),
            std::sync::Arc::new(market_context),
            test_date(2025, 1, 10),
            Money::new(0.0, Currency::USD),
        );

        let calculator = effective_rate::EffectiveRateCalculator;
        let rate = calculator.calculate(&mut context).unwrap();
        assert!((rate - 0.05).abs() < 1e-9);
    }

    #[test]
    fn test_dv01_positive_when_rates_rise_price_falls() {
        use crate::metrics::{standard_registry, MetricId};
        let as_of = test_date(2025, 1, 10);
        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots(vec![(0.0, 1.0), (5.0, 0.80)])
            .build()
            .unwrap();
        let repo = create_test_repo();
        let ctx = create_test_context().insert_discount(disc);
        let pv = repo.value(&ctx, as_of).unwrap();
        let reg = standard_registry();
        let mut mctx = crate::metrics::MetricContext::new(
            std::sync::Arc::new(repo),
            std::sync::Arc::new(ctx),
            as_of,
            pv,
        );
        let res = reg.compute(&[MetricId::Dv01], &mut mctx).unwrap();
        let dv01 = *res.get(&MetricId::Dv01).unwrap();
        assert!(dv01 >= 0.0);
    }

    #[test]
    fn test_repo_interest_metric_matches_direct_interest() {
        use crate::metrics::{standard_registry, MetricId};
        let as_of = test_date(2025, 1, 10);
        let repo = create_test_repo();
        let ctx = create_test_context();
        let pv = repo.value(&ctx, as_of).unwrap();
        let mut mctx = crate::metrics::MetricContext::new(
            std::sync::Arc::new(repo.clone()),
            std::sync::Arc::new(ctx),
            as_of,
            pv,
        );
        let reg = standard_registry();
        let res = reg.compute(&[MetricId::RepoInterest], &mut mctx).unwrap();
        let m_interest = *res.get(&MetricId::RepoInterest).unwrap();
        let direct = repo.interest_amount().unwrap().amount();
        assert!((m_interest - direct).abs() < 1e-9);
    }

    #[test]
    fn test_collateral_coverage_ratio_reasonable() {
        use crate::metrics::{standard_registry, MetricId};
        let as_of = test_date(2025, 1, 10);
        let repo = create_test_repo();
        let ctx = create_test_context();
        let pv = repo.value(&ctx, as_of).unwrap();
        let mut mctx = crate::metrics::MetricContext::new(
            std::sync::Arc::new(repo),
            std::sync::Arc::new(ctx),
            as_of,
            pv,
        );
        let reg = standard_registry();
        let res = reg
            .compute(&[MetricId::CollateralCoverage], &mut mctx)
            .unwrap();
        let cov = *res.get(&MetricId::CollateralCoverage).unwrap();
        // 1020 / 1,020,000 = 0.001
        assert!((cov - 0.001).abs() < 1e-6);
    }

    #[test]
    fn test_time_to_maturity_and_implied_collateral_return() {
        use crate::metrics::{standard_registry, MetricId};
        let as_of = test_date(2025, 1, 10);
        let repo = create_test_repo();
        let ctx = create_test_context();
        let pv = repo.value(&ctx, as_of).unwrap();
        let mut mctx = crate::metrics::MetricContext::new(
            std::sync::Arc::new(repo.clone()),
            std::sync::Arc::new(ctx),
            as_of,
            pv,
        );
        let reg = standard_registry();
        let res = reg
            .compute(
                &[
                    MetricId::TimeToMaturity,
                    MetricId::CollateralValue,
                    MetricId::RequiredCollateral,
                    MetricId::ImpliedCollateralReturn,
                ],
                &mut mctx,
            )
            .unwrap();
        let ttm = *res.get(&MetricId::TimeToMaturity).unwrap();
        assert!(ttm > 0.0);
        let cv = *res.get(&MetricId::CollateralValue).unwrap();
        let req = *res.get(&MetricId::RequiredCollateral).unwrap();
        let implied = *res.get(&MetricId::ImpliedCollateralReturn).unwrap();
        let expected = if req == 0.0 || ttm <= 0.0 {
            0.0
        } else {
            (cv / req - 1.0) / ttm
        };
        assert!((implied - expected).abs() < 1e-9);
    }

    #[test]
    fn test_funding_risk_sign() {
        use crate::metrics::{standard_registry, MetricId};
        let as_of = test_date(2025, 1, 10);
        let repo = create_test_repo();
        let ctx = create_test_context();
        let pv = repo.value(&ctx, as_of).unwrap();
        let mut mctx = crate::metrics::MetricContext::new(
            std::sync::Arc::new(repo),
            std::sync::Arc::new(ctx),
            as_of,
            pv,
        );
        let reg = standard_registry();
        let res = reg.compute(&[MetricId::FundingRisk], &mut mctx).unwrap();
        let fr = *res.get(&MetricId::FundingRisk).unwrap();
        // Increasing repo rate typically increases PV (more interest), so base - bumped <= 0
        assert!(fr <= 0.0);
    }
}
