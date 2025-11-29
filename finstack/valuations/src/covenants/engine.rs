//! Covenant engine for evaluating and applying covenant consequences.
//!
//! This module provides a comprehensive covenant evaluation system that:
//! - Evaluates financial covenants against current metrics
//! - Manages grace/cure periods
//! - Applies consequences when covenants are breached
//! - Supports both financial and non-financial covenants

use crate::covenants::CovenantReport;
use serde::{Deserialize, Serialize};

// Covenant type definitions were previously under loan; re-introduce minimal versions locally
/// Whether a covenant is tested periodically or only upon an action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CovenantScope {
    /// Tested on a schedule (e.g., quarterly leverage tests).
    Maintenance,
    /// Tested only upon specific actions (e.g., incurrence of debt).
    Incurrence,
}

/// Optional activation condition for springing covenants.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpringingCondition {
    /// Metric that controls activation (e.g., revolver utilization).
    pub metric_id: MetricId,
    /// Threshold test applied to the metric.
    pub test: ThresholdTest,
}

/// Financial covenant specification with test frequency and consequences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Covenant {
    /// Type of covenant (leverage, coverage, etc.)
    pub covenant_type: CovenantType,
    /// How frequently the covenant is tested
    pub test_frequency: finstack_core::dates::Frequency,
    /// Optional cure period in days before default
    pub cure_period_days: Option<i32>,
    /// Actions taken if covenant is breached
    pub consequences: Vec<CovenantConsequence>,
    /// Whether the covenant is currently active
    pub is_active: bool,
    /// Whether the covenant is maintenance or incurrence.
    pub scope: CovenantScope,
    /// Optional activation condition for springing covenants.
    pub springing_condition: Option<SpringingCondition>,
}

impl Covenant {
    /// Create a new covenant with default cure period
    pub fn new(
        covenant_type: CovenantType,
        test_frequency: finstack_core::dates::Frequency,
    ) -> Self {
        Self {
            covenant_type,
            test_frequency,
            cure_period_days: Some(30),
            consequences: Vec::new(),
            is_active: true,
            scope: CovenantScope::Maintenance,
            springing_condition: None,
        }
    }

    /// Set cure period (days before breach becomes default)
    pub fn with_cure_period(mut self, days: Option<i32>) -> Self {
        self.cure_period_days = days;
        self
    }

    /// Add a consequence for covenant breach
    pub fn with_consequence(mut self, consequence: CovenantConsequence) -> Self {
        self.consequences.push(consequence);
        self
    }

    /// Set covenant scope (maintenance vs incurrence).
    pub fn with_scope(mut self, scope: CovenantScope) -> Self {
        self.scope = scope;
        self
    }

    /// Attach a springing condition that controls activation.
    pub fn with_springing_condition(mut self, condition: SpringingCondition) -> Self {
        self.springing_condition = Some(condition);
        self
    }

    /// Get human-readable description of the covenant
    pub fn description(&self) -> String {
        match &self.covenant_type {
            CovenantType::MaxDebtToEBITDA { threshold } => {
                format!("Debt/EBITDA ≤ {:.2}x", threshold)
            }
            CovenantType::MinInterestCoverage { threshold } => {
                format!("Interest Coverage ≥ {:.2}x", threshold)
            }
            CovenantType::MinFixedChargeCoverage { threshold } => {
                format!("Fixed Charge Coverage ≥ {:.2}x", threshold)
            }
            CovenantType::MaxTotalLeverage { threshold } => {
                format!("Total Leverage ≤ {:.2}x", threshold)
            }
            CovenantType::MaxSeniorLeverage { threshold } => {
                format!("Senior Leverage ≤ {:.2}x", threshold)
            }
            CovenantType::MinAssetCoverage { threshold } => {
                format!("Asset Coverage ≥ {:.2}x", threshold)
            }
            CovenantType::Negative { restriction } => format!("Negative: {}", restriction),
            CovenantType::Affirmative { requirement } => format!("Affirmative: {}", requirement),
            CovenantType::Custom { metric, test } => match test {
                ThresholdTest::Maximum(v) => format!("{} ≤ {:.2}", metric, v),
                ThresholdTest::Minimum(v) => format!("{} ≥ {:.2}", metric, v),
            },
            CovenantType::Basket { name, limit } => {
                format!("{} Utilization ≤ {:.2}", name, limit)
            }
        }
    }
}

/// Type of financial or operational covenant
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CovenantType {
    /// Maximum debt-to-EBITDA ratio
    MaxDebtToEBITDA {
        /// Maximum allowed ratio
        threshold: f64,
    },
    /// Minimum interest coverage ratio (EBIT/Interest)
    MinInterestCoverage {
        /// Minimum required ratio
        threshold: f64,
    },
    /// Minimum fixed charge coverage ratio
    MinFixedChargeCoverage {
        /// Minimum required coverage
        threshold: f64,
    },
    /// Maximum total leverage ratio
    MaxTotalLeverage {
        /// Maximum allowed leverage
        threshold: f64,
    },
    /// Maximum senior leverage ratio
    MaxSeniorLeverage {
        /// Maximum allowed senior leverage
        threshold: f64,
    },
    /// Minimum asset coverage ratio
    MinAssetCoverage {
        /// Minimum required coverage
        threshold: f64,
    },
    /// Negative covenant (prohibition)
    Negative {
        /// Description of restriction
        restriction: String,
    },
    /// Affirmative covenant (requirement)
    Affirmative {
        /// Description of requirement
        requirement: String,
    },
    /// Custom covenant with metric and threshold test
    Custom {
        /// Name of metric to test
        metric: String,
        /// Threshold test (min or max)
        test: ThresholdTest,
    },
    /// Basket tracking covenant (e.g., available debt baskets)
    Basket {
        /// Basket identifier/metric name
        name: String,
        /// Maximum allowed utilization of the basket
        limit: f64,
    },
}

/// Threshold test type (maximum or minimum bound)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ThresholdTest {
    /// Maximum allowed value
    Maximum(f64),
    /// Minimum required value
    Minimum(f64),
}

/// Consequence of covenant breach
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CovenantConsequence {
    /// Event of default
    Default,
    /// Interest rate margin increase
    RateIncrease {
        /// Increase in basis points
        bp_increase: f64,
    },
    /// Mandatory cash sweep of excess cash flow
    CashSweep {
        /// Percentage of cash flow to sweep
        sweep_percentage: f64,
    },
    /// Block distributions to equity holders
    BlockDistributions,
    /// Require additional collateral
    RequireCollateral {
        /// Description of collateral requirement
        description: String,
    },
    /// Accelerate loan maturity date
    AccelerateMaturity {
        /// New accelerated maturity date
        new_maturity: Date,
    },
}
use crate::metrics::{MetricContext, MetricId};
use finstack_core::prelude::*;

use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for custom evaluator functions.
pub type CustomEvaluator = Arc<dyn Fn(&MetricContext) -> finstack_core::Result<bool> + Send + Sync>;

/// Type alias for custom metric calculators.
pub type CustomMetricCalculator =
    Arc<dyn Fn(&MetricContext) -> finstack_core::Result<f64> + Send + Sync>;

/// Covenant evaluation specification.
///
/// Note: The `custom_evaluator` field is not serialized as it contains
/// a function pointer. When deserializing, it will be set to `None`.
#[derive(Clone, Serialize, Deserialize)]
pub struct CovenantSpec {
    /// The covenant to evaluate
    pub covenant: Covenant,
    /// Metric ID to use for evaluation (for financial covenants)
    pub metric_id: Option<MetricId>,
    /// Custom evaluation function (for complex covenants).
    /// Not serializable - will be `None` after deserialization.
    #[serde(skip)]
    pub custom_evaluator: Option<CustomEvaluator>,
}

// Derive-based Clone now works because custom_evaluator uses Arc

impl std::fmt::Debug for CovenantSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CovenantSpec")
            .field("covenant", &self.covenant)
            .field("metric_id", &self.metric_id)
            .field("custom_evaluator", &self.custom_evaluator.is_some())
            .finish()
    }
}

impl CovenantSpec {
    /// Create a new covenant spec with a standard metric.
    pub fn with_metric(covenant: Covenant, metric_id: MetricId) -> Self {
        Self {
            covenant,
            metric_id: Some(metric_id),
            custom_evaluator: None,
        }
    }

    /// Create a new covenant spec with a custom evaluator.
    pub fn with_evaluator<F>(covenant: Covenant, evaluator: F) -> Self
    where
        F: Fn(&MetricContext) -> finstack_core::Result<bool> + Send + Sync + 'static,
    {
        Self {
            covenant,
            metric_id: None,
            custom_evaluator: Some(Arc::new(evaluator)),
        }
    }
}

/// Covenant test specification with timing windows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantTestSpec {
    /// Covenant specifications to test
    pub specs: Vec<CovenantSpec>,
    /// Test date
    pub test_date: Date,
    /// Reference date for calculating cure periods
    pub reference_date: Option<Date>,
}

/// Covenant window for scheduled testing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantWindow {
    /// Start date of the window
    pub start: Date,
    /// End date of the window
    pub end: Date,
    /// Covenants active during this window
    pub covenants: Vec<CovenantSpec>,
    /// Whether this is a grace period window
    pub is_grace_period: bool,
}

/// Covenant breach tracking.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CovenantBreach {
    /// Covenant that was breached
    pub covenant_type: String,
    /// Date of the breach
    pub breach_date: Date,
    /// Actual value that caused the breach
    pub actual_value: Option<f64>,
    /// Required threshold
    pub threshold: Option<f64>,
    /// Cure period end date (if applicable)
    pub cure_deadline: Option<Date>,
    /// Whether the breach has been cured
    pub is_cured: bool,
    /// Applied consequences
    pub applied_consequences: Vec<CovenantConsequence>,
}

/// Covenant engine for evaluation and consequence application.
///
/// Note: The `custom_metrics` field is not serialized as it contains
/// function pointers. When deserializing, it will be set to default (empty).
#[derive(Serialize, Deserialize)]
pub struct CovenantEngine {
    /// Active covenant specifications
    pub specs: Vec<CovenantSpec>,
    /// Historical breaches
    pub breach_history: Vec<CovenantBreach>,
    /// Covenant testing windows
    pub windows: Vec<CovenantWindow>,
    /// Custom metric calculators.
    /// Not serializable - will be empty after deserialization.
    #[serde(skip)]
    pub custom_metrics: HashMap<String, CustomMetricCalculator>,
}

impl Default for CovenantEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CovenantEngine {
    /// Create a new covenant engine.
    pub fn new() -> Self {
        Self {
            specs: Vec::new(),
            breach_history: Vec::new(),
            windows: Vec::new(),
            custom_metrics: HashMap::new(),
        }
    }

    /// Add a covenant specification.
    pub fn add_spec(&mut self, spec: CovenantSpec) -> &mut Self {
        self.specs.push(spec);
        self
    }

    /// Add a covenant window.
    pub fn add_window(&mut self, window: CovenantWindow) -> &mut Self {
        self.windows.push(window);
        self
    }

    /// Register a custom metric calculator.
    pub fn register_metric<CalcFn>(
        &mut self,
        name: impl Into<String>,
        calculator: CalcFn,
    ) -> &mut Self
    where
        CalcFn: Fn(&MetricContext) -> finstack_core::Result<f64> + Send + Sync + 'static,
    {
        self.custom_metrics
            .insert(name.into(), Arc::new(calculator));
        self
    }

    /// Evaluate covenants against current metrics.
    pub fn evaluate(
        &self,
        context: &mut MetricContext,
        test_date: Date,
    ) -> finstack_core::Result<IndexMap<String, CovenantReport>> {
        let mut reports = IndexMap::new();

        // Find applicable covenants for the test date
        let applicable_specs = self.get_applicable_specs_internal(test_date);

        for spec in applicable_specs {
            let covenant_type = self.get_covenant_description(&spec.covenant.covenant_type);

            // Skip inactive covenants
            if !spec.covenant.is_active {
                reports.insert(
                    covenant_type.clone(),
                    CovenantReport::passed(&covenant_type).with_details("Covenant inactive"),
                );
                continue;
            }

            // Evaluate the covenant
            let evaluation = self.evaluate_spec(spec, context)?;

            let mut report = if evaluation.passed {
                CovenantReport::passed(&covenant_type)
            } else {
                CovenantReport::failed(&covenant_type)
            };

            if let Some(value) = evaluation.actual_value {
                report = report.with_actual(value);
            }
            if let Some(thresh) = evaluation.threshold {
                report = report.with_threshold(thresh);
            }
            if let Some(hr) = evaluation.headroom {
                report = report.with_headroom(hr);
            }

            // Check for cure period
            if !evaluation.passed {
                if let Some(breach) = self.find_active_breach(&covenant_type, test_date) {
                    if breach.cure_deadline.is_some_and(|d| test_date <= d) {
                        report = report.with_details("In cure period");
                    }
                }
            }

            if let Some(detail) = evaluation.detail {
                report = report.with_details(&detail);
            }

            reports.insert(covenant_type, report);
        }

        Ok(reports)
    }

    /// Apply consequences for breached covenants.
    pub fn apply_consequences<T>(
        &mut self,
        instrument: &mut T,
        breaches: &[CovenantBreach],
        as_of: Date,
    ) -> finstack_core::Result<Vec<ConsequenceApplication>>
    where
        T: InstrumentMutator,
    {
        let mut applications = Vec::new();

        for breach in breaches {
            // Skip if already cured or in cure period
            if breach.is_cured {
                continue;
            }
            if let Some(deadline) = breach.cure_deadline {
                if as_of <= deadline {
                    continue;
                }
            }

            // Find the covenant spec
            let spec = self
                .specs
                .iter()
                .find(|s| {
                    self.get_covenant_description(&s.covenant.covenant_type) == breach.covenant_type
                })
                .ok_or(finstack_core::error::InputError::NotFound {
                    id: format!("covenant_spec:{}", breach.covenant_type),
                })?;

            // Apply each consequence
            for consequence in &spec.covenant.consequences {
                let application = self.apply_single_consequence(instrument, consequence, as_of)?;
                applications.push(application);

                // Track in breach history
                if let Some(historical_breach) = self.breach_history.iter_mut().find(|b| {
                    b.covenant_type == breach.covenant_type && b.breach_date == breach.breach_date
                }) {
                    historical_breach
                        .applied_consequences
                        .push(consequence.clone());
                }
            }
        }

        Ok(applications)
    }

    /// Get applicable specs for a given date (public for testing).
    pub fn get_applicable_specs(&self, test_date: Date) -> Vec<&CovenantSpec> {
        self.get_applicable_specs_internal(test_date)
    }

    // Helper methods

    fn get_applicable_specs_internal(&self, test_date: Date) -> Vec<&CovenantSpec> {
        // Check windows first
        for window in &self.windows {
            if test_date >= window.start && test_date <= window.end {
                return window.covenants.iter().collect();
            }
        }

        // Fall back to all specs
        self.specs.iter().collect()
    }

    fn get_covenant_description(&self, covenant_type: &CovenantType) -> String {
        match covenant_type {
            CovenantType::MaxDebtToEBITDA { threshold } => {
                format!("Debt/EBITDA <= {:.2}", threshold)
            }
            CovenantType::MinInterestCoverage { threshold } => {
                format!("Interest Coverage >= {:.2}x", threshold)
            }
            CovenantType::MinFixedChargeCoverage { threshold } => {
                format!("Fixed Charge Coverage >= {:.2}x", threshold)
            }
            CovenantType::MaxTotalLeverage { threshold } => {
                format!("Total Leverage <= {:.2}x", threshold)
            }
            CovenantType::MaxSeniorLeverage { threshold } => {
                format!("Senior Leverage <= {:.2}x", threshold)
            }
            CovenantType::MinAssetCoverage { threshold } => {
                format!("Asset Coverage >= {:.2}x", threshold)
            }
            CovenantType::Negative { restriction } => format!("Negative: {}", restriction),
            CovenantType::Affirmative { requirement } => format!("Affirmative: {}", requirement),
            CovenantType::Custom { metric, test } => match test {
                ThresholdTest::Maximum(t) => format!("{} <= {:.2}", metric, t),
                ThresholdTest::Minimum(t) => format!("{} >= {:.2}", metric, t),
            },
            CovenantType::Basket { name, limit } => {
                format!("{} Utilization ≤ {:.2}", name, limit)
            }
        }
    }

    fn evaluate_spec(
        &self,
        spec: &CovenantSpec,
        context: &mut MetricContext,
    ) -> finstack_core::Result<SpecEvaluation> {
        // Springing conditions: skip evaluation until activation criteria met.
        if let Some(condition) = &spec.covenant.springing_condition {
            let condition_value = self.get_metric_value(context, &condition.metric_id)?;
            let condition_met = match condition.test {
                ThresholdTest::Maximum(t) => condition_value <= t,
                ThresholdTest::Minimum(t) => condition_value >= t,
            };

            if !condition_met {
                return Ok(SpecEvaluation {
                    passed: true,
                    actual_value: None,
                    threshold: None,
                    headroom: None,
                    detail: Some("Springing condition not met".to_string()),
                });
            }
        }

        // Use custom evaluator if provided
        if let Some(ref evaluator) = spec.custom_evaluator {
            let passed = evaluator(context)?;
            return Ok(SpecEvaluation {
                passed,
                actual_value: None,
                threshold: None,
                headroom: None,
                detail: None,
            });
        }

        // Otherwise use metric-based evaluation
        let (metric_value, threshold) = match &spec.covenant.covenant_type {
            CovenantType::MaxDebtToEBITDA { threshold } => {
                let debt_to_ebitda =
                    self.get_metric_value(context, &MetricId::custom("debt_to_ebitda"))?;
                (debt_to_ebitda, *threshold)
            }
            CovenantType::MinInterestCoverage { threshold } => {
                let coverage =
                    self.get_metric_value(context, &MetricId::custom("interest_coverage"))?;
                (coverage, *threshold)
            }
            CovenantType::MinFixedChargeCoverage { threshold } => {
                let coverage =
                    self.get_metric_value(context, &MetricId::custom("fixed_charge_coverage"))?;
                (coverage, *threshold)
            }
            CovenantType::MaxTotalLeverage { threshold } => {
                let leverage =
                    self.get_metric_value(context, &MetricId::custom("total_leverage"))?;
                (leverage, *threshold)
            }
            CovenantType::MaxSeniorLeverage { threshold } => {
                let leverage =
                    self.get_metric_value(context, &MetricId::custom("senior_leverage"))?;
                (leverage, *threshold)
            }
            CovenantType::MinAssetCoverage { threshold } => {
                let coverage =
                    self.get_metric_value(context, &MetricId::custom("asset_coverage"))?;
                (coverage, *threshold)
            }
            CovenantType::Custom { metric, test } => {
                let value = self.get_metric_value(context, &MetricId::custom(metric))?;
                let threshold = match test {
                    ThresholdTest::Maximum(t) | ThresholdTest::Minimum(t) => *t,
                };
                (value, threshold)
            }
            CovenantType::Basket { name, limit } => {
                let value = self.get_metric_value(context, &MetricId::custom(name))?;
                (value, *limit)
            }
            _ => {
                return Ok(SpecEvaluation {
                    passed: true,
                    actual_value: None,
                    threshold: None,
                    headroom: None,
                    detail: None,
                })
            }
        };

        let passed = match &spec.covenant.covenant_type {
            CovenantType::MaxDebtToEBITDA { .. }
            | CovenantType::MaxTotalLeverage { .. }
            | CovenantType::MaxSeniorLeverage { .. }
            | CovenantType::Basket { .. } => metric_value <= threshold,
            CovenantType::MinInterestCoverage { .. }
            | CovenantType::MinFixedChargeCoverage { .. }
            | CovenantType::MinAssetCoverage { .. } => metric_value >= threshold,
            CovenantType::Custom { test, .. } => match test {
                ThresholdTest::Maximum(_) => metric_value <= threshold,
                ThresholdTest::Minimum(_) => metric_value >= threshold,
            },
            _ => true,
        };

        let headroom = Some(headroom_for(
            &spec.covenant.covenant_type,
            metric_value,
            threshold,
        ));

        Ok(SpecEvaluation {
            passed,
            actual_value: Some(metric_value),
            threshold: Some(threshold),
            headroom,
            detail: None,
        })
    }

    fn get_metric_value(
        &self,
        context: &mut MetricContext,
        metric_id: &MetricId,
    ) -> finstack_core::Result<f64> {
        // Check if already computed
        if let Some(&value) = context.computed.get(metric_id) {
            return Ok(value);
        }

        // Check custom metrics
        if metric_id.is_custom() {
            let name = metric_id.as_str();
            if let Some(calculator) = self.custom_metrics.get(name) {
                let value = calculator(context)?;
                context.computed.insert(metric_id.clone(), value);
                return Ok(value);
            }
        }

        Err(finstack_core::error::InputError::NotFound {
            id: "covenant_description".to_string(),
        }
        .into())
    }

    fn find_active_breach(&self, covenant_type: &str, as_of: Date) -> Option<&CovenantBreach> {
        self.breach_history
            .iter()
            .filter(|b| b.covenant_type == covenant_type && !b.is_cured)
            .filter(|b| b.breach_date <= as_of)
            .max_by_key(|b| b.breach_date)
    }

    fn apply_single_consequence<T>(
        &self,
        instrument: &mut T,
        consequence: &CovenantConsequence,
        as_of: Date,
    ) -> finstack_core::Result<ConsequenceApplication>
    where
        T: InstrumentMutator,
    {
        match consequence {
            CovenantConsequence::Default => {
                instrument.set_default_status(true, as_of)?;
                Ok(ConsequenceApplication {
                    consequence_type: "Default".to_string(),
                    applied_date: as_of,
                    details: "Loan in default".to_string(),
                })
            }
            CovenantConsequence::RateIncrease { bp_increase } => {
                instrument.increase_rate(*bp_increase / 10000.0)?;
                Ok(ConsequenceApplication {
                    consequence_type: "Rate Increase".to_string(),
                    applied_date: as_of,
                    details: format!("Rate increased by {} bps", bp_increase),
                })
            }
            CovenantConsequence::CashSweep { sweep_percentage } => {
                instrument.set_cash_sweep(*sweep_percentage)?;
                Ok(ConsequenceApplication {
                    consequence_type: "Cash Sweep".to_string(),
                    applied_date: as_of,
                    details: format!("{}% cash sweep activated", sweep_percentage * 100.0),
                })
            }
            CovenantConsequence::BlockDistributions => {
                instrument.set_distribution_block(true)?;
                Ok(ConsequenceApplication {
                    consequence_type: "Block Distributions".to_string(),
                    applied_date: as_of,
                    details: "Distributions blocked".to_string(),
                })
            }
            CovenantConsequence::RequireCollateral { description } => Ok(ConsequenceApplication {
                consequence_type: "Require Collateral".to_string(),
                applied_date: as_of,
                details: description.clone(),
            }),
            CovenantConsequence::AccelerateMaturity { new_maturity } => {
                instrument.set_maturity(*new_maturity)?;
                Ok(ConsequenceApplication {
                    consequence_type: "Accelerate Maturity".to_string(),
                    applied_date: as_of,
                    details: format!("Maturity accelerated to {}", new_maturity),
                })
            }
        }
    }
}

struct SpecEvaluation {
    passed: bool,
    actual_value: Option<f64>,
    threshold: Option<f64>,
    headroom: Option<f64>,
    detail: Option<String>,
}

fn headroom_for(cov: &CovenantType, value: f64, threshold: f64) -> f64 {
    let denom = if threshold.abs() < f64::EPSILON {
        1.0
    } else {
        threshold
    };

    match cov {
        CovenantType::MaxDebtToEBITDA { .. }
        | CovenantType::MaxTotalLeverage { .. }
        | CovenantType::MaxSeniorLeverage { .. }
        | CovenantType::Basket { .. }
        | CovenantType::Custom {
            test: ThresholdTest::Maximum(_),
            ..
        } => (threshold - value) / denom,
        CovenantType::MinInterestCoverage { .. }
        | CovenantType::MinFixedChargeCoverage { .. }
        | CovenantType::MinAssetCoverage { .. }
        | CovenantType::Custom {
            test: ThresholdTest::Minimum(_),
            ..
        } => (value - threshold) / denom,
        _ => 0.0,
    }
}

/// Result of applying a covenant consequence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsequenceApplication {
    /// Type of consequence applied
    pub consequence_type: String,
    /// Date when applied
    pub applied_date: Date,
    /// Details about the application
    pub details: String,
}

/// Trait for instruments that can be mutated by covenant consequences.
pub trait InstrumentMutator {
    /// Set default status.
    fn set_default_status(&mut self, is_default: bool, as_of: Date) -> finstack_core::Result<()>;

    /// Increase interest rate.
    fn increase_rate(&mut self, increase: f64) -> finstack_core::Result<()>;

    /// Set cash sweep percentage.
    fn set_cash_sweep(&mut self, percentage: f64) -> finstack_core::Result<()>;

    /// Block distributions.
    fn set_distribution_block(&mut self, blocked: bool) -> finstack_core::Result<()>;

    /// Change maturity date.
    fn set_maturity(&mut self, new_maturity: Date) -> finstack_core::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::{
        helpers,
        traits::{Attributes, Instrument},
    };
    use crate::pricer::InstrumentType;
    use finstack_core::{
        currency::Currency,
        dates::{Date, Frequency},
        market_data::context::MarketContext,
        money::Money,
    };
    use std::sync::Arc;
    use time::{Duration, Month};

    #[derive(Clone)]
    struct TestInstrument {
        id: String,
        attrs: Attributes,
        currency: Currency,
        base_value: f64,
        rate: f64,
        defaulted: bool,
        cash_sweep: f64,
        distributions_blocked: bool,
        maturity: Date,
    }

    impl TestInstrument {
        fn new(id: &str, maturity: Date) -> Self {
            Self {
                id: id.to_string(),
                attrs: Attributes::new(),
                currency: Currency::USD,
                base_value: 1_000_000.0,
                rate: 0.05,
                defaulted: false,
                cash_sweep: 0.0,
                distributions_blocked: false,
                maturity,
            }
        }
    }

    impl Instrument for TestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Loan
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &Attributes {
            &self.attrs
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            &mut self.attrs
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn value(&self, _curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
            Ok(Money::new(self.base_value, self.currency))
        }

        fn price_with_metrics(
            &self,
            curves: &MarketContext,
            as_of: Date,
            metrics: &[crate::metrics::MetricId],
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            let base_value = self.value(curves, as_of)?;
            helpers::build_with_metrics_dyn(
                std::sync::Arc::new(self.clone()),
                std::sync::Arc::new(curves.clone()),
                as_of,
                base_value,
                metrics,
            )
        }
    }

    impl InstrumentMutator for TestInstrument {
        fn set_default_status(
            &mut self,
            is_default: bool,
            _as_of: Date,
        ) -> finstack_core::Result<()> {
            self.defaulted = is_default;
            Ok(())
        }

        fn increase_rate(&mut self, increase: f64) -> finstack_core::Result<()> {
            self.rate += increase;
            Ok(())
        }

        fn set_cash_sweep(&mut self, percentage: f64) -> finstack_core::Result<()> {
            self.cash_sweep = percentage;
            Ok(())
        }

        fn set_distribution_block(&mut self, blocked: bool) -> finstack_core::Result<()> {
            self.distributions_blocked = blocked;
            Ok(())
        }

        fn set_maturity(&mut self, new_maturity: Date) -> finstack_core::Result<()> {
            self.maturity = new_maturity;
            Ok(())
        }
    }

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(
            year,
            Month::try_from(month).expect("Valid month (1-12)"),
            day,
        )
        .expect("Valid test date")
    }

    fn metric_context(instrument: &TestInstrument, as_of: Date) -> MetricContext {
        MetricContext::new(
            Arc::new(instrument.clone()),
            Arc::new(MarketContext::new()),
            as_of,
            Money::new(instrument.base_value, instrument.currency),
        )
    }

    #[test]
    fn evaluate_financial_covenants_with_cure_periods() {
        let mut engine = CovenantEngine::new();

        let leverage_cov = Covenant::new(
            CovenantType::MaxTotalLeverage { threshold: 5.0 },
            Frequency::quarterly(),
        );
        let coverage_cov = Covenant::new(
            CovenantType::MinInterestCoverage { threshold: 1.50 },
            Frequency::quarterly(),
        );

        engine.add_spec(CovenantSpec::with_metric(
            leverage_cov,
            MetricId::custom("total_leverage"),
        ));
        engine.add_spec(CovenantSpec::with_metric(
            coverage_cov,
            MetricId::custom("interest_coverage"),
        ));

        let test_date = date(2025, 3, 31);
        engine.breach_history.push(CovenantBreach {
            covenant_type: "Interest Coverage >= 1.50x".to_string(),
            breach_date: test_date - Duration::days(30),
            actual_value: Some(1.1),
            threshold: Some(1.5),
            cure_deadline: Some(test_date + Duration::days(10)),
            is_cured: false,
            applied_consequences: Vec::new(),
        });

        let ctx_instrument = TestInstrument::new("TEST-LOAN", test_date + Duration::days(180));
        let mut ctx = metric_context(&ctx_instrument, test_date);
        ctx.computed.insert(MetricId::custom("total_leverage"), 4.2);
        ctx.computed
            .insert(MetricId::custom("interest_coverage"), 1.0);

        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("Covenant evaluation should succeed in test");

        let leverage = reports
            .get("Total Leverage <= 5.00x")
            .expect("leverage covenant present");
        assert!(leverage.passed);
        assert_eq!(leverage.actual_value, Some(4.2));
        assert_eq!(leverage.threshold, Some(5.0));

        let coverage = reports
            .get("Interest Coverage >= 1.50x")
            .expect("coverage covenant present");
        assert!(!coverage.passed);
        assert_eq!(coverage.actual_value, Some(1.0));
        assert_eq!(coverage.threshold, Some(1.5));
        assert_eq!(coverage.details.as_deref(), Some("In cure period"));

        // Non-financial match ensures description helper handles different variants.
        let negative_cov = Covenant::new(
            CovenantType::Negative {
                restriction: "No additional debt".to_string(),
            },
            Frequency::annual(),
        );
        let neg_description = engine.get_covenant_description(&negative_cov.covenant_type);
        assert_eq!(neg_description, "Negative: No additional debt");
    }

    #[test]
    fn evaluate_respects_windows_and_custom_sources() {
        let mut engine = CovenantEngine::new();

        // Base spec that should be ignored when window is active.
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 4.0 },
                Frequency::quarterly(),
            ),
            MetricId::custom("debt_to_ebitda"),
        ));

        engine.register_metric("liquidity_ratio", |_ctx| Ok(1.25));

        let custom_metric_spec = CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::Custom {
                    metric: "liquidity_ratio".to_string(),
                    test: ThresholdTest::Minimum(1.1),
                },
                Frequency::quarterly(),
            ),
            MetricId::custom("liquidity_ratio"),
        );

        let evaluator_spec = CovenantSpec::with_evaluator(
            Covenant::new(
                CovenantType::Affirmative {
                    requirement: "Provide quarterly reporting".to_string(),
                },
                Frequency::quarterly(),
            ),
            |_ctx| Ok(false),
        );

        let liquidity_desc =
            engine.get_covenant_description(&custom_metric_spec.covenant.covenant_type);
        let affirmative_desc =
            engine.get_covenant_description(&evaluator_spec.covenant.covenant_type);

        engine.add_window(CovenantWindow {
            start: date(2025, 1, 1),
            end: date(2025, 6, 30),
            covenants: vec![custom_metric_spec, evaluator_spec],
            is_grace_period: false,
        });

        let instrument = TestInstrument::new("WINDOW-TEST", date(2026, 1, 1));
        let mut ctx = metric_context(&instrument, date(2025, 3, 31));

        let reports = engine
            .evaluate(&mut ctx, date(2025, 3, 31))
            .expect("evaluation succeeds");
        assert_eq!(reports.len(), 2);
        let liquidity_report = reports
            .get(liquidity_desc.as_str())
            .expect("custom metric report");
        assert!(liquidity_report.passed);
        assert_eq!(liquidity_report.actual_value, Some(1.25));
        assert_eq!(liquidity_report.threshold, Some(1.1));

        let reporting_report = reports
            .get(affirmative_desc.as_str())
            .expect("affirmative covenant");
        assert!(!reporting_report.passed);
        assert_eq!(reporting_report.actual_value, None);
        assert_eq!(reporting_report.threshold, None);

        let applicable = engine.get_applicable_specs(date(2025, 3, 31));
        assert_eq!(applicable.len(), 2);
        let applicable_descriptions: Vec<_> = applicable
            .iter()
            .map(|spec| engine.get_covenant_description(&spec.covenant.covenant_type))
            .collect();
        assert!(applicable_descriptions.contains(&liquidity_desc));
        assert!(applicable_descriptions.contains(&affirmative_desc));
    }

    #[test]
    fn apply_consequences_executes_all_variants() {
        let mut engine = CovenantEngine::new();
        let as_of = date(2025, 4, 30);
        let accelerated_maturity = as_of + Duration::days(90);

        let covenant = Covenant::new(
            CovenantType::MaxSeniorLeverage { threshold: 3.0 },
            Frequency::quarterly(),
        )
        .with_consequence(CovenantConsequence::Default)
        .with_consequence(CovenantConsequence::RateIncrease { bp_increase: 150.0 })
        .with_consequence(CovenantConsequence::CashSweep {
            sweep_percentage: 0.5,
        })
        .with_consequence(CovenantConsequence::BlockDistributions)
        .with_consequence(CovenantConsequence::RequireCollateral {
            description: "Pledge additional securities".to_string(),
        })
        .with_consequence(CovenantConsequence::AccelerateMaturity {
            new_maturity: accelerated_maturity,
        });

        engine.add_spec(CovenantSpec::with_metric(
            covenant,
            MetricId::custom("senior_leverage"),
        ));

        engine.breach_history.push(CovenantBreach {
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(10),
            actual_value: Some(3.8),
            threshold: Some(3.0),
            cure_deadline: Some(as_of - Duration::days(1)),
            is_cured: false,
            applied_consequences: Vec::new(),
        });

        let actionable_breach = CovenantBreach {
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(10),
            actual_value: Some(3.8),
            threshold: Some(3.0),
            cure_deadline: Some(as_of - Duration::days(1)),
            is_cured: false,
            applied_consequences: Vec::new(),
        };

        let cured_breach = CovenantBreach {
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(40),
            actual_value: Some(3.4),
            threshold: Some(3.0),
            cure_deadline: None,
            is_cured: true,
            applied_consequences: Vec::new(),
        };

        let in_cure_breach = CovenantBreach {
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(5),
            actual_value: Some(3.2),
            threshold: Some(3.0),
            cure_deadline: Some(as_of + Duration::days(5)),
            is_cured: false,
            applied_consequences: Vec::new(),
        };

        let mut instrument = TestInstrument::new("COV-LOAN", date(2026, 6, 30));
        let applications = engine
            .apply_consequences(
                &mut instrument,
                &[actionable_breach, cured_breach, in_cure_breach],
                as_of,
            )
            .expect("Consequence application should succeed in test");

        assert_eq!(applications.len(), 6);
        assert!(instrument.defaulted);
        assert!((instrument.rate - 0.065).abs() < 1e-12);
        assert_eq!(instrument.cash_sweep, 0.5);
        assert!(instrument.distributions_blocked);
        assert_eq!(instrument.maturity, accelerated_maturity);

        let history = &engine.breach_history[0];
        assert_eq!(history.applied_consequences.len(), 6);
        assert_eq!(
            applications
                .iter()
                .map(|a| a.consequence_type.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Default",
                "Rate Increase",
                "Cash Sweep",
                "Block Distributions",
                "Require Collateral",
                "Accelerate Maturity"
            ]
        );
    }

    #[test]
    fn springing_condition_controls_activation() {
        let mut engine = CovenantEngine::new();
        let springing = SpringingCondition {
            metric_id: MetricId::custom("utilization"),
            test: ThresholdTest::Minimum(0.5),
        };
        let covenant = Covenant::new(
            CovenantType::MaxTotalLeverage { threshold: 5.0 },
            Frequency::quarterly(),
        )
        .with_springing_condition(springing);

        engine.add_spec(CovenantSpec::with_metric(
            covenant,
            MetricId::custom("total_leverage"),
        ));

        let test_date = date(2025, 3, 31);
        let ctx_instrument = TestInstrument::new("SPRING-TEST", date(2026, 3, 31));
        let mut ctx = metric_context(&ctx_instrument, test_date);
        ctx.computed.insert(MetricId::custom("total_leverage"), 5.5);
        ctx.computed.insert(MetricId::custom("utilization"), 0.4);

        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("Total Leverage <= 5.00x")
            .expect("springing covenant present");
        assert!(report.passed, "should auto-pass when inactive");
        assert_eq!(
            report.details.as_deref(),
            Some("Springing condition not met")
        );

        ctx.computed.insert(MetricId::custom("utilization"), 0.75);
        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("Total Leverage <= 5.00x")
            .expect("springing covenant present");
        assert!(!report.passed, "breach should surface once active");
        assert!(report.details.is_none());
    }

    #[test]
    fn basket_covenant_reports_headroom() {
        let mut engine = CovenantEngine::new();
        let covenant = Covenant::new(
            CovenantType::Basket {
                name: "general_debt".to_string(),
                limit: 100.0,
            },
            Frequency::quarterly(),
        );
        engine.add_spec(CovenantSpec::with_metric(
            covenant,
            MetricId::custom("general_debt"),
        ));

        let test_date = date(2025, 6, 30);
        let ctx_instrument = TestInstrument::new("BASKET-TEST", date(2026, 6, 30));
        let mut ctx = metric_context(&ctx_instrument, test_date);
        ctx.computed.insert(MetricId::custom("general_debt"), 80.0);

        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("general_debt Utilization ≤ 100.00")
            .expect("basket covenant present");
        assert!(report.passed);
        assert_eq!(report.headroom, Some(0.20));

        ctx.computed.insert(MetricId::custom("general_debt"), 120.0);
        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("general_debt Utilization ≤ 100.00")
            .expect("basket covenant present");
        assert!(!report.passed);
        assert!(report
            .headroom
            .is_some_and(|h| h < 0.0 && (h + 0.20).abs() < 1e-6));
    }
}
