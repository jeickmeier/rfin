//! Online statistics for Monte Carlo estimation.
//!
//! Implements Welford's algorithm for numerically stable mean/variance
//! computation and confidence interval calculation.

/// Online statistics accumulator using Welford's algorithm.
///
/// This provides numerically stable computation of mean and variance
/// in a single pass, which is critical for Monte Carlo where we
/// process millions of samples.
///
/// # Example
///
/// ```rust,ignore
/// let mut stats = OnlineStats::new();
/// for sample in samples {
///     stats.update(sample);
/// }
/// println!("Mean: {}, StdErr: {}", stats.mean(), stats.stderr());
/// ```
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
            + delta * delta * (self.count as f64) * (other.count as f64)
                / (total_count as f64);

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
    /// ```rust,ignore
    /// let (lower, upper) = stats.confidence_interval(0.05); // 95% CI
    /// ```
    pub fn confidence_interval(&self, alpha: f64) -> (f64, f64) {
        let z = normal_quantile(1.0 - alpha / 2.0);
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

/// Approximate inverse standard normal CDF for confidence intervals.
///
/// Uses Abramowitz & Stegun approximation (accurate to ~0.45% relative error).
///
/// # Arguments
///
/// * `p` - Probability in (0, 1)
///
/// # Returns
///
/// z such that Φ(z) = p, where Φ is standard normal CDF.
fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return if p <= 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    // For p > 0.5, use symmetry: Φ^(-1)(p) = -Φ^(-1)(1-p)
    let sign = if p < 0.5 { -1.0 } else { 1.0 };
    let p_adj = if p < 0.5 { p } else { 1.0 - p };

    // Abramowitz & Stegun 26.2.23
    // Approximation for small p (lower tail)
    let t = (-2.0 * p_adj.ln()).sqrt();

    // Coefficients
    const C0: f64 = 2.515517;
    const C1: f64 = 0.802853;
    const C2: f64 = 0.010328;
    const D1: f64 = 1.432788;
    const D2: f64 = 0.189269;
    const D3: f64 = 0.001308;

    let numerator = C0 + C1 * t + C2 * t * t;
    let denominator = 1.0 + D1 * t + D2 * t * t + D3 * t * t * t;
    let z = t - numerator / denominator;

    sign * z
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
    let z = normal_quantile(1.0 - alpha / 2.0);
    let n = (z * cv / target_rel_error).powi(2);
    n.ceil() as usize
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
    fn test_normal_quantile() {
        // Test known values
        let z_95 = normal_quantile(0.975); // 95% two-tailed
        assert!((z_95 - 1.96).abs() < 0.01); // Should be ~1.96

        let z_99 = normal_quantile(0.995); // 99% two-tailed
        assert!((z_99 - 2.576).abs() < 0.01); // Should be ~2.576

        // Test symmetry
        assert!((normal_quantile(0.5)).abs() < 0.01); // Should be ~0
        assert!((normal_quantile(0.25) + normal_quantile(0.75)).abs() < 0.01);
    }

    #[test]
    fn test_required_samples() {
        // For CV=1.0, target 1% relative error at 95% confidence
        let n = required_samples(1.0, 0.01, 0.05);
        assert!(n > 38000); // Should be ~38416 for z=1.96
    }
}

