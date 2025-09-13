//! Metrics calculations for Repurchase Agreement (Repo) instruments.

use crate::instruments::traits::Priceable;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::prelude::*;
use finstack_core::market_data::context::BumpSpec;
use finstack_core::F;
use hashbrown::HashMap;

/// Calculate the market value of collateral.
pub struct CollateralValueCalculator;

impl MetricCalculator for CollateralValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        let collateral_value = repo.collateral.market_value(&context.curves)?;
        Ok(collateral_value.amount())
    }
}

/// Calculate required collateral value including haircut.
pub struct RequiredCollateralCalculator;

impl MetricCalculator for RequiredCollateralCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        let required_value = repo.required_collateral_value();
        Ok(required_value.amount())
    }
}

/// Calculate collateral coverage ratio (market value / required value).
pub struct CollateralCoverageCalculator;

impl MetricCalculator for CollateralCoverageCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::CollateralValue, MetricId::RequiredCollateral]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let collateral_value = context.computed.get(&MetricId::CollateralValue)
            .copied()
            .unwrap_or(0.0);
        let required_value = context.computed.get(&MetricId::RequiredCollateral)
            .copied()
            .unwrap_or(1.0);
        
        if required_value == 0.0 {
            return Ok(F::INFINITY);
        }
        
        Ok(collateral_value / required_value)
    }
}

/// Calculate repo interest amount.
pub struct RepoInterestCalculator;

impl MetricCalculator for RepoInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        let interest = repo.interest_amount()?;
        Ok(interest.amount())
    }
}

/// Calculate DV01 for repo (interest rate sensitivity).
pub struct RepoDv01Calculator;

impl MetricCalculator for RepoDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        
        // Get base PV
        let base_pv = repo.value(&context.curves, context.as_of)?;
        
        // Create bumped market context (1bp parallel shift)
        let disc_curve_id = finstack_core::types::CurveId::new(repo.disc_id);
        let mut bumps = HashMap::new();
        bumps.insert(disc_curve_id, BumpSpec::parallel_bp(1.0));
        
        let bumped_context = context.curves.bump(bumps)?;
        let bumped_pv = repo.value(&bumped_context, context.as_of)?;
        
        // DV01 = base_pv - bumped_pv (positive when rates increase, price decreases)
        let dv01 = base_pv.checked_sub(bumped_pv)?;
        Ok(dv01.amount())
    }
}

/// Calculate funding risk (repo rate sensitivity).
pub struct FundingRiskCalculator;

impl MetricCalculator for FundingRiskCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        
        // Calculate PV sensitivity to 1bp change in repo rate
        let base_pv = repo.value(&context.curves, context.as_of)?.amount();
        
        // Create a modified repo with +1bp rate
        let mut repo_bumped = repo.clone();
        repo_bumped.repo_rate += 0.0001; // +1bp
        
        let bumped_pv = repo_bumped.value(&context.curves, context.as_of)?.amount();
        
        // Funding risk = sensitivity to repo rate changes
        Ok(base_pv - bumped_pv)
    }
}

/// Calculate effective repo rate considering special collateral.
pub struct EffectiveRateCalculator;

impl MetricCalculator for EffectiveRateCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        Ok(repo.effective_rate())
    }
}

/// Calculate time to maturity in years.
pub struct TimeToMaturityCalculator;

impl MetricCalculator for TimeToMaturityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        let time_to_maturity = repo.day_count.year_fraction(
            context.as_of,
            repo.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        Ok(time_to_maturity)
    }
}

/// Calculate implied collateral return (mark-to-market gain/loss on collateral).
pub struct ImpliedCollateralReturnCalculator;

impl MetricCalculator for ImpliedCollateralReturnCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::CollateralValue, MetricId::RequiredCollateral]
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let repo = context.instrument_as::<super::types::Repo>()?;
        let collateral_value = context.computed.get(&MetricId::CollateralValue)
            .copied()
            .unwrap_or(0.0);
        let required_value = context.computed.get(&MetricId::RequiredCollateral)
            .copied()
            .unwrap_or(0.0);
        
        // Calculate time to maturity
        let time_to_maturity = repo.day_count.year_fraction(
            context.as_of,
            repo.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        
        if time_to_maturity <= 0.0 || required_value == 0.0 {
            return Ok(0.0);
        }
        
        // Implied return = (current_value / required_value - 1) / time_to_maturity
        let return_rate = (collateral_value / required_value - 1.0) / time_to_maturity;
        Ok(return_rate)
    }
}

/// Register all repo metrics with the registry.
pub fn register_repo_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::CollateralValue,
            Arc::new(CollateralValueCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::RequiredCollateral,
            Arc::new(RequiredCollateralCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::CollateralCoverage,
            Arc::new(CollateralCoverageCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::RepoInterest,
            Arc::new(RepoInterestCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::Dv01,
            Arc::new(RepoDv01Calculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::FundingRisk,
            Arc::new(FundingRiskCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::EffectiveRate,
            Arc::new(EffectiveRateCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::TimeToMaturity,
            Arc::new(TimeToMaturityCalculator),
            &["Repo"],
        )
        .register_metric(
            MetricId::ImpliedCollateralReturn,
            Arc::new(ImpliedCollateralReturnCalculator),
            &["Repo"],
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::repo::{CollateralSpec, Repo};
    use finstack_core::market_data::MarketContext;
    use finstack_core::market_data::primitives::MarketScalar;
    use finstack_core::currency::Currency;
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
            0.05, // 5% repo rate
            test_date(2025, 1, 15),
            test_date(2025, 4, 15), // 3-month term
            "USD-OIS",
        )
    }

    fn create_test_context() -> MarketContext {
        MarketContext::new()
            .insert_price("BOND_ABC_PRICE", MarketScalar::Unitless(1.02)) // Bond trading at 102% of face
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

        let calculator = CollateralValueCalculator;
        let value = calculator.calculate(&mut context).unwrap();
        
        // Expected: 1000 * 1.02 = 1020
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

        let calculator = RequiredCollateralCalculator;
        let value = calculator.calculate(&mut context).unwrap();
        
        // Expected: 1,000,000 * (1 + 0.02) = 1,020,000
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

        let calculator = EffectiveRateCalculator;
        let rate = calculator.calculate(&mut context).unwrap();
        
        // Should be base rate since it's general collateral
        assert!((rate - 0.05).abs() < 1e-9);
    }
}
