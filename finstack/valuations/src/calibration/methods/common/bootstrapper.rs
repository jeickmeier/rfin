/// Generic bootstrapping logic.
///
/// This module provides a generic bootstrapping framework that can be used
/// to calibrate various types of curves, such as discount curves and forward
/// curves.
use crate::calibration::{
    bracket_solve_1d_with_diagnostics, CalibrationConfig, CalibrationReport, OBJECTIVE_VALID_ABS_MAX,
    RESIDUAL_PENALTY_ABS_MIN,
};
use finstack_core::prelude::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Trait defining the specific physics for a bootstrapping process.
pub trait BootstrapTarget {
    /// Type of input quote (e.g., RatesQuote, CreditQuote).
    type Quote;

    /// Type of the curve being built (e.g., DiscountCurve, ForwardCurve).
    type Curve;

    /// Get the time (year fraction) for the knot corresponding to this quote.
    fn quote_time(&self, quote: &Self::Quote) -> Result<f64>;

    /// Build a temporary curve from a set of knots.
    ///
    /// This is called repeatedly during the solver loop.
    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve>;

    #[allow(missing_docs)]
    fn build_curve_for_solver(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        self.build_curve(knots)
    }

    #[allow(missing_docs)]
    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        self.build_curve(knots)
    }

    /// Calculate the pricing residual for a quote given the curve.
    ///
    /// Residual = Model Price - Market Price (or Rate).
    /// Result should be 0.0 when perfectly calibrated.
    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64>;

    /// Provide an initial guess for the solver for the next knot.
    fn initial_guess(&self, quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64>;

    /// Get scan points for root bracketing for the given quote.
    ///
    /// This allows the target to provide adaptive or context-aware scan grids
    /// based on the quote and initial guess.
    fn scan_points(&self, _quote: &Self::Quote, _initial_guess: f64) -> Result<Vec<f64>> {
        Ok(Vec::new())
    }

    /// Optional: Validate the solved value before accepting it.
    /// Default implementation accepts everything.
    fn validate_knot(&self, _time: f64, _value: f64) -> Result<()> {
        Ok(())
    }
}

/// Generic sequential bootstrapper.
pub struct SequentialBootstrapper;

impl SequentialBootstrapper {
    /// Run the sequential bootstrapping algorithm.
    ///
    /// # Arguments
    /// * `target` - The specific curve physics implementation.
    /// * `sorted_quotes` - Quotes sorted by maturity/dependency order.
    /// * `initial_knots` - Initial knots to start the bootstrapping process.
    /// * `config` - Calibration configuration (solver settings, etc.).
    /// * `trace` - Optional explanation trace for detailed debugging.
    pub fn bootstrap<T>(
        target: &T,
        sorted_quotes: &[T::Quote],
        initial_knots: Vec<(f64, f64)>,
        config: &CalibrationConfig,
        mut trace: Option<finstack_core::explain::ExplanationTrace>,
    ) -> Result<(T::Curve, CalibrationReport)>
    where
        T: BootstrapTarget,
    {
        let mut knots = initial_knots;
        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0;
        let mut last_time = knots
            .iter()
            .map(|(t, _)| *t)
            .fold(0.0_f64, f64::max);

        // Iterate through sorted quotes
        for (idx, quote) in sorted_quotes.iter().enumerate() {
            // Calculate knot time
            let time = target.quote_time(quote)?;
            if !time.is_finite() || time <= 0.0 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap quote_time must be finite and > 0; got t={}",
                        time
                    ),
                    category: "bootstrapping".to_string(),
                });
            }
            if time <= last_time {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap requires strictly increasing quote times; got t={:.6} after last_time={:.6}",
                        time, last_time
                    ),
                    category: "bootstrapping".to_string(),
                });
            }

            // Initial guess
            let initial_guess = target.initial_guess(quote, &knots)?;

            // Track first evaluation error for diagnostics only.
            let first_eval_error: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
            let eval_counter = AtomicUsize::new(0);

            // Define objective function
            let objective = |value: f64| -> f64 {
                let eval_idx = eval_counter.fetch_add(1, Ordering::Relaxed) + 1;
                // Build temporary knots list
                // We use a simplified connection: existing knots + new knot
                let mut temp_knots = Vec::with_capacity(knots.len() + 1);
                temp_knots.extend_from_slice(&knots);
                temp_knots.push((time, value));

                // 1. Build temporary curve
                let curve = match target.build_curve_for_solver(&temp_knots) {
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

            // Determine scan points: prefer target-specific points
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

                    let step0 = (1e-4 * (1.0 + center.abs())).max(1e-8);
                    let mut pts = Vec::with_capacity(2 * 16 + 1);
                    pts.push(center);
                    let mut step = step0;
                    for _ in 0..16 {
                        pts.push(center - step);
                        pts.push(center + step);
                        step *= 2.0;
                    }
                    pts
                }
            };
            scan_points.retain(|x| x.is_finite());
            if initial_guess.is_finite() {
                scan_points.push(initial_guess);
            }
            scan_points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            scan_points.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
            if scan_points.is_empty() {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap failed at t={:.6}: scan_points is empty after filtering non-finite values.",
                        time
                    ),
                    category: "bootstrapping".to_string(),
                });
            }
            let scan_points_ref = scan_points.as_slice();

            // Solve using bracket + polish
            let (tentative, diag) = bracket_solve_1d_with_diagnostics(
                &objective,
                initial_guess,
                scan_points_ref,
                config.tolerance,
                config.max_iterations,
            )?;

            total_iterations += diag.eval_count;

            let solved_value = if let Some(root) = tentative {
                root
            } else if let (Some(best_x), Some(best_f)) = (diag.best_point, diag.best_value) {
                if best_f.is_finite() && best_f.abs() <= config.tolerance {
                    best_x
                } else {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Bootstrap failed at t={:.6}: no bracket found and best |residual|={:.3e} exceeds tolerance={:.3e} (scan_bounds=[{:.3e}, {:.3e}])",
                            time,
                            best_f.abs(),
                            config.tolerance,
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
            };

            // Validate result
            target.validate_knot(time, solved_value)?;

            // Compute final residual and commit
            // We re-evaluate to get precise residual and confirmation
            let mut final_knots = knots.clone();
            final_knots.push((time, solved_value));
            let final_curve = target.build_curve_for_solver(&final_knots)?;
            let residual_signed = target.calculate_residual(&final_curve, quote)?;
            let residual_abs = residual_signed.abs();

            if !residual_signed.is_finite() || residual_abs >= RESIDUAL_PENALTY_ABS_MIN {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap converged to invalid/penalty residual at t={:.6}: residual={} (|.|={:.3e})",
                        time, residual_signed, residual_abs
                    ),
                    category: "bootstrapping".to_string(),
                });
            }
            if residual_abs > config.tolerance {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap failed to converge at t={:.6}: residual={} (|.|={:.3e}) exceeds tolerance={:.3e}",
                        time, residual_signed, residual_abs, config.tolerance
                    ),
                    category: "bootstrapping".to_string(),
                });
            }

            knots.push((time, solved_value));
            last_time = time;

            // Store residual
            let key = format!("quote_{:06}", idx);
            residuals.insert(key, residual_signed);

            // Trace
            if let Some(t) = &mut trace {
                use finstack_core::explain::TraceEntry;
                t.push(
                    TraceEntry::CalibrationIteration {
                        iteration: idx,
                        residual: residual_signed,
                        knots_updated: vec![format!("t={:.4}", time)],
                        converged: residual_abs <= config.tolerance,
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
            config.tolerance,
        );
        let report = if let Some(t) = trace {
            report.with_explanation(t)
        } else {
            report
        };

        Ok((final_curve, report))
    }
}
