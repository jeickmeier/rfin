//! Online statistics for Monte Carlo estimation.
//!
//! Implements Welford's algorithm for numerically stable mean/variance
//! computation and confidence interval calculation.

use finstack_core::math::special_functions::standard_normal_inv_cdf;

/// Online statistics accumulator using Welford's algorithm.
///
/// This provides numerically stable computation of mean and variance
/// in a single pass, which is critical for Monte Carlo where we
/// process millions of samples.
///
/// See unit tests and `examples/` for usage.
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
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::common::mc::online_stats::OnlineStats;
    ///
    /// let mut stats = OnlineStats::new();
    /// stats.update(1.0);
    /// stats.update(2.0);
    ///
    /// let (lower, upper) = stats.confidence_interval(0.05); // 95% CI
    /// assert!(lower <= stats.mean() && stats.mean() <= upper);
    /// ```
    pub fn confidence_interval(&self, alpha: f64) -> (f64, f64) {
        let z = standard_normal_inv_cdf(1.0 - alpha / 2.0);
        let margin = z * self.stderr();
        (self.mean - margin, self.mean + margin)
    }

    /// 95% confidence interval (convenience method).
    pub fn ci_95(&self) -> (f64, f64) {
        self.confidence_interval(0.05)
    }

    /// 99% confidence interval (convenience method).
    pub fn ci_99(&self) -> (f64, f64) {
        self.confidence_interval(0.01)
    }

    /// Half-width of the 95% confidence interval.
    pub fn ci_half_width(&self) -> f64 {
        let (lower, upper) = self.ci_95();
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
///
/// # Example
///
/// ```rust
/// use finstack_valuations::instruments::common::mc::online_stats::OnlineCovariance;
///
/// let mut cov = OnlineCovariance::new();
/// cov.update(1.0, 2.0);
/// cov.update(2.0, 4.0);
/// cov.update(3.0, 6.0);
///
/// // Perfect positive correlation (y = 2x)
/// assert!(cov.covariance() > 0.0);
/// assert!((cov.correlation() - 1.0).abs() < 1e-10);
/// ```
///
/// # References
///
/// Chan, T.F., Golub, G.H., LeVeque, R.J. (1979). "Updating Formulae and a
/// Pairwise Algorithm for Computing Sample Variances."
#[derive(Clone, Debug, Default)]
pub struct OnlineCovariance {
    count: usize,
    mean_x: f64,
    mean_y: f64,
    m2_x: f64,  // Sum of squared differences for x
    m2_y: f64,  // Sum of squared differences for y
    c: f64,     // Co-moment sum: Σ(x_i - mean_x)(y_i - mean_y)
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

        // Update variances (Welford's algorithm)
        let dx2 = x - self.mean_x;
        let dy2 = y - self.mean_y;
        self.m2_x += dx * dx2;
        self.m2_y += dy * dy2;

        // Update covariance (Chan et al. formula)
        // c += (x - mean_x_old) * (y - mean_y_new)
        self.c += dx * dy2;
    }

    /// Merge with another covariance accumulator.
    ///
    /// Enables parallel computation followed by reduction.
    pub fn merge(&mut self, other: &Self) {
        if other.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = other.clone();
            return;
        }

        let n_a = self.count as f64;
        let n_b = other.count as f64;
        let n_total = n_a + n_b;

        let delta_x = other.mean_x - self.mean_x;
        let delta_y = other.mean_y - self.mean_y;

        // Combined variances (Chan et al. parallel algorithm)
        let combined_m2_x = self.m2_x + other.m2_x + delta_x * delta_x * n_a * n_b / n_total;
        let combined_m2_y = self.m2_y + other.m2_y + delta_y * delta_y * n_a * n_b / n_total;

        // Combined covariance
        let combined_c = self.c + other.c + delta_x * delta_y * n_a * n_b / n_total;

        // Update state
        self.mean_x = (n_a * self.mean_x + n_b * other.mean_x) / n_total;
        self.mean_y = (n_a * self.mean_y + n_b * other.mean_y) / n_total;
        self.m2_x = combined_m2_x;
        self.m2_y = combined_m2_y;
        self.c = combined_c;
        self.count += other.count;
    }

    /// Number of samples.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Mean of x.
    pub fn mean_x(&self) -> f64 {
        self.mean_x
    }

    /// Mean of y.
    pub fn mean_y(&self) -> f64 {
        self.mean_y
    }

    /// Variance of x (unbiased estimator with n-1 denominator).
    pub fn variance_x(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2_x / (self.count - 1) as f64
    }

    /// Variance of y (unbiased estimator with n-1 denominator).
    pub fn variance_y(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.m2_y / (self.count - 1) as f64
    }

    /// Sample covariance (unbiased estimator with n-1 denominator).
    pub fn covariance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        self.c / (self.count - 1) as f64
    }

    /// Pearson correlation coefficient.
    ///
    /// Returns a value in [-1, 1], or 0.0 if variance is too small.
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

        let (lower, upper) = stats.ci_95();
        assert!(lower < stats.mean());
        assert!(upper > stats.mean());
        assert!(lower < 50.5 && upper > 50.5); // True mean
    }

    #[test]
    fn test_standard_normal_inv_cdf() {
        // Test that we're using the core implementation correctly
        // Test known values
        let z_95 = standard_normal_inv_cdf(0.975); // 95% two-tailed
        assert!((z_95 - 1.96).abs() < 0.01); // Should be ~1.96

        let z_99 = standard_normal_inv_cdf(0.995); // 99% two-tailed
        assert!((z_99 - 2.576).abs() < 0.01); // Should be ~2.576

        // Test symmetry
        assert!((standard_normal_inv_cdf(0.5)).abs() < 0.01); // Should be ~0
        assert!((standard_normal_inv_cdf(0.25) + standard_normal_inv_cdf(0.75)).abs() < 0.01);
    }

    #[test]
    fn test_required_samples() {
        // For CV=1.0, target 1% relative error at 95% confidence
        let n = required_samples(1.0, 0.01, 0.05);
        assert!(n > 38000); // Should be ~38416 for z=1.96
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

        // Perfect positive correlation (y = 2x)
        assert!((cov.correlation() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_negative() {
        let mut cov = OnlineCovariance::new();
        cov.update(1.0, 6.0);
        cov.update(2.0, 4.0);
        cov.update(3.0, 2.0);

        // Perfect negative correlation (y = 8 - 2x)
        assert!((cov.correlation() + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_online_covariance_uncorrelated() {
        let mut cov = OnlineCovariance::new();
        // Pairs with no correlation
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
        // Compare online computation with batch computation
        let data: Vec<(f64, f64)> = vec![
            (1.0, 2.1),
            (2.0, 3.9),
            (3.0, 6.2),
            (4.0, 7.8),
            (5.0, 10.1),
        ];

        // Online computation
        let mut online = OnlineCovariance::new();
        for &(x, y) in &data {
            online.update(x, y);
        }

        // Batch computation
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
        // y = 2x + noise, so beta should be close to 0.5 (1/2)
        cov.update(1.0, 2.0);
        cov.update(2.0, 4.0);
        cov.update(3.0, 6.0);

        // For y = 2x, Cov(X, Y) = 2 * Var(X), Var(Y) = 4 * Var(X)
        // beta = Cov(X, Y) / Var(Y) = 2 * Var(X) / (4 * Var(X)) = 0.5
        assert!((cov.optimal_beta() - 0.5).abs() < 1e-10);
    }
}
