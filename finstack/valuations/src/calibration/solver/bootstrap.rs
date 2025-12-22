//! Generic sequential bootstrapping algorithm.

use super::bracket_solve_1d_with_diagnostics;
use super::traits::BootstrapTarget;
use crate::calibration::{
    CalibrationConfig, CalibrationReport, OBJECTIVE_VALID_ABS_MAX, RESIDUAL_PENALTY_ABS_MIN,
};
use finstack_core::Result;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    /// * `trace` - Optional trace for collecting diagnostics and intermediate steps.
    ///
    /// # Returns
    /// A pair containing the calibrated term structure and a diagnostic report.
    pub fn bootstrap<T>(
        target: &T,
        quotes: &[T::Quote],
        initial_knots: Vec<(f64, f64)>,
        config: &CalibrationConfig,
        mut trace: Option<finstack_core::explain::ExplanationTrace>,
    ) -> Result<(T::Curve, CalibrationReport)>
    where
        T: BootstrapTarget,
        T::Quote: std::fmt::Debug,
    {
        let mut knots = initial_knots;
        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0;
        let mut last_time = knots.iter().map(|(t, _)| *t).fold(0.0_f64, f64::max);

        // Centralized sorting by quote time for deterministic bootstrapping.
        // We intentionally do not assume `quotes` are pre-sorted.
        let mut quote_times: Vec<(f64, usize)> = Vec::with_capacity(quotes.len());
        for (idx, quote) in quotes.iter().enumerate() {
            let time = target
                .quote_time(quote)
                .map_err(|e| finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap failed to compute quote_time for quote index {idx}: {e}"
                    ),
                    category: "bootstrapping".to_string(),
                })?;
            if !time.is_finite() || time <= 0.0 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap quote_time must be finite and > 0; got t={} (quote index {})",
                        time, idx
                    ),
                    category: "bootstrapping".to_string(),
                });
            }
            quote_times.push((time, idx));
        }
        quote_times.sort_by(|(t1, i1), (t2, i2)| {
            t1.partial_cmp(t2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| i1.cmp(i2))
        });

        // Iterate through time-sorted quotes
        for (sorted_idx, (time, original_idx)) in quote_times.into_iter().enumerate() {
            let quote = &quotes[original_idx];
            if time < last_time {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap requires increasing quote times; got t={:.12} after last_time={:.12} (quote index {})",
                        time, last_time, original_idx
                    ),
                    category: "bootstrapping".to_string(),
                });
            }
            if (time - last_time).abs() <= 1e-12 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap rejects duplicate quote times: t={:.12} appears more than once (quote index {})",
                        time, original_idx
                    ),
                    category: "bootstrapping".to_string(),
                });
            }

            // Initial guess
            let initial_guess = target.initial_guess(quote, &knots)?;

            // Track the first evaluation error for diagnostics, but do not fail unless all
            // evaluations are invalid/penalized (market-standard behavior).
            let first_eval_error: std::cell::RefCell<Option<String>> =
                std::cell::RefCell::new(None);
            let eval_counter = AtomicUsize::new(0);

            // Optimization: reuse buffer to avoid allocation in hot loop
            let reuse_buffer = std::cell::RefCell::new(Vec::with_capacity(knots.len() + 1));

            // Define objective function
            let objective = |value: f64| -> f64 {
                let eval_idx = eval_counter.fetch_add(1, Ordering::Relaxed) + 1;

                // Reuse vector from RefCell
                let mut temp_knots_guard = reuse_buffer.borrow_mut();
                temp_knots_guard.clear();
                temp_knots_guard.extend_from_slice(&knots);
                temp_knots_guard.push((time, value));
                let temp_knots = &*temp_knots_guard;

                // 1. Build temporary curve
                let curve = match target.build_curve_for_solver(temp_knots) {
                    Ok(c) => c,
                    Err(e) => {
                        if first_eval_error.borrow().is_none() {
                            *first_eval_error.borrow_mut() = Some(format!(
                                "eval#{eval_idx} curve construction failed at value={value}: {e}"
                            ));
                        }
                        // Market-standard: treat infeasible evaluations as invalid (not a flat penalty),
                        // so the scan/bracketing logic can ignore them without polluting diagnostics.
                        return f64::NAN;
                    }
                };

                // 2. Calculate residual failure
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

            // Determine scan points
            let mut scan_points = {
                let points = target.scan_points(quote, initial_guess)?;
                if !points.is_empty() {
                    points
                } else {
                    let center = if initial_guess.is_finite() {
                        initial_guess
                    } else {
                        0.0
                    };

                    // Default scan grid: geometric expansion around the initial guess.
                    // This is more robust than fixed +/- k*step heuristics across regimes.
                    let step0 =
                        (config.discount_curve.scan_grid_step * (1.0 + center.abs())).max(1e-8);
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
            };
            // Defensive normalization (targets may provide unsorted or duplicated scan grids).
            scan_points.retain(|x| x.is_finite());
            if initial_guess.is_finite() {
                scan_points.push(initial_guess);
            }
            scan_points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            scan_points.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            if scan_points.is_empty() {
                return Err(finstack_core::Error::Calibration {
                    message: format!("Bootstrap failed at t={:.6}: scan_points is empty after filtering non-finite values.", time),
                    category: "bootstrapping".to_string(),
                });
            }
            let scan_points_ref = scan_points.as_slice();

            // Solve using bracket + polish
            let (tentative, diag) = bracket_solve_1d_with_diagnostics(
                &objective,
                initial_guess,
                scan_points_ref,
                config.solver.tolerance(),
                config.solver.max_iterations(),
            )?;

            total_iterations += diag.eval_count;

            let solved_value = if let Some(root) = tentative {
                root
            } else {
                // No sign-change bracket found.
                // Market-standard behavior: if the scan found a point already within tolerance,
                // accept it; otherwise fail fast rather than running a generic solver through
                // infeasible regions (which is often unstable and non-deterministic).
                if let (Some(best_x), Some(best_f)) = (diag.best_point, diag.best_value) {
                    if best_f.is_finite() && best_f.abs() <= config.solver.tolerance() {
                        best_x
                    } else {
                        return Err(finstack_core::Error::Calibration {
                            message: format!(
                                "Bootstrap failed at t={:.6} (quote={:?}): no bracket found and best |residual|={:.3e} exceeds tolerance={:.3e} (scan_bounds=[{:.3e}, {:.3e}])",
                                time,
                                quote,
                                best_f.abs(),
                                config.solver.tolerance(),
                                diag.scan_bounds.0,
                                diag.scan_bounds.1
                            ),
                            category: "bootstrapping".to_string(),
                        });
                    }
                } else if diag.valid_eval_count == 0 {
                    let hint = first_eval_error.borrow().clone().unwrap_or_else(|| {
                        "no error recorded (all evaluations penalized or non-finite)".to_string()
                    });
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Bootstrap failed at t={:.6}: all {} objective evaluations returned invalid/penalized values (|f| >= {:.3e}). First error: {}",
                            time, diag.eval_count, OBJECTIVE_VALID_ABS_MAX, hint
                        ),
                        category: "bootstrapping".to_string(),
                    });
                } else {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Bootstrap failed at t={:.6}: no bracket found despite {} valid evaluations (scan_bounds=[{:.3e}, {:.3e}]).",
                            time,
                            diag.valid_eval_count,
                            diag.scan_bounds.0,
                            diag.scan_bounds.1
                        ),
                        category: "bootstrapping".to_string(),
                    });
                }
            };

            // Validate result
            target.validate_knot(time, solved_value)?;

            // Compute final residual and commit.
            //
            // PERF: avoid `knots.clone()` by temporarily pushing the candidate knot and popping
            // on error. This keeps the hot loop allocation-free while preserving correctness.
            knots.push((time, solved_value));
            let final_curve = match target.build_curve_for_solver(&knots) {
                Ok(c) => c,
                Err(e) => {
                    knots.pop();
                    return Err(e);
                }
            };
            let residual_signed = match target.calculate_residual(&final_curve, quote) {
                Ok(r) => r,
                Err(e) => {
                    knots.pop();
                    return Err(e);
                }
            };
            let residual_abs = residual_signed.abs();

            if !residual_signed.is_finite() || residual_abs >= RESIDUAL_PENALTY_ABS_MIN {
                knots.pop();
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap converged to invalid/penalty residual at t={:.6}: residual={} (|.|={:.3e})",
                        time, residual_signed, residual_abs
                    ),
                    category: "bootstrapping".to_string(),
                });
            }
            if residual_abs > config.solver.tolerance() {
                knots.pop();
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap failed to converge at t={:.6}: residual={} (|.|={:.3e}) exceeds tolerance={:.3e}",
                        time, residual_signed, residual_abs, config.solver.tolerance()
                    ),
                    category: "bootstrapping".to_string(),
                });
            }

            last_time = time;

            // Store residual
            let key = format!("quote_{:06}", sorted_idx);
            residuals.insert(key, residual_signed);

            // Trace
            if let Some(t) = &mut trace {
                use finstack_core::explain::TraceEntry;
                t.push(
                    TraceEntry::CalibrationIteration {
                        iteration: sorted_idx,
                        residual: residual_signed,
                        knots_updated: vec![format!("t={:.4}", time)],
                        converged: residual_abs <= config.solver.tolerance(),
                    },
                    config.explain.max_entries,
                );
            }
        }

        // Build final curve to return
        let final_curve = target.build_curve_final(&knots)?;

        let report = CalibrationReport::for_type_with_tolerance(
            "generic_bootstrap",
            residuals,
            total_iterations,
            config.solver.tolerance(),
        );
        let report = if let Some(t) = trace {
            report.with_explanation(t)
        } else {
            report
        };

        Ok((final_curve, report))
    }
}

#[cfg(test)]
mod tests {
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
                .ok_or(Error::Input(finstack_core::error::InputError::TooFewPoints))
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
            SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None)
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
            SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None)
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
        let err = SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None)
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
                    .ok_or(Error::Input(finstack_core::error::InputError::TooFewPoints))
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
        let err = SequentialBootstrapper::bootstrap(&target, &[q], vec![(0.0, 0.0)], &cfg, None)
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
        let err =
            SequentialBootstrapper::bootstrap(&target, &[q1, q2], vec![(0.0, 0.0)], &cfg, None)
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
        )
        .expect("sorted input should succeed");
        let (curve_shuffled, report_shuffled) = SequentialBootstrapper::bootstrap(
            &target,
            &[q_long, q_short],
            vec![(0.0, 0.0)],
            &cfg,
            None,
        )
        .expect("shuffled input should succeed");

        assert!((curve_sorted - curve_shuffled).abs() < 1e-12);
        assert_eq!(report_sorted.residuals, report_shuffled.residuals);
        assert!((report_sorted.rmse - report_shuffled.rmse).abs() < 1e-12);
        assert!((report_sorted.objective_value - report_shuffled.objective_value).abs() < 1e-12);
    }
}
