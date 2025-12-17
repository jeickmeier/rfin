//! Solver helpers and common penalty/diagnostics utilities for calibration.
//!
//! This module intentionally contains the implementation logic that `calibration/mod.rs`
//! re-exports. Keeping it here allows `mod.rs` to stay export-only.

use finstack_core::Result;

use crate::calibration::CalibrationConfig;
use crate::calibration::solver::SolverConfig;

/// Finite penalty value used in objective functions instead of infinity.
///
/// Using a moderate large finite value (1e6) helps solvers behave more predictably
/// than extremely large values like 1e12, which can cause numerical instability
/// with gradient-based methods. The value is chosen to be:
/// - Large enough to clearly indicate failure/infeasibility
/// - Small enough to avoid gradient explosion issues
/// - Proportional to typical financial quantities (notional-normalized PVs)
pub const PENALTY: f64 = 1e6;

/// Maximum absolute objective value treated as "valid" during bracketing scans.
///
/// Values with `|f(x)| >= OBJECTIVE_VALID_ABS_MAX` are treated as penalized/infeasible
/// during the scan phase (but are still counted toward total evaluations).
pub const OBJECTIVE_VALID_ABS_MAX: f64 = PENALTY / 10.0;

/// Minimum absolute residual value treated as a "penalty" for reporting/diagnostics.
///
/// This aligns with `CalibrationReport` which excludes penalty-like residuals from RMSE/max
/// when non-penalty values exist.
pub const RESIDUAL_PENALTY_ABS_MIN: f64 = PENALTY * 0.5;

/// Solve a 1D root-finding problem using the configured solver kind.
///
/// This replaces the former `with_solver!` macro with a plain helper function
/// to make control flow explicit and IDE-friendly.
pub fn solve_1d<Fun>(solver: &SolverConfig, f: Fun, init: f64) -> Result<f64>
where
    Fun: Fn(f64) -> f64,
{
    use finstack_core::math::{BrentSolver, Solver};

    match solver {
        SolverConfig::Newton { solver } => solver.solve(f, init),
        SolverConfig::Brent { solver } => solver.solve(f, init),
        // For multi-dimensional kinds, fall back to Brent for 1D problems
        SolverConfig::GlobalNewton {
            tolerance,
            max_iterations,
            ..
        } => {
            let solver = BrentSolver::new()
                .with_tolerance(*tolerance)
                .with_max_iterations(*max_iterations);
            solver.solve(f, init)
        }
    }
}

/// Diagnostics from bracketing scan, useful for error reporting.
#[derive(Debug, Clone)]
pub struct BracketDiagnostics {
    /// Whether a sign-change bracket was found
    pub bracket_found: bool,
    /// Best candidate point (minimum |f|)
    pub best_point: Option<f64>,
    /// Best objective value (minimum |f|)
    pub best_value: Option<f64>,
    /// Number of objective evaluations performed
    pub eval_count: usize,
    /// Number of valid (non-penalized) evaluations
    pub valid_eval_count: usize,
    /// Scan bounds used [lo, hi]
    pub scan_bounds: (f64, f64),
}

impl BracketDiagnostics {
    fn new(scan_points: &[f64]) -> Self {
        let lo = scan_points.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = scan_points
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        Self {
            bracket_found: false,
            best_point: None,
            best_value: None,
            eval_count: 0,
            valid_eval_count: 0,
            scan_bounds: (lo, hi),
        }
    }

    fn update(&mut self, point: f64, value: f64) {
        self.eval_count += 1;
        if value.is_finite() && value.abs() < OBJECTIVE_VALID_ABS_MAX {
            self.valid_eval_count += 1;
            let is_better = match self.best_value {
                None => true,
                Some(best) => value.abs() < best.abs(),
            };
            if is_better {
                self.best_point = Some(point);
                self.best_value = Some(value);
            }
        }
    }
}

/// Like `bracket_solve_1d` but also returns diagnostics for error reporting.
pub(crate) fn bracket_solve_1d_with_diagnostics(
    objective: &dyn Fn(f64) -> f64,
    initial: f64,
    scan_points: &[f64],
    tol: f64,
    max_iters: usize,
) -> Result<(Option<f64>, BracketDiagnostics)> {
    let mut diag = BracketDiagnostics::new(scan_points);

    let v0 = objective(initial);
    diag.update(initial, v0);
    if v0.is_finite() && v0.abs() < tol {
        diag.bracket_found = true;
        return Ok((Some(initial), diag));
    }

    let mut last_valid: Option<(f64, f64)> = None;
    for &point in scan_points {
        let value = objective(point);
        diag.update(point, value);

        if !value.is_finite() || value.abs() >= OBJECTIVE_VALID_ABS_MAX {
            continue;
        }

        if let Some((prev_point, prev_value)) = last_valid {
            if prev_value == 0.0 {
                diag.bracket_found = true;
                return Ok((Some(prev_point), diag));
            }
            if value == 0.0 {
                diag.bracket_found = true;
                return Ok((Some(point), diag));
            }
            if prev_value.signum() != value.signum() {
                // Market-standard: bracket is valid; converge primarily on f-space (|f| < tol).
                // We prefer a simple bisection on the bracket to guarantee reduction in |f|
                // for well-behaved monotone objectives. If midpoints become invalid/penalized,
                // we fall back to Brent+Newton.
                let mut a = prev_point;
                let mut b = point;
                let mut fa = prev_value;

                let mut bisection_ok = true;
                for _ in 0..max_iters.max(50) {
                    let m = 0.5 * (a + b);
                    let fm = objective(m);
                    diag.update(m, fm);

                    if fm.is_finite() && fm.abs() < tol {
                        diag.bracket_found = true;
                        return Ok((Some(m), diag));
                    }

                    if !fm.is_finite() || fm.abs() >= OBJECTIVE_VALID_ABS_MAX {
                        bisection_ok = false;
                        break;
                    }

                    if fa.signum() != fm.signum() {
                        b = m;
                    } else {
                        a = m;
                        fa = fm;
                    }
                }

                if bisection_ok {
                    // If we didn't meet tol, return best observed point (if any).
                    if let (Some(best_point), Some(best_value)) = (diag.best_point, diag.best_value)
                    {
                        if best_value.is_finite() && best_value.abs() < tol {
                            diag.bracket_found = true;
                            return Ok((Some(best_point), diag));
                        }
                    }
                }

                // Fallback: robust Brent (x-space) + Newton polish (f-space) from bracket midpoint.
                let guess = 0.5 * (prev_point + point);
                let solver_brent = SolverConfig::brent_default()
                    .with_tolerance(tol)
                    .with_max_iterations(max_iters.max(50));
                let root_brent = solve_1d(&solver_brent, objective, guess)?;
                let fb2 = objective(root_brent);
                diag.update(root_brent, fb2);
                if fb2.is_finite() && fb2.abs() < tol {
                    diag.bracket_found = true;
                    return Ok((Some(root_brent), diag));
                }
                let solver_newton = SolverConfig::newton_default()
                    .with_tolerance(tol)
                    .with_max_iterations(max_iters.max(50));
                if let Ok(root_newton) = solve_1d(&solver_newton, objective, root_brent) {
                    let fnv = objective(root_newton);
                    diag.update(root_newton, fnv);
                    if fnv.is_finite() && fnv.abs() < tol {
                        diag.bracket_found = true;
                        return Ok((Some(root_newton), diag));
                    }
                }

                let root = root_brent;
                diag.bracket_found = true;
                return Ok((Some(root), diag));
            }
        }

        last_valid = Some((point, value));
    }

    Ok((None, diag))
}

/// Create a simple solver wrapper for calibration methods using `solve_1d` internally.
pub fn create_simple_solver(config: &CalibrationConfig) -> impl finstack_core::math::Solver {
    struct SimpleSolver {
        solver: SolverConfig,
    }

    impl finstack_core::math::Solver for SimpleSolver {
        fn solve<Fun>(&self, f: Fun, initial_guess: f64) -> finstack_core::Result<f64>
        where
            Fun: Fn(f64) -> f64,
        {
            solve_1d(&self.solver, f, initial_guess)
        }
    }

    SimpleSolver {
        solver: config.solver.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_solve_1d_finds_root() {
        // f(x) = x - 0.5 has root at 0.5
        let f = |x: f64| x - 0.5;
        let scan = [-1.0, 0.0, 0.25, 0.75, 1.0];
        let (root, _) =
            bracket_solve_1d_with_diagnostics(&f, 0.0, &scan, 1e-12, 100).expect("solver error");
        let r = root.expect("root should be Some");
        assert!((r - 0.5).abs() < 1e-9, "root inaccurate: {}", r);
    }

    #[test]
    fn test_bracket_diagnostics_tracking() {
        // f(x) = x - 0.5 has root at 0.5
        let f = |x: f64| x - 0.5;
        let scan = [0.0, 0.25, 0.5, 0.75, 1.0];
        let (root, diag) =
            bracket_solve_1d_with_diagnostics(&f, 0.3, &scan, 1e-12, 100).expect("solver error");

        assert!(root.is_some());
        assert!(diag.bracket_found);
        // At least 1 eval (initial) + some scan points before finding bracket
        assert!(diag.eval_count >= 1, "eval_count={}", diag.eval_count);
        assert!(
            diag.valid_eval_count >= 1,
            "valid_eval_count={}",
            diag.valid_eval_count
        );
        assert_eq!(diag.scan_bounds, (0.0, 1.0));
    }

    #[test]
    fn test_bracket_diagnostics_no_bracket() {
        // f(x) = x^2 + 1 has no real root
        let f = |x: f64| x * x + 1.0;
        let scan = [0.0, 0.5, 1.0, 1.5, 2.0];
        let (root, diag) =
            bracket_solve_1d_with_diagnostics(&f, 1.0, &scan, 1e-12, 100).expect("solver error");

        assert!(root.is_none());
        assert!(!diag.bracket_found);
        assert!(diag.eval_count >= 5);
        // Best point should be at x=0 where f(0)=1 is minimum
        assert!(diag.best_point.is_some());
        assert!((diag.best_point.expect("best_point asserted above") - 0.0).abs() < 0.01);
        assert!((diag.best_value.expect("best_value should exist") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_bracket_diagnostics_penalized_values() {
        // f(x) returns PENALTY for x < 0.5, otherwise x - 0.5
        let f = |x: f64| if x < 0.5 { PENALTY } else { x - 0.75 };
        let scan = [0.0, 0.25, 0.5, 0.75, 1.0];
        let (root, diag) =
            bracket_solve_1d_with_diagnostics(&f, 0.5, &scan, 1e-12, 100).expect("solver error");

        // Should find root at 0.75
        assert!(root.is_some());
        // Only values >= 0.5 are valid (not penalized)
        assert!(diag.valid_eval_count < diag.eval_count);
    }
}
