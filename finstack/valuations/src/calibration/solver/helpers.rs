//! Solver helpers and common penalty/diagnostics utilities for calibration.
//!
//! This module intentionally contains the implementation logic that `calibration/mod.rs`
//! re-exports. Keeping it here allows `mod.rs` to stay export-only.

use crate::calibration::constants::OBJECTIVE_VALID_ABS_MAX;
use crate::calibration::CalibrationConfig;

/// Finite-difference bump step used by both bootstrap and global diagnostics.
/// Returns `max(config.discount_curve.jacobian_step_size, 1e-8)`.
pub(crate) fn diagnostics_bump_h(config: &CalibrationConfig) -> f64 {
    config.discount_curve.jacobian_step_size.max(1e-8)
}

/// `(max_abs_residual, rms_residual)` over a residual vector — shared between the
/// bootstrap and global diagnostics computations so the two paths cannot drift.
pub(crate) fn residual_stats(resid_values: &[f64]) -> (f64, f64) {
    let max_residual = resid_values.iter().map(|r| r.abs()).fold(0.0_f64, f64::max);
    let rms_residual = if resid_values.is_empty() {
        0.0
    } else {
        (resid_values.iter().map(|r| r * r).sum::<f64>() / resid_values.len() as f64).sqrt()
    };
    (max_residual, rms_residual)
}
#[cfg(test)]
use crate::calibration::constants::PENALTY;
use finstack_core::Result;

/// Diagnostics from bracketing scan, useful for error reporting.
///
/// Tracks the effectiveness of the initial scan grid and identifies the
/// best points observed if formal convergence fails.
#[derive(Debug, Clone)]
pub(crate) struct BracketDiagnostics {
    /// Whether a sign-change bracket was found.
    pub bracket_found: bool,
    /// Best candidate point (minimum |f|) observed during the scan.
    pub best_point: Option<f64>,
    /// Best objective value (minimum |f|) observed during the scan.
    pub best_value: Option<f64>,
    /// Total number of objective evaluations performed.
    pub eval_count: usize,
    /// Number of valid (non-penalized, non-NaN) evaluations.
    pub valid_eval_count: usize,
    /// Scan bounds used by the grid search [lo, hi].
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
    let mut valid_points: Vec<(f64, f64)> = Vec::with_capacity(scan_points.len() + 8);

    let v0 = objective(initial);
    diag.update(initial, v0);
    if v0.is_finite() && v0.abs() < tol {
        diag.bracket_found = true;
        return Ok((Some(initial), diag));
    }
    if v0.is_finite() && v0.abs() < OBJECTIVE_VALID_ABS_MAX {
        valid_points.push((initial, v0));
    }

    for &point in scan_points {
        let value = objective(point);
        diag.update(point, value);

        if !value.is_finite() || value.abs() >= OBJECTIVE_VALID_ABS_MAX {
            continue;
        }
        valid_points.push((point, value));
    }

    if valid_points.is_empty() {
        return Ok((None, diag));
    }

    // Robust bracket selection:
    // sort by x and choose the bracket whose midpoint is closest to the initial guess.
    valid_points.sort_by(|a, b| a.0.total_cmp(&b.0));
    type Bracket = ((f64, f64), (f64, f64), f64); // ((x0,f0),(x1,f1),score)
    let mut best_bracket: Option<Bracket> = None;
    for w in valid_points.windows(2) {
        let (x0, f0) = w[0];
        let (x1, f1) = w[1];
        if f0.abs() < tol {
            diag.bracket_found = true;
            return Ok((Some(x0), diag));
        }
        if f1.abs() < tol {
            diag.bracket_found = true;
            return Ok((Some(x1), diag));
        }
        if f0.signum() == f1.signum() {
            continue;
        }
        let mid = 0.5 * (x0 + x1);
        let score = (mid - initial).abs();
        let replace = match &best_bracket {
            None => true,
            Some((_, _, best_score)) => score < *best_score,
        };
        if replace {
            best_bracket = Some(((x0, f0), (x1, f1), score));
        }
    }

    let Some(((mut a, mut fa), (mut b, mut fb), _)) = best_bracket else {
        // No sign-change found. Try a bounded Newton fallback from the best observed point.
        if let Some(x0) = diag.best_point {
            let mut x = x0;
            let lo = diag.scan_bounds.0;
            let hi = diag.scan_bounds.1;
            let iters = max_iters.clamp(50, 200);

            for _ in 0..iters {
                let fx = objective(x);
                diag.update(x, fx);
                if fx.is_finite() && fx.abs() < tol {
                    diag.bracket_found = true;
                    return Ok((Some(x), diag));
                }
                if !fx.is_finite() || fx.abs() >= OBJECTIVE_VALID_ABS_MAX {
                    break;
                }

                // Finite-difference derivative (central difference).
                let h = (1e-6_f64).max(1e-6 * x.abs());
                let x_lo = (x - h).clamp(lo, hi);
                let x_hi = (x + h).clamp(lo, hi);
                if (x_hi - x_lo).abs() < 1e-16 {
                    break;
                }
                let f_lo = objective(x_lo);
                let f_hi = objective(x_hi);
                diag.update(x_lo, f_lo);
                diag.update(x_hi, f_hi);
                if !f_lo.is_finite() || !f_hi.is_finite() {
                    break;
                }
                let dfdx = (f_hi - f_lo) / (x_hi - x_lo);
                if !dfdx.is_finite() || dfdx.abs() < 1e-16 {
                    break;
                }

                let x_next = (x - fx / dfdx).clamp(lo, hi);
                if !x_next.is_finite() || (x_next - x).abs() < 1e-16 {
                    break;
                }
                x = x_next;
            }
        }

        return Ok((None, diag));
    };

    // Market-standard: bracket is valid; converge primarily on f-space (|f| < tol).
    // We prefer a simple bisection on the bracket to guarantee reduction in |f|
    // for well-behaved monotone objectives. If midpoints become invalid/penalized,
    // we fall back to Brent+Newton.
    //
    // X-space early-break: when the bracket width collapses to machine precision
    // bisection cannot further improve the candidate, but the false-position
    // fallback below may still reduce |f| via secant-style updates inside the
    // tight bracket. So we `break` (don't return) and let the fallback polish.
    let x_tol_abs = 1e-14_f64;
    let x_tol_rel = 1e-12_f64;
    for _ in 0..max_iters.max(50) {
        let m = 0.5 * (a + b);
        let fm = objective(m);
        diag.update(m, fm);

        if fm.is_finite() && fm.abs() < tol {
            diag.bracket_found = true;
            return Ok((Some(m), diag));
        }

        if !fm.is_finite() || fm.abs() >= OBJECTIVE_VALID_ABS_MAX {
            // Midpoint produced a penalized/infeasible value. Stop bisecting; leave
            // (a,fa,b,fb) at their last good values so the false-position fallback
            // below still has a valid sign-changing bracket.
            break;
        }

        if fa.signum() != fm.signum() {
            b = m;
            fb = fm;
        } else {
            a = m;
            fa = fm;
        }

        let bracket_width = b - a;
        let x_tol = x_tol_abs.max(x_tol_rel * (a.abs().max(b.abs())));
        if bracket_width <= x_tol {
            break;
        }
    }

    // If we already met tolerance, return the best observed point.
    if let (Some(best_point), Some(best_value)) = (diag.best_point, diag.best_value) {
        if best_value.is_finite() && best_value.abs() < tol {
            diag.bracket_found = true;
            return Ok((Some(best_point), diag));
        }
    }

    // The false-position fallback requires a valid sign-changing bracket. After bisection
    // either: (a) we converged (handled above), (b) bisection_ok = false because a midpoint
    // produced |f| >= OBJECTIVE_VALID_ABS_MAX and `(a,fa,b,fb)` are last-known-good, or
    // (c) we ran out of iterations with a still-valid bracket. In all three cases the
    // invariant `fa.signum() != fb.signum()` should hold; guard explicitly so that an
    // edge case (e.g. fa or fb became NaN upstream) cannot drive FP on a stale bracket.
    let bracket_valid = fa.is_finite() && fb.is_finite() && fa.signum() != fb.signum() && a < b;
    if !bracket_valid {
        return Ok((diag.best_point, diag));
    }

    // Fallback: stay inside the discovered bracket with bounded false-position updates.
    for _ in 0..max_iters.max(50) {
        let denom = fb - fa;
        let mut candidate = if denom.is_finite() && denom.abs() > f64::EPSILON {
            a - fa * (b - a) / denom
        } else {
            0.5 * (a + b)
        };
        if !candidate.is_finite() || candidate <= a || candidate >= b {
            candidate = 0.5 * (a + b);
        }

        let fc = objective(candidate);
        diag.update(candidate, fc);
        if fc.is_finite() && fc.abs() < tol {
            diag.bracket_found = true;
            return Ok((Some(candidate), diag));
        }
        if !fc.is_finite() || fc.abs() >= OBJECTIVE_VALID_ABS_MAX {
            break;
        }

        if fa.signum() != fc.signum() {
            b = candidate;
            fb = fc;
        } else if fc.signum() != fb.signum() {
            a = candidate;
            fa = fc;
        } else {
            break;
        }
    }

    Ok((None, diag))
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
    fn bracket_solver_evaluates_authoritative_scan_grid() {
        use std::cell::RefCell;

        let seen = RefCell::new(Vec::new());
        let f = |x: f64| {
            seen.borrow_mut().push(x);
            x - 0.5
        };
        let scan = [0.0, 0.25, 0.5, 0.75, 42.0];
        let (root, _) =
            bracket_solve_1d_with_diagnostics(&f, 0.49, &scan, 1e-12, 100).expect("solver error");

        assert!(root.is_some());
        assert!(
            seen.borrow().contains(&42.0),
            "solver must evaluate the caller-supplied scan grid before selecting a bracket"
        );
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

    #[test]
    fn test_bracket_fallback_stays_inside_discovered_domain() {
        let f = |x: f64| {
            if !(0.0..=1.0).contains(&x) || (0.30..0.70).contains(&x) {
                PENALTY
            } else {
                x - 0.15
            }
        };
        let scan = [0.0, 1.0];
        let (root, diag) =
            bracket_solve_1d_with_diagnostics(&f, 0.8, &scan, 1e-12, 100).expect("solver error");

        let root = root.expect("bounded fallback should recover the bracketed root");
        assert!(
            (root - 0.15).abs() < 1e-8,
            "unexpected root from bounded fallback: {root}"
        );
        assert!(diag.bracket_found, "fallback root should be bracket-safe");
    }
}
