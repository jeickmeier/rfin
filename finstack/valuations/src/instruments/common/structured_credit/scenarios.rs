//! Scenario framework for structured credit stress testing.
//!
//! Provides standardized scenarios for:
//! - Prepayment stress (PSA/CPR speeds)
//! - Default stress (CDR/severity rates)
//! - Combined prepayment + default scenarios
//! - Market stress scenarios

use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use std::collections::HashMap;

use super::{DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Scenario definition for structured credit stress testing
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructuredCreditScenario {
    /// Scenario identifier
    pub id: String,
    /// Description
    pub description: String,
    /// Prepayment assumptions
    pub prepayment: Option<PrepaymentScenario>,
    /// Default assumptions
    pub default: Option<DefaultScenario>,
    /// Market conditions
    pub market: Option<MarketScenario>,
}

/// Prepayment scenario definition
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PrepaymentScenario {
    /// PSA speed multiplier
    PsaSpeed { speed: f64 },
    /// Constant CPR rate
    ConstantCpr { cpr_annual: f64 },
    /// ABS speed for auto loans
    AbsSpeed { abs_monthly: f64 },
    /// CPR vector by period
    CprVector { cpr_by_month: Vec<f64> },
    /// Percentile scenario (e.g., 10th, 50th, 90th percentile)
    Percentile { percentile: f64, base_speed: f64 },
}

/// Default scenario definition
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DefaultScenario {
    /// Constant CDR
    ConstantCdr { cdr_annual: f64 },
    /// SDA speed multiplier
    SdaSpeed { speed: f64 },
    /// CDR vector by period
    CdrVector { cdr_by_month: Vec<f64> },
    /// Severity override
    ConstantSeverity { severity: f64 },
    /// Combined CDR + Severity
    CdrWithSeverity { cdr_annual: f64, severity: f64 },
    /// Timing scenario (front-loaded, back-loaded defaults)
    Timing {
        total_cdr: f64,
        peak_month: u32,
        shape: DefaultTimingShape,
    },
}

/// Shape of default timing
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DefaultTimingShape {
    FrontLoaded,
    BackLoaded,
    Uniform,
    Peaked,
}

/// Market scenario affecting prepayment behavior
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MarketScenario {
    /// Refinancing rate shock (bps)
    pub refi_rate_shock_bps: Option<i32>,
    /// Home price appreciation shock
    pub hpa_shock: Option<f64>,
    /// Unemployment rate
    pub unemployment_rate: Option<f64>,
    /// Spread widening (bps)
    pub spread_shock_bps: Option<i32>,
}

impl StructuredCreditScenario {
    /// RMBS prepayment scenarios
    pub fn standard_rmbs_prepay() -> Vec<StructuredCreditScenario> {
        Self::psa_speed_ladder(vec![0.5, 1.0, 1.5, 2.0, 3.0])
    }

    /// RMBS default scenarios
    pub fn standard_rmbs_default() -> Vec<StructuredCreditScenario> {
        Self::cdr_ladder(vec![0.003, 0.006, 0.012, 0.024]) // SDA equivalents
    }

    /// CLO default scenarios
    pub fn standard_clo_default() -> Vec<StructuredCreditScenario> {
        vec![
            StructuredCreditScenario {
                id: "CLO_BASE".to_string(),
                description: "Base case - 2% CDR, 40% recovery".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.02,
                    severity: 0.60,
                }),
                market: None,
            },
            StructuredCreditScenario {
                id: "CLO_RECESSION".to_string(),
                description: "Recession - 5% CDR, 30% recovery".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.05,
                    severity: 0.70,
                }),
                market: None,
            },
            StructuredCreditScenario {
                id: "CLO_SEVERE_STRESS".to_string(),
                description: "Severe stress - 10% CDR, 25% recovery".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.10,
                    severity: 0.75,
                }),
                market: None,
            },
        ]
    }

    /// ABS scenarios for auto loans
    pub fn standard_abs_auto() -> Vec<StructuredCreditScenario> {
        vec![
            StructuredCreditScenario {
                id: "AUTO_BASE".to_string(),
                description: "Base case - 1.5% ABS, 2% CDR".to_string(),
                prepayment: Some(PrepaymentScenario::AbsSpeed { abs_monthly: 0.015 }),
                default: Some(DefaultScenario::ConstantCdr { cdr_annual: 0.02 }),
                market: None,
            },
            StructuredCreditScenario {
                id: "AUTO_SLOW".to_string(),
                description: "Slow prepayments - 1.0% ABS".to_string(),
                prepayment: Some(PrepaymentScenario::AbsSpeed { abs_monthly: 0.010 }),
                default: None,
                market: None,
            },
            StructuredCreditScenario {
                id: "AUTO_FAST".to_string(),
                description: "Fast prepayments - 2.5% ABS".to_string(),
                prepayment: Some(PrepaymentScenario::AbsSpeed { abs_monthly: 0.025 }),
                default: None,
                market: None,
            },
            StructuredCreditScenario {
                id: "AUTO_STRESS".to_string(),
                description: "Credit stress - 5% CDR, 50% severity".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.05,
                    severity: 0.50,
                }),
                market: None,
            },
        ]
    }

    /// Combined scenarios (prepayment + default)
    pub fn standard_combined_stress() -> Vec<StructuredCreditScenario> {
        vec![
            StructuredCreditScenario {
                id: "BEST_CASE".to_string(),
                description: "Best case - slow prepay, low defaults".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 0.5 }),
                default: Some(DefaultScenario::ConstantCdr { cdr_annual: 0.005 }),
                market: None,
            },
            StructuredCreditScenario {
                id: "WORST_CASE".to_string(),
                description: "Worst case - fast prepay, high defaults".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 3.0 }),
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.10,
                    severity: 0.75,
                }),
                market: None,
            },
            StructuredCreditScenario {
                id: "RECESSION".to_string(),
                description: "Recession - moderate prepay, elevated defaults".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 1.2 }),
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.06,
                    severity: 0.65,
                }),
                market: Some(MarketScenario {
                    refi_rate_shock_bps: Some(100),
                    hpa_shock: Some(-0.10), // -10% HPA
                    unemployment_rate: Some(0.08),
                    spread_shock_bps: Some(200),
                }),
            },
        ]
    }

    /// Get all standard scenarios
    pub fn all_standard_scenarios() -> HashMap<String, Vec<StructuredCreditScenario>> {
        [
            ("RMBS_PREPAY".to_string(), Self::standard_rmbs_prepay()),
            ("RMBS_DEFAULT".to_string(), Self::standard_rmbs_default()),
            ("CLO_DEFAULT".to_string(), Self::standard_clo_default()),
            ("ABS_AUTO".to_string(), Self::standard_abs_auto()),
            ("COMBINED".to_string(), Self::standard_combined_stress()),
        ]
        .into_iter()
        .collect()
    }

    /// Generate PSA speed ladder for RMBS
    pub fn psa_speed_ladder(speeds: Vec<f64>) -> Vec<StructuredCreditScenario> {
        speeds
            .into_iter()
            .map(|speed| StructuredCreditScenario {
                id: format!("PSA_{}", (speed * 100.0) as u32),
                description: format!("{}% PSA prepayment speed", (speed * 100.0) as u32),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed }),
                default: None,
                market: None,
            })
            .collect()
    }

    /// Generate CDR ladder for stress testing
    pub fn cdr_ladder(cdrs: Vec<f64>) -> Vec<StructuredCreditScenario> {
        cdrs.into_iter()
            .map(|cdr| StructuredCreditScenario {
                id: format!("CDR_{}", (cdr * 100.0) as u32),
                description: format!("{}% annual CDR", (cdr * 100.0) as u32),
                prepayment: None,
                default: Some(DefaultScenario::ConstantCdr { cdr_annual: cdr }),
                market: None,
            })
            .collect()
    }

    /// Default PSA speeds for scenario analysis
    pub fn default_psa_speeds() -> Vec<f64> {
        super::constants::STANDARD_PSA_SPEEDS.to_vec()
    }

    /// Default CDR rates for scenario analysis
    pub fn default_cdr_rates() -> Vec<f64> {
        super::constants::STANDARD_CDR_RATES.to_vec()
    }

    /// Default severity rates for scenario analysis
    pub fn default_severity_rates() -> Vec<f64> {
        super::constants::STANDARD_SEVERITY_RATES.to_vec()
    }

    /// Run scenario on a CLO instrument
    pub fn run_clo(
        &self,
        clo: &crate::instruments::clo::Clo,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ScenarioResult> {
        let mut clo_copy = clo.clone();
        Self::apply_scenario_to_clo(&mut clo_copy, self);

        use crate::instruments::common::traits::Instrument;
        let pv = clo_copy.value(market, as_of)?;
        // Use WAM as approximation for WAL since we don't have cashflows yet
        let wal = clo_copy.pool.weighted_avg_maturity(as_of);

        Self::build_scenario_result(&self.id, pv.amount(), wal, &clo_copy.pool)
    }

    /// Run scenario on an RMBS instrument
    pub fn run_rmbs(
        &self,
        rmbs: &crate::instruments::rmbs::Rmbs,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ScenarioResult> {
        let mut rmbs_copy = rmbs.clone();
        Self::apply_scenario_to_rmbs(&mut rmbs_copy, self);

        use crate::instruments::common::traits::Instrument;
        let pv = rmbs_copy.value(market, as_of)?;
        let wal = rmbs_copy.pool.weighted_avg_maturity(as_of);

        Self::build_scenario_result(&self.id, pv.amount(), wal, &rmbs_copy.pool)
    }

    /// Run scenario on an ABS instrument
    pub fn run_abs(
        &self,
        abs: &crate::instruments::abs::Abs,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ScenarioResult> {
        let mut abs_copy = abs.clone();
        Self::apply_scenario_to_abs(&mut abs_copy, self);

        use crate::instruments::common::traits::Instrument;
        let pv = abs_copy.value(market, as_of)?;
        let wal = abs_copy.pool.weighted_avg_maturity(as_of);

        Self::build_scenario_result(&self.id, pv.amount(), wal, &abs_copy.pool)
    }

    /// Helper to build scenario result (reduces duplication)
    fn build_scenario_result(
        scenario_id: &str,
        pv: f64,
        wal: f64,
        pool: &crate::instruments::common::structured_credit::AssetPool,
    ) -> Result<ScenarioResult> {
        Ok(ScenarioResult {
            scenario_id: scenario_id.to_string(),
            pv,
            wal,
            duration: None,
            total_defaults: pool.cumulative_defaults.amount(),
            total_prepayments: pool.cumulative_prepayments.amount(),
            total_recoveries: pool.cumulative_recoveries.amount(),
            net_loss: (pool.cumulative_defaults.amount() - pool.cumulative_recoveries.amount()),
            oc_ratios: HashMap::new(),
            ic_ratios: HashMap::new(),
            custom_metrics: HashMap::new(),
        })
    }

    /// Run multiple scenarios and generate comparison
    ///
    /// Generic method that works with CLO, RMBS, or ABS instruments.
    /// Users should call the instrument-specific run methods directly.
    pub fn run_comparison<F>(
        scenarios: &[StructuredCreditScenario],
        base_pv: f64,
        base_wal: f64,
        base_pool: &crate::instruments::common::structured_credit::AssetPool,
        run_scenario: F,
    ) -> Result<ScenarioComparison>
    where
        F: Fn(&StructuredCreditScenario) -> Result<ScenarioResult>,
    {
        let base_case = Self::build_scenario_result("BASE", base_pv, base_wal, base_pool)?;

        let scenario_results: Result<Vec<_>> = scenarios.iter().map(run_scenario).collect();

        Ok(ScenarioComparison {
            base_case,
            scenarios: scenario_results?,
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
            clo.prepayment_spec = PrepaymentModelSpec::ConstantCpr { cpr: cpr_annual };
        }

        // Apply default scenario
        if let Some(ref default_scenario) = scenario.default {
            match default_scenario {
                DefaultScenario::ConstantCdr { cdr_annual } => {
                    clo.default_spec = DefaultModelSpec::ConstantCdr { cdr: *cdr_annual };
                }
                DefaultScenario::CdrWithSeverity {
                    cdr_annual,
                    severity,
                } => {
                    clo.default_spec = DefaultModelSpec::ConstantCdr { cdr: *cdr_annual };
                    let recovery = 1.0 - severity;
                    clo.recovery_spec = RecoveryModelSpec::Constant { rate: recovery };
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
                    rmbs.prepayment_spec = PrepaymentModelSpec::Psa { multiplier: *speed };
                }
                PrepaymentScenario::ConstantCpr { cpr_annual } => {
                    rmbs.prepayment_spec = PrepaymentModelSpec::ConstantCpr { cpr: *cpr_annual };
                }
                _ => {}
            }
        }

        // Apply default scenario
        if let Some(ref default_scenario) = scenario.default {
            match default_scenario {
                DefaultScenario::SdaSpeed { speed } => {
                    rmbs.sda_speed = *speed;
                    rmbs.default_spec = DefaultModelSpec::Sda { multiplier: *speed };
                }
                DefaultScenario::ConstantCdr { cdr_annual } => {
                    rmbs.default_spec = DefaultModelSpec::ConstantCdr { cdr: *cdr_annual };
                }
                DefaultScenario::CdrWithSeverity {
                    cdr_annual,
                    severity,
                } => {
                    rmbs.default_spec = DefaultModelSpec::ConstantCdr { cdr: *cdr_annual };
                    let recovery = 1.0 - severity;
                    rmbs.recovery_spec = RecoveryModelSpec::Constant { rate: recovery };
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
                    abs.cdr_annual = Some(*cdr_annual);
                    abs.default_spec = DefaultModelSpec::ConstantCdr { cdr: *cdr_annual };
                    let recovery = 1.0 - severity;
                    abs.recovery_spec = RecoveryModelSpec::Constant { rate: recovery };
                }
                _ => {}
            }
        }
    }
}

/// Results from running a scenario
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScenarioResult {
    /// Scenario ID
    pub scenario_id: String,
    /// Present value under scenario
    pub pv: f64,
    /// Weighted average life under scenario
    pub wal: f64,
    /// Duration under scenario
    pub duration: Option<f64>,
    /// Total defaults
    pub total_defaults: f64,
    /// Total prepayments
    pub total_prepayments: f64,
    /// Total recoveries
    pub total_recoveries: f64,
    /// Net loss
    pub net_loss: f64,
    /// OC ratios by tranche (if applicable)
    pub oc_ratios: HashMap<String, f64>,
    /// IC ratios by tranche (if applicable)
    pub ic_ratios: HashMap<String, f64>,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Scenario comparison results
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScenarioComparison {
    /// Base scenario result
    pub base_case: ScenarioResult,
    /// Alternative scenario results
    pub scenarios: Vec<ScenarioResult>,
    /// Sensitivity measures
    pub sensitivities: HashMap<String, f64>,
}

impl ScenarioComparison {
    /// Calculate PV sensitivity to each scenario
    pub fn pv_sensitivity(&self) -> Vec<(String, f64)> {
        let base_pv = self.base_case.pv;
        self.scenarios
            .iter()
            .map(|s| {
                let pct_change = if base_pv != 0.0 {
                    (s.pv - base_pv) / base_pv * 100.0
                } else {
                    0.0
                };
                (s.scenario_id.clone(), pct_change)
            })
            .collect()
    }

    /// Calculate WAL sensitivity to each scenario
    pub fn wal_sensitivity(&self) -> Vec<(String, f64)> {
        let base_wal = self.base_case.wal;
        self.scenarios
            .iter()
            .map(|s| {
                let change = s.wal - base_wal;
                (s.scenario_id.clone(), change)
            })
            .collect()
    }

    /// Find worst case scenario by PV
    pub fn worst_case_pv(&self) -> Option<&ScenarioResult> {
        self.scenarios
            .iter()
            .min_by(|a, b| a.pv.partial_cmp(&b.pv).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Find best case scenario by PV
    pub fn best_case_pv(&self) -> Option<&ScenarioResult> {
        self.scenarios
            .iter()
            .max_by(|a, b| a.pv.partial_cmp(&b.pv).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_scenarios() {
        let rmbs_scenarios = StructuredCreditScenario::standard_rmbs_prepay();
        assert_eq!(rmbs_scenarios.len(), 5);

        let clo_scenarios = StructuredCreditScenario::standard_clo_default();
        assert_eq!(clo_scenarios.len(), 3);
    }

    #[test]
    fn test_scenario_ladders() {
        let psa_scenarios = StructuredCreditScenario::psa_speed_ladder(vec![0.5, 1.0, 1.5, 2.0]);
        assert_eq!(psa_scenarios.len(), 4);

        let cdr_scenarios = StructuredCreditScenario::cdr_ladder(vec![0.01, 0.02, 0.05]);
        assert_eq!(cdr_scenarios.len(), 3);
    }

    #[test]
    fn test_combined_scenario() {
        let scenario = StructuredCreditScenario {
            id: "STRESS".to_string(),
            description: "Combined stress".to_string(),
            prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 3.0 }),
            default: Some(DefaultScenario::CdrWithSeverity {
                cdr_annual: 0.10,
                severity: 0.75,
            }),
            market: Some(MarketScenario {
                refi_rate_shock_bps: Some(-200),
                hpa_shock: Some(-0.20),
                unemployment_rate: Some(0.10),
                spread_shock_bps: Some(300),
            }),
        };

        assert!(scenario.prepayment.is_some());
        assert!(scenario.default.is_some());
        assert!(scenario.market.is_some());
    }

    #[test]
    fn test_default_rates() {
        let psa_speeds = StructuredCreditScenario::default_psa_speeds();
        assert_eq!(psa_speeds.len(), 9);

        let cdr_rates = StructuredCreditScenario::default_cdr_rates();
        assert!(cdr_rates.len() > 5);
    }
}
