//! Statistical functions for financial time series analysis.
//!
//! Provides numerically stable implementations of basic statistics using
//! Welford's online algorithm and Kahan summation. All functions are
//! deterministic and produce identical results across platforms.
//!
//! # Algorithms
//!
//! - **Mean**: Kahan compensated summation for numerical stability
//! - **Variance**: Welford's one-pass algorithm (avoids catastrophic cancellation)
//! - **Covariance/Correlation**: Chan's parallel algorithm for stability
//!
//! # Use Cases
//!
//! - Portfolio risk metrics (volatility, correlation matrices)
//! - Time series analysis (returns, volatility estimation)
//! - Factor model estimation (principal components)
//! - Monte Carlo variance reduction (control variates)
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
//!
//! # References
//!
//! - **Welford's Algorithm**:
//!   - Welford, B. P. (1962). "Note on a Method for Calculating Corrected Sums of
//!     Squares and Products." *Technometrics*, 4(3), 419-420.
//!
//! - **Chan's Parallel Algorithm**:
//!   - Chan, T. F., Golub, G. H., & LeVeque, R. J. (1983). "Algorithms for Computing
//!     the Sample Variance: Analysis and Recommendations." *The American Statistician*,
//!     37(3), 242-247.
//!
//! - **Kahan Summation**:
//!   - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."
//!     *Communications of the ACM*, 8(1), 40.

use super::special_functions::standard_normal_inv_cdf;
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

/// Moment matching: adjust samples to have exact mean and variance.
///
/// This variance reduction technique forces the sample to have
/// exactly the theoretical moments.
///
/// # Arguments
///
/// * `samples` - Mutable slice of samples to adjust
/// * `target_mean` - Target mean (default 0.0 for standard normal)
/// * `target_std` - Target standard deviation (default 1.0 for standard normal)
pub fn moment_match(samples: &mut [f64], target_mean: f64, target_std: f64) {
    if samples.is_empty() {
        return;
    }

    // Compute current mean and std dev
    let n = samples.len() as f64;
    let current_mean = samples.iter().sum::<f64>() / n;

    let current_var = samples
        .iter()
        .map(|&x| (x - current_mean).powi(2))
        .sum::<f64>()
        / n;
    let current_std = current_var.sqrt();

    // Adjust samples
    if current_std > 1e-10 {
        for x in samples.iter_mut() {
            *x = (*x - current_mean) * (target_std / current_std) + target_mean;
        }
    }
}

// ====== Realized Variance Calculations ======

/// Methods for calculating realized variance from price series.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

            let k = YANG_ZHANG_K_NUMERATOR
                / (YANG_ZHANG_K_DENOMINATOR_BASE + (n + 1) as f64 / (n - 1) as f64);
            let var_oc = sum_oc / (n - 1) as f64;
            let var_rs = sum_rs / n as f64;

            (k * var_oc + (1.0 - k) * var_rs) * annualization_factor
        }
    }
}

// ====== Online (Streaming) Statistics ======

/// Online statistics accumulator using Welford's algorithm.
///
/// This provides numerically stable computation of mean and variance
/// in a single pass, which is critical for Monte Carlo where we
/// process millions of samples.
#[derive(Clone, Debug, Default)]
pub struct OnlineStats {
    count: usize,
    mean: f64,
    m2: f64, // Sum of squared differences from current mean
}

impl OnlineStats {
    /// Create a new empty statistics accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update with a new sample.
    ///
    /// Uses Welford's algorithm for numerical stability.
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }

    /// Merge with another statistics accumulator.
    ///
    /// This enables parallel computation followed by reduction.
    pub fn merge(&mut self, other: &Self) {
        if other.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = other.clone();
            return;
        }

        let total_count = self.count + other.count;
        let delta = other.mean - self.mean;
        let combined_m2 = self.m2
            + other.m2
            + delta * delta * (self.count as f64) * (other.count as f64) / (total_count as f64);

        self.mean = (self.count as f64 * self.mean + other.count as f64 * other.mean)
            / (total_count as f64);
        self.m2 = combined_m2;
        self.count = total_count;
    }

    /// Number of samples.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Sample mean.
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Sample variance (unbiased estimator with n-1 denominator).
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2 / (self.count - 1) as f64
    }

    /// Sample standard deviation.
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Standard error of the mean (σ / √n).
    pub fn stderr(&self) -> f64 {
        if self.count == 0 {
            return f64::INFINITY;
        }
        self.std_dev() / (self.count as f64).sqrt()
    }

    /// Confidence interval at specified level.
    ///
    /// # Arguments
    ///
    /// * `alpha` - Significance level (e.g., 0.05 for 95% CI)
    ///
    /// # Returns
    ///
    /// (lower, upper) bounds of the confidence interval.
    pub fn confidence_interval(&self, alpha: f64) -> (f64, f64) {
        let z = standard_normal_inv_cdf(1.0 - alpha / 2.0);
        let margin = z * self.stderr();
        (self.mean - margin, self.mean + margin)
    }

    /// Half-width of the 95% confidence interval.
    pub fn ci_half_width(&self) -> f64 {
        let (lower, upper) = self.confidence_interval(0.05);
        (upper - lower) / 2.0
    }

    /// Reset to empty state.
    pub fn reset(&mut self) {
        self.count = 0;
        self.mean = 0.0;
        self.m2 = 0.0;
    }
}

/// Compute relative error bound for target confidence.
///
/// Returns the number of samples required to achieve a target
/// relative error (standard error / mean) at a given confidence level.
///
/// # Arguments
///
/// * `cv` - Coefficient of variation (σ / μ)
/// * `target_rel_error` - Target relative standard error
/// * `alpha` - Significance level (0.05 for 95% confidence)
///
/// # Returns
///
/// Minimum number of samples required.
pub fn required_samples(cv: f64, target_rel_error: f64, alpha: f64) -> usize {
    let z = standard_normal_inv_cdf(1.0 - alpha / 2.0);
    let n = (z * cv / target_rel_error).powi(2);
    n.ceil() as usize
}

/// Online covariance accumulator using Welford's algorithm.
///
/// Computes mean, variance, and covariance for two variables in a single pass.
/// This is essential for control variate estimation where we need covariance
/// between MC samples and control variate samples without storing all values.
///
/// Uses the parallel formula from Chan et al. (1979):
/// C_{n+1} = C_n + (x - mean_x) * (y - mean_y') where mean_y' is the new mean
#[derive(Clone, Debug, Default)]
pub struct OnlineCovariance {
    count: usize,
    mean_x: f64,
    mean_y: f64,
    m2_x: f64, // Sum of squared differences for x
    m2_y: f64, // Sum of squared differences for y
    c: f64,    // Co-moment sum: Σ(x_i - mean_x)(y_i - mean_y)
}

impl OnlineCovariance {
    /// Create a new empty covariance accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update with a new sample pair (x, y).
    ///
    /// Uses Welford's algorithm extended to covariance.
    pub fn update(&mut self, x: f64, y: f64) {
        self.count += 1;
        let n = self.count as f64;

        // Update means (Welford's algorithm)
        let dx = x - self.mean_x;
        let dy = y - self.mean_y;
        self.mean_x += dx / n;
        self.mean_y += dy / n;

        // Update variance and covariance sums
        self.m2_x += dx * (x - self.mean_x);
        self.m2_y += dy * (y - self.mean_y);
        self.c += dx * (y - self.mean_y);
    }

    /// Merge with another covariance accumulator.
    pub fn merge(&mut self, other: &Self) {
        if other.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = other.clone();
            return;
        }

        let total_count = self.count + other.count;
        let delta_x = other.mean_x - self.mean_x;
        let delta_y = other.mean_y - self.mean_y;

        self.c += other.c
            + delta_x * delta_y * (self.count as f64) * (other.count as f64) / (total_count as f64);
        self.m2_x += other.m2_x
            + delta_x * delta_x * (self.count as f64) * (other.count as f64) / (total_count as f64);
        self.m2_y += other.m2_y
            + delta_y * delta_y * (self.count as f64) * (other.count as f64) / (total_count as f64);
        self.mean_x = (self.count as f64 * self.mean_x + other.count as f64 * other.mean_x)
            / (total_count as f64);
        self.mean_y = (self.count as f64 * self.mean_y + other.count as f64 * other.mean_y)
            / (total_count as f64);
        self.count = total_count;
    }

    /// Number of samples.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Sample mean of x.
    pub fn mean_x(&self) -> f64 {
        self.mean_x
    }

    /// Sample mean of y.
    pub fn mean_y(&self) -> f64 {
        self.mean_y
    }

    /// Sample variance of x (unbiased).
    pub fn variance_x(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2_x / (self.count - 1) as f64
    }

    /// Sample variance of y (unbiased).
    pub fn variance_y(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2_y / (self.count - 1) as f64
    }

    /// Sample covariance (unbiased).
    pub fn covariance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.c / (self.count - 1) as f64
    }

    /// Sample correlation.
    pub fn correlation(&self) -> f64 {
        let var_x = self.variance_x();
        let var_y = self.variance_y();

        if var_x < 1e-20 || var_y < 1e-20 {
            return 0.0;
        }

        self.covariance() / (var_x * var_y).sqrt()
    }

    /// Optimal beta coefficient for control variate.
    ///
    /// Returns Cov(X, Y) / Var(Y), the coefficient that minimizes
    /// the variance of X - β(Y - E[Y]).
    pub fn optimal_beta(&self) -> f64 {
        let var_y = self.variance_y();
        if var_y < 1e-20 {
            return 0.0;
        }
        self.covariance() / var_y
    }

    /// Reset to empty state.
    pub fn reset(&mut self) {
        self.count = 0;
        self.mean_x = 0.0;
        self.mean_y = 0.0;
        self.m2_x = 0.0;
        self.m2_y = 0.0;
        self.c = 0.0;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_online_stats_basic() {
        let mut stats = OnlineStats::new();
        stats.update(1.0);
        stats.update(2.0);
        stats.update(3.0);

        assert_eq!(stats.count(), 3);
        assert_eq!(stats.mean(), 2.0);
        assert_eq!(stats.variance(), 1.0);
        assert_eq!(stats.std_dev(), 1.0);
    }

    #[test]
    fn test_online_stats_merge() {
        let mut stats1 = OnlineStats::new();
        stats1.update(1.0);
        stats1.update(2.0);

        let mut stats2 = OnlineStats::new();
        stats2.update(3.0);
        stats2.update(4.0);

        stats1.merge(&stats2);
        assert_eq!(stats1.count(), 4);
        assert_eq!(stats1.mean(), 2.5);
    }

    #[test]
    fn test_confidence_intervals() {
        let mut stats = OnlineStats::new();
        for i in 1..=100 {
            stats.update(i as f64);
        }

        let (lower, upper) = stats.confidence_interval(0.05);
        assert!(lower < stats.mean());
        assert!(upper > stats.mean());
        assert!(lower < 50.5 && upper > 50.5);
    }

    #[test]
    fn test_moment_match() {
        let mut samples = vec![-1.5, -0.5, 0.0, 0.5, 1.5];
        moment_match(&mut samples, 0.0, 1.0);

        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let var = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;

        assert!(mean.abs() < 1e-10);
        assert!((var - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_standard_normal_inv_cdf() {
        let z_95 = standard_normal_inv_cdf(0.975);
        assert!((z_95 - 1.96).abs() < 0.01);

        let z_99 = standard_normal_inv_cdf(0.995);
        assert!((z_99 - 2.576).abs() < 0.01);

        assert!((standard_normal_inv_cdf(0.5)).abs() < 0.01);
        assert!((standard_normal_inv_cdf(0.25) + standard_normal_inv_cdf(0.75)).abs() < 0.01);
    }

    #[test]
    fn test_required_samples() {
        let n = required_samples(1.0, 0.01, 0.05);
        assert!(n > 38000);
    }

    #[test]
    fn test_online_covariance_basic() {
        let mut cov = OnlineCovariance::new();
        cov.update(1.0, 2.0);
        cov.update(2.0, 4.0);
        cov.update(3.0, 6.0);

        assert_eq!(cov.count(), 3);
        assert_eq!(cov.mean_x(), 2.0);
        assert_eq!(cov.mean_y(), 4.0);
        assert!((cov.correlation() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_negative() {
        let mut cov = OnlineCovariance::new();
        cov.update(1.0, 6.0);
        cov.update(2.0, 4.0);
        cov.update(3.0, 2.0);

        assert!((cov.correlation() + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_uncorrelated() {
        let mut cov = OnlineCovariance::new();
        cov.update(1.0, 1.0);
        cov.update(1.0, -1.0);
        cov.update(-1.0, 1.0);
        cov.update(-1.0, -1.0);

        assert!(cov.correlation().abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_merge() {
        let mut cov1 = OnlineCovariance::new();
        cov1.update(1.0, 2.0);
        cov1.update(2.0, 4.0);

        let mut cov2 = OnlineCovariance::new();
        cov2.update(3.0, 6.0);
        cov2.update(4.0, 8.0);

        cov1.merge(&cov2);

        assert_eq!(cov1.count(), 4);
        assert_eq!(cov1.mean_x(), 2.5);
        assert_eq!(cov1.mean_y(), 5.0);
        assert!((cov1.correlation() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_matches_batch() {
        let data: Vec<(f64, f64)> =
            vec![(1.0, 2.1), (2.0, 3.9), (3.0, 6.2), (4.0, 7.8), (5.0, 10.1)];

        let mut online = OnlineCovariance::new();
        for &(x, y) in &data {
            online.update(x, y);
        }

        let n = data.len() as f64;
        let mean_x: f64 = data.iter().map(|(x, _)| x).sum::<f64>() / n;
        let mean_y: f64 = data.iter().map(|(_, y)| y).sum::<f64>() / n;
        let cov_batch: f64 = data
            .iter()
            .map(|(x, y)| (x - mean_x) * (y - mean_y))
            .sum::<f64>()
            / (n - 1.0);

        assert!((online.mean_x() - mean_x).abs() < 1e-10);
        assert!((online.mean_y() - mean_y).abs() < 1e-10);
        assert!((online.covariance() - cov_batch).abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_optimal_beta() {
        let mut cov = OnlineCovariance::new();
        cov.update(1.0, 2.0);
        cov.update(2.0, 4.0);
        cov.update(3.0, 6.0);

        assert!((cov.optimal_beta() - 0.5).abs() < 1e-10);
    }
}
