//! Generic sequential bootstrapping algorithm.

use super::traits::BootstrapTarget;
use crate::calibration::{
    bracket_solve_1d_with_diagnostics, create_simple_solver, CalibrationConfig, CalibrationReport,
    PENALTY,
};
use finstack_core::prelude::*;
use std::collections::BTreeMap;

/// Generic sequential bootstrapper.
pub struct SequentialBootstrapper;

impl SequentialBootstrapper {
    /// Run the sequential bootstrapping algorithm.
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

        let solver = create_simple_solver(config);

        // Iterate through sorted quotes
        for (idx, quote) in sorted_quotes.iter().enumerate() {
            // Calculate knot time
            let time = target.quote_time(quote)?;

            // Initial guess
            let initial_guess = target.initial_guess(quote, &knots)?;

            let residual_error: std::cell::RefCell<Option<finstack_core::Error>> =
                std::cell::RefCell::new(None);

            // Define objective function
            let objective = |value: f64| -> f64 {
                // Build temporary knots list
                let mut temp_knots = Vec::with_capacity(knots.len() + 1);
                temp_knots.extend_from_slice(&knots);
                temp_knots.push((time, value));

                // 1. Build temporary curve
                let curve = match target.build_curve_for_solver(&temp_knots) {
                    Ok(c) => c,
                    Err(e) => {
                        if residual_error.borrow().is_none() {
                            *residual_error.borrow_mut() = Some(e);
                        }
                        return PENALTY;
                    }
                };

                // 2. Calculate residual failure
                match target.calculate_residual(&curve, quote) {
                    Ok(r) => r,
                    Err(e) => {
                        if residual_error.borrow().is_none() {
                            *residual_error.borrow_mut() = Some(e);
                        }
                        PENALTY
                    }
                }
            };

            // Determine scan points
            let scan_points = {
                let points = target.scan_points(quote, initial_guess)?;
                if !points.is_empty() {
                    points
                } else {
                    let center = if initial_guess.is_finite() {
                        initial_guess
                    } else {
                        0.0
                    };

                    let mag = center.abs().max(1.0);
                    let step = (0.25 * mag).max(1e-6);
                    vec![
                        center - 4.0 * step,
                        center - 2.0 * step,
                        center - 1.0 * step,
                        center,
                        center + 1.0 * step,
                        center + 2.0 * step,
                        center + 4.0 * step,
                    ]
                }
            };
            let scan_points_ref = scan_points.as_slice();

            // Solve using bracket + polish
            let (tentative, diag) = bracket_solve_1d_with_diagnostics(
                &objective,
                initial_guess,
                scan_points_ref,
                config.tolerance,
                config.max_iterations,
            )?;

            if let Some(e) = residual_error.borrow_mut().take() {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Bootstrap residual evaluation failed at t={:.4}: {}",
                        time, e
                    ),
                    category: "bootstrapping".to_string(),
                });
            }

            total_iterations += diag.eval_count;

            let solved_value = if let Some(root) = tentative {
                root
            } else {
                // No bracket found - try direct solver if we had valid points
                if diag.valid_eval_count == 0 {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "Bootstrap failed at t={:.4}: all {} objective evaluations returned invalid/penalized values.", 
                            time, diag.eval_count
                        ),
                        category: "bootstrapping".to_string(),
                    });
                }

                let best_guess = diag.best_point.unwrap_or(initial_guess);
                solver.solve(objective, best_guess).map_err(|e| {
                    finstack_core::Error::Calibration {
                        message: format!("Bootstrap solver failed at t={:.4}: {}", time, e),
                        category: "bootstrapping".to_string(),
                    }
                })?
            };

            // Validate result
            target.validate_knot(time, solved_value)?;

            // Compute final residual and commit
            let mut final_knots = knots.clone();
            final_knots.push((time, solved_value));
            let final_curve = target.build_curve_for_solver(&final_knots)?;
            let residual = target.calculate_residual(&final_curve, quote)?.abs();

            if !residual.is_finite() || residual > PENALTY * 0.5 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Solver converged to invalid residual at t={:.4}: {}",
                        time, residual
                    ),
                    category: "bootstrapping".to_string(),
                });
            }

            knots.push((time, solved_value));

            // Store residual
            let key = format!("quote_{:06}", idx);
            residuals.insert(key, residual);

            // Trace
            if let Some(t) = &mut trace {
                use finstack_core::explain::TraceEntry;
                t.push(
                    TraceEntry::CalibrationIteration {
                        iteration: idx,
                        residual,
                        knots_updated: vec![format!("t={:.4}", time)],
                        converged: true,
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
