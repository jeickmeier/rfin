//! Scenario runner for structured credit stress testing.
//!
//! Applies scenarios to instruments and generates comparison results.

use super::scenarios::{
    DefaultScenario, PrepaymentScenario, ScenarioComparison, ScenarioResult,
    StructuredCreditScenario,
};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

/// Scenario runner for structured credit instruments
pub struct ScenarioRunner;

impl ScenarioRunner {
    /// Run scenarios on a CLO instrument
    pub fn run_clo_scenarios(
        clo: &crate::instruments::clo::Clo,
        scenarios: &[StructuredCreditScenario],
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ScenarioComparison> {
        // Run base case (current assumptions)
        let base_pv = Self::calculate_clo_pv(clo, market, as_of)?;
        let base_wal = clo.pool.weighted_avg_life(as_of);

        let base_case = ScenarioResult {
            scenario_id: "BASE".to_string(),
            pv: base_pv.amount(),
            wal: base_wal,
            duration: None,
            total_defaults: clo.pool.cumulative_defaults.amount(),
            total_prepayments: clo.pool.cumulative_prepayments.amount(),
            total_recoveries: clo.pool.cumulative_recoveries.amount(),
            net_loss: (clo.pool.cumulative_defaults.amount()
                - clo.pool.cumulative_recoveries.amount()),
            oc_ratios: HashMap::new(),
            ic_ratios: HashMap::new(),
            custom_metrics: HashMap::new(),
        };

        // Run each scenario
        let mut scenario_results = Vec::new();
        for scenario in scenarios {
            let mut clo_copy = clo.clone();

            // Apply scenario to CLO
            Self::apply_scenario_to_clo(&mut clo_copy, scenario);

            // Recalculate
            let pv = Self::calculate_clo_pv(&clo_copy, market, as_of)?;
            let wal = clo_copy.pool.weighted_avg_life(as_of);

            scenario_results.push(ScenarioResult {
                scenario_id: scenario.id.clone(),
                pv: pv.amount(),
                wal,
                duration: None,
                total_defaults: 0.0,
                total_prepayments: 0.0,
                total_recoveries: 0.0,
                net_loss: 0.0,
                oc_ratios: HashMap::new(),
                ic_ratios: HashMap::new(),
                custom_metrics: HashMap::new(),
            });
        }

        Ok(ScenarioComparison {
            base_case,
            scenarios: scenario_results,
            sensitivities: HashMap::new(),
        })
    }

    /// Run scenarios on RMBS instrument
    pub fn run_rmbs_scenarios(
        rmbs: &crate::instruments::rmbs::Rmbs,
        scenarios: &[StructuredCreditScenario],
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ScenarioComparison> {
        let base_pv = Self::calculate_rmbs_pv(rmbs, market, as_of)?;
        let base_wal = rmbs.pool.weighted_avg_life(as_of);

        let base_case = ScenarioResult {
            scenario_id: "BASE".to_string(),
            pv: base_pv.amount(),
            wal: base_wal,
            duration: None,
            total_defaults: rmbs.pool.cumulative_defaults.amount(),
            total_prepayments: rmbs.pool.cumulative_prepayments.amount(),
            total_recoveries: rmbs.pool.cumulative_recoveries.amount(),
            net_loss: (rmbs.pool.cumulative_defaults.amount()
                - rmbs.pool.cumulative_recoveries.amount()),
            oc_ratios: HashMap::new(),
            ic_ratios: HashMap::new(),
            custom_metrics: HashMap::new(),
        };

        let mut scenario_results = Vec::new();
        for scenario in scenarios {
            let mut rmbs_copy = rmbs.clone();

            Self::apply_scenario_to_rmbs(&mut rmbs_copy, scenario);

            let pv = Self::calculate_rmbs_pv(&rmbs_copy, market, as_of)?;
            let wal = rmbs_copy.pool.weighted_avg_life(as_of);

            scenario_results.push(ScenarioResult {
                scenario_id: scenario.id.clone(),
                pv: pv.amount(),
                wal,
                duration: None,
                total_defaults: 0.0,
                total_prepayments: 0.0,
                total_recoveries: 0.0,
                net_loss: 0.0,
                oc_ratios: HashMap::new(),
                ic_ratios: HashMap::new(),
                custom_metrics: HashMap::new(),
            });
        }

        Ok(ScenarioComparison {
            base_case,
            scenarios: scenario_results,
            sensitivities: HashMap::new(),
        })
    }

    /// Run scenarios on ABS instrument
    pub fn run_abs_scenarios(
        abs: &crate::instruments::abs::Abs,
        scenarios: &[StructuredCreditScenario],
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ScenarioComparison> {
        let base_pv = Self::calculate_abs_pv(abs, market, as_of)?;
        let base_wal = abs.pool.weighted_avg_life(as_of);

        let base_case = ScenarioResult {
            scenario_id: "BASE".to_string(),
            pv: base_pv.amount(),
            wal: base_wal,
            duration: None,
            total_defaults: abs.pool.cumulative_defaults.amount(),
            total_prepayments: abs.pool.cumulative_prepayments.amount(),
            total_recoveries: abs.pool.cumulative_recoveries.amount(),
            net_loss: (abs.pool.cumulative_defaults.amount()
                - abs.pool.cumulative_recoveries.amount()),
            oc_ratios: HashMap::new(),
            ic_ratios: HashMap::new(),
            custom_metrics: HashMap::new(),
        };

        let mut scenario_results = Vec::new();
        for scenario in scenarios {
            let mut abs_copy = abs.clone();

            Self::apply_scenario_to_abs(&mut abs_copy, scenario);

            let pv = Self::calculate_abs_pv(&abs_copy, market, as_of)?;
            let wal = abs_copy.pool.weighted_avg_life(as_of);

            scenario_results.push(ScenarioResult {
                scenario_id: scenario.id.clone(),
                pv: pv.amount(),
                wal,
                duration: None,
                total_defaults: 0.0,
                total_prepayments: 0.0,
                total_recoveries: 0.0,
                net_loss: 0.0,
                oc_ratios: HashMap::new(),
                ic_ratios: HashMap::new(),
                custom_metrics: HashMap::new(),
            });
        }

        Ok(ScenarioComparison {
            base_case,
            scenarios: scenario_results,
            sensitivities: HashMap::new(),
        })
    }

    // Helper methods to apply scenarios

    fn apply_scenario_to_clo(
        clo: &mut crate::instruments::clo::Clo,
        scenario: &StructuredCreditScenario,
    ) {
        // Apply prepayment scenario
        if let Some(PrepaymentScenario::ConstantCpr { cpr_annual }) = scenario.prepayment {
            use crate::instruments::common::structured_credit::PrepaymentModelFactory;
            clo.prepayment_model =
                std::sync::Arc::from(PrepaymentModelFactory::create_cpr(cpr_annual));
        }

        // Apply default scenario
        if let Some(ref default_scenario) = scenario.default {
            match default_scenario {
                DefaultScenario::ConstantCdr { cdr_annual } => {
                    use crate::instruments::common::structured_credit::{
                        CDRModel, DefaultBehavior,
                    };
                    clo.default_model = std::sync::Arc::from(
                        Box::new(CDRModel::new(*cdr_annual)) as Box<dyn DefaultBehavior>
                    );
                }
                DefaultScenario::CdrWithSeverity {
                    cdr_annual,
                    severity,
                } => {
                    use crate::instruments::common::structured_credit::{
                        CDRModel, ConstantRecoveryModel, DefaultBehavior, RecoveryBehavior,
                    };
                    clo.default_model = std::sync::Arc::from(
                        Box::new(CDRModel::new(*cdr_annual)) as Box<dyn DefaultBehavior>
                    );
                    let recovery = 1.0 - severity;
                    clo.recovery_model =
                        std::sync::Arc::from(Box::new(ConstantRecoveryModel::new(recovery))
                            as Box<dyn RecoveryBehavior>);
                }
                _ => {}
            }
        }
    }

    fn apply_scenario_to_rmbs(
        rmbs: &mut crate::instruments::rmbs::Rmbs,
        scenario: &StructuredCreditScenario,
    ) {
        // Apply prepayment scenario
        if let Some(ref prepay_scenario) = scenario.prepayment {
            match prepay_scenario {
                PrepaymentScenario::PsaSpeed { speed } => {
                    rmbs.psa_speed = *speed;
                }
                PrepaymentScenario::ConstantCpr { cpr_annual } => {
                    use crate::instruments::common::structured_credit::PrepaymentModelFactory;
                    rmbs.prepayment_model =
                        std::sync::Arc::from(PrepaymentModelFactory::create_cpr(*cpr_annual));
                }
                _ => {}
            }
        }

        // Apply default scenario
        if let Some(ref default_scenario) = scenario.default {
            match default_scenario {
                DefaultScenario::SdaSpeed { speed } => {
                    rmbs.sda_speed = *speed;
                }
                DefaultScenario::ConstantCdr { cdr_annual } => {
                    use crate::instruments::common::structured_credit::{
                        CDRModel, DefaultBehavior,
                    };
                    rmbs.default_model = std::sync::Arc::from(
                        Box::new(CDRModel::new(*cdr_annual)) as Box<dyn DefaultBehavior>
                    );
                }
                DefaultScenario::CdrWithSeverity {
                    cdr_annual,
                    severity,
                } => {
                    use crate::instruments::common::structured_credit::{
                        CDRModel, ConstantRecoveryModel, DefaultBehavior, RecoveryBehavior,
                    };
                    rmbs.default_model = std::sync::Arc::from(
                        Box::new(CDRModel::new(*cdr_annual)) as Box<dyn DefaultBehavior>
                    );
                    let recovery = 1.0 - severity;
                    rmbs.recovery_model =
                        std::sync::Arc::from(Box::new(ConstantRecoveryModel::new(recovery))
                            as Box<dyn RecoveryBehavior>);
                }
                _ => {}
            }
        }

        // Apply market scenario
        if let Some(ref market_scenario) = scenario.market {
            if let Some(refi_shock) = market_scenario.refi_rate_shock_bps {
                rmbs.market_conditions.refi_rate += (refi_shock as f64) / 10000.0;
            }
            if let Some(hpa) = market_scenario.hpa_shock {
                rmbs.market_conditions.hpa = Some(hpa);
            }
            if let Some(unemployment) = market_scenario.unemployment_rate {
                rmbs.credit_factors.unemployment_rate = Some(unemployment);
            }
        }
    }

    fn apply_scenario_to_abs(
        abs: &mut crate::instruments::abs::Abs,
        scenario: &StructuredCreditScenario,
    ) {
        // Apply prepayment scenario
        if let Some(ref prepay_scenario) = scenario.prepayment {
            match prepay_scenario {
                PrepaymentScenario::AbsSpeed { abs_monthly } => {
                    abs.abs_speed = Some(*abs_monthly);
                }
                PrepaymentScenario::ConstantCpr { cpr_annual } => {
                    use crate::instruments::common::structured_credit::cpr_to_smm;
                    abs.abs_speed = Some(cpr_to_smm(*cpr_annual));
                }
                _ => {}
            }
        }

        // Apply default scenario
        if let Some(ref default_scenario) = scenario.default {
            match default_scenario {
                DefaultScenario::ConstantCdr { cdr_annual } => {
                    abs.cdr_annual = Some(*cdr_annual);
                }
                DefaultScenario::CdrWithSeverity {
                    cdr_annual,
                    severity,
                } => {
                    use crate::instruments::common::structured_credit::{
                        ConstantRecoveryModel, RecoveryBehavior,
                    };
                    abs.cdr_annual = Some(*cdr_annual);
                    let recovery = 1.0 - severity;
                    abs.recovery_model =
                        std::sync::Arc::from(Box::new(ConstantRecoveryModel::new(recovery))
                            as Box<dyn RecoveryBehavior>);
                }
                _ => {}
            }
        }
    }

    // Helper PV calculation methods

    fn calculate_clo_pv(
        clo: &crate::instruments::clo::Clo,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Money> {
        use crate::instruments::common::traits::Instrument;
        clo.value(market, as_of)
    }

    fn calculate_rmbs_pv(
        rmbs: &crate::instruments::rmbs::Rmbs,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Money> {
        use crate::instruments::common::traits::Instrument;
        rmbs.value(market, as_of)
    }

    fn calculate_abs_pv(
        abs: &crate::instruments::abs::Abs,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<Money> {
        use crate::instruments::common::traits::Instrument;
        abs.value(market, as_of)
    }

    /// Generate PSA speed ladder for RMBS
    pub fn psa_speed_ladder() -> Vec<f64> {
        vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0]
    }

    /// Generate CDR ladder for stress testing
    pub fn cdr_ladder() -> Vec<f64> {
        vec![0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.10, 0.15, 0.20]
    }

    /// Generate severity ladder
    pub fn severity_ladder() -> Vec<f64> {
        vec![0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psa_ladder() {
        let ladder = ScenarioRunner::psa_speed_ladder();
        assert_eq!(ladder.len(), 9);
        assert_eq!(ladder[0], 0.25);
        assert_eq!(ladder[8], 3.0);
    }

    #[test]
    fn test_cdr_ladder() {
        let ladder = ScenarioRunner::cdr_ladder();
        assert!(ladder.len() > 5);
        assert!(ladder.iter().all(|&cdr| (0.0..=1.0).contains(&cdr)));
    }
}
