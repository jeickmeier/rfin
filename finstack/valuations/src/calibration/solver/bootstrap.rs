//! Generic sequential bootstrapping algorithm.

use super::bracket_solve_1d_with_diagnostics;
use super::helpers::BracketDiagnostics;
use super::traits::BootstrapTarget;
use crate::calibration::constants::{OBJECTIVE_VALID_ABS_MAX, RESIDUAL_PENALTY_ABS_MIN};
use crate::calibration::{CalibrationConfig, CalibrationReport};
use finstack_core::Result;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

// =============================================================================
// Helper Types
// =============================================================================

/// A quote reference with its computed time, used for sorting.
struct SortedQuote {
    time: f64,
    original_idx: usize,
}

/// Context for resolving what to do when no bracket is found.
struct NoBracketContext<'a, Q> {
    time: f64,
    quote: &'a Q,
    diag: &'a BracketDiagnostics,
    validation_tolerance: f64,
    first_eval_error: Option<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Sort quotes by time, validating each quote produces a valid time.
fn sort_quotes_by_time<T: BootstrapTarget>(
    target: &T,
    quotes: &[T::Quote],
) -> Result<Vec<SortedQuote>> {
    let mut quote_times = Vec::with_capacity(quotes.len());
    for (idx, quote) in quotes.iter().enumerate() {
        let time = target
            .quote_time(quote)
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Bootstrap failed to compute quote_time for quote index {idx}: {e}"
                ),
                category: "bootstrapping".to_string(),
            })?;
        validate_quote_time(time, idx)?;
        quote_times.push(SortedQuote {
            time,
            original_idx: idx,
        });
    }
    quote_times.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.original_idx.cmp(&b.original_idx))
    });
    Ok(quote_times)
}

/// Validate that a quote time is finite and positive.
fn validate_quote_time(time: f64, idx: usize) -> Result<()> {
    if !time.is_finite() || time <= 0.0 {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap quote_time must be finite and > 0; got t={time} (quote index {idx})"
            ),
            category: "bootstrapping".to_string(),
        });
    }
    Ok(())
}

/// Validate that quote times are strictly increasing.
fn validate_time_ordering(time: f64, last_time: f64, original_idx: usize) -> Result<()> {
    if time < last_time {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap requires increasing quote times; got t={time:.12} after last_time={last_time:.12} (quote index {original_idx})"
            ),
            category: "bootstrapping".to_string(),
        });
    }
    if (time - last_time).abs() <= crate::calibration::constants::TOLERANCE_DUP_KNOTS {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap rejects duplicate quote times: t={time:.12} appears more than once (quote index {original_idx})"
            ),
            category: "bootstrapping".to_string(),
        });
    }
    Ok(())
}

/// Build the default geometric scan grid around an initial guess.
fn build_default_scan_grid(initial_guess: f64, config: &CalibrationConfig) -> Vec<f64> {
    let center = if initial_guess.is_finite() {
        initial_guess
    } else {
        0.0
    };

    let step0 = (config.discount_curve.scan_grid_step * (1.0 + center.abs())).max(1e-8);
    let grid_size = config.discount_curve.scan_grid_points;
    let mut pts = Vec::with_capacity(2 * grid_size + 1);
    pts.push(center);
    let mut step = step0;
    for _ in 0..grid_size {
        pts.push(center - step);
        pts.push(center + step);
        step *= 2.0;
    }
    pts
}

/// Normalize and deduplicate scan points.
fn normalize_scan_points(mut points: Vec<f64>, initial_guess: f64, time: f64) -> Result<Vec<f64>> {
    points.retain(|x| x.is_finite());
    if initial_guess.is_finite() {
        points.push(initial_guess);
    }
    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    points.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

    if points.is_empty() {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap failed at t={time:.6}: scan_points is empty after filtering non-finite values."
            ),
            category: "bootstrapping".to_string(),
        });
    }
    Ok(points)
}

/// Resolve the solved value when no sign-change bracket was found.
fn resolve_no_bracket<Q: std::fmt::Debug>(ctx: NoBracketContext<'_, Q>) -> Result<f64> {
    // Check if best point is within tolerance
    if let (Some(best_x), Some(best_f)) = (ctx.diag.best_point, ctx.diag.best_value) {
        if best_f.is_finite() && best_f.abs() <= ctx.validation_tolerance {
            return Ok(best_x);
        }
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap failed at t={:.6} (quote={:?}): no bracket found and best |residual|={:.3e} exceeds tolerance={:.3e} (scan_bounds=[{:.3e}, {:.3e}])",
                ctx.time,
                ctx.quote,
                best_f.abs(),
                ctx.validation_tolerance,
                ctx.diag.scan_bounds.0,
                ctx.diag.scan_bounds.1
            ),
            category: "bootstrapping".to_string(),
        });
    }

    // All evaluations invalid
    if ctx.diag.valid_eval_count == 0 {
        let hint = ctx.first_eval_error.unwrap_or_else(|| {
            "no error recorded (all evaluations penalized or non-finite)".to_string()
        });
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap failed at t={:.6}: all {} objective evaluations returned invalid/penalized values (|f| >= {:.3e}). First error: {}",
                ctx.time, ctx.diag.eval_count, OBJECTIVE_VALID_ABS_MAX, hint
            ),
            category: "bootstrapping".to_string(),
        });
    }

    // Valid evaluations but no bracket
    Err(finstack_core::Error::Calibration {
        message: format!(
            "Bootstrap failed at t={:.6}: no bracket found despite {} valid evaluations (scan_bounds=[{:.3e}, {:.3e}]).",
            ctx.time,
            ctx.diag.valid_eval_count,
            ctx.diag.scan_bounds.0,
            ctx.diag.scan_bounds.1
        ),
        category: "bootstrapping".to_string(),
    })
}

/// Validate the final residual after solving.
fn validate_residual(time: f64, residual: f64, tolerance: f64) -> Result<()> {
    let abs = residual.abs();
    if !residual.is_finite() || abs >= RESIDUAL_PENALTY_ABS_MIN {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap converged to invalid/penalty residual at t={time:.6}: residual={residual} (|.|={abs:.3e})"
            ),
            category: "bootstrapping".to_string(),
        });
    }
    if abs > tolerance {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Bootstrap failed to converge at t={time:.6}: residual={residual} (|.|={abs:.3e}) exceeds tolerance={tolerance:.3e}"
            ),
            category: "bootstrapping".to_string(),
        });
    }
    Ok(())
}

/// Validate solved value and commit to knots, returning the residual.
fn validate_and_commit_knot<T: BootstrapTarget>(
    target: &T,
    knots: &mut Vec<(f64, f64)>,
    time: f64,
    solved_value: f64,
    quote: &T::Quote,
    validation_tolerance: f64,
) -> Result<f64> {
    target.validate_knot(time, solved_value)?;

    // PERF: avoid `knots.clone()` by temporarily pushing the candidate knot and popping
    // on error. This keeps the hot loop allocation-free while preserving correctness.
    knots.push((time, solved_value));
    let result = (|| {
        let curve = target.build_curve_for_solver(knots)?;
        let residual = target.calculate_residual(&curve, quote)?;
        validate_residual(time, residual, validation_tolerance)?;
        Ok(residual)
    })();

    if result.is_err() {
        knots.pop();
    }
    result
}

/// Record a calibration iteration in the trace.
fn record_iteration(
    trace: &mut finstack_core::explain::ExplanationTrace,
    sorted_idx: usize,
    time: f64,
    residual: f64,
    validation_tolerance: f64,
    config: &CalibrationConfig,
) {
    use finstack_core::explain::TraceEntry;
    trace.push(
        TraceEntry::CalibrationIteration {
            iteration: sorted_idx,
            residual,
            knots_updated: vec![format!("t={time:.4}")],
            converged: residual.abs() <= validation_tolerance,
        },
        config.explain.max_entries,
    );
}

// =============================================================================
// Main Bootstrapper
// =============================================================================

/// Generic sequential bootstrapper.
///
/// Implements a robust sequential bootstrapping algorithm that iterates through
/// a sorted list of market quotes and solves for each curve/surface knot
/// independently. This is the industry standard for liquid interest rate
/// and credit curves where causality (independence of knots at earlier times)
/// is preserved.
///
/// The algorithm uses a hybrid bracketing-plus-polishing approach:
/// 1. **Scan**: Evaluates the objective on a grid to find a sign-change bracket.
/// 2. **Bracket**: If no bracket is found, fall back to initial guess or best point.
/// 3. **Solve**: Use Brent's method (bracketing) for robustness followed by optional
///    Newton-Raphson polishing for high-precision convergence in f-space.
pub struct SequentialBootstrapper;

impl SequentialBootstrapper {
    /// Execute the sequential bootstrapping algorithm.
    ///
    /// # Generic Parameters
    /// * `T` - The calibration target (e.g., [`DiscountCurveTarget`](crate::calibration::targets::discount::DiscountCurveTarget)).
    ///
    /// # Arguments
    /// * `target` - The domain-specific implementation of the [`BootstrapTarget`] trait.
    /// * `quotes` - The list of high-level market quotes to fit.
    /// * `initial_knots` - Optional pre-existing knots (e.g., spot or short-end anchors).
    /// * `config` - Calibration settings specifying tolerances and methods.
    /// * `success_tolerance` - Target-specific validation tolerance for determining calibration success.
    ///   If `None`, falls back to `config.solver.tolerance()`.
    /// * `trace` - Optional trace for collecting diagnostics and intermediate steps.
    ///
    /// # Returns
    /// A pair containing the calibrated term structure and a diagnostic report.
    pub fn bootstrap<T>(
        target: &T,
        quotes: &[T::Quote],
        initial_knots: Vec<(f64, f64)>,
        config: &CalibrationConfig,
        success_tolerance: Option<f64>,
        mut trace: Option<finstack_core::explain::ExplanationTrace>,
    ) -> Result<(T::Curve, CalibrationReport)>
    where
        T: BootstrapTarget,
        T::Quote: std::fmt::Debug,
    {
        let validation_tolerance = success_tolerance.unwrap_or(config.solver.tolerance());
        let sorted_quotes = sort_quotes_by_time(target, quotes)?;

        let mut knots = initial_knots;
        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0;
        let mut last_time = knots.iter().map(|(t, _)| *t).fold(0.0_f64, f64::max);

        for (sorted_idx, sq) in sorted_quotes.into_iter().enumerate() {
            validate_time_ordering(sq.time, last_time, sq.original_idx)?;
            let quote = &quotes[sq.original_idx];
            let time = sq.time;

            let (solved_value, eval_count) =
                Self::solve_single_knot(target, quote, &knots, time, config, validation_tolerance)?;

            let residual = validate_and_commit_knot(
                target,
                &mut knots,
                time,
                solved_value,
                quote,
                validation_tolerance,
            )?;

            total_iterations += eval_count;
            last_time = time;
            residuals.insert(format!("quote_{sorted_idx:06}"), residual);

            if let Some(t) = &mut trace {
                record_iteration(t, sorted_idx, time, residual, validation_tolerance, config);
            }
        }

        let final_curve = target.build_curve_final(&knots)?;
        let report = CalibrationReport::for_type_with_tolerance(
            "generic_bootstrap",
            residuals,
            total_iterations,
            validation_tolerance,
        );
        let report = match trace {
            Some(t) => report.with_explanation(t),
            None => report,
        };

        Ok((final_curve, report))
    }

    /// Solve for a single knot value using bracket + polish.
    ///
    /// Returns `(solved_value, eval_count)`.
    fn solve_single_knot<T>(
        target: &T,
        quote: &T::Quote,
        knots: &[(f64, f64)],
        time: f64,
        config: &CalibrationConfig,
        validation_tolerance: f64,
    ) -> Result<(f64, usize)>
    where
        T: BootstrapTarget,
        T::Quote: std::fmt::Debug,
    {
        let initial_guess = target.initial_guess(quote, knots)?;

        // Track the first evaluation error for diagnostics
        let first_eval_error: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        let eval_counter = AtomicUsize::new(0);

        // Optimization: reuse buffer to avoid allocation in hot loop
        let reuse_buffer = std::cell::RefCell::new(Vec::with_capacity(knots.len() + 1));

        // Define objective function
        let objective = |value: f64| -> f64 {
            let eval_idx = eval_counter.fetch_add(1, Ordering::Relaxed) + 1;

            let mut temp_knots_guard = reuse_buffer.borrow_mut();
            temp_knots_guard.clear();
            temp_knots_guard.extend_from_slice(knots);
            temp_knots_guard.push((time, value));
            let temp_knots = &*temp_knots_guard;

            let curve = match target.build_curve_for_solver(temp_knots) {
                Ok(c) => c,
                Err(e) => {
                    if first_eval_error.borrow().is_none() {
                        *first_eval_error.borrow_mut() = Some(format!(
                            "eval#{eval_idx} curve construction failed at value={value}: {e}"
                        ));
                    }
                    return f64::NAN;
                }
            };

            match target.calculate_residual(&curve, quote) {
                Ok(r) => r,
                Err(e) => {
                    if first_eval_error.borrow().is_none() {
                        *first_eval_error.borrow_mut() = Some(format!(
                            "eval#{eval_idx} residual evaluation failed at value={value}: {e}"
                        ));
                    }
                    f64::NAN
                }
            }
        };

        // Build scan grid
        let scan_points = {
            let points = target.scan_points(quote, initial_guess)?;
            if points.is_empty() {
                build_default_scan_grid(initial_guess, config)
            } else {
                points
            }
        };
        let scan_points = normalize_scan_points(scan_points, initial_guess, time)?;

        // Solve
        let (tentative, diag) = bracket_solve_1d_with_diagnostics(
            &objective,
            initial_guess,
            &scan_points,
            config.solver.tolerance(),
            config.solver.max_iterations(),
        )?;

        let solved_value = match tentative {
            Some(root) => root,
            None => resolve_no_bracket(NoBracketContext {
                time,
                quote,
                diag: &diag,
                validation_tolerance,
                first_eval_error: first_eval_error.borrow().clone(),
            })?,
        };

        Ok((solved_value, diag.eval_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::solver::traits::BootstrapTarget;

    #[derive(Clone, Debug)]
    struct DummyTarget;

    #[derive(Clone, Debug, PartialEq)]
    struct DummyCurve(Vec<(f64, f64)>);

    impl BootstrapTarget for DummyTarget {
        type Quote = (f64, f64);
        type Curve = DummyCurve;

        fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
            Ok(quote.0)
        }

        fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
            Ok(DummyCurve(knots.to_vec()))
        }

        fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
            let current = curve
                .0
                .iter()
                .find(|(t, _)| (*t - quote.0).abs() < 1e-12)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);
            Ok(current - quote.1)
        }

        fn initial_guess(
            &self,
            quote: &Self::Quote,
            _previous_knots: &[(f64, f64)],
        ) -> Result<f64> {
            Ok(quote.1)
        }
    }

    #[test]
    fn bootstrap_is_order_invariant() -> Result<()> {
        let target = DummyTarget;
        let quotes = vec![(0.5, 0.01), (1.0, 0.015), (2.0, 0.02)];
        let mut shuffled = quotes.clone();
        shuffled.reverse();

        let config = CalibrationConfig::default();
        let (curve_a, _) =
            SequentialBootstrapper::bootstrap(&target, &quotes, Vec::new(), &config, None, None)?;
        let (curve_b, _) =
            SequentialBootstrapper::bootstrap(&target, &shuffled, Vec::new(), &config, None, None)?;

        assert_eq!(curve_a, curve_b);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod solver_tests {
    use super::*;
    use finstack_core::Error;

    #[derive(Clone, Debug)]
    struct DummyQuote {
        t: f64,
        root: f64,
        scale: f64,
        unsorted_scan: bool,
        infeasible_below: Option<f64>,
    }

    struct DummyTarget;

    impl BootstrapTarget for DummyTarget {
        type Quote = DummyQuote;
        type Curve = f64;

        fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
            Ok(quote.t)
        }

        fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
            knots
                .last()
                .map(|(_, v)| *v)
                .ok_or(Error::Input(finstack_core::InputError::TooFewPoints))
        }

        fn build_curve_for_solver(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
            self.build_curve(knots)
        }

        fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
            if let Some(th) = quote.infeasible_below {
                if *curve < th {
                    return Err(Error::Calibration {
                        message: format!("infeasible curve value {}", curve),
                        category: "test".to_string(),
                    });
                }
            }
            // Residual is scaled in f-space to test tolerance enforcement.
            Ok(quote.scale * (*curve - quote.root))
        }

        fn initial_guess(
            &self,
            _quote: &Self::Quote,
            _previous_knots: &[(f64, f64)],
        ) -> Result<f64> {
            Ok(0.0)
        }

        fn scan_points(&self, quote: &Self::Quote, _initial_guess: f64) -> Result<Vec<f64>> {
            let base = vec![-1.0, 0.0, 0.25, 0.75, 1.0];
            if quote.unsorted_scan {
                Ok(vec![1.0, 0.0, 0.75, 0.25, -1.0])
            } else {
                Ok(base)
            }
        }

        fn validate_knot(&self, _time: f64, value: f64) -> Result<()> {
            if !value.is_finite() {
                return Err(Error::Calibration {
                    message: "non-finite knot".to_string(),
                    category: "test".to_string(),
                });
            }
            Ok(())
        }
    }

    #[test]
    fn bootstrap_succeeds_with_unsorted_scan_points() {
        let target = DummyTarget;
        let q = DummyQuote {
            t: 1.0,
            root: 0.5,
            scale: 1.0,
            unsorted_scan: true,
            infeasible_below: None,
        };
        let cfg = CalibrationConfig {
            solver: crate::calibration::solver::SolverConfig::brent_default()
                .with_tolerance(1e-10)
                .with_max_iterations(200),
            ..CalibrationConfig::default()
        };
        let (curve, report) =
            SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None, None)
                .expect("bootstrap should succeed");
        assert!((curve - 0.5).abs() < 1e-6);
        assert!(report.success);
    }

    #[test]
    fn bootstrap_succeeds_with_infeasible_trial_points() {
        // Some objective evaluations error out (infeasible region), but a valid root exists.
        let target = DummyTarget;
        let q = DummyQuote {
            t: 1.0,
            root: 0.5,
            scale: 1.0,
            unsorted_scan: false,
            infeasible_below: Some(0.0),
        };
        let cfg = CalibrationConfig {
            solver: crate::calibration::solver::SolverConfig::brent_default()
                .with_tolerance(1e-10)
                .with_max_iterations(200),
            ..CalibrationConfig::default()
        };
        let (curve, report) =
            SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None, None)
                .expect("bootstrap should succeed despite infeasible points");
        assert!((curve - 0.5).abs() < 1e-6);
        assert!(report.success);
    }

    #[test]
    fn bootstrap_rejects_when_all_objective_evals_are_penalized() {
        // Extremely steep residuals can exceed the objective validity cap used by the
        // bracketing diagnostics. In that case, we should fail with a clear error.
        let target = DummyTarget;
        let q = DummyQuote {
            t: 1.0,
            root: 0.5,
            scale: 1e9, // makes |f| >> OBJECTIVE_VALID_ABS_MAX across the scan grid
            unsorted_scan: false,
            infeasible_below: None,
        };
        let cfg = CalibrationConfig {
            solver: crate::calibration::solver::SolverConfig::brent_default()
                .with_tolerance(1e-10)
                .with_max_iterations(200),
            ..CalibrationConfig::default()
        };
        let err =
            SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None, None)
                .expect_err("should fail when all evaluations are penalized");
        let msg = format!("{err}");
        assert!(
            msg.contains("all")
                && msg.contains("objective evaluations")
                && msg.contains("invalid/penalized"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn bootstrap_enforces_f_space_tolerance_not_just_x_space() {
        // Core Brent termination is x-space based. For large-magnitude roots and steep residuals,
        // x-space termination can occur while |residual| is still far above tolerance.
        // This test ensures the bootstrapper enforces |residual| <= tolerance after solving.
        #[derive(Clone, Debug)]
        struct SteepQuote {
            t: f64,
            root: f64,
            scale: f64,
        }

        struct SteepTarget;

        impl BootstrapTarget for SteepTarget {
            type Quote = SteepQuote;
            type Curve = f64;

            fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
                Ok(quote.t)
            }

            fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
                knots
                    .last()
                    .map(|(_, v)| *v)
                    .ok_or(Error::Input(finstack_core::InputError::TooFewPoints))
            }

            fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
                let dx = *curve - quote.root;
                // Make the true root occur at a sub-ULP shift from `quote.root` at this magnitude
                // so no representable x can achieve |residual| <= tol.
                // This forces the bootstrapper's post-solve f-space tolerance check to trigger.
                Ok(quote.scale * dx + dx * dx * dx + 1e-6)
            }

            fn initial_guess(
                &self,
                quote: &Self::Quote,
                _previous_knots: &[(f64, f64)],
            ) -> Result<f64> {
                // Avoid starting at the root, and avoid symmetric brackets that hit the root at the first midpoint.
                Ok(quote.root + 1.3)
            }

            fn scan_points(&self, quote: &Self::Quote, _initial_guess: f64) -> Result<Vec<f64>> {
                // Deliberately asymmetric bracket so bisection midpoints don't immediately equal `root`.
                Ok(vec![quote.root - 1.0, quote.root + 2.0])
            }
        }

        let target = SteepTarget;
        let q = SteepQuote {
            t: 1.0,
            root: 1.0e8 + 0.1,
            // Keep |f| within the objective-valid cap for scan points (dx=±1 => |f| ~ 1e4).
            scale: 1.0e4,
        };
        let cfg = CalibrationConfig {
            solver: crate::calibration::solver::SolverConfig::brent_default()
                .with_tolerance(1e-10)
                .with_max_iterations(200),
            ..CalibrationConfig::default()
        };
        let err =
            SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None, None)
                .expect_err("bootstrap should fail due to f-space tolerance enforcement");
        let msg = format!("{err}");
        assert!(
            msg.contains("exceeds tolerance"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn bootstrap_rejects_non_increasing_times() {
        let target = DummyTarget;
        let q1 = DummyQuote {
            t: 1.0,
            root: 0.5,
            scale: 1.0,
            unsorted_scan: false,
            infeasible_below: None,
        };
        let q2 = DummyQuote {
            t: 1.0,
            ..q1.clone()
        };
        let cfg = CalibrationConfig::default();
        let err = SequentialBootstrapper::bootstrap(
            &target,
            &[q1, q2],
            vec![(0.0, 0.0)],
            &cfg,
            None,
            None,
        )
        .expect_err("should reject duplicate times");
        assert!(format!("{err}").contains("duplicate quote times"));
    }

    #[test]
    fn bootstrap_is_deterministic_under_quote_shuffling() {
        let target = DummyTarget;
        let q_short = DummyQuote {
            t: 1.0,
            root: 0.25,
            scale: 1.0,
            unsorted_scan: false,
            infeasible_below: None,
        };
        let q_long = DummyQuote {
            t: 2.0,
            root: 0.75,
            scale: 1.0,
            unsorted_scan: false,
            infeasible_below: None,
        };
        let cfg = CalibrationConfig {
            solver: crate::calibration::solver::SolverConfig::brent_default()
                .with_tolerance(1e-12)
                .with_max_iterations(200),
            ..CalibrationConfig::default()
        };

        let (curve_sorted, report_sorted) = SequentialBootstrapper::bootstrap(
            &target,
            &[q_short.clone(), q_long.clone()],
            vec![(0.0, 0.0)],
            &cfg,
            None,
            None,
        )
        .expect("sorted input should succeed");
        let (curve_shuffled, report_shuffled) = SequentialBootstrapper::bootstrap(
            &target,
            &[q_long, q_short],
            vec![(0.0, 0.0)],
            &cfg,
            None,
            None,
        )
        .expect("shuffled input should succeed");

        assert!((curve_sorted - curve_shuffled).abs() < 1e-12);
        assert_eq!(report_sorted.residuals, report_shuffled.residuals);
        assert!((report_sorted.rmse - report_shuffled.rmse).abs() < 1e-12);
        assert!((report_sorted.objective_value - report_shuffled.objective_value).abs() < 1e-12);
    }
}
