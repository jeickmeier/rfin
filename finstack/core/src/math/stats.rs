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

use super::summation::kahan_sum;

/// Arithmetic mean.
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    // Use Kahan summation for numerical stability by default
    let s = kahan_sum(xs.iter().copied());
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    if prices.len() < 2 {
        return 0.0;
    }

    match method {
        RealizedVarMethod::CloseToClose => {
            let returns = log_returns(prices);
            variance(&returns) * annualization_factor
        }
        // For other methods, we need OHLC data which isn't available in simple price series
        // These methods require high/low/open data, so fall back to close-to-close
        RealizedVarMethod::Parkinson
        | RealizedVarMethod::GarmanKlass
        | RealizedVarMethod::RogersSatchell
        | RealizedVarMethod::YangZhang => {
            // These methods require OHLC data, use realized_variance_ohlc instead
            // For single price series, fall back to close-to-close
            let returns = log_returns(prices);
            variance(&returns) * annualization_factor
        }
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
            // Parkinson (1980) high-low range estimator
            // More efficient than close-to-close, using intraday range information
            //
            // Formula: σ² = [1/(4·ln(2))] · (1/n) · Σ[ln(H/L)]²
            //
            // Reference: Parkinson, M. (1980). "The Extreme Value Method for
            // Estimating the Variance of the Rate of Return."
            // Journal of Business, 53(1), 61-65.
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
            // Garman-Klass (1980) OHLC estimator
            // Extends Parkinson by incorporating open and close prices for improved efficiency
            //
            // Formula: σ² = (1/n) · Σ[0.5·[ln(H/L)]² - (2·ln(2) - 1)·[ln(C/O)]²]
            //
            // The coefficient (2·ln(2) - 1) ≈ 0.386 is the optimal weight for the
            // close-open component under the assumption of Brownian motion.
            //
            // Reference: Garman, M. B., & Klass, M. J. (1980). "On the Estimation of
            // Security Price Volatilities from Historical Data."
            // Journal of Business, 53(1), 67-78.
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
            // Rogers-Satchell (1991) drift-independent OHLC estimator
            // Allows for non-zero drift, making it more robust than Parkinson or Garman-Klass
            // when the underlying asset has significant directional movement.
            //
            // Formula: σ² = (1/n) · Σ[ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]
            //
            // Reference: Rogers, L. C. G., & Satchell, S. E. (1991). "Estimating Variance
            // From High, Low and Closing Prices." Annals of Applied Probability, 1(4), 504-512.
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
            // Yang-Zhang (2000) estimator: includes overnight jumps and opening gaps
            // Combines overnight variance with Rogers-Satchell intraday variance
            // using optimal weighting to minimize bias and variance of the estimator.
            //
            // Reference: Yang, D., & Zhang, Q. (2000). "Drift-Independent Volatility
            // Estimation Based on High, Low, Open, and Close Prices."
            // Journal of Business, 73(3), 477-491.
            
            /// Yang-Zhang optimal weight numerator for combining variance components
            const YANG_ZHANG_K_NUMERATOR: f64 = 0.34;
            /// Yang-Zhang optimal weight denominator base adjustment
            const YANG_ZHANG_K_DENOMINATOR_BASE: f64 = 1.34;
            
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

            let k = YANG_ZHANG_K_NUMERATOR / (YANG_ZHANG_K_DENOMINATOR_BASE + (n + 1) as f64 / (n - 1) as f64);
            let var_oc = sum_oc / (n - 1) as f64;
            let var_rs = sum_rs / n as f64;

            (k * var_oc + (1.0 - k) * var_rs) * annualization_factor
        }
    }
}
