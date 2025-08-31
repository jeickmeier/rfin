//! Covenant engine for evaluating and applying covenant consequences.
//!
//! This module provides a comprehensive covenant evaluation system that:
//! - Evaluates financial covenants against current metrics
//! - Manages grace/cure periods
//! - Applies consequences when covenants are breached
//! - Supports both financial and non-financial covenants

use crate::instruments::fixed_income::loan::covenants::{
    Covenant, CovenantConsequence, CovenantType, ThresholdTest,
};
use crate::metrics::{MetricContext, MetricId};
use crate::results::CovenantReport;
use finstack_core::prelude::*;
use finstack_core::F;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for custom evaluator functions.
pub type CustomEvaluator = Arc<dyn Fn(&MetricContext) -> finstack_core::Result<bool> + Send + Sync>;

/// Type alias for custom metric calculators.
pub type CustomMetricCalculator =
    Arc<dyn Fn(&MetricContext) -> finstack_core::Result<finstack_core::F> + Send + Sync>;

/// Covenant evaluation specification.
#[derive(Clone)]
pub struct CovenantSpec {
    /// The covenant to evaluate
    pub covenant: Covenant,
    /// Metric ID to use for evaluation (for financial covenants)
    pub metric_id: Option<MetricId>,
    /// Custom evaluation function (for complex covenants)
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
#[derive(Clone, Debug)]
pub struct CovenantTestSpec {
    /// Covenant specifications to test
    pub specs: Vec<CovenantSpec>,
    /// Test date
    pub test_date: Date,
    /// Reference date for calculating cure periods
    pub reference_date: Option<Date>,
}

/// Covenant window for scheduled testing.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct CovenantBreach {
    /// Covenant that was breached
    pub covenant_type: String,
    /// Date of the breach
    pub breach_date: Date,
    /// Actual value that caused the breach
    pub actual_value: Option<F>,
    /// Required threshold
    pub threshold: Option<F>,
    /// Cure period end date (if applicable)
    pub cure_deadline: Option<Date>,
    /// Whether the breach has been cured
    pub is_cured: bool,
    /// Applied consequences
    pub applied_consequences: Vec<CovenantConsequence>,
}

/// Covenant engine for evaluation and consequence application.
pub struct CovenantEngine {
    /// Active covenant specifications
    pub specs: Vec<CovenantSpec>,
    /// Historical breaches
    pub breach_history: Vec<CovenantBreach>,
    /// Covenant testing windows
    pub windows: Vec<CovenantWindow>,
    /// Custom metric calculators
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
        CalcFn:
            Fn(&MetricContext) -> finstack_core::Result<finstack_core::F> + Send + Sync + 'static,
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
                    CovenantReport::passed(covenant_type).with_details("Covenant inactive"),
                );
                continue;
            }

            // Evaluate the covenant
            let (passed, actual_value, threshold) = self.evaluate_spec(spec, context)?;

            let mut report = if passed {
                CovenantReport::passed(&covenant_type)
            } else {
                CovenantReport::failed(&covenant_type)
            };

            if let Some(value) = actual_value {
                report = report.with_actual(value);
            }
            if let Some(thresh) = threshold {
                report = report.with_threshold(thresh);
            }

            // Check for cure period
            if !passed {
                if let Some(breach) = self.find_active_breach(&covenant_type, test_date) {
                    if breach.cure_deadline.is_some_and(|d| test_date <= d) {
                        report = report.with_details("In cure period");
                    }
                }
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
                .ok_or(finstack_core::error::InputError::NotFound)?;

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
        }
    }

    fn evaluate_spec(
        &self,
        spec: &CovenantSpec,
        context: &mut MetricContext,
    ) -> finstack_core::Result<(bool, Option<F>, Option<F>)> {
        // Use custom evaluator if provided
        if let Some(ref evaluator) = spec.custom_evaluator {
            let passed = evaluator(context)?;
            return Ok((passed, None, None));
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
            _ => return Ok((true, None, None)), // Non-financial covenants pass by default
        };

        let passed = match &spec.covenant.covenant_type {
            CovenantType::MaxDebtToEBITDA { .. }
            | CovenantType::MaxTotalLeverage { .. }
            | CovenantType::MaxSeniorLeverage { .. } => metric_value <= threshold,
            CovenantType::MinInterestCoverage { .. }
            | CovenantType::MinFixedChargeCoverage { .. }
            | CovenantType::MinAssetCoverage { .. } => metric_value >= threshold,
            CovenantType::Custom { test, .. } => match test {
                ThresholdTest::Maximum(_) => metric_value <= threshold,
                ThresholdTest::Minimum(_) => metric_value >= threshold,
            },
            _ => true,
        };

        Ok((passed, Some(metric_value), Some(threshold)))
    }

    fn get_metric_value(
        &self,
        context: &mut MetricContext,
        metric_id: &MetricId,
    ) -> finstack_core::Result<F> {
        // Check if already computed
        if let Some(&value) = context.computed.get(metric_id) {
            return Ok(value);
        }

        // Check custom metrics
        if let MetricId::Custom(name) = metric_id {
            if let Some(calculator) = self.custom_metrics.get(name) {
                let value = calculator(context)?;
                context.computed.insert(metric_id.clone(), value);
                return Ok(value);
            }
        }

        Err(finstack_core::error::InputError::NotFound.into())
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

/// Result of applying a covenant consequence.
#[derive(Clone, Debug)]
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
    fn increase_rate(&mut self, increase: F) -> finstack_core::Result<()>;

    /// Set cash sweep percentage.
    fn set_cash_sweep(&mut self, percentage: F) -> finstack_core::Result<()>;

    /// Block distributions.
    fn set_distribution_block(&mut self, blocked: bool) -> finstack_core::Result<()>;

    /// Change maturity date.
    fn set_maturity(&mut self, new_maturity: Date) -> finstack_core::Result<()>;
}
