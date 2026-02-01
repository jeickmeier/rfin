//! Stochastic metrics calculator.
//!
//! Computes risk metrics from scenario trees or Monte Carlo paths.
#![allow(dead_code)] // Public API items may be used by external bindings

use crate::instruments::fixed_income::structured_credit::pricing::stochastic::tree::{
    ScenarioTree, ScenarioTreeConfig,
};

/// Stochastic risk metrics for structured credit.
#[derive(Clone, Debug)]
pub struct StochasticMetrics {
    // === Loss metrics ===
    /// Expected loss (probability-weighted average)
    pub expected_loss: f64,

    /// Unexpected loss (standard deviation of loss)
    pub unexpected_loss: f64,

    /// Loss skewness
    pub loss_skewness: f64,

    /// Loss kurtosis (excess)
    pub loss_kurtosis: f64,

    // === Tail risk metrics ===
    /// Value at Risk at 95% confidence
    pub var_95: f64,

    /// Value at Risk at 99% confidence
    pub var_99: f64,

    /// Expected Shortfall at 95% confidence
    pub expected_shortfall_95: f64,

    /// Expected Shortfall at 99% confidence
    pub expected_shortfall_99: f64,

    // === Behavioral metrics ===
    /// Expected prepayment amount
    pub expected_prepayments: f64,

    /// Expected default amount
    pub expected_defaults: f64,

    /// Expected recovery amount
    pub expected_recoveries: f64,

    /// Expected terminal pool balance
    pub expected_terminal_balance: f64,

    // === Correlation metrics ===
    /// Prepay-default correlation (implied from scenarios)
    pub implied_prepay_default_correlation: f64,

    /// Loss-factor correlation
    pub loss_factor_correlation: f64,

    // === Scenario statistics ===
    /// Number of terminal scenarios
    pub num_scenarios: usize,

    /// Minimum loss across scenarios
    pub min_loss: f64,

    /// Maximum loss across scenarios
    pub max_loss: f64,
}

impl StochasticMetrics {
    /// Create metrics with all values set to zero.
    pub fn zero() -> Self {
        Self {
            expected_loss: 0.0,
            unexpected_loss: 0.0,
            loss_skewness: 0.0,
            loss_kurtosis: 0.0,
            var_95: 0.0,
            var_99: 0.0,
            expected_shortfall_95: 0.0,
            expected_shortfall_99: 0.0,
            expected_prepayments: 0.0,
            expected_defaults: 0.0,
            expected_recoveries: 0.0,
            expected_terminal_balance: 0.0,
            implied_prepay_default_correlation: 0.0,
            loss_factor_correlation: 0.0,
            num_scenarios: 0,
            min_loss: 0.0,
            max_loss: 0.0,
        }
    }

    /// Get loss ratio (EL / (EL + Expected Terminal Balance)).
    pub fn loss_ratio(&self) -> f64 {
        let total = self.expected_loss + self.expected_terminal_balance;
        if total > 1e-10 {
            self.expected_loss / total
        } else {
            0.0
        }
    }

    /// Get coefficient of variation of loss (UL / EL).
    pub fn loss_cv(&self) -> f64 {
        if self.expected_loss > 1e-10 {
            self.unexpected_loss / self.expected_loss
        } else {
            0.0
        }
    }

    /// Get loss severity (EL / Expected Defaults).
    pub fn loss_severity(&self) -> f64 {
        if self.expected_defaults > 1e-10 {
            (self.expected_defaults - self.expected_recoveries) / self.expected_defaults
        } else {
            0.0
        }
    }
}

/// Calculator for stochastic risk metrics.
pub struct StochasticMetricsCalculator {
    notional: f64,
}

impl StochasticMetricsCalculator {
    /// Create a new metrics calculator.
    pub fn new(notional: f64) -> Self {
        Self {
            notional: notional.max(1.0),
        }
    }

    /// Compute metrics from a scenario tree.
    pub fn compute_from_tree(&self, tree: &ScenarioTree) -> StochasticMetrics {
        let n = tree.num_terminal_nodes();
        if n == 0 {
            return StochasticMetrics::zero();
        }

        // Collect terminal node data
        let mut losses: Vec<(f64, f64)> = Vec::with_capacity(n);
        let mut prepayments: Vec<(f64, f64)> = Vec::with_capacity(n);
        let mut defaults: Vec<(f64, f64)> = Vec::with_capacity(n);
        let mut balances: Vec<(f64, f64)> = Vec::with_capacity(n);

        for node in tree.terminal_nodes() {
            let prob = node.cumulative_probability;
            let loss = node.cumulative_losses * self.notional;
            let prepay = node.cumulative_prepayments * self.notional;
            let default = node.cumulative_defaults * self.notional;
            let balance = node.pool_balance * self.notional;

            losses.push((loss, prob));
            prepayments.push((prepay, prob));
            defaults.push((default, prob));
            balances.push((balance, prob));
        }

        // Normalize probabilities
        let total_prob: f64 = losses.iter().map(|(_, p)| p).sum();
        if total_prob < 1e-10 {
            return StochasticMetrics::zero();
        }

        // Compute expected values
        let expected_loss = self.weighted_mean(&losses, total_prob);
        let expected_prepayments = self.weighted_mean(&prepayments, total_prob);
        let expected_defaults = self.weighted_mean(&defaults, total_prob);
        let expected_terminal_balance = self.weighted_mean(&balances, total_prob);

        // Compute variance and higher moments for loss
        let variance = self.weighted_variance(&losses, expected_loss, total_prob);
        let unexpected_loss = variance.sqrt();

        let (skewness, kurtosis) =
            self.compute_higher_moments(&losses, expected_loss, unexpected_loss, total_prob);

        // Compute VaR and ES
        let var_95 = self.compute_var(&losses, 0.95);
        let var_99 = self.compute_var(&losses, 0.99);
        let es_95 = self.compute_expected_shortfall(&losses, 0.95);
        let es_99 = self.compute_expected_shortfall(&losses, 0.99);

        // Compute correlations
        let implied_corr = self.compute_implied_correlation(&prepayments, &defaults, total_prob);
        let loss_factor_corr = self.compute_loss_factor_correlation(tree);

        // Min/max loss
        let min_loss = losses.iter().map(|(l, _)| *l).fold(f64::INFINITY, f64::min);
        let max_loss = losses
            .iter()
            .map(|(l, _)| *l)
            .fold(f64::NEG_INFINITY, f64::max);

        // Expected recoveries
        let expected_recoveries = expected_defaults * (1.0 - self.compute_avg_lgd(tree));

        StochasticMetrics {
            expected_loss,
            unexpected_loss,
            loss_skewness: skewness,
            loss_kurtosis: kurtosis,
            var_95,
            var_99,
            expected_shortfall_95: es_95,
            expected_shortfall_99: es_99,
            expected_prepayments,
            expected_defaults,
            expected_recoveries,
            expected_terminal_balance,
            implied_prepay_default_correlation: implied_corr,
            loss_factor_correlation: loss_factor_corr,
            num_scenarios: n,
            min_loss,
            max_loss,
        }
    }

    /// Compute metrics from configuration (builds tree internally).
    pub fn compute_from_config(
        &self,
        config: &ScenarioTreeConfig,
    ) -> Result<StochasticMetrics, String> {
        let tree = ScenarioTree::build(config)?;
        Ok(self.compute_from_tree(&tree))
    }

    // === Private helpers ===

    fn weighted_mean(&self, values: &[(f64, f64)], total_prob: f64) -> f64 {
        values.iter().map(|(v, p)| v * p).sum::<f64>() / total_prob
    }

    fn weighted_variance(&self, values: &[(f64, f64)], mean: f64, total_prob: f64) -> f64 {
        values
            .iter()
            .map(|(v, p)| (v - mean).powi(2) * p)
            .sum::<f64>()
            / total_prob
    }

    fn compute_higher_moments(
        &self,
        values: &[(f64, f64)],
        mean: f64,
        std_dev: f64,
        total_prob: f64,
    ) -> (f64, f64) {
        if std_dev < 1e-10 {
            return (0.0, 0.0);
        }

        let mut m3 = 0.0;
        let mut m4 = 0.0;

        for (v, p) in values {
            let z = (v - mean) / std_dev;
            m3 += z.powi(3) * p;
            m4 += z.powi(4) * p;
        }

        let skewness = m3 / total_prob;
        let kurtosis = m4 / total_prob - 3.0; // Excess kurtosis

        (skewness, kurtosis)
    }

    fn compute_var(&self, values: &[(f64, f64)], confidence: f64) -> f64 {
        // Sort by loss value
        let mut sorted: Vec<(f64, f64)> = values.to_vec();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let total_prob: f64 = sorted.iter().map(|(_, p)| p).sum();
        let target = confidence * total_prob;

        let mut cumulative = 0.0;
        let mut last_loss = 0.0;
        for (loss, prob) in &sorted {
            last_loss = *loss;
            cumulative += prob;
            if cumulative >= target {
                return *loss;
            }
        }

        last_loss
    }

    fn compute_expected_shortfall(&self, values: &[(f64, f64)], confidence: f64) -> f64 {
        // Sort by loss value (descending for tail)
        let mut sorted: Vec<(f64, f64)> = values.to_vec();
        sorted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let total_prob: f64 = sorted.iter().map(|(_, p)| p).sum();
        let tail_prob = (1.0 - confidence) * total_prob;

        let mut cumulative = 0.0;
        let mut tail_sum = 0.0;
        let mut tail_weight = 0.0;

        for (loss, prob) in &sorted {
            if cumulative < tail_prob {
                let include_prob = (tail_prob - cumulative).min(*prob);
                tail_sum += loss * include_prob;
                tail_weight += include_prob;
            }
            cumulative += *prob;
        }

        if tail_weight > 1e-10 {
            tail_sum / tail_weight
        } else {
            0.0
        }
    }

    fn compute_implied_correlation(
        &self,
        prepayments: &[(f64, f64)],
        defaults: &[(f64, f64)],
        total_prob: f64,
    ) -> f64 {
        if prepayments.len() != defaults.len() || prepayments.is_empty() {
            return 0.0;
        }

        let mean_prepay = self.weighted_mean(prepayments, total_prob);
        let mean_default = self.weighted_mean(defaults, total_prob);

        let var_prepay = self.weighted_variance(prepayments, mean_prepay, total_prob);
        let var_default = self.weighted_variance(defaults, mean_default, total_prob);

        if var_prepay < 1e-10 || var_default < 1e-10 {
            return 0.0;
        }

        // Compute covariance
        let covariance: f64 = prepayments
            .iter()
            .zip(defaults.iter())
            .map(|((p, prob), (d, _))| (p - mean_prepay) * (d - mean_default) * prob)
            .sum::<f64>()
            / total_prob;

        covariance / (var_prepay.sqrt() * var_default.sqrt())
    }

    fn compute_loss_factor_correlation(&self, tree: &ScenarioTree) -> f64 {
        // Compute correlation between loss and first factor
        let mut loss_sum = 0.0;
        let mut factor_sum = 0.0;
        let mut prob_sum = 0.0;

        for node in tree.terminal_nodes() {
            let factor = node.factor_realizations.first().copied().unwrap_or(0.0);
            loss_sum += node.cumulative_losses * node.cumulative_probability;
            factor_sum += factor * node.cumulative_probability;
            prob_sum += node.cumulative_probability;
        }

        if prob_sum < 1e-10 {
            return 0.0;
        }

        let mean_loss = loss_sum / prob_sum;
        let mean_factor = factor_sum / prob_sum;

        let mut var_loss = 0.0;
        let mut var_factor = 0.0;
        let mut covariance = 0.0;

        for node in tree.terminal_nodes() {
            let factor = node.factor_realizations.first().copied().unwrap_or(0.0);
            let p = node.cumulative_probability;
            var_loss += (node.cumulative_losses - mean_loss).powi(2) * p;
            var_factor += (factor - mean_factor).powi(2) * p;
            covariance += (node.cumulative_losses - mean_loss) * (factor - mean_factor) * p;
        }

        var_loss /= prob_sum;
        var_factor /= prob_sum;
        covariance /= prob_sum;

        if var_loss < 1e-10 || var_factor < 1e-10 {
            return 0.0;
        }

        covariance / (var_loss.sqrt() * var_factor.sqrt())
    }

    fn compute_avg_lgd(&self, tree: &ScenarioTree) -> f64 {
        let mut lgd_sum = 0.0;
        let mut prob_sum = 0.0;

        for node in tree.terminal_nodes() {
            lgd_sum += (1.0 - node.recovery_rate) * node.cumulative_probability;
            prob_sum += node.cumulative_probability;
        }

        if prob_sum > 1e-10 {
            lgd_sum / prob_sum
        } else {
            0.60 // Default LGD
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::structured_credit::pricing::stochastic::tree::BranchingSpec;

    #[test]
    fn test_metrics_zero() {
        let metrics = StochasticMetrics::zero();
        assert!((metrics.expected_loss - 0.0).abs() < 1e-10);
        assert!((metrics.unexpected_loss - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_from_tree() {
        let config = ScenarioTreeConfig::new(3, 0.25, BranchingSpec::fixed(3));
        let tree = ScenarioTree::build(&config).expect("Tree should build");

        let calc = StochasticMetricsCalculator::new(1_000_000.0);
        let metrics = calc.compute_from_tree(&tree);

        // Should have reasonable values
        assert!(metrics.num_scenarios > 0);
        assert!(metrics.expected_loss >= 0.0);
        assert!(metrics.unexpected_loss >= 0.0);

        // VaR should be ordered
        assert!(metrics.var_99 >= metrics.var_95);

        // ES should be >= VaR at same confidence
        assert!(metrics.expected_shortfall_95 >= metrics.var_95 - 1e-6);
        assert!(metrics.expected_shortfall_99 >= metrics.var_99 - 1e-6);
    }

    #[test]
    fn test_loss_ratio() {
        let mut metrics = StochasticMetrics::zero();
        metrics.expected_loss = 50_000.0;
        metrics.expected_terminal_balance = 950_000.0;

        assert!((metrics.loss_ratio() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_loss_cv() {
        let mut metrics = StochasticMetrics::zero();
        metrics.expected_loss = 100_000.0;
        metrics.unexpected_loss = 50_000.0;

        assert!((metrics.loss_cv() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_compute_from_config() {
        let config = ScenarioTreeConfig::new(2, 0.167, BranchingSpec::fixed(2));
        let calc = StochasticMetricsCalculator::new(1_000_000.0);
        let metrics = calc.compute_from_config(&config);

        assert!(metrics.is_ok());
        let metrics = metrics.expect("Metrics should compute");
        assert!(metrics.num_scenarios > 0);
    }
}
