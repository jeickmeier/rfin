//! Named suites of checks with filtering and merge support.

use super::traits::{Check, CheckContext};
use super::types::{CheckConfig, CheckReport, CheckResult, Severity};
use crate::evaluator::StatementResult;
use crate::types::FinancialModelSpec;
use crate::Result;

/// A named, self-contained collection of checks with its own configuration.
pub struct CheckSuite {
    /// Suite name for display/logging.
    name: String,
    /// Optional description.
    description: Option<String>,
    /// Checks in this suite.
    checks: Vec<Box<dyn Check>>,
    /// Configuration applied when running the suite.
    config: CheckConfig,
}

impl CheckSuite {
    /// Start building a new suite.
    pub fn builder(name: impl Into<String>) -> CheckSuiteBuilder {
        CheckSuiteBuilder {
            name: name.into(),
            description: None,
            checks: Vec::new(),
            config: CheckConfig::default(),
        }
    }

    /// Merge another suite's checks into this one, consuming the other suite.
    pub fn merge(mut self, other: CheckSuite) -> Self {
        self.checks.extend(other.checks);
        self
    }

    /// Execute all checks in the suite, applying `min_severity` and
    /// `materiality_threshold` filters from the config.
    pub fn run(
        &self,
        model: &FinancialModelSpec,
        results: &StatementResult,
    ) -> Result<CheckReport> {
        let context = CheckContext::with_config(model, results, self.config.clone());
        self.run_internal(&context)
    }

    /// Number of checks in the suite.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// True when the suite contains no checks.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// Suite name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Suite description, if set.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    // ------------------------------------------------------------------
    // Internal
    // ------------------------------------------------------------------

    fn run_internal(&self, context: &CheckContext) -> Result<CheckReport> {
        let min_severity = context.config.min_severity;
        let mat_threshold = context.config.materiality_threshold;

        let mut filtered_results: Vec<CheckResult> = Vec::with_capacity(self.checks.len());

        for check in &self.checks {
            let mut result = check.execute(context)?;

            result.findings.retain(|f| {
                if f.severity < min_severity {
                    return false;
                }
                if mat_threshold > 0.0 {
                    if let Some(ref m) = f.materiality {
                        if m.absolute.abs() < mat_threshold {
                            return false;
                        }
                    }
                }
                true
            });

            result.passed = !result
                .findings
                .iter()
                .any(|f| f.severity == Severity::Error);

            filtered_results.push(result);
        }

        let summary = crate::checks::runner::build_summary(&filtered_results);
        Ok(CheckReport {
            results: filtered_results,
            summary,
        })
    }
}

/// Fluent builder for [`CheckSuite`].
pub struct CheckSuiteBuilder {
    name: String,
    description: Option<String>,
    checks: Vec<Box<dyn Check>>,
    config: CheckConfig,
}

impl CheckSuiteBuilder {
    /// Set the suite description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a check to the suite.
    pub fn add_check(mut self, check: impl Check + 'static) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Override the default configuration.
    pub fn config(mut self, config: CheckConfig) -> Self {
        self.config = config;
        self
    }

    /// Consume the builder and produce a [`CheckSuite`].
    pub fn build(self) -> CheckSuite {
        CheckSuite {
            name: self.name,
            description: self.description,
            checks: self.checks,
            config: self.config,
        }
    }
}
