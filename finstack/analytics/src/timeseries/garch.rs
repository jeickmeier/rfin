//! GARCH model trait and shared types.
//!
//! Defines the `GarchModel` trait that all GARCH-family models implement,
//! along with parameter, fit result, and configuration types.

use super::diagnostics;
use super::forecast::VarianceForecast;
use super::innovations::InnovationDist;

/// GARCH-family model tag used to interpret [`GarchParams`] correctly.
///
/// Different family members have different persistence definitions and
/// different unconditional-variance formulas. Tagging the params struct
/// removes the ambiguity that otherwise silently mis-reports both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GarchFamily {
    /// Symmetric GARCH(1,1): sigma^2_t = omega + alpha*eps^2 + beta*sigma^2_{t-1}.
    Garch11,
    /// GJR-GARCH(1,1) with asymmetric leverage gamma.
    GjrGarch11,
    /// EGARCH(1,1): log(sigma^2) = omega + beta*log(sigma^2_{t-1}) + ...
    Egarch11,
}

/// Default GARCH family tag used by `#[serde(default = "...")]`.
fn default_garch_family() -> GarchFamily {
    GarchFamily::Garch11
}

/// Default mean parameter (`0.0`) used by `#[serde(default = "...")]`.
fn default_mean() -> f64 {
    0.0
}

/// Model parameters in canonical order.
///
/// Each GARCH variant interprets these fields according to its own
/// parameterization. The `gamma` field holds model-specific parameters
/// (e.g., leverage coefficient for EGARCH/GJR). The `family` tag is used
/// by [`persistence`](Self::persistence), [`half_life`](Self::half_life),
/// and [`unconditional_variance`](Self::unconditional_variance) to apply
/// the correct formula.
///
/// Deserialising older snapshots that lack the `family` field defaults to
/// [`crate::timeseries::GarchFamily::Garch11`].
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GarchParams {
    /// Intercept (omega).
    pub omega: f64,
    /// ARCH coefficient (alpha).
    pub alpha: f64,
    /// GARCH coefficient (beta).
    pub beta: f64,
    /// Model-specific extra parameter (leverage gamma for EGARCH/GJR; unused in GARCH(1,1)).
    pub gamma: Option<f64>,
    /// Innovation distribution (includes estimated dof for Student-t).
    pub dist: InnovationDist,
    /// GARCH-family tag. Controls persistence / unconditional-variance formulas.
    #[serde(default = "default_garch_family")]
    pub family: GarchFamily,
    /// Constant mean `mu` in the return model `r_t = mu + eps_t` where
    /// `eps_t ~ N(0, sigma^2_t)` (Gaussian) or the rescaled Student-t.
    ///
    /// Defaults to `0.0` for backward compatibility. `fit_garch_mle`
    /// pins this to the sample mean before the MLE starts so the
    /// variance recursion and the log-likelihood both see the demeaned
    /// residual `eps_t = r_t - mu` rather than the raw return. Skipping
    /// this demeaning biases `omega` upward for equity return series
    /// with non-zero drift.
    #[serde(default = "default_mean")]
    pub mean: f64,
}

impl GarchParams {
    /// Persistence of volatility shocks under the model's own recursion.
    ///
    /// - `Garch11`: `alpha + beta`
    /// - `GjrGarch11`: `alpha + beta + gamma/2` (assumes symmetric
    ///   innovations, e.g. Gaussian or symmetric Student-t, so
    ///   `E[I{z<0} z^2] = 1/2`)
    /// - `Egarch11`: `beta` (EGARCH operates on log-variance; alpha acts
    ///   on the magnitude innovation g(z_t) not on previous log-variance)
    #[must_use]
    pub fn persistence(&self) -> f64 {
        match self.family {
            GarchFamily::Garch11 => self.alpha + self.beta,
            GarchFamily::GjrGarch11 => self.alpha + self.beta + self.gamma.unwrap_or(0.0) / 2.0,
            GarchFamily::Egarch11 => self.beta,
        }
    }

    /// Unconditional variance under the model's stationary distribution.
    ///
    /// - Symmetric GARCH(1,1): `omega / (1 - alpha - beta)` when persistence < 1.
    /// - GJR-GARCH(1,1): `omega / (1 - alpha - beta - gamma/2)` (symmetric
    ///   innovations, so `E[I{z<0} z^2] = 1/2`).
    /// - EGARCH(1,1): the log-variance unconditional level is
    ///   `omega / (1 - beta)`. The unconditional *variance* under a Gaussian
    ///   standardised innovation is `exp(omega/(1-beta) + 0.5 * sigma_g^2)`
    ///   with a non-trivial correction that depends on `alpha` and `gamma`.
    ///   Returning a simple point estimate here would be misleading, so
    ///   EGARCH returns `None`; callers should use EGARCH-specific tooling
    ///   or sample from the filter for a long horizon.
    ///
    /// Returns `None` for non-stationary (persistence >= 1) or
    /// ill-conditioned (omega <= 0, persistence <= 0) parameterisations.
    #[must_use]
    pub fn unconditional_variance(&self) -> Option<f64> {
        match self.family {
            GarchFamily::Egarch11 => None,
            GarchFamily::Garch11 | GarchFamily::GjrGarch11 => {
                let p = self.persistence();
                if !self.omega.is_finite()
                    || self.omega <= 0.0
                    || !p.is_finite()
                    || p >= 1.0
                    || p <= 0.0
                {
                    return None;
                }
                let variance = self.omega / (1.0 - p);
                if variance.is_finite() && variance > 0.0 {
                    Some(variance)
                } else {
                    None
                }
            }
        }
    }

    /// Half-life of a variance shock in periods: `ln(2) / (-ln(persistence))`.
    ///
    /// Defined when `0 < persistence < 1`.
    #[must_use]
    pub fn half_life(&self) -> Option<f64> {
        let p = self.persistence();
        if !p.is_finite() || p <= 0.0 || p >= 1.0 {
            return None;
        }
        Some(2.0_f64.ln() / (-p.ln()))
    }

    /// Pack parameters into a flat slice for the optimizer.
    ///
    /// `mean` is deliberately excluded — it is fixed to the sample mean
    /// before the MLE starts so variance parameters are estimated on
    /// demeaned residuals.
    #[must_use]
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = vec![self.omega, self.alpha, self.beta];
        if let Some(g) = self.gamma {
            v.push(g);
        }
        if let InnovationDist::StudentT(nu) = self.dist {
            v.push(nu);
        }
        v
    }

    /// Unpack from a flat slice. Model-specific interpretation.
    ///
    /// `mean` defaults to 0.0; callers that want a demeaned fit should
    /// set it explicitly on the returned value (or call
    /// [`Self::with_mean`]).
    #[must_use]
    pub fn from_vec(v: &[f64], dist: InnovationDist, has_gamma: bool, family: GarchFamily) -> Self {
        let omega = v[0];
        let alpha = v[1];
        let beta = v[2];
        let mut idx = 3;
        let gamma = if has_gamma {
            let g = v[idx];
            idx += 1;
            Some(g)
        } else {
            None
        };
        let dist = match dist {
            InnovationDist::StudentT(_) => InnovationDist::StudentT(v[idx]),
            InnovationDist::Gaussian => InnovationDist::Gaussian,
        };
        Self {
            family,
            omega,
            alpha,
            beta,
            gamma,
            dist,
            mean: 0.0,
        }
    }

    /// Return a copy with the mean field set to `mean`.
    #[must_use]
    pub fn with_mean(mut self, mean: f64) -> Self {
        self.mean = mean;
        self
    }
}

/// Configuration for GARCH MLE fitting.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FitConfig {
    /// Maximum optimizer iterations (default: 500).
    pub max_iter: usize,
    /// Function-value convergence tolerance (default: 1e-7).
    pub tol: f64,
    /// Use variance targeting for initial omega (default: true).
    pub variance_targeting: bool,
    /// Number of grid points per dimension for initial parameter search (default: 10).
    pub grid_points: usize,
    /// Number of Halton-perturbed restarts applied after the initial solve
    /// (default: 4). Set to `0` to disable multi-start and restore the
    /// legacy single-start behavior.
    #[serde(default = "default_num_restarts")]
    pub num_restarts: usize,
    /// Perturbation scale for multi-start initial points, expressed as a
    /// fraction of `|x0_i| + 1e-6` per dimension (default: 0.25 = 25%).
    #[serde(default = "default_perturbation_scale")]
    pub perturbation_scale: f64,
}

fn default_num_restarts() -> usize {
    4
}

fn default_perturbation_scale() -> f64 {
    0.25
}

impl Default for FitConfig {
    fn default() -> Self {
        Self {
            max_iter: 500,
            tol: 1e-7,
            variance_targeting: true,
            grid_points: 10,
            num_restarts: default_num_restarts(),
            perturbation_scale: default_perturbation_scale(),
        }
    }
}

/// Complete result of a GARCH model fit.
#[must_use]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GarchFit {
    /// Model name.
    pub model: String,
    /// Estimated parameters.
    pub params: GarchParams,
    /// Approximate standard errors from inverse Hessian (Cramer-Rao bound).
    pub std_errors: Option<Vec<f64>>,
    /// Maximized log-likelihood.
    pub log_likelihood: f64,
    /// Number of observations used in fitting.
    pub n_obs: usize,
    /// Number of estimated parameters (model + distribution).
    pub n_params: usize,
    /// Akaike Information Criterion: -2*LL + 2*k.
    pub aic: f64,
    /// Bayesian Information Criterion: -2*LL + k*ln(n).
    pub bic: f64,
    /// Hannan-Quinn Information Criterion: -2*LL + 2*k*ln(ln(n)).
    pub hqic: f64,
    /// Conditional variance series (length = n_obs).
    pub conditional_variances: Vec<f64>,
    /// Standardized residuals: z_t = epsilon_t / sigma_t (length = n_obs).
    pub standardized_residuals: Vec<f64>,
    /// Terminal conditional variance (last sigma^2_t), used as forecast anchor.
    pub terminal_variance: f64,
    /// Convergence flag from optimizer.
    pub converged: bool,
    /// Number of optimizer iterations.
    pub iterations: usize,
}

impl GarchFit {
    /// Ljung-Box test p-value on squared standardized residuals.
    ///
    /// Tests H0: no remaining ARCH effects.
    #[must_use]
    pub fn ljung_box_squared(&self, lags: usize) -> f64 {
        let sq: Vec<f64> = self.standardized_residuals.iter().map(|z| z * z).collect();
        let (_, pval) = diagnostics::ljung_box(&sq, lags);
        pval
    }

    /// ARCH-LM test p-value on standardized residuals.
    #[must_use]
    pub fn arch_lm_test(&self, lags: usize) -> f64 {
        let (_, pval) = diagnostics::arch_lm(&self.standardized_residuals, lags);
        pval
    }

    /// Summary string for display.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut s = format!("{} Model Results\n", self.model);
        s.push_str(&format!(
            "Observations: {}  Parameters: {}\n",
            self.n_obs, self.n_params
        ));
        s.push_str(&format!(
            "Log-Likelihood: {:.4}  AIC: {:.4}  BIC: {:.4}\n",
            self.log_likelihood, self.aic, self.bic
        ));
        s.push_str(&format!(
            "omega: {:.6}  alpha: {:.6}  beta: {:.6}",
            self.params.omega, self.params.alpha, self.params.beta
        ));
        if let Some(g) = self.params.gamma {
            s.push_str(&format!("  gamma: {:.6}", g));
        }
        if let InnovationDist::StudentT(nu) = self.params.dist {
            s.push_str(&format!("  nu: {:.2}", nu));
        }
        s.push_str(&format!(
            "\nPersistence: {:.6}  Converged: {}",
            self.params.persistence(),
            self.converged
        ));
        s
    }
}

/// Common interface for all GARCH-family models.
pub trait GarchModel: Send + Sync {
    /// Human-readable model name.
    fn name(&self) -> &'static str;

    /// GARCH-family tag; used to build correctly-typed [`GarchParams`].
    fn family(&self) -> GarchFamily;

    /// Number of model-specific parameters (excludes innovation distribution params).
    fn num_params(&self) -> usize;

    /// Whether the model includes a leverage/asymmetry parameter (`gamma`).
    fn has_gamma(&self) -> bool;

    /// Parameter bounds for the optimizer in canonical order
    /// (`omega`, `alpha`, `beta`, optional `gamma`). The innovation-distribution
    /// bound (e.g. Student-t dof) is appended by the default [`Self::fit`]
    /// implementation and must not be included here.
    ///
    /// `sample_var` is supplied for models that anchor `omega` bounds to the
    /// sample variance of returns (e.g. symmetric GARCH / GJR).
    fn parameter_bounds(&self, sample_var: f64) -> Vec<(f64, f64)>;

    /// Stationarity check on a packed parameter slice (same order as
    /// [`Self::parameter_bounds`]). Returning `false` makes the optimizer
    /// assign a large penalty to that point.
    fn is_stationary(&self, params: &[f64]) -> bool;

    /// Fit the model to a return series via maximum likelihood.
    ///
    /// The default implementation wires the model-specific hooks
    /// ([`Self::has_gamma`], [`Self::parameter_bounds`],
    /// [`Self::is_stationary`]) into the shared `fit_garch_mle`
    /// driver. Models override only when they need non-standard
    /// initial-value or optimizer strategies.
    ///
    /// # Arguments
    ///
    /// * `returns` - Log return series.
    /// * `dist` - Innovation distribution for the standardized residuals.
    /// * `config` - Optional optimizer and initialization configuration.
    ///
    /// # Returns
    ///
    /// A [`GarchFit`] containing estimated parameters, diagnostics, and
    /// forecast state.
    ///
    /// # Errors
    ///
    /// Returns an error when there are fewer than 10 observations, variance is
    /// zero or non-finite, no stationary initial point is found, or the
    /// optimizer cannot produce a finite likelihood.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use finstack_analytics::timeseries::{Garch11, GarchModel, InnovationDist};
    ///
    /// let returns = vec![
    ///     0.004, -0.006, 0.002, 0.009, -0.011, 0.003, -0.004, 0.006, -0.008,
    ///     0.005, 0.002, -0.003, 0.007, -0.010, 0.004, 0.006, -0.002, 0.001,
    ///     -0.005, 0.008,
    /// ];
    /// let fit = Garch11.fit(&returns, InnovationDist::Gaussian, None)?;
    /// assert_eq!(fit.model, "GARCH(1,1)");
    /// # Ok::<(), finstack_core::Error>(())
    /// ```
    fn fit(
        &self,
        returns: &[f64],
        dist: InnovationDist,
        config: Option<&FitConfig>,
    ) -> crate::Result<GarchFit> {
        let default_config = FitConfig::default();
        let config = config.unwrap_or(&default_config);

        let sample_var = {
            let n = returns.len() as f64;
            if n < 2.0 {
                0.0
            } else {
                let mean = returns.iter().sum::<f64>() / n;
                returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0)
            }
        };

        let mut bounds = self.parameter_bounds(sample_var);
        if let InnovationDist::StudentT(_) = dist {
            bounds.push((InnovationDist::dof_lower_bound(), 100.0));
        }

        fit_garch_mle(
            self,
            returns,
            dist,
            config,
            self.has_gamma(),
            &bounds,
            |x| self.is_stationary(x),
        )
    }

    /// Compute conditional variance series given parameters and returns.
    fn filter(&self, returns: &[f64], params: &GarchParams, sigma2_out: &mut [f64]);

    /// Compute the log-likelihood given parameters and a return series.
    fn log_likelihood(&self, returns: &[f64], params: &GarchParams, dist: InnovationDist) -> f64;

    /// h-step ahead variance forecast from the last fitted state.
    ///
    /// `terminal_residual`, when supplied, is the last demeaned residual
    /// `r_t - mu`. That lets the 1-step forecast use the observable last
    /// shock instead of the iterated conditional expectation from
    /// `fit.terminal_variance`.
    fn forecast(
        &self,
        fit: &GarchFit,
        horizons: &[usize],
        trading_days_per_year: f64,
        terminal_residual: Option<f64>,
    ) -> Vec<VarianceForecast>;

    /// Parameter names in canonical order.
    fn param_names(&self) -> Vec<&'static str>;
}

#[inline]
fn variance_targeted_omega(
    family: GarchFamily,
    sample_var: f64,
    alpha: f64,
    beta: f64,
    gamma: f64,
) -> f64 {
    match family {
        GarchFamily::Garch11 => sample_var * (1.0 - alpha - beta),
        GarchFamily::GjrGarch11 => sample_var * (1.0 - alpha - beta - gamma / 2.0),
        GarchFamily::Egarch11 => sample_var.max(1e-20).ln() * (1.0 - beta),
    }
}

/// Shared MLE fitting logic used by all GARCH-family models.
///
/// Implements variance targeting grid search for initial parameters,
/// Nelder-Mead optimization, Hessian-based standard errors, and
/// information criteria computation.
#[tracing::instrument(level = "debug", skip(model, returns, config, bounds, stationarity_check), fields(n = returns.len(), dist = ?dist, has_gamma))]
pub(crate) fn fit_garch_mle<M: GarchModel + ?Sized>(
    model: &M,
    returns: &[f64],
    dist: InnovationDist,
    config: &FitConfig,
    has_gamma: bool,
    bounds: &[(f64, f64)],
    stationarity_check: impl Fn(&[f64]) -> bool,
) -> crate::Result<GarchFit> {
    let n = returns.len();
    if n < 10 {
        tracing::debug!(
            n,
            reason = "insufficient_observations",
            "GARCH fit rejected input"
        );
        return Err(finstack_core::Error::Validation(
            "GARCH fitting requires at least 10 observations".to_string(),
        ));
    }

    // Sample mean: pinned throughout the fit so the variance recursion
    // and log-likelihood both operate on demeaned residuals. This is the
    // standard "mean + GARCH variance" two-step decomposition.
    let sample_mean: f64 = returns.iter().sum::<f64>() / n as f64;

    let sample_var = {
        returns
            .iter()
            .map(|r| (r - sample_mean).powi(2))
            .sum::<f64>()
            / (n as f64 - 1.0)
    };

    if sample_var < 1e-20 || !sample_var.is_finite() {
        tracing::debug!(
            n,
            sample_var,
            reason = "zero_or_non_finite_variance",
            "GARCH fit rejected input"
        );
        return Err(finstack_core::Error::Validation(
            "Return series has zero or non-finite variance".to_string(),
        ));
    }

    // Grid search for initial parameters
    let gp = config.grid_points.max(3);
    let mut best_ll = f64::NEG_INFINITY;
    let mut best_params_vec: Vec<f64> = Vec::new();

    let alpha_grid: Vec<f64> = (1..gp)
        .map(|i| 0.02 + 0.18 * i as f64 / gp as f64)
        .collect();
    let beta_grid: Vec<f64> = (1..gp)
        .map(|i| 0.70 + 0.27 * i as f64 / gp as f64)
        .collect();
    let gamma_grid: Vec<f64> = if has_gamma {
        vec![-0.10, -0.05, 0.0, 0.05, 0.10, 0.15]
    } else {
        vec![]
    };

    for &alpha in &alpha_grid {
        for &beta in &beta_grid {
            let gammas = if has_gamma {
                gamma_grid.clone()
            } else {
                vec![0.0]
            };
            for &g in &gammas {
                let omega = if config.variance_targeting {
                    variance_targeted_omega(model.family(), sample_var, alpha, beta, g)
                } else {
                    sample_var * 0.05
                };

                if omega <= 0.0 {
                    continue;
                }

                let mut pvec = vec![omega, alpha, beta];
                if has_gamma {
                    pvec.push(g);
                }
                if let InnovationDist::StudentT(_) = dist {
                    pvec.push(8.0); // initial dof guess
                }

                if !stationarity_check(&pvec) {
                    continue;
                }

                let params = GarchParams::from_vec(&pvec, dist, has_gamma, model.family())
                    .with_mean(sample_mean);
                let ll = model.log_likelihood(returns, &params, dist);

                if ll.is_finite() && ll > best_ll {
                    best_ll = ll;
                    best_params_vec = pvec;
                }
            }
        }
    }

    if best_params_vec.is_empty() {
        tracing::debug!(
            n,
            has_gamma,
            dist = ?dist,
            reason = "no_stationary_grid_point",
            "GARCH fit using fallback starting point"
        );
        // Fallback starting point
        let alpha = 0.05;
        let beta = 0.90;
        let omega = variance_targeted_omega(model.family(), sample_var, alpha, beta, 0.0);
        best_params_vec = vec![omega, alpha, beta];
        if has_gamma {
            best_params_vec.push(0.0);
        }
        if let InnovationDist::StudentT(_) = dist {
            best_params_vec.push(8.0);
        }
    }

    // Optimize with Nelder-Mead
    let stationarity_check_clone = &stationarity_check;
    let neg_ll = |x: &[f64]| -> f64 {
        if !stationarity_check_clone(x) {
            return 1e18;
        }
        let params =
            GarchParams::from_vec(x, dist, has_gamma, model.family()).with_mean(sample_mean);
        let ll = model.log_likelihood(returns, &params, dist);
        if ll.is_finite() {
            -ll
        } else {
            1e18
        }
    };

    let optimizer = super::optimizer::NelderMead::new(config.max_iter, config.tol);
    let opt_bounds: super::optimizer::Bounds = bounds.to_vec();
    let result = if config.num_restarts == 0 {
        optimizer.minimize(neg_ll, &best_params_vec, &opt_bounds)
    } else {
        // Halton-perturbed multi-start escapes plateau/ridge behavior
        // near the GARCH stationarity frontier where single-start
        // Nelder-Mead can stall at a local minimum.
        optimizer.minimize_multi_start(
            neg_ll,
            &best_params_vec,
            &opt_bounds,
            config.num_restarts,
            config.perturbation_scale,
        )
    };

    let final_params =
        GarchParams::from_vec(&result.x, dist, has_gamma, model.family()).with_mean(sample_mean);
    let final_ll = -result.f_val;
    if !result.converged {
        tracing::warn!(
            iterations = result.iterations,
            final_objective = result.f_val,
            final_log_likelihood = final_ll,
            "GARCH optimizer did not converge"
        );
    }

    // Compute conditional variances and standardized residuals.
    // Residuals are demeaned, so z_t = (r_t - mu) / sigma_t.
    let mut sigma2 = vec![0.0; n];
    model.filter(returns, &final_params, &mut sigma2);

    let std_resid: Vec<f64> = returns
        .iter()
        .zip(sigma2.iter())
        .map(|(&r, &s2)| {
            if s2 < 1e-20 {
                tracing::debug!(
                    variance = s2,
                    reason = "variance_floor",
                    "GARCH standardized residual using variance floor"
                );
            }
            let s = s2.max(1e-20).sqrt();
            (r - sample_mean) / s
        })
        .collect();

    let terminal_var = *sigma2.last().unwrap_or(&sample_var);

    // Standard errors via finite-difference Hessian of negative log-likelihood
    let std_errors = {
        let hess = super::optimizer::finite_diff_hessian(&neg_ll, &result.x, 1e-5);
        super::optimizer::invert_hessian_diag(&hess).map(|diag| {
            diag.iter()
                .map(|&d| if d > 0.0 { d.sqrt() } else { f64::NAN })
                .collect()
        })
    };

    let n_params = model.num_params() + dist.num_params();

    Ok(GarchFit {
        model: model.name().to_string(),
        params: final_params,
        std_errors,
        log_likelihood: final_ll,
        n_obs: n,
        n_params,
        aic: diagnostics::aic(final_ll, n_params),
        bic: diagnostics::bic(final_ll, n_params, n),
        hqic: diagnostics::hqic(final_ll, n_params, n),
        conditional_variances: sigma2,
        standardized_residuals: std_resid,
        terminal_variance: terminal_var,
        converged: result.converged,
        iterations: result.iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::{variance_targeted_omega, GarchFamily, GarchParams};
    use crate::timeseries::InnovationDist;

    #[test]
    fn variance_targeting_matches_garch_family() {
        let sample_var = 0.04_f64;
        let alpha = 0.10_f64;
        let beta = 0.85_f64;
        let gamma = 0.20_f64;

        let garch = variance_targeted_omega(GarchFamily::Garch11, sample_var, alpha, beta, gamma);
        assert!((garch - 0.002).abs() < 1e-12);

        let gjr = variance_targeted_omega(GarchFamily::GjrGarch11, sample_var, alpha, beta, gamma);
        assert!((gjr + 0.002).abs() < 1e-12);

        let egarch = variance_targeted_omega(GarchFamily::Egarch11, sample_var, alpha, beta, gamma);
        assert!((egarch - sample_var.ln() * (1.0 - beta)).abs() < 1e-12);
    }

    #[test]
    fn unconditional_variance_rejects_invalid_symmetric_params() {
        let params = GarchParams {
            omega: -0.00001,
            alpha: 0.05,
            beta: 0.90,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: GarchFamily::Garch11,
            mean: 0.0,
        };
        assert!(params.unconditional_variance().is_none());

        let params = GarchParams {
            omega: f64::NAN,
            alpha: 0.05,
            beta: 0.90,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: GarchFamily::Garch11,
            mean: 0.0,
        };
        assert!(params.unconditional_variance().is_none());
    }

    #[test]
    fn half_life_rejects_non_finite_persistence() {
        let params = GarchParams {
            omega: 0.00001,
            alpha: f64::NAN,
            beta: 0.90,
            gamma: None,
            dist: InnovationDist::Gaussian,
            family: GarchFamily::Garch11,
            mean: 0.0,
        };
        assert!(params.half_life().is_none());
    }
}
