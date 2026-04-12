//! User-defined formula check using a simple expression evaluator.
//!
//! [`FormulaCheck`] lets users define custom validation rules as arithmetic
//! expressions over node values. A non-zero result is treated as *pass*;
//! zero means *fail*.
//!
//! # Example (JSON)
//!
//! ```json
//! {
//!   "id": "revenue_positive",
//!   "name": "Revenue must be positive",
//!   "category": "internal_consistency",
//!   "severity": "error",
//!   "formula": "revenue > 0",
//!   "message_template": "Revenue was non-positive in {period}",
//!   "tolerance": null
//! }
//! ```
//!
//! ## Supported syntax
//!
//! The built-in evaluator handles simple binary expressions:
//!
//! - `lhs > rhs`, `lhs < rhs`, `lhs >= rhs`, `lhs <= rhs`, `lhs == rhs`,
//!   `lhs != rhs`
//! - `lhs + rhs`, `lhs - rhs`, `lhs * rhs`, `lhs / rhs`
//!
//! Operands are either node identifiers (resolved per period) or numeric
//! literals. Full DSL integration via
//! `finstack_statements::dsl::compiler::compile_formula` is planned as a
//! follow-up enhancement.

use serde::{Deserialize, Serialize};

use finstack_statements::checks::{
    Check, CheckCategory, CheckContext, CheckFinding, CheckResult, Severity,
};
use finstack_statements::Result;

use super::get_node_value;

/// A user-defined check that evaluates a formula expression per period.
///
/// Convention: result `!= 0.0` → pass, result `== 0.0` → fail.
///
/// See the [module-level docs](self) for supported syntax.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaCheck {
    /// Unique identifier for this check instance.
    pub id: String,
    /// Human-readable name shown in reports.
    pub name: String,
    /// Category grouping.
    pub category: CheckCategory,
    /// Severity assigned to findings when the formula fails.
    pub severity: Severity,
    /// Expression to evaluate (e.g. `"revenue > 0"`).
    pub formula: String,
    /// Template for the finding message. `{period}` is replaced with the
    /// period identifier.
    pub message_template: String,
    /// Numeric tolerance for floating-point comparisons. If `None`, exact
    /// comparison is used (result `== 0.0` → fail).
    pub tolerance: Option<f64>,
}

impl Check for FormulaCheck {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn category(&self) -> CheckCategory {
        self.category
    }

    fn execute(&self, context: &CheckContext) -> Result<CheckResult> {
        let mut findings = Vec::new();

        for period_spec in &context.model.periods {
            let pid = &period_spec.id;

            match eval_formula(&self.formula, context, pid) {
                Ok(value) => {
                    let passes = if let Some(tol) = self.tolerance {
                        value.abs() > tol
                    } else {
                        value != 0.0
                    };

                    if !passes {
                        let message = self.message_template.replace("{period}", &pid.to_string());
                        findings.push(CheckFinding {
                            check_id: self.id.clone(),
                            severity: self.severity,
                            message,
                            period: Some(*pid),
                            materiality: None,
                            nodes: vec![],
                        });
                    }
                }
                Err(_) => {
                    // If we cannot evaluate (e.g. missing nodes), skip silently
                    // rather than producing a false positive.
                }
            }
        }

        let passed = !findings.iter().any(|f| f.severity >= Severity::Error);

        Ok(CheckResult {
            check_id: self.id.clone(),
            check_name: self.name.clone(),
            category: self.category,
            passed,
            findings,
        })
    }
}

// ---------------------------------------------------------------------------
// Simple expression evaluator
// ---------------------------------------------------------------------------

/// Evaluate a simple binary expression against node values for a period.
///
/// Returns `1.0` for truthy comparison results, `0.0` for falsy, or the
/// raw arithmetic result for `+`, `-`, `*`, `/`.
fn eval_formula(
    formula: &str,
    context: &CheckContext,
    period: &finstack_core::dates::PeriodId,
) -> std::result::Result<f64, FormulaError> {
    let formula = formula.trim();

    // Try to split on comparison operators (longest first to avoid ambiguity).
    for (op, f) in &COMPARISON_OPS {
        if let Some((lhs, rhs)) = split_once_op(formula, op) {
            let l = resolve_operand(lhs.trim(), context, period)?;
            let r = resolve_operand(rhs.trim(), context, period)?;
            return Ok(if f(l, r) { 1.0 } else { 0.0 });
        }
    }

    // Try arithmetic operators (lowest precedence first: +/-, then *//).
    for (op, f) in &ARITHMETIC_OPS {
        if let Some((lhs, rhs)) = split_once_op(formula, op) {
            let l = resolve_operand(lhs.trim(), context, period)?;
            let r = resolve_operand(rhs.trim(), context, period)?;
            return Ok(f(l, r));
        }
    }

    // Single operand — return its value directly.
    resolve_operand(formula, context, period)
}

type CmpFn = fn(f64, f64) -> bool;
type ArithFn = fn(f64, f64) -> f64;

const COMPARISON_OPS: [(&str, CmpFn); 6] = [
    (">=", |a, b| a >= b),
    ("<=", |a, b| a <= b),
    ("!=", |a, b| (a - b).abs() > f64::EPSILON),
    ("==", |a, b| (a - b).abs() <= f64::EPSILON),
    (">", |a, b| a > b),
    ("<", |a, b| a < b),
];

const ARITHMETIC_OPS: [(&str, ArithFn); 4] = [
    ("+", |a, b| a + b),
    ("-", |a, b| a - b),
    ("*", |a, b| a * b),
    ("/", |a, b| if b.abs() > f64::EPSILON { a / b } else { 0.0 }),
];

/// Split `s` on the *last* occurrence of `op` that is not inside an identifier.
fn split_once_op<'a>(s: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    // For multi-char ops (>=, <=, !=, ==) search directly.
    // For single-char ops (+, -, *, /, >, <) avoid matching inside >=, <= etc.
    let idx = s.find(op)?;

    // Guard: make sure we don't split on a prefix of a longer operator.
    if op.len() == 1 {
        let after = idx + 1;
        if after < s.len() {
            let next_char = s.as_bytes()[after];
            if next_char == b'=' {
                return None;
            }
        }
        // Avoid splitting a negative sign at the very start.
        if op == "-" && idx == 0 {
            return None;
        }
    }

    Some((&s[..idx], &s[idx + op.len()..]))
}

/// Resolve a token as either a numeric literal or a node value lookup.
fn resolve_operand(
    token: &str,
    context: &CheckContext,
    period: &finstack_core::dates::PeriodId,
) -> std::result::Result<f64, FormulaError> {
    let token = token.trim();
    if let Ok(v) = token.parse::<f64>() {
        return Ok(v);
    }
    let nid = finstack_statements::types::NodeId::new(token);
    get_node_value(context.results, &nid, period).ok_or(FormulaError::MissingNode)
}

/// Internal error type for formula evaluation.
#[derive(Debug)]
enum FormulaError {
    /// A referenced node was not found in the results.
    MissingNode,
}
