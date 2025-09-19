//! Small statistics helpers (mean/variance/covariance/correlation).
//!
//! We implement these ourselves rather than using external crates to ensure:
//! - Deterministic results using our custom summation algorithm
//! - Feature-flag controlled behaviour (deterministic vs. fast)
//! - No dependencies on external crates for basic operations
//! - Consistent numerical behaviour across platforms
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::{mean, variance, covariance, correlation};
//! let xs = [1.0, 2.0, 3.0, 4.0];
//! let ys = [2.0, 4.0, 6.0, 8.0];
//! assert!((mean(&xs) - 2.5).abs() < 1e-12);
//! assert!(variance(&xs) > 0.0);
//! assert!(covariance(&xs, &ys) > 0.0);
//! assert!((correlation(&xs, &ys) - 1.0).abs() < 1e-12);
//! ```

use super::summation::{kahan_sum, pairwise_sum};

/// Arithmetic mean.
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let s = if cfg!(feature = "deterministic") {
        pairwise_sum(xs)
    } else {
        kahan_sum(xs.iter().copied())
    };
    s / xs.len() as f64
}

/// Variance (population) using a single-pass Welford algorithm (deterministic order).
pub fn variance(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n == 0 {
        return 0.0;
    }
    let mut mean = 0.0;
    let mut m2 = 0.0;
    let mut k = 0.0;
    for &x in xs {
        k += 1.0;
        let delta = x - mean;
        mean += delta / k;
        let delta2 = x - mean;
        m2 += delta * delta2;
    }
    m2 / n as f64
}

/// Return (mean, variance) pair.
pub fn mean_var(xs: &[f64]) -> (f64, f64) {
    (mean(xs), variance(xs))
}

/// Covariance (population) between two equal-length slices.
pub fn covariance(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    if n == 0 {
        return 0.0;
    }
    // One-pass Chan/Welford style covariance for deterministic, stable accumulation
    let mut mean_x = 0.0;
    let mut mean_y = 0.0;
    let mut co_moment = 0.0;
    let mut k = 0.0;
    for i in 0..n {
        let xi = x[i];
        let yi = y[i];
        k += 1.0;
        let dx = xi - mean_x;
        mean_x += dx / k;
        let dy = yi - mean_y;
        mean_y += dy / k;
        // Use updated mean_y for the second factor per Chan's formulation
        co_moment += dx * (yi - mean_y);
    }
    co_moment / n as f64
}

/// Pearson correlation.
pub fn correlation(x: &[f64], y: &[f64]) -> f64 {
    let cov = covariance(x, y);
    let vx = variance(x);
    let vy = variance(y);
    if vx == 0.0 || vy == 0.0 {
        return 0.0;
    }
    cov / (vx.sqrt() * vy.sqrt())
}

// ====== Realized Variance Calculations ======

/// Methods for calculating realized variance from price series.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealizedVarMethod {
    /// Standard close-to-close returns
    CloseToClose,
    /// Parkinson (1980) high-low estimator
    Parkinson,
    /// Garman-Klass (1980) OHLC estimator
    GarmanKlass,
    /// Rogers-Satchell (1991) drift-independent OHLC estimator
    RogersSatchell,
    /// Yang-Zhang (2000) drift and opening gaps estimator
    YangZhang,
}

/// Calculate log returns from a price series.
pub fn log_returns(prices: &[f64]) -> Vec<f64> {
    if prices.len() < 2 {
        return vec![];
    }
    prices.windows(2).map(|w| (w[1] / w[0]).ln()).collect()
}

/// Calculate realized variance from price series.
///
/// # Arguments
/// * `prices` - Price series (for CloseToClose method)
/// * `method` - Method to use for calculation
/// * `annualization_factor` - Factor to annualize variance (e.g., 252 for daily data)
///
/// # Returns
/// Annualized realized variance
pub fn realized_variance(
    prices: &[f64],
    method: RealizedVarMethod,
    annualization_factor: f64,
) -> f64 {
    match method {
        RealizedVarMethod::CloseToClose => {
            if prices.len() < 2 {
                return 0.0;
            }
            let returns = log_returns(prices);
            variance(&returns) * annualization_factor
        }
        // Other methods can be implemented incrementally
        _ => realized_variance(
            prices,
            RealizedVarMethod::CloseToClose,
            annualization_factor,
        ),
    }
}

/// Calculate realized variance from OHLC data using advanced estimators.
///
/// # Arguments
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `method` - Method to use for calculation
/// * `annualization_factor` - Factor to annualize variance
///
/// # Returns
/// Annualized realized variance
pub fn realized_variance_ohlc(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    method: RealizedVarMethod,
    annualization_factor: f64,
) -> f64 {
    let n = open.len();
    if n < 2 {
        return 0.0;
    }

    match method {
        RealizedVarMethod::CloseToClose => realized_variance(close, method, annualization_factor),
        RealizedVarMethod::Parkinson => {
            // Parkinson estimator: uses high-low range
            let sum: f64 = (0..n)
                .map(|i| {
                    let hl_ratio = high[i] / low[i];
                    (hl_ratio.ln()).powi(2)
                })
                .sum();
            let factor = 1.0 / (4.0 * (2.0_f64).ln());
            (sum / n as f64) * factor * annualization_factor
        }
        RealizedVarMethod::GarmanKlass => {
            // Garman-Klass estimator
            let sum: f64 = (0..n)
                .map(|i| {
                    let hl = (high[i] / low[i]).ln();
                    let co = (close[i] / open[i]).ln();
                    0.5 * hl.powi(2) - (2.0 * (2.0_f64).ln() - 1.0) * co.powi(2)
                })
                .sum();
            (sum / n as f64) * annualization_factor
        }
        RealizedVarMethod::RogersSatchell => {
            // Rogers-Satchell estimator: drift-independent
            let sum: f64 = (0..n)
                .map(|i| {
                    let hc = (high[i] / close[i]).ln();
                    let ho = (high[i] / open[i]).ln();
                    let lc = (low[i] / close[i]).ln();
                    let lo = (low[i] / open[i]).ln();
                    hc * ho + lc * lo
                })
                .sum();
            (sum / n as f64) * annualization_factor
        }
        RealizedVarMethod::YangZhang => {
            // Yang-Zhang estimator: includes overnight jumps
            // Simplified version - full implementation would need opening jumps
            let mut sum_oc = 0.0;
            let mut sum_rs = 0.0;

            for i in 1..n {
                // Overnight component
                let overnight = (open[i] / close[i - 1]).ln();
                sum_oc += overnight.powi(2);

                // Rogers-Satchell component for intraday
                let hc = (high[i] / close[i]).ln();
                let ho = (high[i] / open[i]).ln();
                let lc = (low[i] / close[i]).ln();
                let lo = (low[i] / open[i]).ln();
                sum_rs += hc * ho + lc * lo;
            }

            let k = 0.34 / (1.34 + (n + 1) as f64 / (n - 1) as f64);
            let var_oc = sum_oc / (n - 1) as f64;
            let var_rs = sum_rs / n as f64;

            (k * var_oc + (1.0 - k) * var_rs) * annualization_factor
        }
    }
}
