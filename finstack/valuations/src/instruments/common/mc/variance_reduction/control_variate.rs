//! Control variate variance reduction using Black-Scholes.
//!
//! Uses the analytical Black-Scholes formula as a control variate
//! to reduce variance for European options under GBM.
//!
//! The control variate estimator is:
//! ```text
//! X̂ = X̄ - β(Ȳ - E[Y])
//! ```
//! where Y is the control (BS price), E[Y] is known analytically,
//! and β is the optimal coefficient.

use super::super::results::Estimate;
use finstack_core::math::special_functions::norm_cdf;

/// Black-Scholes formula for European call option.
///
/// # Arguments
///
/// * `spot` - Current spot price
/// * `strike` - Strike price
/// * `time_to_maturity` - Time to maturity in years
/// * `rate` - Risk-free rate
/// * `dividend_yield` - Dividend yield
/// * `volatility` - Volatility
///
/// # Returns
///
/// Call option price
pub fn black_scholes_call(
    spot: f64,
    strike: f64,
    time_to_maturity: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> f64 {
    if time_to_maturity <= 0.0 {
        return (spot - strike).max(0.0);
    }

    let sqrt_t = time_to_maturity.sqrt();
    let d1 = ((spot / strike).ln()
        + (rate - dividend_yield + 0.5 * volatility * volatility) * time_to_maturity)
        / (volatility * sqrt_t);
    let d2 = d1 - volatility * sqrt_t;

    let discount_factor = (-rate * time_to_maturity).exp();

    spot * (-dividend_yield * time_to_maturity).exp() * norm_cdf(d1)
        - strike * discount_factor * norm_cdf(d2)
}

/// Black-Scholes formula for European put option.
pub fn black_scholes_put(
    spot: f64,
    strike: f64,
    time_to_maturity: f64,
    rate: f64,
    dividend_yield: f64,
    volatility: f64,
) -> f64 {
    if time_to_maturity <= 0.0 {
        return (strike - spot).max(0.0);
    }

    let sqrt_t = time_to_maturity.sqrt();
    let d1 = ((spot / strike).ln()
        + (rate - dividend_yield + 0.5 * volatility * volatility) * time_to_maturity)
        / (volatility * sqrt_t);
    let d2 = d1 - volatility * sqrt_t;

    let discount_factor = (-rate * time_to_maturity).exp();

    strike * discount_factor * norm_cdf(-d2)
        - spot * (-dividend_yield * time_to_maturity).exp() * norm_cdf(-d1)
}

/// Apply control variate adjustment to Monte Carlo estimate.
///
/// # Arguments
///
/// * `mc_mean` - Monte Carlo sample mean
/// * `mc_var` - Monte Carlo sample variance
/// * `control_mean` - Control variate sample mean
/// * `control_var` - Control variate sample variance
/// * `covariance` - Covariance between MC and control
/// * `control_analytical` - Analytical value of control
/// * `num_samples` - Number of samples
///
/// # Returns
///
/// Adjusted estimate with reduced variance
pub fn apply_control_variate(
    mc_mean: f64,
    mc_var: f64,
    control_mean: f64,
    control_var: f64,
    covariance: f64,
    control_analytical: f64,
    num_samples: usize,
) -> Estimate {
    // Optimal beta coefficient
    let beta = if control_var > 1e-10 {
        covariance / control_var
    } else {
        0.0
    };

    // Adjusted mean
    let adjusted_mean = mc_mean - beta * (control_mean - control_analytical);

    // Adjusted variance
    let adjusted_var = mc_var - 2.0 * beta * covariance + beta * beta * control_var;
    let adjusted_stderr = (adjusted_var / num_samples as f64).sqrt();

    // 95% confidence interval
    let z_95 = 1.96;
    let margin = z_95 * adjusted_stderr;
    let ci_95 = (adjusted_mean - margin, adjusted_mean + margin);

    Estimate::new(adjusted_mean, adjusted_stderr, ci_95, num_samples)
        .with_std_dev(adjusted_var.sqrt())
}

/// Compute covariance between two samples.
pub fn covariance(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len(), "Samples must have same length");
    let n = x.len();
    if n == 0 {
        return 0.0;
    }

    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;

    let cov: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum();

    cov / (n - 1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_scholes_call() {
        // ATM call: S=100, K=100, T=1, r=5%, q=2%, σ=20%
        let price = black_scholes_call(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);

        // Should be around 8-9 for these parameters
        assert!(price > 7.0 && price < 10.0);
    }

    #[test]
    fn test_black_scholes_put() {
        // ATM put: S=100, K=100, T=1, r=5%, q=2%, σ=20%
        let price = black_scholes_put(100.0, 100.0, 1.0, 0.05, 0.02, 0.2);

        // Should be positive
        assert!(price > 5.0 && price < 8.0);
    }

    #[test]
    fn test_put_call_parity() {
        let s = 100.0;
        let k = 100.0;
        let t = 1.0;
        let r = 0.05;
        let q = 0.02;
        let sigma = 0.2;

        let call = black_scholes_call(s, k, t, r, q, sigma);
        let put = black_scholes_put(s, k, t, r, q, sigma);

        // Put-call parity: C - P = S*e^(-qT) - K*e^(-rT)
        let lhs = call - put;
        let rhs = s * (-q * t).exp() - k * (-r * t).exp();

        assert!(
            (lhs - rhs).abs() < 1e-8,
            "Put-call parity failed: {} vs {}",
            lhs,
            rhs
        );
    }

    #[test]
    fn test_control_variate_adjustment() {
        // Simulate some correlated samples
        let mc_samples: Vec<f64> = vec![10.0, 12.0, 11.0, 13.0, 10.5];
        let control_samples: Vec<f64> = vec![9.8, 12.2, 10.9, 13.1, 10.4];
        let control_analytical = 11.0;

        let mc_mean = mc_samples.iter().sum::<f64>() / mc_samples.len() as f64;
        let control_mean = control_samples.iter().sum::<f64>() / control_samples.len() as f64;

        let mc_var = mc_samples
            .iter()
            .map(|&x| (x - mc_mean).powi(2))
            .sum::<f64>()
            / (mc_samples.len() - 1) as f64;

        let control_var = control_samples
            .iter()
            .map(|&x| (x - control_mean).powi(2))
            .sum::<f64>()
            / (control_samples.len() - 1) as f64;

        let cov = covariance(&mc_samples, &control_samples);

        let result = apply_control_variate(
            mc_mean,
            mc_var,
            control_mean,
            control_var,
            cov,
            control_analytical,
            mc_samples.len(),
        );

        // Adjusted mean should be different from raw MC mean
        assert!((result.mean - mc_mean).abs() > 0.0);

        // Should have valid stderr
        assert!(result.stderr > 0.0);
    }

    #[test]
    fn test_covariance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];

        let cov = covariance(&x, &y);

        // Perfect positive correlation: y = 2x
        // Var(x) = 2.5, Var(y) = 10, Cov(x,y) = 5
        assert!(cov > 0.0);
        assert!((cov - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_bs_itm_call() {
        // Deep ITM call should be close to intrinsic value
        let price = black_scholes_call(120.0, 100.0, 0.01, 0.05, 0.0, 0.01);
        assert!((price - 20.0).abs() < 0.5);
    }

    #[test]
    fn test_bs_otm_call() {
        // Deep OTM call should be close to zero
        let price = black_scholes_call(80.0, 100.0, 0.01, 0.05, 0.0, 0.01);
        assert!(price < 0.1);
    }
}
