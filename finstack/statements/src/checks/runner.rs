//! Check runner that executes a collection of checks and produces a report.

use super::traits::{Check, CheckContext};
use super::types::{CheckReport, CheckResult, CheckSummary, Severity};
use crate::evaluator::StatementResult;
use crate::types::FinancialModelSpec;
use crate::Result;

/// Runs a set of [`Check`] implementations against a model and its results.
pub struct CheckRunner {
    checks: Vec<Box<dyn Check>>,
}

impl CheckRunner {
    /// Create an empty runner.
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Register a check.
    pub fn add_check(&mut self, check: impl Check + 'static) -> &mut Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Execute all registered checks and produce a [`CheckReport`].
    pub fn run(
        &self,
        model: &FinancialModelSpec,
        results: &StatementResult,
    ) -> Result<CheckReport> {
        let context = CheckContext::new(model, results);
        let mut check_results: Vec<CheckResult> = Vec::with_capacity(self.checks.len());

        for check in &self.checks {
            check_results.push(check.execute(&context)?);
        }

        let summary = build_summary(&check_results);
        Ok(CheckReport {
            results: check_results,
            summary,
        })
    }
}

impl Default for CheckRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a [`CheckSummary`] from a slice of results.
pub(crate) fn build_summary(results: &[CheckResult]) -> CheckSummary {
    let total_checks = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total_checks - passed;

    let mut errors: usize = 0;
    let mut warnings: usize = 0;
    let mut infos: usize = 0;

    for finding in results.iter().flat_map(|r| &r.findings) {
        match finding.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }

    CheckSummary {
        total_checks,
        passed,
        failed,
        errors,
        warnings,
        infos,
    }
}
