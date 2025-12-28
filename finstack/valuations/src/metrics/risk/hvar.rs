//! Generic Historical VaR metric calculator.
//!
//! Integrates Historical VaR into the standard metrics framework as a
//! `MetricCalculator` that can be registered and computed alongside other
//! risk metrics like DV01, Theta, etc.

use crate::metrics::core::traits::{MetricCalculator, MetricContext};
use crate::metrics::risk::{calculate_var, VarConfig, VarMethod};
use crate::metrics::MetricId;
use finstack_core::Result;

/// Generic Historical VaR calculator that works with any instrument.
///
/// This calculator integrates Historical VaR into the standard metrics
/// framework. It requires `MarketHistory` to be attached to the
/// [`MarketContext`] via its `market_history` field.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::metrics::{GenericHVar, MetricId, MetricRegistry, VarConfig};
/// use std::sync::Arc;
///
/// // Create VaR calculator with 95% confidence
/// let var_calc = GenericHVar::new(VarConfig::var_95());
///
/// // Register in metric registry
/// let mut registry = MetricRegistry::new();
/// registry.register_metric(MetricId::HVAR, Arc::new(var_calc), &[]);
/// ```
pub struct GenericHVar {
    config: VarConfig,
}

impl GenericHVar {
    /// Create a new VaR calculator with the given configuration.
    pub fn new(config: VarConfig) -> Self {
        Self { config }
    }

    /// Create a VaR calculator with 95% confidence level.
    pub fn var_95() -> Self {
        Self::new(VarConfig::var_95())
    }

    /// Create a VaR calculator with 99% confidence level.
    pub fn var_99() -> Self {
        Self::new(VarConfig::var_99())
    }

    /// Create a VaR calculator with custom confidence level.
    pub fn with_confidence(confidence_level: f64) -> Self {
        Self::new(VarConfig::new(confidence_level))
    }

    /// Set the calculation method.
    pub fn with_method(mut self, method: VarMethod) -> Self {
        self.config.method = method;
        self
    }
}

impl MetricCalculator for GenericHVar {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        // Get market history from market context
        let history = context
            .curves
            .market_history
            .as_ref()
            .and_then(|h| h.downcast_ref::<crate::metrics::risk::MarketHistory>())
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Market history required for VaR calculation. Attach it via MarketContext::insert_market_history(...)"
                        .to_string(),
                )
            })?;

        // Calculate VaR for this instrument
        let result = calculate_var(
            context.instrument.as_ref(),
            &context.curves,
            history,
            context.as_of,
            &self.config,
        )?;

        // Store additional metrics
        // Store Expected Shortfall as a separate metric
        context
            .computed
            .insert(MetricId::EXPECTED_SHORTFALL, result.expected_shortfall);

        // TODO: Store P&L distribution as a series metric if needed
        // context.store_series(...)?;

        // Return VaR as the primary metric value
        Ok(result.var)
    }

    fn dependencies(&self) -> &[MetricId] {
        // VaR doesn't depend on other metrics (it revalues directly)
        &[]
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use crate::metrics::risk::test_utils::{
        history_from_rate_shifts, sample_as_of, standard_bond, usd_ois_market,
    };
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_generic_hvar_creation() {
        let var_calc = GenericHVar::var_95();
        assert_eq!(var_calc.config.confidence_level, 0.95);

        let var_calc = GenericHVar::var_99();
        assert_eq!(var_calc.config.confidence_level, 0.99);

        let var_calc = GenericHVar::with_confidence(0.975);
        assert_eq!(var_calc.config.confidence_level, 0.975);
    }

    #[test]
    fn test_hvar_via_metrics_framework() -> Result<()> {
        let as_of = sample_as_of();
        let bond = standard_bond("TEST-BOND", as_of, date!(2029 - 01 - 01));

        let history = Arc::new(history_from_rate_shifts(
            as_of,
            &[
                (date!(2023 - 12 - 31), 0.0050),
                (date!(2023 - 12 - 30), -0.0030),
            ],
        )) as Arc<dyn std::any::Any + Send + Sync>;

        let market = Arc::new(usd_ois_market(as_of)?.insert_market_history(history));

        // Calculate VaR via metrics framework
        let result = bond.price_with_metrics(
            market.as_ref(),
            as_of,
            &[MetricId::HVAR, MetricId::EXPECTED_SHORTFALL],
        )?;

        // Verify VaR was calculated
        let var = result.measures.get("hvar").expect("VaR should be computed");
        assert!(*var > 0.0, "VaR should be positive");

        // Verify ES was calculated
        let es = result
            .measures
            .get("expected_shortfall")
            .expect("ES should be computed");
        assert!(*es >= *var, "ES should be >= VaR");

        Ok(())
    }
}
