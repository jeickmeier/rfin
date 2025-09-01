//! Loan-specific metric calculators.
//!
//! Provides comprehensive metrics for loan facilities including expected exposure,
//! utilization metrics, and detailed PV breakdowns from the forward simulation model.

use super::ddtl::DelayedDrawTermLoan;
use super::revolver::RevolvingCreditFacility;
use super::simulation::{LoanFacility, LoanSimulator, SimulationConfig};
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Expected Exposure calculator for loan facilities.
///
/// Calculates the expected drawn balance at a specific future date (1 year forward by default).
/// This metric is essential for credit risk management and regulatory capital calculations.
pub struct ExpectedExposureCalculator {
    /// Time horizon in years for the exposure calculation
    pub horizon_years: F,
}

impl ExpectedExposureCalculator {
    /// Create calculator for 1-year expected exposure
    pub fn one_year() -> Self {
        Self { horizon_years: 1.0 }
    }

    /// Create calculator for custom horizon
    pub fn with_horizon(horizon_years: F) -> Self {
        Self { horizon_years }
    }
}

impl MetricCalculator for ExpectedExposureCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Try to downcast to known loan facility types
        if let Ok(ddtl) = context.instrument_as::<DelayedDrawTermLoan>() {
            self.calculate_for_facility(ddtl, context)
        } else if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            self.calculate_for_facility(revolver, context)
        } else {
            // Unknown loan type
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[] // No dependencies - calculated directly from simulation
    }
}

impl ExpectedExposureCalculator {
    fn calculate_for_facility<T: LoanFacility>(
        &self,
        facility: &T,
        context: &MetricContext,
    ) -> Result<F> {
        let simulator = LoanSimulator::new();
        let result = simulator.simulate(facility, &context.curves, context.as_of)?;

        // Find the expected exposure at the specified horizon
        let target_date =
            context.as_of + time::Duration::days((self.horizon_years * 365.25) as i64);

        // Linear interpolation if target date falls between simulation points
        for window in result.expected_exposure.windows(2) {
            let (date1, ee1) = window[0];
            let (date2, ee2) = window[1];

            if target_date >= date1 && target_date <= date2 {
                if date1 == date2 {
                    return Ok(ee1);
                }

                let total_days = (date2 - date1).whole_days() as F;
                let elapsed_days = (target_date - date1).whole_days() as F;
                let weight = elapsed_days / total_days;

                return Ok(ee1 + weight * (ee2 - ee1));
            }
        }

        // If target date is beyond the simulation, return the last value
        if let Some((_, ee)) = result.expected_exposure.last() {
            Ok(*ee)
        } else {
            Ok(facility.drawn_amount().amount())
        }
    }
}

/// Current utilization percentage calculator
pub struct UtilizationCalculator;

impl MetricCalculator for UtilizationCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Ok(ddtl) = context.instrument_as::<DelayedDrawTermLoan>() {
            let drawn = ddtl.drawn_amount.amount();
            let commitment = ddtl.commitment.amount();
            Ok(if commitment > 0.0 {
                drawn / commitment
            } else {
                0.0
            })
        } else if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            Ok(revolver.utilization())
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Undrawn commitment amount calculator
pub struct UndrawnAmountCalculator;

impl MetricCalculator for UndrawnAmountCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Ok(ddtl) = context.instrument_as::<DelayedDrawTermLoan>() {
            Ok(ddtl.undrawn_amount().amount())
        } else if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            Ok(revolver.undrawn_amount().amount())
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Commitment fee PV calculator
pub struct CommitmentFeePvCalculator;

impl MetricCalculator for CommitmentFeePvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Ok(ddtl) = context.instrument_as::<DelayedDrawTermLoan>() {
            self.calculate_for_facility(ddtl, context)
        } else if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            self.calculate_for_facility(revolver, context)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

impl CommitmentFeePvCalculator {
    fn calculate_for_facility<T: LoanFacility>(
        &self,
        facility: &T,
        context: &MetricContext,
    ) -> Result<F> {
        let simulator = LoanSimulator::new();
        let result = simulator.simulate(facility, &context.curves, context.as_of)?;
        Ok(result.pv_breakdown.commitment_fees)
    }
}

/// Utilization fee PV calculator (revolvers only)
pub struct UtilizationFeePvCalculator;

impl MetricCalculator for UtilizationFeePvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            if revolver.utilization_fees.is_some() {
                let simulator = LoanSimulator::new();
                let result = simulator.simulate(revolver, &context.curves, context.as_of)?;
                Ok(result.pv_breakdown.utilization_fees)
            } else {
                Ok(0.0)
            }
        } else {
            // Not applicable for non-revolvers
            Ok(0.0)
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Enhanced Expected Exposure calculator with Monte Carlo for utilization tiers
pub struct ExpectedExposureMCCalculator {
    /// Time horizon in years
    pub horizon_years: F,
    /// Number of Monte Carlo paths
    pub mc_paths: usize,
}

impl ExpectedExposureMCCalculator {
    /// Create calculator with Monte Carlo enhancement
    pub fn new(horizon_years: F, mc_paths: usize) -> Self {
        Self {
            horizon_years,
            mc_paths,
        }
    }
}

impl MetricCalculator for ExpectedExposureMCCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        // Use Monte Carlo simulation for enhanced accuracy
        let config = SimulationConfig {
            monte_carlo_paths: self.mc_paths,
            random_seed: Some(42), // Deterministic for testing
            use_mid_point_averaging: true,
            rate_simulation: super::simulation::RateSimulationConfig::Deterministic,
            credit_config: None,
        };

        let simulator = LoanSimulator::with_config(config);

        if let Ok(ddtl) = context.instrument_as::<DelayedDrawTermLoan>() {
            let result = simulator.simulate(ddtl, &context.curves, context.as_of)?;
            self.extract_ee_at_horizon(&result.expected_exposure, context.as_of)
        } else if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            let result = simulator.simulate(revolver, &context.curves, context.as_of)?;
            self.extract_ee_at_horizon(&result.expected_exposure, context.as_of)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

impl ExpectedExposureMCCalculator {
    fn extract_ee_at_horizon(
        &self,
        exposure_path: &[(finstack_core::dates::Date, F)],
        as_of: finstack_core::dates::Date,
    ) -> Result<F> {
        let target_date = as_of + time::Duration::days((self.horizon_years * 365.25) as i64);

        // Find exposure at target date using linear interpolation
        for window in exposure_path.windows(2) {
            let (date1, ee1) = window[0];
            let (date2, ee2) = window[1];

            if target_date >= date1 && target_date <= date2 {
                if date1 == date2 {
                    return Ok(ee1);
                }

                let total_days = (date2 - date1).whole_days() as F;
                let elapsed_days = (target_date - date1).whole_days() as F;
                let weight = elapsed_days / total_days;

                return Ok(ee1 + weight * (ee2 - ee1));
            }
        }

        // Return last value if beyond simulation
        if let Some((_, ee)) = exposure_path.last() {
            Ok(*ee)
        } else {
            Ok(0.0)
        }
    }
}

/// PV breakdown calculator - returns incremental interest PV
pub struct IncrementalInterestPvCalculator;

impl MetricCalculator for IncrementalInterestPvCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        if let Ok(ddtl) = context.instrument_as::<DelayedDrawTermLoan>() {
            let simulator = LoanSimulator::new();
            let result = simulator.simulate(ddtl, &context.curves, context.as_of)?;
            Ok(result.pv_breakdown.incremental_interest)
        } else if let Ok(revolver) = context.instrument_as::<RevolvingCreditFacility>() {
            let simulator = LoanSimulator::new();
            let result = simulator.simulate(revolver, &context.curves, context.as_of)?;
            Ok(result.pv_breakdown.incremental_interest)
        } else {
            Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid,
            ))
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register all loan metrics with the registry
pub fn register_loan_metrics(registry: &mut MetricRegistry) {
    // Standard expected exposure (1 year)
    registry.register_metric(
        MetricId::custom("expected_exposure_1y"),
        Arc::new(ExpectedExposureCalculator::one_year()),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );

    // Current utilization
    registry.register_metric(
        MetricId::custom("utilization"),
        Arc::new(UtilizationCalculator),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );

    // Undrawn amount
    registry.register_metric(
        MetricId::custom("undrawn_amount"),
        Arc::new(UndrawnAmountCalculator),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );

    // Commitment fee PV
    registry.register_metric(
        MetricId::custom("commitment_fee_pv"),
        Arc::new(CommitmentFeePvCalculator),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );

    // Utilization fee PV (revolvers only)
    registry.register_metric(
        MetricId::custom("utilization_fee_pv"),
        Arc::new(UtilizationFeePvCalculator),
        &["RevolvingCreditFacility"],
    );

    // Incremental interest PV
    registry.register_metric(
        MetricId::custom("incremental_interest_pv"),
        Arc::new(IncrementalInterestPvCalculator),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );

    // Enhanced expected exposure with Monte Carlo (3-month horizon, 1000 paths)
    registry.register_metric(
        MetricId::custom("expected_exposure_mc_3m"),
        Arc::new(ExpectedExposureMCCalculator::new(0.25, 1000)),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );

    // Enhanced expected exposure with Monte Carlo (1-year horizon, 1000 paths)
    registry.register_metric(
        MetricId::custom("expected_exposure_mc_1y"),
        Arc::new(ExpectedExposureMCCalculator::new(1.0, 1000)),
        &["DelayedDrawTermLoan", "RevolvingCreditFacility"],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn test_expected_exposure_calculator_creation() {
        let calc = ExpectedExposureCalculator::one_year();
        assert_eq!(calc.horizon_years, 1.0);

        let calc_custom = ExpectedExposureCalculator::with_horizon(0.5);
        assert_eq!(calc_custom.horizon_years, 0.5);
    }

    #[test]
    fn test_utilization_calculator() {
        // Test the utilization calculation logic directly
        let revolver = RevolvingCreditFacility::new(
            "TEST_RCF",
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2028, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        );

        // Test direct utilization calculation
        assert_eq!(revolver.utilization(), 0.0); // No draws yet

        // Test with draws
        let mut revolver_with_draw = revolver;
        revolver_with_draw.drawn_amount = Money::new(250_000.0, Currency::USD);
        assert_eq!(revolver_with_draw.utilization(), 0.25); // 25% utilization
    }

    #[test]
    fn test_mc_calculator_creation() {
        let calc = ExpectedExposureMCCalculator::new(1.0, 5000);
        assert_eq!(calc.horizon_years, 1.0);
        assert_eq!(calc.mc_paths, 5000);
    }
}
