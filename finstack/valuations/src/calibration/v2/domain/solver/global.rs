//! Generic global optimization algorithm.

use super::traits::GlobalSolveTarget;
use crate::calibration::{CalibrationConfig, CalibrationReport, PENALTY};
use finstack_core::prelude::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Generic global optimizer using Levenberg-Marquardt.
pub struct GlobalOptimizer;

impl GlobalOptimizer {
    /// Run the global optimization.
    pub fn optimize<T>(
        target: &T,
        quotes: &[T::Quote],
        config: &CalibrationConfig,
    ) -> Result<(T::Curve, CalibrationReport)>
    where
        T: GlobalSolveTarget,
        T::Quote: Clone, // Needed for safe sharing in closure
    {
        // 1. Build grid and guesses
        let (times, initials, active_quotes) = target.build_time_grid_and_guesses(quotes)?;
        let n_residuals = active_quotes.len();

        if n_residuals == 0 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        let solver = config.create_lm_solver();
        
        // Trackers
        let eval_counter = Arc::new(AtomicUsize::new(0));
        let residual_error: Arc<Mutex<Option<finstack_core::Error>>> = Arc::new(Mutex::new(None));
        
        // Clones for closure
        let eval_counter_clone = Arc::clone(&eval_counter);
        let residual_error_clone = Arc::clone(&residual_error);
        
        // We can't easily move `target` into the closure if it's a reference.
        // The LM solver requires the closure to be `Send` if we were using multi-threading,
        // but here it's executed sequentially by the solver.
        // However, `solver.solve_system` takes `Fn`.
        // If `target` is not `Clone` or `Send`, we might have issues if the solver was parallel.
        // But the current `LevenbergMarquardtSolver` is sequential.
        // We need to ensure `target` outlives the closure or is ref-counted.
        // Since we are inside a function, `&T` is fine as long as the closure doesn't escape.
        // But `solve_system_with_dim` might require `'static` or similar depending on implementation.
        // Let's assume standard borrow checking works here.
        
        let residuals_func = |params: &[f64], resid: &mut [f64]| {
            eval_counter_clone.fetch_add(1, Ordering::Relaxed);

            // Zero out buffer
            for r in resid.iter_mut() {
                *r = 0.0;
            }

            // 1. Build curve
            let curve = match target.build_curve_from_params(&times, params) {
                Ok(c) => c,
                Err(e) => {
                    if let Ok(mut err) = residual_error_clone.lock() {
                        if err.is_none() {
                            *err = Some(e);
                        }
                    }
                    for r in resid.iter_mut() {
                        *r = PENALTY;
                    }
                    return;
                }
            };

            // 2. Calculate residuals
            if let Err(e) = target.calculate_residuals(&curve, &active_quotes, resid) {
                if let Ok(mut err) = residual_error_clone.lock() {
                    if err.is_none() {
                        *err = Some(e);
                    }
                }
                for r in resid.iter_mut() {
                    *r = PENALTY;
                }
            }
        };

        // Solve
        let solved_params = solver.solve_system_with_dim(residuals_func, &initials, n_residuals)?;
        let evals = eval_counter.load(Ordering::Relaxed);

        if let Some(e) = residual_error.lock().ok().and_then(|mut err| err.take()) {
            return Err(finstack_core::Error::Calibration {
                message: format!("Global solve residual evaluation failed: {}", e),
                category: "global_solve".to_string(),
            });
        }

        // Build final curve
        let final_curve = target.build_curve_from_params(&times, &solved_params)?;

        // Build report
        let mut residuals_map = BTreeMap::new();
        let mut resid_values = vec![0.0; n_residuals];
        target.calculate_residuals(&final_curve, &active_quotes, &mut resid_values)?;

        for (i, &val) in resid_values.iter().enumerate() {
            residuals_map.insert(format!("GLOBAL-{:06}", i), val.abs());
        }

        let l2_norm: f64 = resid_values.iter().map(|r| r * r).sum::<f64>().sqrt();
        let max_abs_residual = resid_values.iter().copied().fold(0.0_f64, f64::max);

        let report = CalibrationReport::for_type_with_tolerance(
            "global_solve",
            residuals_map,
            evals,
            config.tolerance,
        )
        .with_metadata("method", "global_solve")
        .with_metadata("residual_evals", evals.to_string())
        .with_metadata("l2_norm", format!("{:.2e}", l2_norm))
        .with_metadata("max_abs_residual", format!("{:.2e}", max_abs_residual));

        Ok((final_curve, report))
    }
}


