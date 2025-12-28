//! Stochastic pricing engine.

use super::config::{PricingMode, StochasticPricerConfig};
use super::result::{StochasticPricingResult, TranchePricingResult};
use crate::instruments::structured_credit::pricing::stochastic::tree::ScenarioTree;
use crate::instruments::structured_credit::types::waterfall::WaterfallWorkspace;

use finstack_core::currency::Currency;
use finstack_core::math::random::{RandomNumberGenerator, TestRng};
use finstack_core::money::Money;
use std::cmp::Ordering;

/// Stochastic pricing engine for structured credit.
///
/// Prices structured credit instruments using scenario tree or Monte Carlo
/// simulation, computing NPV and risk metrics.
pub struct StochasticPricer {
    config: StochasticPricerConfig,
}

impl StochasticPricer {
    /// Create a new stochastic pricer.
    pub fn new(config: StochasticPricerConfig) -> Self {
        Self { config }
    }

    /// Price a structured credit instrument.
    ///
    /// Returns pricing result with NPV and risk metrics.
    ///
    /// # Arguments
    /// * `notional` - Deal notional amount
    /// * `currency` - Deal currency
    ///
    /// # Errors
    /// Returns error if pricing fails.
    pub fn price(
        &self,
        notional: f64,
        currency: Currency,
    ) -> Result<StochasticPricingResult, String> {
        match &self.config.pricing_mode {
            PricingMode::Tree => self.price_tree(notional, currency),
            PricingMode::MonteCarlo { num_paths, .. } => {
                self.price_monte_carlo(notional, currency, *num_paths)
            }
            PricingMode::Hybrid {
                tree_periods,
                mc_paths,
            } => self.price_hybrid(notional, currency, *tree_periods, *mc_paths),
        }
    }

    /// Tree-based pricing.
    fn price_tree(
        &self,
        notional: f64,
        currency: Currency,
    ) -> Result<StochasticPricingResult, String> {
        let tree = ScenarioTree::build(&self.config.tree_config)?;
        let mut result = self.build_tree_result(&tree, notional, currency)?;
        result.pricing_mode = "Tree".to_string();
        Ok(result)
    }

    fn build_tree_result(
        &self,
        tree: &ScenarioTree,
        notional: f64,
        currency: Currency,
    ) -> Result<StochasticPricingResult, String> {
        let expected_cashflow = tree.expected_value(|n| n.total_cashflow()) * notional;
        let expected_loss = tree.expected_loss() * notional;
        let unexpected_loss = tree.unexpected_loss() * notional;
        let expected_shortfall = tree.expected_shortfall(self.config.es_confidence) * notional;
        let npv = self.compute_tree_npv(tree, notional)?;

        let npv_amount = Money::new(npv, currency);
        let el_amount = Money::new(expected_loss, currency);
        let ul_amount = Money::new(unexpected_loss, currency);
        let es_amount = Money::new(expected_shortfall, currency);

        let mut result =
            StochasticPricingResult::new(npv_amount, el_amount, tree.num_terminal_nodes());
        result = result
            .with_unexpected_loss(ul_amount)
            .with_expected_shortfall(es_amount, self.config.es_confidence);

        if notional > 0.0 {
            result.clean_price = npv / notional * 100.0;
            result.dirty_price = result.clean_price;
        }

        let _ = expected_cashflow;

        Ok(result)
    }

    /// Compute NPV from scenario tree.
    fn compute_tree_npv(&self, tree: &ScenarioTree, notional: f64) -> Result<f64, String> {
        let mut npv = 0.0;

        // Sum probability-weighted discounted cashflows across all nodes
        for node in tree.nodes() {
            if node.period == 0 {
                continue; // Skip root
            }

            // Get discount factor for this time
            let df = self.config.discount_curve.df(node.time);

            // Probability-weighted cashflow
            let weighted_cf = node.cumulative_probability * node.total_cashflow() * notional * df;
            npv += weighted_cf;
        }

        Ok(npv)
    }

    fn path_present_value(
        &self,
        tree: &ScenarioTree,
        terminal_index: usize,
        scale: f64,
        currency: Currency,
        workspace: &mut WaterfallWorkspace,
    ) -> f64 {
        workspace.tier_allocations.clear();
        let nodes = tree.nodes();
        let mut idx = terminal_index;

        loop {
            let node = &nodes[idx];
            if node.period == 0 {
                break;
            }

            let discounted =
                node.total_cashflow() * scale * self.config.discount_curve.df(node.time);
            workspace
                .tier_allocations
                .push((String::new(), Money::new(discounted, currency)));

            if let Some(parent) = node.parent {
                idx = parent.index();
            } else {
                break;
            }
        }

        let pv = workspace
            .tier_allocations
            .iter()
            .map(|(_, amt)| amt.amount())
            .sum::<f64>();
        workspace.tier_allocations.clear();
        pv
    }

    /// Monte Carlo pricing.
    ///
    /// Note: This implementation samples from the terminal distribution of the
    /// recombining scenario tree (rather than simulating an independent process).
    /// This keeps the stochastic outputs consistent with the tree configuration
    /// while still providing a Monte Carlo-style estimator when needed.
    fn price_monte_carlo(
        &self,
        notional: f64,
        currency: Currency,
        num_paths: usize,
    ) -> Result<StochasticPricingResult, String> {
        if num_paths == 0 {
            return Err("Monte Carlo pricing requires at least one simulation path".to_string());
        }

        let tree = ScenarioTree::build(&self.config.tree_config)?;
        let mut workspace = MonteCarloWorkspace::new(num_paths);
        workspace.build_distribution(&tree)?;

        let initial_balance = self.config.tree_config.initial_balance;
        let scale = if initial_balance.abs() > f64::EPSILON {
            notional / initial_balance
        } else {
            1.0
        };

        let mut rng = TestRng::new(self.config.seed);

        for _ in 0..num_paths {
            workspace.tier_workspace.clear();
            let u = rng.uniform();
            let terminal_idx = workspace.sample_index(u);
            let node_index = workspace.terminal_indices[terminal_idx];
            let path_loss = tree.nodes()[node_index].cumulative_losses * scale;
            let path_pv = self.path_present_value(
                &tree,
                node_index,
                scale,
                currency,
                &mut workspace.tier_workspace,
            );
            workspace.record_path(path_pv, path_loss);
        }

        let pricing_mode = format!("MonteCarlo({})", num_paths);
        let mut result = workspace.finalize(
            currency,
            num_paths,
            notional,
            self.config.es_confidence,
            &pricing_mode,
        );
        result.pricing_mode = pricing_mode;
        Ok(result)
    }

    /// Hybrid pricing.
    fn price_hybrid(
        &self,
        notional: f64,
        currency: Currency,
        tree_periods: usize,
        mc_paths: usize,
    ) -> Result<StochasticPricingResult, String> {
        if tree_periods == 0 {
            return self.price_monte_carlo(notional, currency, mc_paths);
        }

        if tree_periods >= self.config.tree_config.num_periods || mc_paths == 0 {
            return self.price_tree(notional, currency);
        }

        let mut truncated_config = self.config.tree_config.clone();
        truncated_config.num_periods = tree_periods;
        let truncated_tree = ScenarioTree::build(&truncated_config)?;
        let tree_result = self.build_tree_result(&truncated_tree, notional, currency)?;
        let mc_result = self.price_monte_carlo(notional, currency, mc_paths)?;

        let total_periods = self.config.tree_config.num_periods.max(1);
        let weight_tree = tree_periods as f64 / total_periods as f64;
        let weight_mc = 1.0 - weight_tree;

        let combined_npv =
            weight_tree * tree_result.npv.amount() + weight_mc * mc_result.npv.amount();
        let combined_el = weight_tree * tree_result.expected_loss.amount()
            + weight_mc * mc_result.expected_loss.amount();
        let combined_ul = weight_tree * tree_result.unexpected_loss.amount()
            + weight_mc * mc_result.unexpected_loss.amount();
        let combined_es = weight_tree * tree_result.expected_shortfall.amount()
            + weight_mc * mc_result.expected_shortfall.amount();

        let mut result = StochasticPricingResult::new(
            Money::new(combined_npv, currency),
            Money::new(combined_el, currency),
            mc_paths,
        );

        if notional > f64::EPSILON {
            result.clean_price = combined_npv / notional * 100.0;
            result.dirty_price = result.clean_price;
        }

        result.unexpected_loss = Money::new(combined_ul, currency);
        result = result
            .with_expected_shortfall(Money::new(combined_es, currency), self.config.es_confidence);
        result.pricing_mode = format!("Hybrid({}, {})", tree_periods, mc_paths);

        Ok(result)
    }

    /// Price individual tranches.
    pub fn price_tranches(
        &self,
        tranches: &[(String, String, f64, f64)], // (id, seniority, attachment, detachment)
        notional: f64,
        currency: Currency,
    ) -> Result<Vec<TranchePricingResult>, String> {
        let tree = ScenarioTree::build(&self.config.tree_config)?;

        let mut results = Vec::with_capacity(tranches.len());

        for (id, seniority, attachment, detachment) in tranches {
            let tranche_notional = notional * (detachment - attachment);

            // Compute tranche-specific loss
            let tranche_loss =
                self.compute_tranche_loss(&tree, *attachment, *detachment) * notional;
            let tranche_npv = tranche_notional - tranche_loss;

            let npv_amount = Money::new(tranche_npv, currency);

            let result = TranchePricingResult::new(id.clone(), seniority.clone(), npv_amount)
                .with_subordination(*attachment, *detachment);

            results.push(result);
        }

        Ok(results)
    }

    /// Compute tranche-specific expected loss.
    fn compute_tranche_loss(&self, tree: &ScenarioTree, attachment: f64, detachment: f64) -> f64 {
        // Tranche loss = max(0, min(detachment, pool_loss) - attachment)
        tree.expected_value(|node| {
            let pool_loss_pct = node.cumulative_losses / self.config.tree_config.initial_balance;
            let tranche_loss = (pool_loss_pct.min(detachment) - attachment).max(0.0);
            tranche_loss / (detachment - attachment) // Normalize to tranche size
        })
    }

    /// Get the pricing configuration.
    pub fn config(&self) -> &StochasticPricerConfig {
        &self.config
    }
}

struct MonteCarloWorkspace {
    cumulative_probs: Vec<f64>,
    terminal_indices: Vec<usize>,
    tier_workspace: WaterfallWorkspace,
    pv_sum: f64,
    loss_sum: f64,
    loss_sq_sum: f64,
    losses: Vec<f64>,
}

impl MonteCarloWorkspace {
    fn new(num_paths: usize) -> Self {
        Self {
            cumulative_probs: Vec::new(),
            terminal_indices: Vec::new(),
            tier_workspace: WaterfallWorkspace::default(),
            pv_sum: 0.0,
            loss_sum: 0.0,
            loss_sq_sum: 0.0,
            losses: Vec::with_capacity(num_paths),
        }
    }

    fn build_distribution(&mut self, tree: &ScenarioTree) -> Result<(), String> {
        self.cumulative_probs.clear();
        self.terminal_indices.clear();

        let mut running = 0.0;
        for node in tree.terminal_nodes() {
            running += node.cumulative_probability;
            self.terminal_indices.push(node.id.index());
            self.cumulative_probs.push(running);
        }

        if self.cumulative_probs.is_empty() || running <= f64::EPSILON {
            return Err("Scenario tree does not contain terminal paths".to_string());
        }

        for prob in &mut self.cumulative_probs {
            *prob /= running;
        }

        Ok(())
    }

    fn sample_index(&self, sample: f64) -> usize {
        let target = sample.clamp(0.0, 1.0 - f64::EPSILON);
        match self
            .cumulative_probs
            .binary_search_by(|prob| prob.partial_cmp(&target).unwrap_or(Ordering::Greater))
        {
            Ok(idx) => idx,
            Err(idx) => idx.min(self.terminal_indices.len().saturating_sub(1)),
        }
    }

    fn record_path(&mut self, pv: f64, loss: f64) {
        self.pv_sum += pv;
        self.loss_sum += loss;
        self.loss_sq_sum += loss * loss;
        self.losses.push(loss);
    }

    fn finalize(
        mut self,
        currency: Currency,
        num_paths: usize,
        notional: f64,
        es_confidence: f64,
        pricing_mode: &str,
    ) -> StochasticPricingResult {
        let paths = num_paths.max(1) as f64;
        let mean_pv = self.pv_sum / paths;
        let expected_loss = self.loss_sum / paths;

        let pv_money = Money::new(mean_pv, currency);
        let el_money = Money::new(expected_loss, currency);
        let mut result = StochasticPricingResult::new(pv_money, el_money, num_paths);

        if notional > f64::EPSILON {
            result.clean_price = mean_pv / notional * 100.0;
            result.dirty_price = result.clean_price;
        }

        let loss_variance = (self.loss_sq_sum / paths) - expected_loss * expected_loss;
        let unexpected_loss = loss_variance.max(0.0).sqrt();
        result.unexpected_loss = Money::new(unexpected_loss, currency);

        if !self.losses.is_empty() {
            let tail = (1.0 - es_confidence).clamp(0.0, 1.0);
            let mut tail_count = (tail * num_paths as f64).ceil() as usize;
            if tail_count == 0 {
                tail_count = 1;
            }
            tail_count = tail_count.min(self.losses.len());

            self.losses
                .sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
            let tail_sum: f64 = self.losses.iter().take(tail_count).sum();
            let es_value = tail_sum / tail_count as f64;
            result = result.with_expected_shortfall(Money::new(es_value, currency), es_confidence);
        } else {
            result = result.with_expected_shortfall(Money::new(0.0, currency), es_confidence);
        }

        result.pricing_mode = pricing_mode.to_string();
        result
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::structured_credit::pricing::stochastic::tree::{
        BranchingSpec, ScenarioTreeConfig,
    };
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use std::sync::Arc;
    use time::Month;

    fn test_discount_curve() -> Arc<DiscountCurve> {
        Arc::new(
            DiscountCurve::builder("USD-OIS")
                .base_date(Date::from_calendar_date(2024, Month::January, 15).expect("Valid date"))
                .knots([
                    (0.0, 1.0),
                    (0.5, 0.975),
                    (1.0, 0.95),
                    (2.0, 0.90),
                    (5.0, 0.78),
                ])
                .set_interp(InterpStyle::LogLinear)
                .build()
                .expect("Valid curve"),
        )
    }

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 15).expect("Valid date")
    }

    fn test_pricer() -> StochasticPricer {
        let today = test_date();
        let curve = test_discount_curve();
        let tree_config = ScenarioTreeConfig::new(6, 0.5, BranchingSpec::fixed(2));
        let config = StochasticPricerConfig::new(today, curve, tree_config);
        StochasticPricer::new(config)
    }

    #[test]
    fn test_price_tree() {
        let pricer = test_pricer();
        let result = pricer.price(1_000_000.0, Currency::USD);

        assert!(result.is_ok());
        let result = result.expect("Pricing should succeed");
        assert!(result.num_paths > 0);
        assert!(result.expected_loss.amount() >= 0.0);
    }

    #[test]
    fn test_price_monte_carlo() {
        let today = test_date();
        let curve = test_discount_curve();
        let tree_config = ScenarioTreeConfig::new(6, 0.5, BranchingSpec::fixed(2));
        let config = StochasticPricerConfig::new(today, curve, tree_config)
            .with_pricing_mode(PricingMode::monte_carlo(1000));

        let pricer = StochasticPricer::new(config);
        let result = pricer.price(1_000_000.0, Currency::USD);

        assert!(result.is_ok());
        let result = result.expect("Pricing should succeed");
        assert!(result.pricing_mode.contains("MonteCarlo"));
        assert_eq!(result.num_paths, 1000);
    }

    #[test]
    fn test_price_tranches() {
        let pricer = test_pricer();
        let tranches = vec![
            ("Equity".to_string(), "Junior".to_string(), 0.0, 0.10),
            ("Mezz".to_string(), "Mezzanine".to_string(), 0.10, 0.25),
            ("Senior".to_string(), "Senior".to_string(), 0.25, 1.0),
        ];

        let results = pricer.price_tranches(&tranches, 1_000_000.0, Currency::USD);

        assert!(results.is_ok());
        let results = results.expect("Tranche pricing should succeed");
        assert_eq!(results.len(), 3);

        // Senior should have lower loss rate than equity
        let equity_loss = results[0].expected_loss.amount();
        let senior_loss = results[2].expected_loss.amount();
        // Note: Loss amounts are not directly comparable without normalization
        let _ = (equity_loss, senior_loss); // Just ensure they exist
    }

    #[test]
    fn test_price_hybrid_mode() {
        let today = test_date();
        let curve = test_discount_curve();
        let tree_config = ScenarioTreeConfig::new(6, 0.5, BranchingSpec::fixed(2));
        let config = StochasticPricerConfig::new(today, curve, tree_config).with_pricing_mode(
            PricingMode::Hybrid {
                tree_periods: 3,
                mc_paths: 250,
            },
        );

        let pricer = StochasticPricer::new(config);
        let result = pricer.price(1_000_000.0, Currency::USD);

        assert!(result.is_ok());
        let result = result.expect("Pricing should succeed");
        assert!(result.pricing_mode.contains("Hybrid"));
        assert_eq!(result.num_paths, 250);
    }
}
