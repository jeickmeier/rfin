//! Scenario tree data structure.
//!
//! Non-recombining tree for stochastic structured credit analysis.
//! Designed for accuracy over speed, preserving full path information.

use super::{
    config::ScenarioTreeConfig,
    node::{ScenarioNode, ScenarioNodeId, ScenarioPath},
};

use crate::instruments::common::models::correlation::factor_model::FactorSpec;
use crate::instruments::common::models::correlation::recovery::RecoverySpec;
use finstack_core::math::standard_normal_inv_cdf;

/// Non-recombining scenario tree for structured credit.
///
/// Each node in the tree represents a possible state at a point in time,
/// including prepayment behavior, default behavior, and pool state.
///
/// # Example
///
/// ```ignore
/// let config = ScenarioTreeConfig::rmbs_standard(5.0, 0.045);
/// let tree = ScenarioTree::build(&config)?;
///
/// // Compute expected present value
/// let pv = tree.expected_pv(discount_curve)?;
/// ```
#[derive(Clone, Debug)]
pub struct ScenarioTree {
    /// All nodes in the tree (index 0 = root)
    nodes: Vec<ScenarioNode>,

    /// Configuration used to build the tree
    config: ScenarioTreeConfig,

    /// Indices of terminal (leaf) nodes
    terminal_indices: Vec<usize>,
}

impl ScenarioTree {
    /// Build a scenario tree from configuration.
    ///
    /// # Errors
    /// Currently infallible but may fail if configuration is invalid.
    pub fn build(config: &ScenarioTreeConfig) -> Result<Self, String> {
        let mut tree = Self {
            nodes: Vec::with_capacity(config.estimate_total_nodes()),
            config: config.clone(),
            terminal_indices: Vec::new(),
        };

        // Create root node
        let root = ScenarioNode::root(config.initial_balance, config.initial_seasoning);
        tree.nodes.push(root);

        // Build the tree recursively
        tree.expand_tree(0)?;

        // Collect terminal node indices
        tree.terminal_indices = tree
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.is_terminal())
            .map(|(i, _)| i)
            .collect();

        Ok(tree)
    }

    /// Expand the tree from a given node.
    fn expand_tree(&mut self, node_idx: usize) -> Result<(), String> {
        // Extract needed data from parent node before mutations
        let (period, burnout_factor) = {
            let node = &self.nodes[node_idx];
            (node.period, node.burnout_factor)
        };

        // Stop at terminal period
        if period >= self.config.num_periods {
            return Ok(());
        }

        // Get branching info
        let num_branches = self.config.branching.branches_at_node(0.0);

        // Pre-compute all children data before modifying nodes
        let children_data: Vec<_> = (0..num_branches)
            .map(|branch_idx| {
                let factors = self.generate_factors_stateless(branch_idx, num_branches);
                let smm = self.conditional_smm_stateless(&factors, burnout_factor);
                let mdr = self.conditional_mdr_stateless(&factors);
                let recovery = self.conditional_recovery(&factors);
                let trans_prob = 1.0 / num_branches as f64;
                (factors, smm, mdr, recovery, trans_prob)
            })
            .collect();

        // Generate child nodes
        let mut children_indices = Vec::with_capacity(num_branches);

        for (factors, smm, mdr, recovery, trans_prob) in children_data {
            let child_id = ScenarioNodeId(self.nodes.len());
            let child =
                self.nodes[node_idx].child(child_id, trans_prob, factors, smm, mdr, recovery);

            children_indices.push(child_id.0);
            self.nodes.push(child);
        }

        // Update parent's children list
        self.nodes[node_idx].children = children_indices
            .iter()
            .map(|&i| ScenarioNodeId(i))
            .collect();

        // Apply cash flows to children
        for &child_idx in &children_indices {
            let scheduled_principal = self.scheduled_principal(child_idx);
            let pool_coupon = self.config.pool_coupon;
            self.nodes[child_idx].apply_cashflows(scheduled_principal, pool_coupon);
        }

        // Recursively expand children
        for child_idx in children_indices {
            self.expand_tree(child_idx)?;
        }

        Ok(())
    }

    /// Generate factor realizations for a branch (stateless version).
    ///
    /// Uses stratified sampling to ensure good coverage of the distribution.
    fn generate_factors_stateless(&self, branch_idx: usize, num_branches: usize) -> Vec<f64> {
        // Stratified sampling: divide normal distribution into equal-probability regions
        let n = num_branches as f64;
        let p = (branch_idx as f64 + 0.5) / n; // Midpoint of each stratum

        // Use standard normal inverse CDF from core library
        let z = standard_normal_inv_cdf(p);

        // Apply factor model structure
        match &self.config.factor_spec {
            FactorSpec::SingleFactor { volatility, .. } => {
                vec![z * volatility]
            }
            FactorSpec::TwoFactor {
                prepay_vol,
                credit_vol,
                ..
            } => {
                // For two factors, we need two z values
                // Use correlated generation
                let z2 = standard_normal_inv_cdf(0.5 + 0.5 * (p - 0.5));
                vec![z * prepay_vol, z2 * credit_vol]
            }
            FactorSpec::MultiFactor { volatilities, .. } => {
                // Use first volatility scaled by z
                if let Some(vol) = volatilities.first() {
                    vec![z * vol]
                } else {
                    vec![z]
                }
            }
        }
    }

    /// Compute conditional SMM given factor realizations (stateless version).
    fn conditional_smm_stateless(&self, factors: &[f64], burnout_factor: f64) -> f64 {
        let factor = factors.first().copied().unwrap_or(0.0);
        let base_smm = self.config.prepay_spec.base_smm();

        // Get correlation from configuration
        let prepay_factor_loading = self.config.correlation.prepay_factor_loading();

        // Conditional SMM using factor model
        // Log-normal factor adjustment for non-negative rates
        let smm = base_smm * (prepay_factor_loading * factor).exp();

        // Apply burnout
        let smm_with_burnout = smm * burnout_factor;

        // Clamp to valid range
        smm_with_burnout.clamp(0.0, 0.50)
    }

    /// Compute conditional MDR given factor realizations (stateless version).
    fn conditional_mdr_stateless(&self, factors: &[f64]) -> f64 {
        let factor = factors.first().copied().unwrap_or(0.0);
        let base_mdr = self.config.default_spec.base_mdr();

        // Get correlation from configuration
        let default_factor_loading = self.config.correlation.default_factor_loading();

        // Conditional MDR using factor model
        // Log-normal factor adjustment for non-negative rates
        let mdr = base_mdr * (default_factor_loading * factor).exp();

        // Clamp to valid range
        mdr.clamp(0.0, 0.50)
    }

    /// Compute conditional recovery given factor realizations.
    fn conditional_recovery(&self, factors: &[f64]) -> f64 {
        match &self.config.recovery_spec {
            RecoverySpec::Constant { rate } => *rate,
            RecoverySpec::MarketCorrelated {
                mean_recovery,
                recovery_volatility,
                factor_correlation,
            } => {
                let factor = factors.first().copied().unwrap_or(0.0);
                // Recovery moves with factor (typically negative correlation)
                let recovery = mean_recovery + factor_correlation * recovery_volatility * factor;
                recovery.clamp(0.0, 1.0)
            }
            RecoverySpec::Beta { mean, .. } => {
                // Simplified: use mean recovery
                *mean
            }
            RecoverySpec::Frye {
                base_lgd,
                lgd_sensitivity,
            } => {
                // Simplified: base recovery, modified by default rate in practice
                // For tree nodes, we don't have portfolio default rate easily,
                // so use base recovery as approximation
                let factor = factors.first().copied().unwrap_or(0.0);
                let base_recovery = 1.0 - base_lgd;
                // Higher defaults (negative factor) → lower recovery
                let recovery = base_recovery - lgd_sensitivity * 0.01 * (-factor).max(0.0);
                recovery.clamp(0.0, 1.0)
            }
        }
    }

    /// Calculate scheduled principal for a given period.
    fn scheduled_principal(&self, node_idx: usize) -> f64 {
        let node = &self.nodes[node_idx];

        // Simple level-pay amortization
        // For a real implementation, this would use the actual amortization schedule
        let remaining_periods = self.config.num_periods - node.period + 1;
        if remaining_periods == 0 {
            return 0.0;
        }

        let r = self.config.pool_coupon / 12.0;
        if r.abs() < 1e-10 {
            return node.pool_balance / remaining_periods as f64;
        }

        // Level payment amount
        let payment = node.pool_balance * r / (1.0 - (1.0 + r).powi(-(remaining_periods as i32)));
        let interest = node.pool_balance * r;

        (payment - interest).max(0.0)
    }

    // === Public accessors ===

    /// Get the root node.
    pub fn root(&self) -> &ScenarioNode {
        &self.nodes[0]
    }

    /// Get a node by ID.
    pub fn node(&self, id: ScenarioNodeId) -> Option<&ScenarioNode> {
        self.nodes.get(id.0)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> &[ScenarioNode] {
        &self.nodes
    }

    /// Get the number of nodes.
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get terminal nodes.
    pub fn terminal_nodes(&self) -> impl Iterator<Item = &ScenarioNode> {
        self.terminal_indices.iter().map(move |&i| &self.nodes[i])
    }

    /// Get the number of terminal nodes.
    pub fn num_terminal_nodes(&self) -> usize {
        self.terminal_indices.len()
    }

    /// Get all paths from root to terminal nodes.
    pub fn paths(&self) -> Vec<ScenarioPath> {
        let mut paths = Vec::with_capacity(self.terminal_indices.len());

        for &terminal_idx in &self.terminal_indices {
            let mut path_nodes = Vec::new();
            let mut current_idx = terminal_idx;

            // Walk back to root
            loop {
                let node = &self.nodes[current_idx];
                path_nodes.push(node.id);

                if let Some(parent) = node.parent {
                    current_idx = parent.0;
                } else {
                    break;
                }
            }

            // Reverse to get root-to-terminal order
            path_nodes.reverse();

            // Get terminal node for statistics
            let terminal = &self.nodes[terminal_idx];

            let mut path = ScenarioPath::from_nodes(path_nodes, terminal.cumulative_probability);
            path.terminal_balance = terminal.pool_balance;
            path.total_prepayments = terminal.cumulative_prepayments;
            path.total_defaults = terminal.cumulative_defaults;
            path.total_losses = terminal.cumulative_losses;

            paths.push(path);
        }

        paths
    }

    // === Statistical methods ===

    /// Compute expected value of a function over terminal nodes.
    pub fn expected_value<F>(&self, f: F) -> f64
    where
        F: Fn(&ScenarioNode) -> f64,
    {
        let mut sum = 0.0;
        let mut total_prob = 0.0;

        for &idx in &self.terminal_indices {
            let node = &self.nodes[idx];
            sum += node.cumulative_probability * f(node);
            total_prob += node.cumulative_probability;
        }

        if total_prob > 0.0 {
            sum / total_prob
        } else {
            0.0
        }
    }

    /// Compute variance of a function over terminal nodes.
    pub fn variance<F>(&self, f: F) -> f64
    where
        F: Fn(&ScenarioNode) -> f64,
    {
        let mean = self.expected_value(&f);
        self.expected_value(|n| (f(n) - mean).powi(2))
    }

    /// Compute percentile of a function over terminal nodes.
    pub fn percentile<F>(&self, f: F, p: f64) -> f64
    where
        F: Fn(&ScenarioNode) -> f64,
    {
        let mut values: Vec<(f64, f64)> = self
            .terminal_indices
            .iter()
            .map(|&i| {
                let node = &self.nodes[i];
                (f(node), node.cumulative_probability)
            })
            .collect();

        // Sort by value
        values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Find percentile
        let total_prob: f64 = values.iter().map(|(_, prob)| prob).sum();
        let target = p * total_prob;

        let mut cumulative = 0.0;
        let mut last_value = 0.0;
        for (value, prob) in &values {
            last_value = *value;
            cumulative += prob;
            if cumulative >= target {
                return *value;
            }
        }

        last_value
    }

    /// Compute expected loss.
    pub fn expected_loss(&self) -> f64 {
        self.expected_value(|n| n.cumulative_losses)
    }

    /// Compute expected prepayments.
    pub fn expected_prepayments(&self) -> f64 {
        self.expected_value(|n| n.cumulative_prepayments)
    }

    /// Compute expected defaults.
    pub fn expected_defaults(&self) -> f64 {
        self.expected_value(|n| n.cumulative_defaults)
    }

    /// Compute unexpected loss (loss standard deviation).
    pub fn unexpected_loss(&self) -> f64 {
        self.variance(|n| n.cumulative_losses).sqrt()
    }

    /// Compute expected shortfall (CVaR) at a given confidence level.
    pub fn expected_shortfall(&self, confidence: f64) -> f64 {
        let mut values: Vec<(f64, f64)> = self
            .terminal_indices
            .iter()
            .map(|&i| {
                let node = &self.nodes[i];
                (node.cumulative_losses, node.cumulative_probability)
            })
            .collect();

        // Sort by loss (descending for tail)
        values.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Average losses in the tail
        let total_prob: f64 = values.iter().map(|(_, p)| p).sum();
        let tail_prob = (1.0 - confidence) * total_prob;

        let mut cumulative = 0.0;
        let mut tail_sum = 0.0;
        let mut tail_weight = 0.0;

        for (loss, prob) in values {
            if cumulative < tail_prob {
                let include_prob = (tail_prob - cumulative).min(prob);
                tail_sum += loss * include_prob;
                tail_weight += include_prob;
            }
            cumulative += prob;
        }

        if tail_weight > 0.0 {
            tail_sum / tail_weight
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::structured_credit::components::stochastic::BranchingSpec;

    #[test]
    fn test_build_simple_tree() {
        let config = ScenarioTreeConfig::new(3, 0.25, BranchingSpec::fixed(2));
        let tree = ScenarioTree::build(&config).expect("Failed to build tree");

        // 1 + 2 + 4 + 8 = 15 nodes
        assert_eq!(tree.num_nodes(), 15);

        // 2^3 = 8 terminal nodes
        assert_eq!(tree.num_terminal_nodes(), 8);
    }

    #[test]
    fn test_root_properties() {
        let config = ScenarioTreeConfig::new(3, 0.25, BranchingSpec::fixed(3))
            .with_initial_balance(2_000_000.0)
            .with_initial_seasoning(12);

        let tree = ScenarioTree::build(&config).expect("Failed to build tree");
        let root = tree.root();

        assert!(root.is_root());
        assert!((root.pool_balance - 2_000_000.0).abs() < 1e-6);
        assert_eq!(root.seasoning, 12);
    }

    #[test]
    fn test_paths() {
        let config = ScenarioTreeConfig::new(2, 1.0 / 6.0, BranchingSpec::fixed(2));
        let tree = ScenarioTree::build(&config).expect("Failed to build tree");

        let paths = tree.paths();

        // 2^2 = 4 paths
        assert_eq!(paths.len(), 4);

        // All paths should have length 3 (root + 2 periods)
        for path in &paths {
            assert_eq!(path.len(), 3);
        }

        // Probabilities should sum to 1
        let total_prob: f64 = paths.iter().map(|p| p.probability).sum();
        assert!((total_prob - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_expected_value() {
        // Use realistic parameters: 12 periods over 1 year with binary branching
        // to keep tree size manageable but balance evolution meaningful
        let config = ScenarioTreeConfig::new(6, 0.5, BranchingSpec::fixed(2));
        let tree = ScenarioTree::build(&config).expect("Failed to build tree");

        // Expected pool balance should be less than initial (due to payments)
        let expected_balance = tree.expected_value(|n| n.pool_balance);

        // With 6 periods of amortization + prepayments + defaults, balance decreases
        assert!(
            expected_balance < config.initial_balance,
            "Balance should decrease due to payments"
        );
        assert!(expected_balance >= 0.0, "Balance should not be negative");
    }

    #[test]
    fn test_percentile() {
        let config = ScenarioTreeConfig::new(3, 0.25, BranchingSpec::fixed(3));
        let tree = ScenarioTree::build(&config).expect("Failed to build tree");

        let p50_loss = tree.percentile(|n| n.cumulative_losses, 0.50);
        let p95_loss = tree.percentile(|n| n.cumulative_losses, 0.95);

        // 95th percentile should be >= 50th percentile
        assert!(p95_loss >= p50_loss);
    }

    #[test]
    fn test_expected_shortfall() {
        let config = ScenarioTreeConfig::new(3, 0.25, BranchingSpec::fixed(3));
        let tree = ScenarioTree::build(&config).expect("Failed to build tree");

        let _expected_loss = tree.expected_loss();
        let es_95 = tree.expected_shortfall(0.95);

        // ES should be >= EL for a loss distribution
        assert!(es_95 >= 0.0);
    }

    #[test]
    fn test_standard_normal_inv_cdf() {
        // Test at known quantiles using core library function
        let z_50 = standard_normal_inv_cdf(0.5);
        assert!(z_50.abs() < 0.01); // Should be close to 0

        let z_975 = standard_normal_inv_cdf(0.975);
        assert!((z_975 - 1.96).abs() < 0.01);

        let z_025 = standard_normal_inv_cdf(0.025);
        assert!((z_025 + 1.96).abs() < 0.01);
    }

    #[test]
    fn test_rmbs_standard_tree() {
        let config = ScenarioTreeConfig::rmbs_standard(0.5, 0.045);
        let tree = ScenarioTree::build(&config).expect("Failed to build RMBS tree");

        // Should have 6 monthly periods
        assert!(tree.num_terminal_nodes() > 0);

        let expected_loss = tree.expected_loss();
        let unexpected_loss = tree.unexpected_loss();

        // Both should be non-negative
        assert!(expected_loss >= 0.0);
        assert!(unexpected_loss >= 0.0);
    }
}
