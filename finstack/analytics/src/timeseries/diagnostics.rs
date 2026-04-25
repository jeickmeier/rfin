//! Diagnostic tests for GARCH model residuals.
//!
//! Provides Ljung-Box test for serial correlation, ARCH-LM test for
//! remaining heteroskedasticity, and information criteria (AIC, BIC, HQIC).
//!
//! # References
//!
//! - Ljung & Box (1978): see docs/REFERENCES.md#ljungBox1978
//! - Engle (1982): see docs/REFERENCES.md#engle1982Arch

/// Ljung-Box test for serial correlation in a series.
///
/// Tests H0: no autocorrelation up to lag `k`.
/// Q = n(n+2) * sum_{j=1}^{k} rho_j^2 / (n - j) ~ chi^2(k)
///
/// # Returns
/// (test_statistic, p_value). Low p-value rejects H0 (autocorrelation present).
#[must_use]
pub fn ljung_box(series: &[f64], lags: usize) -> (f64, f64) {
    let n = series.len();
    if lags == 0 || n <= lags {
        return (0.0, 1.0);
    }

    let nf = n as f64;
    let mean = series.iter().sum::<f64>() / nf;

    // Compute variance (denominator for autocorrelation)
    let var: f64 = series.iter().map(|x| (x - mean).powi(2)).sum();
    if var < 1e-30 {
        return (0.0, 1.0);
    }

    // Compute Q statistic
    let mut q = 0.0;
    for lag in 1..=lags {
        let mut auto_cov = 0.0;
        for t in lag..n {
            auto_cov += (series[t] - mean) * (series[t - lag] - mean);
        }
        let rho = auto_cov / var;
        q += rho * rho / (nf - lag as f64);
    }
    q *= nf * (nf + 2.0);

    // Chi-squared p-value with `lags` degrees of freedom
    let pval = chi2_sf(q, lags as f64);

    (q, pval)
}

/// ARCH-LM test for remaining heteroskedasticity.
///
/// Regresses z_t^2 on a constant and z_{t-1}^2, ..., z_{t-q}^2.
/// Test statistic: T * R^2 ~ chi^2(q) under H0 of no ARCH effects.
///
/// # Returns
/// (test_statistic, p_value). Low p-value indicates remaining ARCH effects.
#[must_use]
pub fn arch_lm(residuals: &[f64], lags: usize) -> (f64, f64) {
    let n = residuals.len();
    if lags == 0 || n.saturating_sub(1) <= lags {
        return (0.0, 1.0);
    }

    // Compute squared residuals
    let z2: Vec<f64> = residuals.iter().map(|z| z * z).collect();

    let t = n - lags;
    let tf = t as f64;

    // Dependent variable: z2[lags..n]
    // Regressors: constant + z2[lags-1..n-1], z2[lags-2..n-2], ...

    // Compute mean of dependent variable
    let y_mean: f64 = z2[lags..].iter().sum::<f64>() / tf;

    // Compute OLS via normal equations for R^2
    // We compute R^2 = 1 - SSR/SST
    // SST = sum((y - ybar)^2)
    let sst: f64 = z2[lags..].iter().map(|&y| (y - y_mean).powi(2)).sum();

    if sst < 1e-30 {
        return (0.0, 1.0);
    }

    // Simple OLS: regress z2[lags..] on z2[lags-k..n-k] for k = 1..lags
    // Use the correlation-based approach for simplicity
    // Fit via: y = Xb + e, b = (X'X)^-1 X'y
    // But for the test we just need R^2 = 1 - SSR/SST

    // Build X'X and X'y (including constant)
    let ncols = lags + 1; // constant + lags regressors
    let mut xtx = vec![vec![0.0; ncols]; ncols];
    let mut xty = vec![0.0; ncols];

    for i in 0..t {
        let row = lags + i;
        let y = z2[row];

        // Constant term
        xtx[0][0] += 1.0;
        xty[0] += y;

        for k in 1..ncols {
            let xk = z2[row - k];
            xtx[0][k] += xk;
            xtx[k][0] += xk;
            xty[k] += xk * y;

            for j in k..ncols {
                let xj = z2[row - j];
                xtx[k][j] += xk * xj;
                xtx[j][k] = xtx[k][j];
            }
        }
    }

    // Solve via Cholesky or simple matrix inversion using nalgebra
    let mat = nalgebra::DMatrix::from_fn(ncols, ncols, |i, j| xtx[i][j]);
    let rhs = nalgebra::DVector::from_fn(ncols, |i, _| xty[i]);

    let beta = match mat.clone().try_inverse() {
        Some(inv) => inv * rhs,
        None => return (0.0, 1.0),
    };

    // Compute SSR = sum((y - X*beta)^2)
    let mut ssr = 0.0;
    for i in 0..t {
        let row = lags + i;
        let y = z2[row];
        let mut y_hat = beta[0]; // constant
        for k in 1..ncols {
            y_hat += beta[k] * z2[row - k];
        }
        ssr += (y - y_hat).powi(2);
    }

    let r2 = 1.0 - ssr / sst;
    let r2 = r2.clamp(0.0, 1.0);
    let stat = tf * r2;

    let pval = chi2_sf(stat, lags as f64);

    (stat, pval)
}

/// Akaike Information Criterion.
///
/// AIC = -2 * log_likelihood + 2 * n_params
#[must_use]
pub fn aic(log_likelihood: f64, n_params: usize) -> f64 {
    -2.0 * log_likelihood + 2.0 * n_params as f64
}

/// Bayesian Information Criterion.
///
/// BIC = -2 * log_likelihood + n_params * ln(n_obs)
#[must_use]
pub fn bic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    -2.0 * log_likelihood + n_params as f64 * (n_obs as f64).ln()
}

/// Hannan-Quinn Information Criterion.
///
/// HQIC = -2 * log_likelihood + 2 * n_params * ln(ln(n_obs))
#[must_use]
pub fn hqic(log_likelihood: f64, n_params: usize, n_obs: usize) -> f64 {
    -2.0 * log_likelihood + 2.0 * n_params as f64 * (n_obs as f64).ln().ln()
}

/// Survival function of chi-squared distribution: P(X > x) for X ~ chi^2(df).
///
/// Uses the regularized incomplete gamma function: chi2_sf(x, df) = Q(df/2, x/2)
/// where Q(a, x) = 1 - P(a, x) is the upper regularized incomplete gamma.
fn chi2_sf(x: f64, df: f64) -> f64 {
    if x <= 0.0 || df <= 0.0 {
        return 1.0;
    }
    upper_regularized_gamma(df / 2.0, x / 2.0)
}

/// Upper regularized incomplete gamma function Q(a, x) = 1 - P(a, x).
///
/// P(a,x) is computed via series for x < a+1, continued fraction otherwise.
fn upper_regularized_gamma(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        1.0 - lower_gamma_series(a, x)
    } else {
        upper_gamma_cf(a, x)
    }
}

/// Lower regularized incomplete gamma P(a,x) via series expansion.
///
/// P(a,x) = e^{-x} x^a / Gamma(a) * sum_{n=0}^{inf} x^n / prod_{k=0}^{n}(a+k)
fn lower_gamma_series(a: f64, x: f64) -> f64 {
    let ln_gamma_a = finstack_core::math::special_functions::ln_gamma(a);

    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = 1.0 / a;

    for _ in 1..300 {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * 1e-14 {
            break;
        }
    }

    let log_p = -x + a * x.ln() - ln_gamma_a + sum.ln();
    log_p.exp().clamp(0.0, 1.0)
}

/// Upper regularized incomplete gamma Q(a,x) via continued fraction (Lentz).
///
/// Uses the standard CF representation from Numerical Recipes:
/// Q(a,x) = e^{-x} x^a / Gamma(a) * (1/(x-a+1+ K_{i=1}^{inf} a_i/b_i))
/// where a_i = -i*(i-a), b_i = x - a + 1 + 2i
fn upper_gamma_cf(a: f64, x: f64) -> f64 {
    let ln_gamma_a = finstack_core::math::special_functions::ln_gamma(a);

    let fpmin = 1e-30;
    let b0 = x + 1.0 - a;
    let mut c = 1.0 / fpmin;
    let mut d = 1.0 / b0;
    let mut h = d;

    for i in 1..300 {
        let ai = -(i as f64) * (i as f64 - a);
        let bi = b0 + 2.0 * i as f64;

        d = ai * d + bi;
        if d.abs() < fpmin {
            d = fpmin;
        }
        d = 1.0 / d;

        c = bi + ai / c;
        if c.abs() < fpmin {
            c = fpmin;
        }

        let delta = c * d;
        h *= delta;

        if (delta - 1.0).abs() < 1e-14 {
            break;
        }
    }

    let log_q = -x + a * x.ln() - ln_gamma_a + h.ln();
    log_q.exp().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aic_bic_formulas() {
        let ll = -100.0;
        assert!((aic(ll, 3) - 206.0).abs() < 1e-10);
        // BIC with n=100: -2*(-100) + 3*ln(100) = 200 + 3*4.605 = 213.816
        let b = bic(ll, 3, 100);
        assert!((b - (200.0 + 3.0 * 100.0_f64.ln())).abs() < 1e-10);
    }

    #[test]
    fn ljung_box_white_noise() {
        // White noise should not reject H0
        // Use a simple deterministic pseudo-random-like sequence
        let n = 500;
        let series: Vec<f64> = (0..n)
            .map(|i| {
                let x = (i as f64 * std::f64::consts::E * 100.0).sin();
                x * 0.01
            })
            .collect();

        let (stat, pval) = ljung_box(&series, 10);
        assert!(stat.is_finite());
        assert!((0.0..=1.0).contains(&pval));
    }

    #[test]
    fn ljung_box_handles_extreme_lag_without_overflow() {
        assert_eq!(ljung_box(&[0.01, -0.01], usize::MAX), (0.0, 1.0));
    }

    #[test]
    fn ljung_box_autocorrelated_series() {
        // AR(1) process should show autocorrelation
        let n = 500;
        let phi = 0.8;
        let mut series = vec![0.0; n];
        for t in 1..n {
            // Deterministic pseudo-innovations
            let e = ((t as f64 * 1.618033988 * 100.0).sin()) * 0.01;
            series[t] = phi * series[t - 1] + e;
        }

        let (stat, pval) = ljung_box(&series, 10);
        assert!(stat > 0.0);
        // Should reject H0 for strongly autocorrelated series
        assert!(
            pval < 0.05,
            "Ljung-Box should reject H0 for AR(1) with phi=0.8, pval={}",
            pval
        );
    }

    #[test]
    fn arch_lm_basic() {
        // i.i.d. residuals should not show ARCH effects
        let n = 200;
        let resid: Vec<f64> = (0..n)
            .map(|i| (i as f64 * std::f64::consts::PI * 100.0).sin() * 0.01)
            .collect();

        let (stat, pval) = arch_lm(&resid, 5);
        assert!(stat.is_finite());
        assert!((0.0..=1.0).contains(&pval));
    }

    #[test]
    fn arch_lm_handles_extreme_lag_without_overflow() {
        assert_eq!(arch_lm(&[0.01, -0.01], usize::MAX), (0.0, 1.0));
    }

    #[test]
    fn chi2_sf_basic() {
        // chi2(1) at x=3.84 should give p ~ 0.05
        let p = chi2_sf(3.841, 1.0);
        assert!(
            (p - 0.05).abs() < 0.01,
            "chi2_sf(3.841, 1) = {}, expected ~0.05",
            p
        );

        // chi2(1) at x=0 should give p = 1
        let p0 = chi2_sf(0.0, 1.0);
        assert!((p0 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn hqic_formula() {
        let ll = -100.0;
        let h = hqic(ll, 3, 100);
        let expected = 200.0 + 6.0 * (100.0_f64.ln()).ln();
        assert!((h - expected).abs() < 1e-10);
    }
}
