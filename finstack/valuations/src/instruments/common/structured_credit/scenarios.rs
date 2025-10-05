//! Scenario framework for structured credit stress testing.
//!
//! Provides standardized scenarios for:
//! - Prepayment stress (PSA/CPR speeds)
//! - Default stress (CDR/severity rates)
//! - Combined prepayment + default scenarios
//! - Market stress scenarios

use std::collections::HashMap;

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

/// Standard scenario library for structured credit
pub struct ScenarioLibrary;

impl ScenarioLibrary {
    /// RMBS prepayment scenarios
    pub fn rmbs_prepayment_scenarios() -> Vec<StructuredCreditScenario> {
        vec![
            StructuredCreditScenario {
                id: "RMBS_50PSA".to_string(),
                description: "Slow prepayments - 50% PSA".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 0.5 }),
                default: None,
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_100PSA".to_string(),
                description: "Base case - 100% PSA".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 1.0 }),
                default: None,
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_150PSA".to_string(),
                description: "Moderate prepayments - 150% PSA".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 1.5 }),
                default: None,
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_200PSA".to_string(),
                description: "Fast prepayments - 200% PSA".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 2.0 }),
                default: None,
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_300PSA".to_string(),
                description: "Very fast prepayments - 300% PSA".to_string(),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed: 3.0 }),
                default: None,
                market: None,
            },
        ]
    }

    /// RMBS default scenarios
    pub fn rmbs_default_scenarios() -> Vec<StructuredCreditScenario> {
        vec![
            StructuredCreditScenario {
                id: "RMBS_50SDA".to_string(),
                description: "Low defaults - 50% SDA".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::SdaSpeed { speed: 0.5 }),
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_100SDA".to_string(),
                description: "Base defaults - 100% SDA".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::SdaSpeed { speed: 1.0 }),
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_200SDA".to_string(),
                description: "Elevated defaults - 200% SDA".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::SdaSpeed { speed: 2.0 }),
                market: None,
            },
            StructuredCreditScenario {
                id: "RMBS_STRESS".to_string(),
                description: "Severe stress - 400% SDA, 60% severity".to_string(),
                prepayment: None,
                default: Some(DefaultScenario::CdrWithSeverity {
                    cdr_annual: 0.024, // 400% SDA ≈ 2.4% CDR
                    severity: 0.60,
                }),
                market: None,
            },
        ]
    }

    /// CLO default scenarios
    pub fn clo_default_scenarios() -> Vec<StructuredCreditScenario> {
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
    pub fn abs_auto_scenarios() -> Vec<StructuredCreditScenario> {
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
    pub fn combined_stress_scenarios() -> Vec<StructuredCreditScenario> {
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
    pub fn all_scenarios() -> HashMap<String, Vec<StructuredCreditScenario>> {
        let mut scenarios = HashMap::new();
        scenarios.insert("RMBS_PREPAY".to_string(), Self::rmbs_prepayment_scenarios());
        scenarios.insert("RMBS_DEFAULT".to_string(), Self::rmbs_default_scenarios());
        scenarios.insert("CLO_DEFAULT".to_string(), Self::clo_default_scenarios());
        scenarios.insert("ABS_AUTO".to_string(), Self::abs_auto_scenarios());
        scenarios.insert("COMBINED".to_string(), Self::combined_stress_scenarios());
        scenarios
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

/// Builder for creating scenario sets
pub struct ScenarioBuilder {
    scenarios: Vec<StructuredCreditScenario>,
}

impl ScenarioBuilder {
    pub fn new() -> Self {
        Self {
            scenarios: Vec::new(),
        }
    }

    /// Add a prepayment scenario
    pub fn add_prepayment_scenario(
        mut self,
        id: impl Into<String>,
        description: impl Into<String>,
        scenario: PrepaymentScenario,
    ) -> Self {
        self.scenarios.push(StructuredCreditScenario {
            id: id.into(),
            description: description.into(),
            prepayment: Some(scenario),
            default: None,
            market: None,
        });
        self
    }

    /// Add a default scenario
    pub fn add_default_scenario(
        mut self,
        id: impl Into<String>,
        description: impl Into<String>,
        scenario: DefaultScenario,
    ) -> Self {
        self.scenarios.push(StructuredCreditScenario {
            id: id.into(),
            description: description.into(),
            prepayment: None,
            default: Some(scenario),
            market: None,
        });
        self
    }

    /// Add a combined scenario
    pub fn add_combined_scenario(
        mut self,
        id: impl Into<String>,
        description: impl Into<String>,
        prepayment: PrepaymentScenario,
        default: DefaultScenario,
    ) -> Self {
        self.scenarios.push(StructuredCreditScenario {
            id: id.into(),
            description: description.into(),
            prepayment: Some(prepayment),
            default: Some(default),
            market: None,
        });
        self
    }

    /// Add PSA speed ladder
    pub fn add_psa_ladder(mut self, speeds: Vec<f64>) -> Self {
        for speed in speeds {
            self.scenarios.push(StructuredCreditScenario {
                id: format!("PSA_{}", (speed * 100.0) as u32),
                description: format!("{}% PSA prepayment speed", (speed * 100.0) as u32),
                prepayment: Some(PrepaymentScenario::PsaSpeed { speed }),
                default: None,
                market: None,
            });
        }
        self
    }

    /// Add CDR ladder
    pub fn add_cdr_ladder(mut self, cdrs: Vec<f64>) -> Self {
        for cdr in cdrs {
            self.scenarios.push(StructuredCreditScenario {
                id: format!("CDR_{}", (cdr * 100.0) as u32),
                description: format!("{}% annual CDR", (cdr * 100.0) as u32),
                prepayment: None,
                default: Some(DefaultScenario::ConstantCdr { cdr_annual: cdr }),
                market: None,
            });
        }
        self
    }

    /// Build the scenario set
    pub fn build(self) -> Vec<StructuredCreditScenario> {
        self.scenarios
    }
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_library() {
        let rmbs_scenarios = ScenarioLibrary::rmbs_prepayment_scenarios();
        assert_eq!(rmbs_scenarios.len(), 5);

        let clo_scenarios = ScenarioLibrary::clo_default_scenarios();
        assert_eq!(clo_scenarios.len(), 3);
    }

    #[test]
    fn test_scenario_builder() {
        let scenarios = ScenarioBuilder::new()
            .add_psa_ladder(vec![0.5, 1.0, 1.5, 2.0])
            .add_cdr_ladder(vec![0.01, 0.02, 0.05])
            .build();

        assert_eq!(scenarios.len(), 7);
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
}
