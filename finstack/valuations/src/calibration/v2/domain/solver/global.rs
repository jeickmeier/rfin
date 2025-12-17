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

        validate_global_inputs(&times, &initials, n_residuals)?;

        let solver = config.create_lm_solver();

        // Trackers
        let residual_error: Arc<Mutex<Option<finstack_core::Error>>> = Arc::new(Mutex::new(None));
        let eval_counter = Arc::new(AtomicUsize::new(0));

        // Clones for closure
        let residual_error_clone = Arc::clone(&residual_error);
        let eval_counter_clone = Arc::clone(&eval_counter);

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

        let mut weights = vec![1.0_f64; n_residuals];
        target.residual_weights(&active_quotes, &mut weights)?;
        for (idx, w) in weights.iter().enumerate() {
            if !w.is_finite() || *w < 0.0 {
                return Err(finstack_core::Error::Calibration {
                    message: format!(
                        "Global solve requires non-negative finite residual weights; got {} at index {}.",
                        w, idx
                    ),
                    category: "global_solve".to_string(),
                });
            }
        }
        if weights.iter().all(|w| *w == 0.0) {
            return Err(finstack_core::Error::Calibration {
                message: "Global solve requires at least one positive residual weight.".to_string(),
                category: "global_solve".to_string(),
            });
        }
        let weight_scales: Vec<f64> = weights.iter().map(|w| w.sqrt()).collect();

        let residuals_func = |params: &[f64], resid: &mut [f64]| {
            let eval_idx = eval_counter_clone.fetch_add(1, Ordering::Relaxed) + 1;

            // Zero out buffer
            for r in resid.iter_mut() {
                *r = 0.0;
            }

            // 1. Build curve
            let curve = match target.build_curve_for_solver_from_params(&times, params) {
                Ok(c) => c,
                Err(e) => {
                    record_eval_error(
                        &residual_error_clone,
                        eval_idx,
                        "curve construction",
                        params,
                        &format!("{}", e),
                    );
                    for r in resid.iter_mut() {
                        *r = PENALTY;
                    }
                    return;
                }
            };

            // 2. Calculate residuals
            if let Err(e) = target.calculate_residuals(&curve, &active_quotes, resid) {
                record_eval_error(
                    &residual_error_clone,
                    eval_idx,
                    "residual evaluation",
                    params,
                    &format!("while evaluating {} quotes: {}", active_quotes.len(), e),
                );
                for r in resid.iter_mut() {
                    *r = PENALTY;
                }
                return;
            }

            for (i, r) in resid.iter_mut().enumerate().take(n_residuals) {
                *r *= weight_scales[i];
            }
        };

        // Solve
        let solution =
            solver.solve_system_with_dim_stats(residuals_func, &initials, n_residuals)?;
        let solved_params = solution.params;
        let stats = solution.stats;

        if let Some(e) = residual_error.lock().ok().and_then(|mut err| err.take()) {
            return Err(e);
        }

        // Build final curve
        let final_curve = target.build_curve_final_from_params(&times, &solved_params)?;

        // Build report
        let mut residuals_map = BTreeMap::new();
        let mut resid_values = vec![0.0; n_residuals];
        target.calculate_residuals(&final_curve, &active_quotes, &mut resid_values)?;

        for (i, (&val, quote)) in resid_values.iter().zip(active_quotes.iter()).enumerate() {
            residuals_map.insert(target.residual_key(quote, i), val.abs());
        }

        let l2_norm: f64 = resid_values.iter().map(|r| r * r).sum::<f64>().sqrt();
        let weighted_l2_norm: f64 = resid_values
            .iter()
            .zip(weight_scales.iter())
            .map(|(r, w)| (r * w).powi(2))
            .sum::<f64>()
            .sqrt();
        let max_abs_residual = resid_values.iter().map(|r| r.abs()).fold(0.0_f64, f64::max);
        let weighted_max_abs_residual = resid_values
            .iter()
            .zip(weight_scales.iter())
            .map(|(r, w)| (r * w).abs())
            .fold(0.0_f64, f64::max);

        let report = CalibrationReport::for_type_with_tolerance(
            "global_solve",
            residuals_map,
            stats.iterations,
            config.tolerance,
        )
        .with_metadata("method", "global_solve")
        .with_metadata("residual_evals", stats.residual_evals.to_string())
        .with_metadata(
            "lm_termination_reason",
            format!("{:?}", stats.termination_reason),
        )
        .with_metadata("lm_jacobian_evals", stats.jacobian_evals.to_string())
        .with_metadata(
            "lm_final_resid_norm",
            format!("{:.2e}", stats.final_residual_norm),
        )
        .with_metadata(
            "lm_final_step_norm",
            format!("{:.2e}", stats.final_step_norm),
        )
        .with_metadata("lm_lambda_final", format!("{:.2e}", stats.lambda_final))
        .with_metadata("l2_norm", format!("{:.2e}", l2_norm))
        .with_metadata("max_abs_residual", format!("{:.2e}", max_abs_residual))
        .with_metadata("weighted_l2_norm", format!("{:.2e}", weighted_l2_norm))
        .with_metadata(
            "weighted_max_abs_residual",
            format!("{:.2e}", weighted_max_abs_residual),
        );

        Ok((final_curve, report))
    }
}

fn validate_global_inputs(times: &[f64], initials: &[f64], n_residuals: usize) -> Result<()> {
    if times.len() != initials.len() {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Global solve requires times.len() == initials.len(); got {} vs {}.",
                times.len(),
                initials.len()
            ),
            category: "global_solve".to_string(),
        });
    }

    if times.len() != n_residuals {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Global solve requires times.len() == active_quotes.len(); got {} vs {}.",
                times.len(),
                n_residuals
            ),
            category: "global_solve".to_string(),
        });
    }

    for (idx, &t) in times.iter().enumerate() {
        if !t.is_finite() || t <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve requires strictly positive finite times; got {} at index {}.",
                    t, idx
                ),
                category: "global_solve".to_string(),
            });
        }
    }

    for (idx, &init) in initials.iter().enumerate() {
        if !init.is_finite() {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve requires finite initial guesses; got {} at index {}.",
                    init, idx
                ),
                category: "global_solve".to_string(),
            });
        }
    }

    for idx in 1..times.len() {
        let prev = times[idx - 1];
        let next = times[idx];
        if next <= prev {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve requires strictly increasing times; indices {} -> {} have values {} -> {}.",
                    idx - 1,
                    idx,
                    prev,
                    next
                ),
                category: "global_solve".to_string(),
            });
        }
    }

    Ok(())
}

fn record_eval_error(
    store: &Arc<Mutex<Option<finstack_core::Error>>>,
    eval_idx: usize,
    stage: &str,
    params: &[f64],
    detail: &str,
) {
    if let Ok(mut err) = store.lock() {
        if err.is_some() {
            return;
        }
        let (min_param, max_param) = param_range(params);
        let message = format!(
            "Global solve {stage} failed at eval #{eval_idx} (param_range=[{min:.4e}, {max:.4e}]): {detail}",
            stage = stage,
            eval_idx = eval_idx,
            min = min_param,
            max = max_param,
            detail = detail
        );
        *err = Some(finstack_core::Error::Calibration {
            message,
            category: "global_solve".to_string(),
        });
    }
}

fn param_range(params: &[f64]) -> (f64, f64) {
    if params.is_empty() {
        return (0.0, 0.0);
    }
    let mut min_val = f64::INFINITY;
    let mut max_val = f64::NEG_INFINITY;
    for &p in params {
        if p.is_finite() {
            if p < min_val {
                min_val = p;
            }
            if p > max_val {
                max_val = p;
            }
        }
    }
    if !min_val.is_finite() || !max_val.is_finite() {
        (0.0, 0.0)
    } else {
        (min_val, max_val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::CalibrationConfig;
    use finstack_core::Error;

    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    struct DummyCurve(Vec<f64>);

    struct TestTarget {
        times: Vec<f64>,
        initials: Vec<f64>,
        residuals: Vec<f64>,
        weights: Option<Vec<f64>>,
        key_prefix: Option<String>,
    }

    impl TestTarget {
        fn new(times: Vec<f64>, initials: Vec<f64>, residuals: Vec<f64>) -> Self {
            Self {
                times,
                initials,
                residuals,
                weights: None,
                key_prefix: None,
            }
        }

        fn from_len(len: usize, residuals: Vec<f64>) -> Self {
            let times: Vec<f64> = (1..=len).map(|i| i as f64).collect();
            let initials = vec![0.0; len];
            Self::new(times, initials, residuals)
        }

        fn with_weights(mut self, weights: Vec<f64>) -> Self {
            self.weights = Some(weights);
            self
        }

        fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
            self.key_prefix = Some(prefix.into());
            self
        }
    }

    impl GlobalSolveTarget for TestTarget {
        type Quote = usize;
        type Curve = DummyCurve;

        fn build_time_grid_and_guesses(
            &self,
            quotes: &[Self::Quote],
        ) -> Result<(Vec<f64>, Vec<f64>, Vec<Self::Quote>)> {
            Ok((self.times.clone(), self.initials.clone(), quotes.to_vec()))
        }

        fn build_curve_from_params(&self, _times: &[f64], params: &[f64]) -> Result<Self::Curve> {
            Ok(DummyCurve(params.to_vec()))
        }

        fn calculate_residuals(
            &self,
            _curve: &Self::Curve,
            _quotes: &[Self::Quote],
            residuals: &mut [f64],
        ) -> Result<()> {
            for (i, r) in residuals.iter_mut().enumerate() {
                *r = *self.residuals.get(i).unwrap_or(&0.0);
            }
            Ok(())
        }

        fn residual_key(&self, quote: &Self::Quote, _idx: usize) -> String {
            if let Some(prefix) = &self.key_prefix {
                format!("{}-{}", prefix, quote)
            } else {
                format!("GLOBAL-{:06}", quote)
            }
        }

        fn residual_weights(&self, quotes: &[Self::Quote], weights_out: &mut [f64]) -> Result<()> {
            if let Some(custom) = &self.weights {
                assert_eq!(
                    custom.len(),
                    quotes.len(),
                    "weights must align with quotes for TestTarget"
                );
                for (out, value) in weights_out.iter_mut().zip(custom.iter()) {
                    *out = *value;
                }
                Ok(())
            } else {
                if quotes.len() != weights_out.len() {
                    return Err(finstack_core::Error::Calibration {
                        message: format!(
                            "weights_out length mismatch ({} vs {})",
                            weights_out.len(),
                            quotes.len()
                        ),
                        category: "global_solve".to_string(),
                    });
                }
                for weight in weights_out.iter_mut() {
                    *weight = 1.0;
                }
                Ok(())
            }
        }
    }

    #[test]
    fn max_abs_residual_metadata_uses_absolute_value() {
        let target = TestTarget::from_len(2, vec![-0.02, -0.01]);
        let quotes = vec![0usize, 1usize];
        let config = CalibrationConfig::default().with_tolerance(1.0);

        let (_curve, report) =
            GlobalOptimizer::optimize(&target, &quotes, &config).expect("optimization succeeds");

        let meta = report
            .metadata
            .get("max_abs_residual")
            .expect("metadata should contain max_abs_residual");
        let parsed: f64 = meta.parse().expect("metadata should parse as f64");

        assert!(
            (parsed - 2.0e-2).abs() < 1e-12,
            "max_abs_residual metadata should use absolute values (got {})",
            parsed
        );
    }

    #[test]
    fn rejects_length_mismatch_between_times_and_initials() {
        let target = TestTarget::new(vec![1.0, 2.0], vec![0.0], vec![0.0, 0.0]);
        let quotes = vec![0usize, 1usize];
        let config = CalibrationConfig::default().with_tolerance(1.0);

        let err = GlobalOptimizer::optimize(&target, &quotes, &config).expect_err("should fail");
        match err {
            Error::Calibration { message, .. } => {
                assert!(
                    message.contains("times.len() == initials.len()"),
                    "unexpected message: {}",
                    message
                );
            }
            other => panic!("unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn rejects_non_increasing_times() {
        let target = TestTarget::new(vec![1.0, 0.5], vec![0.0, 0.0], vec![0.0, 0.0]);
        let quotes = vec![0usize, 1usize];
        let config = CalibrationConfig::default().with_tolerance(1.0);

        let err = GlobalOptimizer::optimize(&target, &quotes, &config).expect_err("should fail");
        match err {
            Error::Calibration { message, .. } => {
                assert!(
                    message.contains("strictly increasing"),
                    "unexpected message: {}",
                    message
                );
            }
            other => panic!("unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn rejects_non_finite_inputs() {
        let target = TestTarget::new(vec![f64::NAN], vec![f64::INFINITY], vec![0.0]);
        let quotes = vec![0usize];
        let config = CalibrationConfig::default().with_tolerance(1.0);

        let err = GlobalOptimizer::optimize(&target, &quotes, &config).expect_err("should fail");
        match err {
            Error::Calibration { message, .. } => {
                assert!(
                    message.contains("strictly positive finite times")
                        || message.contains("finite initial guesses"),
                    "unexpected message: {}",
                    message
                );
            }
            other => panic!("unexpected error type: {:?}", other),
        }
    }

    #[test]
    fn uses_custom_residual_keys_and_weights() {
        let target = TestTarget::from_len(2, vec![0.01, 0.02])
            .with_weights(vec![4.0, 1.0])
            .with_key_prefix("TEST");
        let quotes = vec![5usize, 7usize];
        let config = CalibrationConfig::default().with_tolerance(1.0);

        let (_curve, report) =
            GlobalOptimizer::optimize(&target, &quotes, &config).expect("optimization succeeds");

        assert!(report.residuals.contains_key("TEST-5"));
        assert!(report.residuals.contains_key("TEST-7"));

        let weighted_l2 = report
            .metadata
            .get("weighted_l2_norm")
            .expect("metadata should include weighted_l2_norm");
        let expected = ((0.01_f64 * 2.0).powi(2) + (0.02_f64 * 1.0).powi(2)).sqrt();
        assert_eq!(
            weighted_l2,
            &format!("{:.2e}", expected),
            "weighted_l2_norm should reflect weights"
        );
    }
}
