//! Sensitivity analysis types.

use crate::evaluator::Results;
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Parameter to vary in sensitivity analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSpec {
    /// Node identifier
    pub node_id: String,

    /// Period to vary
    pub period_id: PeriodId,

    /// Base value
    pub base_value: f64,

    /// Perturbations to apply (e.g., [-10%, 0%, +10%])
    pub perturbations: Vec<f64>,
}

impl ParameterSpec {
    /// Create a new parameter specification.
    pub fn new(
        node_id: impl Into<String>,
        period_id: PeriodId,
        base_value: f64,
        perturbations: Vec<f64>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            period_id,
            base_value,
            perturbations,
        }
    }

    /// Create a parameter spec with percentage perturbations.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier
    /// * `period_id` - Period to vary
    /// * `base_value` - Base value
    /// * `pct_range` - Percentage range (e.g., vec![-10.0, 0.0, 10.0] for ±10%)
    pub fn with_percentages(
        node_id: impl Into<String>,
        period_id: PeriodId,
        base_value: f64,
        pct_range: Vec<f64>,
    ) -> Self {
        let perturbations = pct_range
            .into_iter()
            .map(|pct| base_value * (1.0 + pct / 100.0))
            .collect();

        Self {
            node_id: node_id.into(),
            period_id,
            base_value,
            perturbations,
        }
    }
}

/// Sensitivity analysis mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitivityMode {
    /// One-at-a-time parameter variations
    Diagonal,

    /// Full factorial grid
    FullGrid,

    /// Ranked by impact magnitude
    Tornado,
}

/// Sensitivity analysis configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityConfig {
    /// Analysis mode
    pub mode: SensitivityMode,

    /// Parameters to vary
    pub parameters: Vec<ParameterSpec>,

    /// Target metrics to track
    pub target_metrics: Vec<String>,
}

impl SensitivityConfig {
    /// Create a new sensitivity configuration.
    pub fn new(mode: SensitivityMode) -> Self {
        Self {
            mode,
            parameters: Vec::new(),
            target_metrics: Vec::new(),
        }
    }

    /// Add a parameter to vary.
    pub fn add_parameter(&mut self, param: ParameterSpec) {
        self.parameters.push(param);
    }

    /// Add a target metric to track.
    pub fn add_target_metric(&mut self, metric: impl Into<String>) {
        self.target_metrics.push(metric.into());
    }
}

/// Result of a single sensitivity scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityScenario {
    /// Parameter values for this scenario
    pub parameter_values: IndexMap<String, f64>,

    /// Full evaluation results
    pub results: Results,
}

/// Results of sensitivity analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityResult {
    /// Configuration used
    pub config: SensitivityConfig,

    /// All scenario results
    pub scenarios: Vec<SensitivityScenario>,
}

impl SensitivityResult {
    /// Get scenarios count.
    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }
}

