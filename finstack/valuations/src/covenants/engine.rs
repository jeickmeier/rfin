//! Covenant engine for evaluating and applying covenant consequences.
//!
//! This module provides a comprehensive covenant evaluation system that:
//! - Evaluates financial covenants against current metrics
//! - Manages grace/cure periods
//! - Applies consequences when covenants are breached
//! - Supports both financial and non-financial covenants

use crate::covenants::schedule::{threshold_for_date, ThresholdSchedule};
use crate::covenants::CovenantReport;
use finstack_core::dates::Date;
use serde::{Deserialize, Serialize};

// Covenant type definitions were previously under loan; re-introduce minimal versions locally
/// Whether a covenant is tested periodically or only upon an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CovenantScope {
    /// Tested on a schedule (e.g., quarterly leverage tests).
    Maintenance,
    /// Tested only upon specific actions (e.g., incurrence of debt).
    Incurrence,
}

/// Optional activation condition for springing covenants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpringingCondition {
    /// Metric that controls activation (e.g., revolver utilization).
    pub metric_id: MetricId,
    /// Threshold test applied to the metric.
    pub test: ThresholdTest,
}

/// Financial covenant specification with test frequency and consequences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Covenant {
    /// Type of covenant (leverage, coverage, etc.)
    pub covenant_type: CovenantType,
    /// How frequently the covenant is tested
    pub test_frequency: finstack_core::dates::Tenor,
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
    pub fn new(covenant_type: CovenantType, test_frequency: finstack_core::dates::Tenor) -> Self {
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
        self.covenant_type.to_string()
    }
}

/// Type of financial or operational covenant
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Minimum debt service coverage ratio (EBITDA / Debt Service)
    MinDSCR {
        /// Minimum required coverage
        threshold: f64,
    },
    /// Maximum net debt to EBITDA ratio (net of cash)
    MaxNetDebtToEBITDA {
        /// Maximum allowed ratio
        threshold: f64,
    },
    /// Maximum capital expenditure
    MaxCapex {
        /// Maximum allowed capex amount
        threshold: f64,
    },
    /// Minimum liquidity (cash + available revolver)
    MinLiquidity {
        /// Minimum required liquidity
        threshold: f64,
    },
}

impl std::fmt::Display for CovenantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CovenantType::MaxDebtToEBITDA { threshold } => {
                write!(f, "Debt/EBITDA <= {:.2}x", threshold)
            }
            CovenantType::MinInterestCoverage { threshold } => {
                write!(f, "Interest Coverage >= {:.2}x", threshold)
            }
            CovenantType::MinFixedChargeCoverage { threshold } => {
                write!(f, "Fixed Charge Coverage >= {:.2}x", threshold)
            }
            CovenantType::MaxTotalLeverage { threshold } => {
                write!(f, "Total Leverage <= {:.2}x", threshold)
            }
            CovenantType::MaxSeniorLeverage { threshold } => {
                write!(f, "Senior Leverage <= {:.2}x", threshold)
            }
            CovenantType::MinAssetCoverage { threshold } => {
                write!(f, "Asset Coverage >= {:.2}x", threshold)
            }
            CovenantType::Negative { restriction } => write!(f, "Negative: {}", restriction),
            CovenantType::Affirmative { requirement } => {
                write!(f, "Affirmative: {}", requirement)
            }
            CovenantType::Custom { metric, test } => match test {
                ThresholdTest::Maximum(v) => write!(f, "{} <= {:.2}", metric, v),
                ThresholdTest::Minimum(v) => write!(f, "{} >= {:.2}", metric, v),
            },
            CovenantType::Basket { name, limit } => {
                write!(f, "{} Utilization <= {:.2}", name, limit)
            }
            CovenantType::MinDSCR { threshold } => {
                write!(f, "DSCR >= {:.2}x", threshold)
            }
            CovenantType::MaxNetDebtToEBITDA { threshold } => {
                write!(f, "Net Debt/EBITDA <= {:.2}x", threshold)
            }
            CovenantType::MaxCapex { threshold } => {
                write!(f, "Capex <= {:.2}", threshold)
            }
            CovenantType::MinLiquidity { threshold } => {
                write!(f, "Liquidity >= {:.2}", threshold)
            }
        }
    }
}

/// Threshold test type (maximum or minimum bound)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ThresholdTest {
    /// Maximum allowed value
    Maximum(f64),
    /// Minimum required value
    Minimum(f64),
}

/// Direction of inequality for numeric covenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundKind {
    /// Covenant passes when the metric is less than or equal to the threshold.
    AtMost,
    /// Covenant passes when the metric is greater than or equal to the threshold.
    AtLeast,
}

impl CovenantType {
    /// Returns the inequality direction required for numeric covenants.
    pub fn bound_kind(&self) -> Option<BoundKind> {
        match self {
            CovenantType::MaxDebtToEBITDA { .. }
            | CovenantType::MaxTotalLeverage { .. }
            | CovenantType::MaxSeniorLeverage { .. }
            | CovenantType::MaxNetDebtToEBITDA { .. }
            | CovenantType::MaxCapex { .. }
            | CovenantType::Basket { .. }
            | CovenantType::Custom {
                test: ThresholdTest::Maximum(_),
                ..
            } => Some(BoundKind::AtMost),
            CovenantType::MinInterestCoverage { .. }
            | CovenantType::MinFixedChargeCoverage { .. }
            | CovenantType::MinAssetCoverage { .. }
            | CovenantType::MinDSCR { .. }
            | CovenantType::MinLiquidity { .. }
            | CovenantType::Custom {
                test: ThresholdTest::Minimum(_),
                ..
            } => Some(BoundKind::AtLeast),
            CovenantType::Negative { .. } | CovenantType::Affirmative { .. } => None,
        }
    }

    /// Returns the scalar threshold (if any) associated with the covenant type.
    pub(crate) fn threshold_value(&self) -> Option<f64> {
        match self {
            CovenantType::MaxDebtToEBITDA { threshold }
            | CovenantType::MinInterestCoverage { threshold }
            | CovenantType::MinFixedChargeCoverage { threshold }
            | CovenantType::MaxTotalLeverage { threshold }
            | CovenantType::MaxSeniorLeverage { threshold }
            | CovenantType::MinAssetCoverage { threshold }
            | CovenantType::MinDSCR { threshold }
            | CovenantType::MaxNetDebtToEBITDA { threshold }
            | CovenantType::MaxCapex { threshold }
            | CovenantType::MinLiquidity { threshold } => Some(*threshold),
            CovenantType::Custom { test, .. } => match test {
                ThresholdTest::Maximum(t) | ThresholdTest::Minimum(t) => Some(*t),
            },
            CovenantType::Basket { limit, .. } => Some(*limit),
            CovenantType::Negative { .. } | CovenantType::Affirmative { .. } => None,
        }
    }

    /// Returns the canonical metric identifier for the covenant type when one exists.
    pub(crate) fn default_metric_name(&self) -> Option<&'static str> {
        match self {
            CovenantType::MaxDebtToEBITDA { .. } => Some("debt_to_ebitda"),
            CovenantType::MinInterestCoverage { .. } => Some("interest_coverage"),
            CovenantType::MinFixedChargeCoverage { .. } => Some("fixed_charge_coverage"),
            CovenantType::MaxTotalLeverage { .. } => Some("total_leverage"),
            CovenantType::MaxSeniorLeverage { .. } => Some("senior_leverage"),
            CovenantType::MinAssetCoverage { .. } => Some("asset_coverage"),
            CovenantType::MinDSCR { .. } => Some("dscr"),
            CovenantType::MaxNetDebtToEBITDA { .. } => Some("net_debt_to_ebitda"),
            CovenantType::MaxCapex { .. } => Some("capex"),
            CovenantType::MinLiquidity { .. } => Some("liquidity"),
            CovenantType::Custom { .. }
            | CovenantType::Basket { .. }
            | CovenantType::Negative { .. }
            | CovenantType::Affirmative { .. } => None,
        }
    }

    /// Stable machine-readable identifier based on the variant discriminant only.
    ///
    /// Thresholds are **not** included because they can be amended by waivers or
    /// overridden by threshold schedules. If multiple covenants of the same type
    /// exist, callers should assign a disambiguating label externally.
    pub fn covenant_id(&self) -> &'static str {
        match self {
            CovenantType::MaxDebtToEBITDA { .. } => "max_debt_ebitda",
            CovenantType::MinInterestCoverage { .. } => "min_interest_coverage",
            CovenantType::MinFixedChargeCoverage { .. } => "min_fcc",
            CovenantType::MaxTotalLeverage { .. } => "max_total_leverage",
            CovenantType::MaxSeniorLeverage { .. } => "max_senior_leverage",
            CovenantType::MinAssetCoverage { .. } => "min_asset_coverage",
            CovenantType::MinDSCR { .. } => "min_dscr",
            CovenantType::MaxNetDebtToEBITDA { .. } => "max_net_debt_ebitda",
            CovenantType::MaxCapex { .. } => "max_capex",
            CovenantType::MinLiquidity { .. } => "min_liquidity",
            CovenantType::Negative { .. } => "negative",
            CovenantType::Affirmative { .. } => "affirmative",
            CovenantType::Custom { .. } => "custom",
            CovenantType::Basket { .. } => "basket",
        }
    }
}

/// Consequence of covenant breach
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Whether the covenant test is triggered by a scheduled maintenance check or
/// a specific incurrence action. The engine uses this to filter specs by scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationTrigger {
    /// Scheduled periodic test (e.g., quarterly compliance).
    Maintenance,
    /// Test triggered by a specific action (e.g., new debt issuance).
    Incurrence {
        /// Description of the triggering action.
        action: String,
    },
}

/// A covenant waiver or amendment granted by lenders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovenantWaiver {
    /// Stable identifier of the waived covenant (from [`CovenantType::covenant_id`]).
    pub covenant_id: String,
    /// Start date of the waiver period.
    pub effective_date: Date,
    /// End date of the waiver period (None = permanent amendment).
    pub expiry_date: Option<Date>,
    /// Amended threshold (if this is an amendment rather than a full waiver).
    pub amended_threshold: Option<f64>,
    /// Free-text description of the waiver terms.
    pub description: String,
}

use crate::metrics::{MetricContext, MetricId};

use finstack_core::HashMap;
use indexmap::IndexMap;
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
    /// Time-varying threshold schedule that overrides the static threshold in
    /// [`CovenantType`] when present. Enables leverage step-down schedules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_schedule: Option<ThresholdSchedule>,
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
            threshold_schedule: None,
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
            threshold_schedule: None,
            custom_evaluator: Some(Arc::new(evaluator)),
        }
    }

    /// Attach a time-varying threshold schedule (e.g., leverage step-downs).
    pub fn with_threshold_schedule(mut self, schedule: ThresholdSchedule) -> Self {
        self.threshold_schedule = Some(schedule);
        self
    }
}

/// Covenant test specification with timing windows.
///
/// This is a serialization-friendly envelope used by higher-level tooling.
/// The `CovenantEngine` does not currently evaluate `CovenantTestSpec`
/// instances directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantTestSpec {
    /// Covenant specifications to test
    pub specs: Vec<CovenantSpec>,
    /// Test date
    pub test_date: Date,
    /// Reference date for calculating cure periods
    pub reference_date: Option<Date>,
}

/// Covenant window for scheduled testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantWindow {
    /// Start date of the window
    pub start: Date,
    /// End date of the window
    pub end: Date,
    /// Covenants active during this window
    pub covenants: Vec<CovenantSpec>,
}

/// Covenant breach tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovenantBreach {
    /// Stable identifier matching [`CovenantType::covenant_id`].
    #[serde(default)]
    pub covenant_id: String,
    /// Human-readable description (from `Display`).
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
    /// Active waivers and amendments
    #[serde(default)]
    pub waivers: Vec<CovenantWaiver>,
    /// Custom metric calculators.
    /// Not serializable - will be empty after deserialization.
    #[serde(skip)]
    pub custom_metrics: HashMap<String, CustomMetricCalculator>,
}

impl std::fmt::Debug for CovenantEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CovenantEngine")
            .field("specs", &self.specs)
            .field("breach_history", &self.breach_history)
            .field("windows", &self.windows)
            .field("waivers", &self.waivers)
            .field(
                "custom_metrics",
                &self.custom_metrics.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
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
            waivers: Vec::new(),
            custom_metrics: HashMap::default(),
        }
    }

    /// Add a covenant specification.
    pub fn add_spec(&mut self, spec: CovenantSpec) -> &mut Self {
        self.specs.push(spec);
        self
    }

    /// Add a covenant window.
    ///
    /// # Panics (debug builds)
    ///
    /// Panics via `debug_assert!` if the new window overlaps with an existing
    /// window. Windows must have non-overlapping date ranges to avoid ambiguity
    /// about which covenants apply on a given date.
    pub fn add_window(&mut self, window: CovenantWindow) -> &mut Self {
        debug_assert!(
            !self.windows.iter().any(|w| {
                window.start <= w.end && window.end >= w.start
            }),
            "Covenant windows must not overlap. New window [{}, {}] overlaps with an existing window.",
            window.start,
            window.end,
        );
        self.windows.push(window);
        self
    }

    /// Record a covenant waiver or amendment.
    pub fn add_waiver(&mut self, waiver: CovenantWaiver) -> &mut Self {
        self.waivers.push(waiver);
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

    /// Evaluate all covenants against current metrics (both maintenance and incurrence).
    ///
    /// Use [`evaluate_for_trigger`](Self::evaluate_for_trigger) to test only
    /// covenants matching a specific scope.
    pub fn evaluate(
        &self,
        context: &mut MetricContext,
        test_date: Date,
    ) -> finstack_core::Result<IndexMap<String, CovenantReport>> {
        let applicable_specs = self.get_applicable_specs_internal(test_date);
        self.evaluate_specs(&applicable_specs, context, test_date)
    }

    fn evaluate_specs(
        &self,
        specs: &[&CovenantSpec],
        context: &mut MetricContext,
        test_date: Date,
    ) -> finstack_core::Result<IndexMap<String, CovenantReport>> {
        tracing::debug!(spec_count = specs.len(), %test_date, "evaluating covenants");
        let mut reports = IndexMap::new();

        for spec in specs {
            let cid = spec.covenant.covenant_type.covenant_id();
            let description = spec.covenant.description();

            if !spec.covenant.is_active {
                reports.insert(
                    description.clone(),
                    CovenantReport::passed(&description)
                        .with_covenant_id(cid)
                        .with_details("Covenant inactive"),
                );
                continue;
            }

            if let Some(waiver) = self.active_waiver(cid, test_date) {
                if waiver.amended_threshold.is_none() {
                    tracing::info!(covenant_id = cid, %test_date, "covenant waived by lender agreement");
                    reports.insert(
                        description.clone(),
                        CovenantReport::passed(&description)
                            .with_covenant_id(cid)
                            .with_details("Waived by lender agreement"),
                    );
                    continue;
                }
            }

            let evaluation = self.evaluate_spec(spec, context, test_date)?;

            let mut report = if evaluation.passed {
                CovenantReport::passed(&description)
            } else {
                CovenantReport::failed(&description)
            };
            report = report.with_covenant_id(cid);

            if let Some(value) = evaluation.actual_value {
                report = report.with_actual(value);
            }
            if let Some(thresh) = evaluation.threshold {
                report = report.with_threshold(thresh);
            }
            if let Some(hr) = evaluation.headroom {
                report = report.with_headroom(hr);
            }

            if !evaluation.passed {
                tracing::warn!(
                    covenant_id = cid,
                    actual = evaluation.actual_value,
                    threshold = evaluation.threshold,
                    %test_date,
                    "covenant breach detected",
                );
                if let Some(breach) = self.find_active_breach(cid, test_date) {
                    if breach.cure_deadline.is_some_and(|d| test_date <= d) {
                        report = report.with_details("In cure period");
                    }
                }
            }

            if let Some(detail) = evaluation.detail {
                report = report.with_details(&detail);
            }

            reports.insert(description, report);
        }

        Ok(reports)
    }

    /// Evaluate only covenants matching the given trigger scope.
    ///
    /// `Maintenance` triggers test covenants with [`CovenantScope::Maintenance`].
    /// `Incurrence` triggers test covenants with [`CovenantScope::Incurrence`].
    /// This avoids the common error of testing incurrence covenants on a
    /// periodic schedule when they should only fire on specific actions.
    pub fn evaluate_for_trigger(
        &self,
        context: &mut MetricContext,
        test_date: Date,
        trigger: &EvaluationTrigger,
    ) -> finstack_core::Result<IndexMap<String, CovenantReport>> {
        let required_scope = match trigger {
            EvaluationTrigger::Maintenance => CovenantScope::Maintenance,
            EvaluationTrigger::Incurrence { .. } => CovenantScope::Incurrence,
        };

        let applicable_specs = self.get_applicable_specs_internal(test_date);
        let filtered: Vec<&CovenantSpec> = applicable_specs
            .into_iter()
            .filter(|s| s.covenant.scope == required_scope)
            .collect();

        self.evaluate_specs(&filtered, context, test_date)
    }

    /// Evaluate covenants and automatically record breaches in history.
    ///
    /// Combines [`evaluate`](Self::evaluate) with breach tracking: any failing
    /// covenant that doesn't already have an active (uncured) breach record
    /// gets a new [`CovenantBreach`] entry in `breach_history`.
    pub fn evaluate_and_track(
        &mut self,
        context: &mut MetricContext,
        test_date: Date,
    ) -> finstack_core::Result<IndexMap<String, CovenantReport>> {
        let reports = self.evaluate(context, test_date)?;

        for (description, report) in &reports {
            if report.passed {
                continue;
            }

            let cid = report.covenant_id.as_deref().unwrap_or("unknown");

            let already_tracked = self
                .breach_history
                .iter()
                .any(|b| b.covenant_id == cid && !b.is_cured && b.breach_date == test_date);
            if already_tracked {
                continue;
            }

            let spec = self
                .specs
                .iter()
                .find(|s| s.covenant.covenant_type.covenant_id() == cid);

            let cure_deadline = spec.and_then(|s| {
                s.covenant
                    .cure_period_days
                    .map(|d| test_date + time::Duration::days(d as i64))
            });

            tracing::warn!(
                covenant_id = cid,
                actual = report.actual_value,
                threshold = report.threshold,
                %test_date,
                "recording new covenant breach",
            );

            self.breach_history.push(CovenantBreach {
                covenant_id: cid.to_string(),
                covenant_type: description.clone(),
                breach_date: test_date,
                actual_value: report.actual_value,
                threshold: report.threshold,
                cure_deadline,
                is_cured: false,
                applied_consequences: Vec::new(),
            });
        }

        Ok(reports)
    }

    /// Apply consequences for breached covenants.
    ///
    /// Consequences that have already been applied (recorded in `breach_history`)
    /// are skipped to prevent double-application.
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
            if breach.is_cured {
                continue;
            }
            if let Some(deadline) = breach.cure_deadline {
                if as_of <= deadline {
                    continue;
                }
            }

            // Guard: skip if consequences were already applied for this breach
            let already_applied = self.breach_history.iter().any(|b| {
                b.covenant_id == breach.covenant_id
                    && b.breach_date == breach.breach_date
                    && !b.applied_consequences.is_empty()
            });
            if already_applied {
                tracing::debug!(
                    covenant_id = %breach.covenant_id,
                    breach_date = %breach.breach_date,
                    "skipping consequence application — already applied",
                );
                continue;
            }

            let spec = self
                .specs
                .iter()
                .find(|s| s.covenant.covenant_type.covenant_id() == breach.covenant_id)
                .ok_or(finstack_core::InputError::NotFound {
                    id: format!("covenant_spec:{}", breach.covenant_id),
                })?;

            for consequence in &spec.covenant.consequences {
                let application = self.apply_single_consequence(instrument, consequence, as_of)?;
                tracing::info!(
                    covenant_id = %breach.covenant_id,
                    consequence = %application.consequence_type,
                    %as_of,
                    "applied covenant consequence",
                );
                applications.push(application);

                if let Some(historical_breach) = self.breach_history.iter_mut().find(|b| {
                    b.covenant_id == breach.covenant_id && b.breach_date == breach.breach_date
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

    fn evaluate_spec(
        &self,
        spec: &CovenantSpec,
        context: &mut MetricContext,
        test_date: Date,
    ) -> finstack_core::Result<SpecEvaluation> {
        // Springing conditions: skip evaluation until activation criteria met.
        if let Some(condition) = &spec.covenant.springing_condition {
            let condition_value = self.get_metric_value(context, &condition.metric_id)?;
            let condition_met = match condition.test {
                ThresholdTest::Maximum(t) => condition_value <= t,
                ThresholdTest::Minimum(t) => condition_value >= t,
            };

            if !condition_met {
                tracing::debug!(
                    metric = condition.metric_id.as_str(),
                    value = condition_value,
                    "springing condition not met — covenant inactive",
                );
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

        let covenant_type = &spec.covenant.covenant_type;

        // Non-numeric covenants auto-pass until they have explicit evaluators.
        let Some(base_threshold) = covenant_type.threshold_value() else {
            return Ok(SpecEvaluation {
                passed: true,
                actual_value: None,
                threshold: None,
                headroom: None,
                detail: None,
            });
        };

        // Resolve the effective threshold: waiver amendment > schedule > static.
        let covenant_cid = covenant_type.covenant_id();
        let threshold = self
            .active_waiver(covenant_cid, test_date)
            .and_then(|w| w.amended_threshold)
            .or_else(|| {
                spec.threshold_schedule
                    .as_ref()
                    .and_then(|s| threshold_for_date(s, test_date))
            })
            .unwrap_or(base_threshold);

        // Otherwise use metric-based evaluation
        let metric_value = if let Some(metric_id) = &spec.metric_id {
            self.get_metric_value(context, metric_id)?
        } else if let Some(name) = covenant_type.default_metric_name() {
            self.get_metric_value(context, &MetricId::custom(name))?
        } else {
            match covenant_type {
                CovenantType::Custom { metric, .. } => {
                    self.get_metric_value(context, &MetricId::custom(metric))?
                }
                CovenantType::Basket { name, .. } => {
                    self.get_metric_value(context, &MetricId::custom(name))?
                }
                _ => unreachable!("Non-numeric covenants return early above"),
            }
        };

        let passed = match covenant_type.bound_kind() {
            Some(BoundKind::AtMost) => metric_value <= threshold,
            Some(BoundKind::AtLeast) => metric_value >= threshold,
            None => true,
        };

        let headroom = Some(headroom_for(
            covenant_type.bound_kind(),
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

        Err(finstack_core::InputError::NotFound {
            id: format!("metric:{}", metric_id.as_str()),
        }
        .into())
    }

    fn active_waiver(&self, covenant_id: &str, as_of: Date) -> Option<&CovenantWaiver> {
        self.waivers.iter().find(|w| {
            w.covenant_id == covenant_id
                && w.effective_date <= as_of
                && w.expiry_date.is_none_or(|exp| as_of <= exp)
        })
    }

    fn find_active_breach(&self, cid: &str, as_of: Date) -> Option<&CovenantBreach> {
        self.breach_history
            .iter()
            .filter(|b| b.covenant_id == cid && !b.is_cured)
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

pub(crate) fn headroom_for(bound: Option<BoundKind>, value: f64, threshold: f64) -> f64 {
    let denom = if threshold.abs() < f64::EPSILON {
        1.0
    } else {
        threshold
    };

    match bound {
        Some(BoundKind::AtMost) => (threshold - value) / denom,
        Some(BoundKind::AtLeast) => (value - threshold) / denom,
        None => 0.0,
    }
}

/// Result of applying a covenant consequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::{
        helpers,
        traits::{Attributes, Instrument},
    };
    use crate::pricer::InstrumentType;
    use finstack_core::{
        currency::Currency,
        dates::{Date, Tenor},
        market_data::context::MarketContext,
        money::Money,
    };
    use std::sync::Arc;
    use time::{Duration, Month};

    #[test]
    fn covenant_type_helper_mappings_cover_variants() {
        let leverage = CovenantType::MaxTotalLeverage { threshold: 5.0 };
        assert_eq!(leverage.bound_kind(), Some(BoundKind::AtMost));
        assert_eq!(leverage.threshold_value(), Some(5.0));
        assert_eq!(leverage.default_metric_name(), Some("total_leverage"));
        assert!((headroom_for(leverage.bound_kind(), 4.0, 5.0) - 0.2).abs() < 1e-12);

        let coverage = CovenantType::MinInterestCoverage { threshold: 1.50 };
        assert_eq!(coverage.bound_kind(), Some(BoundKind::AtLeast));
        assert_eq!(coverage.threshold_value(), Some(1.50));
        assert_eq!(coverage.default_metric_name(), Some("interest_coverage"));
        assert!((headroom_for(coverage.bound_kind(), 2.0, 1.5) - (2.0 - 1.5) / 1.5).abs() < 1e-12);

        let custom_max = CovenantType::Custom {
            metric: "liquidity_ratio".to_string(),
            test: ThresholdTest::Maximum(1.1),
        };
        assert_eq!(custom_max.bound_kind(), Some(BoundKind::AtMost));
        assert_eq!(custom_max.threshold_value(), Some(1.1));
        assert_eq!(custom_max.default_metric_name(), None);

        let custom_min = CovenantType::Custom {
            metric: "dscr".to_string(),
            test: ThresholdTest::Minimum(1.2),
        };
        assert_eq!(custom_min.bound_kind(), Some(BoundKind::AtLeast));
        assert_eq!(custom_min.threshold_value(), Some(1.2));

        let basket = CovenantType::Basket {
            name: "general_debt".to_string(),
            limit: 100.0,
        };
        assert_eq!(basket.bound_kind(), Some(BoundKind::AtMost));
        assert_eq!(basket.threshold_value(), Some(100.0));
        assert_eq!(basket.default_metric_name(), None);

        let negative = CovenantType::Negative {
            restriction: "No additional debt".to_string(),
        };
        assert_eq!(negative.bound_kind(), None);
        assert_eq!(negative.threshold_value(), None);
        assert_eq!(negative.default_metric_name(), None);
        assert_eq!(headroom_for(negative.bound_kind(), 1.0, 1.0), 0.0);
    }

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

    crate::impl_empty_cashflow_provider!(
        TestInstrument,
        crate::cashflow::builder::CashflowRepresentation::NoResidual
    );

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

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
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
            options: crate::instruments::common_impl::traits::PricingOptions,
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            let base_value = self.value(curves, as_of)?;
            helpers::build_with_metrics_dyn(
                std::sync::Arc::new(self.clone()),
                std::sync::Arc::new(curves.clone()),
                as_of,
                base_value,
                metrics,
                helpers::MetricBuildOptions {
                    cfg: options.config,
                    market_history: options.market_history,
                    ..helpers::MetricBuildOptions::default()
                },
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
            MetricContext::default_config(),
        )
    }

    #[test]
    fn evaluate_financial_covenants_with_cure_periods() {
        let mut engine = CovenantEngine::new();

        let leverage_cov = Covenant::new(
            CovenantType::MaxTotalLeverage { threshold: 5.0 },
            Tenor::quarterly(),
        );
        let coverage_cov = Covenant::new(
            CovenantType::MinInterestCoverage { threshold: 1.50 },
            Tenor::quarterly(),
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
            covenant_id: "min_interest_coverage".to_string(),
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
            Tenor::annual(),
        );
        let neg_description = negative_cov.covenant_type.to_string();
        assert_eq!(neg_description, "Negative: No additional debt");
    }

    #[test]
    fn evaluate_respects_windows_and_custom_sources() {
        let mut engine = CovenantEngine::new();

        // Base spec that should be ignored when window is active.
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxDebtToEBITDA { threshold: 4.0 },
                Tenor::quarterly(),
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
                Tenor::quarterly(),
            ),
            MetricId::custom("liquidity_ratio"),
        );

        let evaluator_spec = CovenantSpec::with_evaluator(
            Covenant::new(
                CovenantType::Affirmative {
                    requirement: "Provide quarterly reporting".to_string(),
                },
                Tenor::quarterly(),
            ),
            |_ctx| Ok(false),
        );

        let liquidity_desc = custom_metric_spec.covenant.description();
        let affirmative_desc = evaluator_spec.covenant.description();

        engine.add_window(CovenantWindow {
            start: date(2025, 1, 1),
            end: date(2025, 6, 30),
            covenants: vec![custom_metric_spec, evaluator_spec],
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
            .map(|spec| spec.covenant.description())
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
            Tenor::quarterly(),
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
            covenant_id: "max_senior_leverage".to_string(),
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(10),
            actual_value: Some(3.8),
            threshold: Some(3.0),
            cure_deadline: Some(as_of - Duration::days(1)),
            is_cured: false,
            applied_consequences: Vec::new(),
        });

        let actionable_breach = CovenantBreach {
            covenant_id: "max_senior_leverage".to_string(),
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(10),
            actual_value: Some(3.8),
            threshold: Some(3.0),
            cure_deadline: Some(as_of - Duration::days(1)),
            is_cured: false,
            applied_consequences: Vec::new(),
        };

        let cured_breach = CovenantBreach {
            covenant_id: "max_senior_leverage".to_string(),
            covenant_type: "Senior Leverage <= 3.00x".to_string(),
            breach_date: as_of - Duration::days(40),
            actual_value: Some(3.4),
            threshold: Some(3.0),
            cure_deadline: None,
            is_cured: true,
            applied_consequences: Vec::new(),
        };

        let in_cure_breach = CovenantBreach {
            covenant_id: "max_senior_leverage".to_string(),
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
            Tenor::quarterly(),
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
            Tenor::quarterly(),
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
            .get("general_debt Utilization <= 100.00")
            .expect("basket covenant present");
        assert!(report.passed);
        assert_eq!(report.headroom, Some(0.20));

        ctx.computed.insert(MetricId::custom("general_debt"), 120.0);
        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("general_debt Utilization <= 100.00")
            .expect("basket covenant present");
        assert!(!report.passed);
        assert!(report
            .headroom
            .is_some_and(|h| h < 0.0 && (h + 0.20).abs() < 1e-6));
    }

    #[test]
    fn evaluate_and_track_creates_breach_records() {
        let mut engine = CovenantEngine::new();
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxTotalLeverage { threshold: 5.0 },
                Tenor::quarterly(),
            )
            .with_cure_period(Some(30)),
            MetricId::custom("total_leverage"),
        ));

        let test_date = date(2025, 6, 30);
        let instrument = TestInstrument::new("TRACK-TEST", date(2026, 6, 30));
        let mut ctx = metric_context(&instrument, test_date);
        ctx.computed.insert(MetricId::custom("total_leverage"), 5.5);

        assert!(engine.breach_history.is_empty());
        let reports = engine
            .evaluate_and_track(&mut ctx, test_date)
            .expect("evaluation succeeds");
        assert!(!reports["Total Leverage <= 5.00x"].passed);
        assert_eq!(engine.breach_history.len(), 1);
        assert_eq!(engine.breach_history[0].covenant_id, "max_total_leverage");
        assert_eq!(engine.breach_history[0].actual_value, Some(5.5));
        assert!(engine.breach_history[0].cure_deadline.is_some());

        // Second call on same date should not duplicate
        let _ = engine
            .evaluate_and_track(&mut ctx, test_date)
            .expect("evaluation succeeds");
        assert_eq!(engine.breach_history.len(), 1);
    }

    #[test]
    fn evaluate_for_trigger_filters_by_scope() {
        let mut engine = CovenantEngine::new();
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxTotalLeverage { threshold: 5.0 },
                Tenor::quarterly(),
            )
            .with_scope(CovenantScope::Maintenance),
            MetricId::custom("total_leverage"),
        ));
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxSeniorLeverage { threshold: 3.0 },
                Tenor::quarterly(),
            )
            .with_scope(CovenantScope::Incurrence),
            MetricId::custom("senior_leverage"),
        ));

        let test_date = date(2025, 6, 30);
        let instrument = TestInstrument::new("TRIGGER-TEST", date(2026, 6, 30));
        let mut ctx = metric_context(&instrument, test_date);
        ctx.computed.insert(MetricId::custom("total_leverage"), 6.0);
        ctx.computed
            .insert(MetricId::custom("senior_leverage"), 4.0);

        let maintenance_reports = engine
            .evaluate_for_trigger(&mut ctx, test_date, &EvaluationTrigger::Maintenance)
            .expect("evaluation succeeds");
        assert_eq!(maintenance_reports.len(), 1);
        assert!(maintenance_reports.contains_key("Total Leverage <= 5.00x"));

        let incurrence_reports = engine
            .evaluate_for_trigger(
                &mut ctx,
                test_date,
                &EvaluationTrigger::Incurrence {
                    action: "New debt issuance".to_string(),
                },
            )
            .expect("evaluation succeeds");
        assert_eq!(incurrence_reports.len(), 1);
        assert!(incurrence_reports.contains_key("Senior Leverage <= 3.00x"));
    }

    #[test]
    fn waiver_bypasses_covenant_evaluation() {
        let mut engine = CovenantEngine::new();
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxTotalLeverage { threshold: 5.0 },
                Tenor::quarterly(),
            ),
            MetricId::custom("total_leverage"),
        ));

        let test_date = date(2025, 6, 30);

        // Full waiver (no amended threshold)
        engine.add_waiver(CovenantWaiver {
            covenant_id: "max_total_leverage".to_string(),
            effective_date: date(2025, 1, 1),
            expiry_date: Some(date(2025, 12, 31)),
            amended_threshold: None,
            description: "Temporary waiver for Q2-Q4".to_string(),
        });

        let instrument = TestInstrument::new("WAIVER-TEST", date(2026, 6, 30));
        let mut ctx = metric_context(&instrument, test_date);
        ctx.computed.insert(MetricId::custom("total_leverage"), 7.0);

        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("Total Leverage <= 5.00x")
            .expect("leverage covenant present");
        assert!(report.passed, "should pass due to waiver");
        assert_eq!(
            report.details.as_deref(),
            Some("Waived by lender agreement")
        );
    }

    #[test]
    fn waiver_amendment_overrides_threshold() {
        let mut engine = CovenantEngine::new();
        engine.add_spec(CovenantSpec::with_metric(
            Covenant::new(
                CovenantType::MaxTotalLeverage { threshold: 5.0 },
                Tenor::quarterly(),
            ),
            MetricId::custom("total_leverage"),
        ));

        let test_date = date(2025, 6, 30);

        // Amendment: raise threshold from 5.0 to 7.0
        engine.add_waiver(CovenantWaiver {
            covenant_id: "max_total_leverage".to_string(),
            effective_date: date(2025, 1, 1),
            expiry_date: Some(date(2025, 12, 31)),
            amended_threshold: Some(7.0),
            description: "Amended leverage threshold".to_string(),
        });

        let instrument = TestInstrument::new("AMEND-TEST", date(2026, 6, 30));
        let mut ctx = metric_context(&instrument, test_date);
        ctx.computed.insert(MetricId::custom("total_leverage"), 6.0);

        let reports = engine
            .evaluate(&mut ctx, test_date)
            .expect("evaluation succeeds");
        let report = reports
            .get("Total Leverage <= 5.00x")
            .expect("leverage covenant present");
        assert!(report.passed, "6.0 should pass with amended 7.0 threshold");
        assert_eq!(report.threshold, Some(7.0));
    }
}
