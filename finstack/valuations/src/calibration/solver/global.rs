//! Simultaneous (single-start) weighted least-squares calibration using Levenberg–Marquardt.
//!
//! Notes:
//! - This is **not** a global-search optimizer (no multi-start / basin-hopping).
//! - Residual weighting is supported via per-quote weights (weighted least squares).

use super::traits::GlobalSolveTarget;
use crate::calibration::constants::PENALTY;
use crate::calibration::{CalibrationConfig, CalibrationReport};
use finstack_core::Result;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

fn penalty_residual_value(params: &[f64]) -> f64 {
    // Avoid a perfectly flat penalty surface: add a small, scale-aware component
    // so LM has a direction to move toward feasible regions.
    let mut norm2 = 0.0_f64;
    for &p in params {
        if p.is_finite() {
            norm2 += p * p;
        }
    }
    PENALTY + (norm2.min(PENALTY))
}

fn fill_penalty(resid: &mut [f64], n_residuals: usize, params: &[f64]) {
    let v = penalty_residual_value(params);
    for r in resid.iter_mut().take(n_residuals) {
        *r = v;
    }
}

/// Simultaneous weighted least-squares optimizer (single-start LM).
///
/// Implements a global optimization approach that fits all knots simultaneously
/// by minimizing the sum of weighted squared residuals. This is particularly
/// useful for overdetermined systems (e.g., fitting a smooth curve to many
/// noisy market quotes), multi-curve systems with complex inter-dependencies,
/// or when analytical Jacobians are available for performance.
///
/// Under the hood, it uses the Levenberg–Marquardt algorithm from `finstack-core`,
/// which provides robust convergence by damping the Gauss-Newton step toward
/// gradient descent when in non-linear or ill-conditioned regions.
pub struct GlobalFitOptimizer;

impl GlobalFitOptimizer {
    /// Execute a simultaneous weighted least-squares fit.
    ///
    /// # Generic Parameters
    /// * `T` - The calibration target (e.g., [`DiscountCurveTarget`](crate::calibration::targets::discount::DiscountCurveTarget)).
    ///
    /// # Arguments
    /// * `target` - The domain-specific implementation of the [`GlobalSolveTarget`] trait.
    /// * `quotes` - The list of high-level market quotes to fit.
    /// * `config` - Calibration settings specifying tolerances and methods.
    ///
    /// # Returns
    /// A pair containing the calibrated term structure and a diagnostic report.
    ///
    /// # Tolerance Semantics
    /// The configured tolerance is applied to the **L2 norm of the weighted residual vector**,
    /// i.e., after scaling each residual \(r_i\) by \(\sqrt{w_i}\).
    pub fn optimize<T>(
        target: &T,
        quotes: &[T::Quote],
        config: &CalibrationConfig,
    ) -> Result<(T::Curve, CalibrationReport)>
    where
        T: GlobalSolveTarget,
    {
        // 1. Build grid and guesses
        let (times, initials, active_quotes) = target.build_time_grid_and_guesses(quotes)?;
        let n_residuals = active_quotes.len();

        if n_residuals == 0 {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::TooFewPoints,
            ));
        }

        validate_global_inputs(&times, &initials, n_residuals)?;

        // Determine if we should use efficient (target-provided) Jacobian
        let use_efficient = match config.calibration_method {
            crate::calibration::config::CalibrationMethod::GlobalSolve {
                use_analytical_jacobian,
            } => use_analytical_jacobian && target.supports_efficient_jacobian(),
            _ => false,
        };

        if config.verbose && use_efficient {
            tracing::info!("GlobalFitOptimizer: using efficient (target-provided) Jacobian");
        }

        let solver = config.create_lm_solver();

        // Trackers (hot path): solver closures only require `Fn`, not `Send + Sync`,
        // so we can avoid `Arc/Atomic/Mutex` and use cheap interior mutability.
        let eval_diagnostics: RefCell<EvalDiagnostics> = RefCell::new(EvalDiagnostics::default());
        let eval_counter: Cell<usize> = Cell::new(0);

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
            let eval_idx = eval_counter.get() + 1;
            eval_counter.set(eval_idx);

            if resid.len() < n_residuals {
                record_eval_error(
                    &eval_diagnostics,
                    eval_idx,
                    "residual buffer",
                    params,
                    &format!(
                        "residual buffer too small: got {}, need {}",
                        resid.len(),
                        n_residuals
                    ),
                );
                fill_penalty(resid, resid.len(), params);
                return;
            }

            // Zero out only the used prefix (n_residuals); the solver may pass a larger scratch.
            for r in resid.iter_mut().take(n_residuals) {
                *r = 0.0;
            }

            // 1. Build curve
            let curve = match target.build_curve_for_solver_from_params(&times, params) {
                Ok(c) => c,
                Err(e) => {
                    record_eval_error(
                        &eval_diagnostics,
                        eval_idx,
                        "curve construction",
                        params,
                        &format!("{}", e),
                    );
                    fill_penalty(resid, n_residuals, params);
                    return;
                }
            };

            // 2. Calculate residuals
            if let Err(e) =
                target.calculate_residuals(&curve, &active_quotes, &mut resid[..n_residuals])
            {
                record_eval_error(
                    &eval_diagnostics,
                    eval_idx,
                    "residual evaluation",
                    params,
                    &format!("while evaluating {} quotes: {}", active_quotes.len(), e),
                );
                fill_penalty(resid, n_residuals, params);
                return;
            }

            for (r, w) in resid[..n_residuals].iter_mut().zip(weight_scales.iter()) {
                *r *= *w;
            }
        };

        // 3. Wrapper for AnalyticalDerivatives
        #[allow(clippy::type_complexity)]
        struct TargetDerivatives<'a, T: GlobalSolveTarget> {
            target: &'a T,
            active_quotes: &'a [T::Quote],
            weight_scales: &'a [f64],
            times: &'a [f64],
        }

        impl<'a, T: GlobalSolveTarget> finstack_core::math::solver_multi::AnalyticalDerivatives
            for TargetDerivatives<'a, T>
        {
            fn gradient(&self, _params: &[f64], _gradient: &mut [f64]) {
                // Not used for system solving
            }

            fn has_gradient(&self) -> bool {
                false
            }

            fn jacobian(&self, params: &[f64], jacobian: &mut [Vec<f64>]) -> Option<()> {
                // Optional feasibility check: avoid producing a Jacobian for infeasible params.
                if self
                    .target
                    .build_curve_for_solver_from_params(self.times, params)
                    .is_err()
                {
                    return None;
                }

                // 2. Compute Jacobian
                if self
                    .target
                    .jacobian(params, self.times, self.active_quotes, jacobian)
                    .is_err()
                {
                    return None;
                }

                // 3. Apply weights: J_ij *= weight_scales[i]
                for (i, row) in jacobian.iter_mut().enumerate() {
                    let scale = self.weight_scales.get(i).copied().unwrap_or(1.0);
                    for val in row.iter_mut() {
                        *val *= scale;
                    }
                }
                Some(())
            }

            fn has_jacobian(&self) -> bool {
                true
            }
        }

        // Solve
        let solution = if use_efficient {
            let derivatives = TargetDerivatives {
                target,
                active_quotes: &active_quotes,
                weight_scales: &weight_scales,
                times: &times,
            };
            solver.solve_system_with_jacobian_stats(residuals_func, &derivatives, &initials)?
        } else {
            solver.solve_system_with_dim_stats(residuals_func, &initials, n_residuals)?
        };
        let solved_params = solution.params;
        let stats = solution.stats;

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

        let success_tolerance = config.discount_curve.validation_tolerance;
        let mut report = CalibrationReport::for_type_with_tolerance(
            "global_fit",
            residuals_map,
            stats.iterations,
            success_tolerance,
        )
        .with_metadata("method", "global_fit_lm_weighted_lsq")
        .with_metadata("tolerance_definition", "abs_l2(weighted_residuals)")
        .with_metadata("validation_tolerance", format!("{:.2e}", success_tolerance))
        .with_metadata(
            "solver_tolerance",
            format!("{:.2e}", config.solver.tolerance()),
        )
        .with_metadata("residual_evals", stats.residual_evals.to_string())
        .with_metadata("residual_closure_evals", eval_counter.get().to_string())
        .with_metadata(
            "lm_termination_reason",
            format!("{:?}", stats.termination_reason),
        )
        .with_metadata("lm_jacobian_evals", stats.jacobian_evals.to_string())
        .with_metadata(
            "lm_final_weighted_resid_l2_norm",
            format!("{:.2e}", stats.final_residual_norm),
        )
        .with_metadata(
            "lm_final_step_norm",
            format!("{:.2e}", stats.final_step_norm),
        )
        .with_metadata("lm_lambda_final", format!("{:.2e}", stats.lambda_final))
        .with_metadata("final_unweighted_resid_l2_norm", format!("{:.2e}", l2_norm))
        .with_metadata(
            "final_unweighted_max_abs_residual",
            format!("{:.2e}", max_abs_residual),
        )
        .with_metadata(
            "final_weighted_resid_l2_norm",
            format!("{:.2e}", weighted_l2_norm),
        )
        .with_metadata(
            "final_weighted_max_abs_residual",
            format!("{:.2e}", weighted_max_abs_residual),
        );

        // Attach diagnostics from any infeasible evaluations encountered during solving.
        {
            let diag = eval_diagnostics.borrow();
            report.metadata.insert(
                "invalid_eval_count".to_string(),
                diag.invalid_eval_count.to_string(),
            );
            if let Some(first) = &diag.first_invalid_eval {
                report
                    .metadata
                    .insert("first_invalid_eval".to_string(), first.clone());
            }
        }

        report.update_solver_config(config.solver.clone());

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

    if initials.is_empty() {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Global fit requires at least one parameter; got n_params=0 and n_residuals={}.",
                n_residuals
            ),
            category: "global_fit".to_string(),
        });
    }

    if n_residuals < initials.len() {
        return Err(finstack_core::Error::Calibration {
            message: format!(
                "Global fit requires n_residuals >= n_params for a stable least-squares solve; got {} vs {}.",
                n_residuals,
                initials.len()
            ),
            category: "global_fit".to_string(),
        });
    }

    for (idx, &t) in times.iter().enumerate() {
        if !t.is_finite() || t <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Global solve requires strictly positive finite times; got {} at index {}.",
                    t, idx
                ),
                category: "global_fit".to_string(),
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
                category: "global_fit".to_string(),
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
                category: "global_fit".to_string(),
            });
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
struct EvalDiagnostics {
    invalid_eval_count: usize,
    first_invalid_eval: Option<String>,
}

fn record_eval_error(
    store: &RefCell<EvalDiagnostics>,
    eval_idx: usize,
    stage: &str,
    params: &[f64],
    detail: &str,
) {
    let mut diag = store.borrow_mut();
    diag.invalid_eval_count += 1;
    if diag.first_invalid_eval.is_some() {
        return;
    }
    let (min_param, max_param) = param_range(params);
    diag.first_invalid_eval = Some(format!(
        "Global fit {stage} failed at eval #{eval_idx} (param_range=[{min:.4e}, {max:.4e}]): {detail}",
        stage = stage,
        eval_idx = eval_idx,
        min = min_param,
        max = max_param,
        detail = detail
    ));
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
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::calibration::CalibrationConfig;
    use finstack_core::Error;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    #[derive(Clone, Debug)]
    struct DummyCurve(#[allow(dead_code)] Vec<f64>);

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
            GlobalFitOptimizer::optimize(&target, &quotes, &config).expect("optimization succeeds");

        let meta = report
            .metadata
            .get("final_unweighted_max_abs_residual")
            .expect("metadata should contain final_unweighted_max_abs_residual");
        let parsed: f64 = meta.parse().expect("metadata should parse as f64");

        assert!(
            (parsed - 2.0e-2).abs() < 1e-12,
            "final_unweighted_max_abs_residual metadata should use absolute values (got {})",
            parsed
        );
    }

    #[test]
    fn rejects_length_mismatch_between_times_and_initials() {
        let target = TestTarget::new(vec![1.0, 2.0], vec![0.0], vec![0.0, 0.0]);
        let quotes = vec![0usize, 1usize];
        let config = CalibrationConfig::default().with_tolerance(1.0);

        let err = GlobalFitOptimizer::optimize(&target, &quotes, &config).expect_err("should fail");
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

        let err = GlobalFitOptimizer::optimize(&target, &quotes, &config).expect_err("should fail");
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

        let err = GlobalFitOptimizer::optimize(&target, &quotes, &config).expect_err("should fail");
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
            GlobalFitOptimizer::optimize(&target, &quotes, &config).expect("optimization succeeds");

        assert!(report.residuals.contains_key("TEST-5"));
        assert!(report.residuals.contains_key("TEST-7"));

        let weighted_l2 = report
            .metadata
            .get("final_weighted_resid_l2_norm")
            .expect("metadata should include final_weighted_resid_l2_norm");
        let expected = ((0.01_f64 * 2.0).powi(2) + (0.02_f64 * 1.0).powi(2)).sqrt();
        assert_eq!(
            weighted_l2,
            &format!("{:.2e}", expected),
            "weighted_l2_norm should reflect weights"
        );
    }

    #[test]
    fn supports_overdetermined_least_squares() {
        let target = TestTarget::new(vec![1.0, 2.0], vec![0.0, 0.0], vec![0.01, -0.02, 0.03]);
        let quotes = vec![10usize, 11usize, 12usize];
        let config = CalibrationConfig::default().with_tolerance(1e12);

        let (_curve, report) =
            GlobalFitOptimizer::optimize(&target, &quotes, &config).expect("should succeed");

        assert_eq!(report.residuals.len(), 3);
        assert!(report.residuals.contains_key("GLOBAL-000010"));
        assert!(report.residuals.contains_key("GLOBAL-000011"));
        assert!(report.residuals.contains_key("GLOBAL-000012"));
    }

    #[test]
    fn intermediate_eval_errors_are_reported_but_not_fatal_if_final_evaluation_succeeds() {
        struct FlakyResidualTarget {
            inner: TestTarget,
            fail_once: AtomicBool,
        }

        impl GlobalSolveTarget for FlakyResidualTarget {
            type Quote = usize;
            type Curve = DummyCurve;

            fn build_time_grid_and_guesses(
                &self,
                quotes: &[Self::Quote],
            ) -> Result<(Vec<f64>, Vec<f64>, Vec<Self::Quote>)> {
                self.inner.build_time_grid_and_guesses(quotes)
            }

            fn build_curve_from_params(
                &self,
                times: &[f64],
                params: &[f64],
            ) -> Result<Self::Curve> {
                self.inner.build_curve_from_params(times, params)
            }

            fn calculate_residuals(
                &self,
                curve: &Self::Curve,
                quotes: &[Self::Quote],
                residuals: &mut [f64],
            ) -> Result<()> {
                if self
                    .fail_once
                    .compare_exchange(false, true, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
                    .is_ok()
                {
                    return Err(finstack_core::Error::Calibration {
                        message: "intentional transient residual failure".to_string(),
                        category: "test".to_string(),
                    });
                }
                self.inner.calculate_residuals(curve, quotes, residuals)
            }
        }

        let target = FlakyResidualTarget {
            inner: TestTarget::from_len(2, vec![0.01, 0.02]),
            fail_once: AtomicBool::new(false),
        };
        let quotes = vec![0usize, 1usize];
        let config = CalibrationConfig::default().with_tolerance(1e12);

        let (_curve, report) =
            GlobalFitOptimizer::optimize(&target, &quotes, &config).expect("should succeed");

        let invalid_count: usize = report
            .metadata
            .get("invalid_eval_count")
            .expect("should include invalid_eval_count")
            .parse()
            .expect("invalid_eval_count should parse");
        assert!(invalid_count >= 1);
        assert!(
            report.metadata.contains_key("first_invalid_eval"),
            "should include first_invalid_eval"
        );
    }
}
