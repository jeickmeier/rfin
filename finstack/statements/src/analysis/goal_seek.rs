//! Goal seek functionality for financial models.
//!
//! This module provides root-finding capabilities to solve for input drivers
//! that achieve specific target metric values. It wraps the core solver infrastructure
//! around the statement evaluator.
//!
//! # Examples
//!
//! ```rust
//! use finstack_statements::prelude::*;
//! use finstack_statements::analysis::goal_seek;
//! use finstack_core::dates::PeriodId;
//!
//! # fn main() -> Result<()> {
//! let mut model = ModelBuilder::new("goal_seek_test")
//!     .periods("2025Q1..Q4", None)?
//!     .value("revenue", &[
//!         (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
//!     ])
//!     .forecast("revenue", ForecastSpec::growth(0.05))
//!     .compute("interest_expense", "10000.0")?
//!     .compute("ebitda", "revenue * 0.3")?
//!     .compute("interest_coverage", "ebitda / interest_expense")?
//!     .build()?;
//!
//! // Solve for Q4 revenue that achieves 2.0x interest coverage
//! let target_period = PeriodId::quarter(2025, 4);
//! let solved_revenue = goal_seek(
//!     &mut model,
//!     "interest_coverage",
//!     target_period,
//!     2.0,
//!     "revenue",
//!     target_period,
//!     true,  // Update model with solution
//!     None,
//! )?;
//!
//! println!("Revenue needed: ${:.2}", solved_revenue);
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::evaluator::Evaluator;
use crate::types::{AmountOrScalar, FinancialModelSpec};
use finstack_core::dates::PeriodId;
use finstack_core::math::solver::{BrentSolver, Solver};

/// Perform goal seek on a financial model.
///
/// Solves for the driver node value that achieves a target metric value in a specific period.
/// This uses Brent's method for robust root-finding.
///
/// # Arguments
///
/// * `model` - Mutable reference to the financial model
/// * `target_node` - Node identifier for the target metric
/// * `target_period` - Period in which to evaluate the target
/// * `target_value` - Desired value for the target metric
/// * `driver_node` - Node identifier for the driver input to vary
/// * `driver_period` - Period in which to vary the driver
/// * `update_model` - If true, update the model with the solved driver value
/// * `bounds` - Optional `(lower, upper)` bracket to constrain the search
///
/// # Returns
///
/// Returns the solved driver value that achieves the target, or an error if no solution exists.
///
/// # Errors
///
/// Returns an error if:
/// - The target or driver node doesn't exist
/// - The specified periods are not in the model
/// - No solution exists within reasonable bounds
/// - The model evaluation fails
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_statements::prelude::*;
/// use finstack_statements::analysis::goal_seek;
/// use finstack_core::dates::PeriodId;
///
/// # fn main() -> Result<()> {
/// let mut model = ModelBuilder::new("example")
///     .periods("2025Q1", None)?
///     .value("revenue", &[(PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0))])
///     .compute("profit_margin", "0.15")?
///     .compute("net_income", "revenue * profit_margin")?
///     .build()?;
///
/// // Verify model evaluates correctly first
/// let mut evaluator = Evaluator::new();
/// let results = evaluator.evaluate(&model)?;
/// let initial_net_income = results.get("net_income", &PeriodId::quarter(2025, 1)).unwrap();
/// assert!((initial_net_income - 15_000.0).abs() < 0.01);
///
/// // Solve for revenue that achieves $18,000 net income (closer to initial value for better convergence)
/// let period = PeriodId::quarter(2025, 1);
/// let solved = goal_seek(&mut model, "net_income", period, 18_000.0, "revenue", period, false, None)?;
/// // Expected: 18_000 / 0.15 = 120_000
/// assert!((solved - 120_000.0).abs() < 10.0);
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn goal_seek(
    model: &mut FinancialModelSpec,
    target_node: &str,
    target_period: PeriodId,
    target_value: f64,
    driver_node: &str,
    driver_period: PeriodId,
    update_model: bool,
    bounds: Option<(f64, f64)>,
) -> Result<f64> {
    // Validate that nodes exist
    if !model.has_node(target_node) {
        return Err(Error::invalid_input(format!(
            "Target node '{}' not found in model",
            target_node
        )));
    }

    if !model.has_node(driver_node) {
        return Err(Error::invalid_input(format!(
            "Driver node '{}' not found in model",
            driver_node
        )));
    }

    // Validate that periods exist
    if !model.periods.iter().any(|p| p.id == target_period) {
        return Err(Error::invalid_input(format!(
            "Target period '{}' not found in model",
            target_period
        )));
    }

    if !model.periods.iter().any(|p| p.id == driver_period) {
        return Err(Error::invalid_input(format!(
            "Driver period '{}' not found in model",
            driver_period
        )));
    }

    // Get initial guess from current driver value if available.
    //
    // # Initial Guess Strategy
    //
    // When the driver node has an existing value for the target period, we use that
    // as the initial guess. This provides a reasonable starting point that is likely
    // close to the solution.
    //
    // When no existing value is available, we use a fallback of 1.0 rather than an
    // arbitrary large number. This is because:
    // - For ratio/percentage drivers (e.g., margins), 1.0 is a reasonable neutral value
    // - For absolute drivers (e.g., revenue), the adaptive bounds logic below will
    //   expand the search range based on abs(initial_guess), so starting at 1.0
    //   gives a ±10.0 initial bracket which is then refined
    // - Using 0.0 would cause division-based bounds to collapse
    //
    // For better convergence on specific use cases, callers should either:
    // 1. Ensure the driver node has a value for the period, or
    // 2. Provide explicit bounds via the `bounds` parameter
    let initial_guess = model
        .get_node(driver_node)
        .and_then(|node| node.values.as_ref())
        .and_then(|values| values.get(&driver_period))
        .map(|v| match v {
            AmountOrScalar::Scalar(s) => *s,
            AmountOrScalar::Amount(a) => a.amount(),
        })
        .unwrap_or(1.0); // Default initial guess (see rationale above)

    // Create the objective function
    let objective = |driver_value: f64| -> f64 {
        // Clone the model to avoid modifying it during search
        let mut temp_model = model.clone();

        // Update driver value
        if let Some(node) = temp_model.nodes.get_mut(driver_node) {
            let mut values = node.values.clone().unwrap_or_default();
            values.insert(driver_period, AmountOrScalar::scalar(driver_value));
            node.values = Some(values);
        }

        // Evaluate the model
        let mut evaluator = Evaluator::new();
        match evaluator.evaluate(&temp_model) {
            Ok(results) => {
                // Get the target value
                match results.get(target_node, &target_period) {
                    Some(actual_value) => actual_value - target_value,
                    None => {
                        // If we can't get the value, return a large error
                        1e10
                    }
                }
            }
            Err(_) => {
                // If evaluation fails, return a large error
                1e10
            }
        }
    };

    if let Some((lower, upper)) = bounds {
        let solution = solve_with_bounds(&objective, lower, upper)?;
        return apply_solution(model, driver_node, driver_period, update_model, solution);
    }

    // Derive adaptive bounds when none supplied
    let (auto_lower, auto_upper) = {
        let abs_guess = initial_guess.abs().max(1.0);
        (
            initial_guess - abs_guess * 10.0,
            initial_guess + abs_guess * 10.0,
        )
    };
    let mut solver = BrentSolver::new();
    let bracket_size = ((auto_upper - auto_lower).abs() / 2.0).max(1e-6);
    solver.initial_bracket_size = Some(bracket_size);
    let clamped_guess = initial_guess.clamp(auto_lower, auto_upper);

    let solution = solver.solve(objective, clamped_guess).map_err(|e| {
        Error::eval(format!(
            "Goal seek failed to find solution: target_node='{}', target_value={}, driver_node='{}'. {}",
            target_node, target_value, driver_node, e
        ))
    })?;

    apply_solution(model, driver_node, driver_period, update_model, solution)
}

fn apply_solution(
    model: &mut FinancialModelSpec,
    driver_node: &str,
    driver_period: PeriodId,
    update_model: bool,
    value: f64,
) -> Result<f64> {
    if update_model {
        if let Some(node) = model.nodes.get_mut(driver_node) {
            let mut values = node.values.clone().unwrap_or_default();
            values.insert(driver_period, AmountOrScalar::scalar(value));
            node.values = Some(values);
        }
    }
    Ok(value)
}

fn solve_with_bounds<F>(f: &F, lower: f64, upper: f64) -> Result<f64>
where
    F: Fn(f64) -> f64,
{
    const MAX_ITER: usize = 128;
    const TOLERANCE: f64 = 1e-9;

    if !lower.is_finite() || !upper.is_finite() {
        return Err(Error::invalid_input(
            "Goal seek bounds must be finite values",
        ));
    }

    if lower >= upper {
        return Err(Error::invalid_input(
            "Goal seek lower bound must be less than upper bound",
        ));
    }

    let mut lo = lower;
    let mut hi = upper;
    let mut flo = f(lo);
    let fhi = f(hi);

    if !flo.is_finite() || !fhi.is_finite() {
        return Err(Error::eval(
            "Goal seek bounds produced non-finite objective values",
        ));
    }

    if flo == 0.0 {
        return Ok(lo);
    }
    if fhi == 0.0 {
        return Ok(hi);
    }

    if flo * fhi > 0.0 {
        return Err(Error::eval(format!(
            "Goal seek bounds [{:.4}, {:.4}] do not bracket a root",
            lower, upper
        )));
    }

    for _ in 0..MAX_ITER {
        let mid = 0.5 * (lo + hi);
        let fmid = f(mid);

        if !fmid.is_finite() {
            return Err(Error::eval(
                "Goal seek produced non-finite value within bounds",
            ));
        }

        // Use combined absolute + relative tolerance so that:
        // - For small values, absolute tolerance governs
        // - For large values (e.g., billions), relative tolerance governs
        let relative_tol = TOLERANCE * (1.0 + hi.abs().max(lo.abs()));
        if fmid.abs() < TOLERANCE || (hi - lo).abs() < relative_tol {
            return Ok(mid);
        }

        if flo * fmid < 0.0 {
            hi = mid;
        } else {
            lo = mid;
            flo = fmid;
        }
    }

    Err(Error::eval(
        "Goal seek failed to converge within provided bounds",
    ))
}
