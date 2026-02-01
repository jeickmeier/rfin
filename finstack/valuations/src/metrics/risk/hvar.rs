//! Generic Historical VaR metric calculator.
//!
//! Integrates Historical VaR into the standard metrics framework as a
//! `MetricCalculator` that can be registered and computed alongside other
//! risk metrics like DV01, Theta, etc.

use crate::metrics::core::traits::{MetricCalculator, MetricContext};
use crate::metrics::risk::{calculate_var, VarConfig};
use crate::metrics::MetricId;
use finstack_core::Result;

/// Generic Historical VaR calculator that works with any instrument.
///
/// This calculator integrates Historical VaR into the standard metrics
/// framework. It requires a `MarketHistory` to be provided at the pricing
/// boundary (see [`crate::instruments::common::traits::Instrument::price_with_options`]).
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::metrics::{MetricId, MetricRegistry};
/// use finstack_valuations::metrics::risk::{GenericHVar, VarConfig};
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
}

impl MetricCalculator for GenericHVar {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let history = context.market_history.as_deref().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Market history required for VaR calculation. Provide it via Instrument::price_with_options(...) with PricingOptions::with_market_history(...)"
                    .to_string(),
            )
        })?;

        // Calculate VaR for this instrument
        let result = calculate_var(
            &[context.instrument.as_ref()],
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

/// Generic Expected Shortfall (ES / CVaR) calculator that works with any instrument.
///
/// This is the companion to [`GenericHVar`]. It computes the same historical simulation
/// distribution but returns **Expected Shortfall** as the primary metric value.
///
/// Notes:
/// - If both `MetricId::HVAR` and `MetricId::EXPECTED_SHORTFALL` are requested, whichever is
///   computed first will populate the other in `context.computed` so the second computation
///   will be skipped by the registry (deterministic and avoids duplicated repricing).
pub struct GenericExpectedShortfall {
    config: VarConfig,
}

impl GenericExpectedShortfall {
    /// Create a new Expected Shortfall calculator with the given configuration.
    pub fn new(config: VarConfig) -> Self {
        Self { config }
    }
}

impl MetricCalculator for GenericExpectedShortfall {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let history = context.market_history.as_deref().ok_or_else(|| {
            finstack_core::Error::Validation(
                "Market history required for VaR/ES calculation. Provide it via Instrument::price_with_options(...) with PricingOptions::with_market_history(...)"
                    .to_string(),
            )
        })?;

        let result = calculate_var(
            &[context.instrument.as_ref()],
            &context.curves,
            history,
            context.as_of,
            &self.config,
        )?;

        // Populate VaR as an auxiliary metric so callers requesting both get
        // deterministic, single-pass results.
        context.computed.insert(MetricId::HVAR, result.var);

        Ok(result.expected_shortfall)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[allow(clippy::expect_used, dead_code, unused_imports)]
    mod test_utils {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/metrics_risk_test_utils.rs"
        ));
    }

    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use std::sync::Arc;
    use test_utils::{history_from_rate_shifts, sample_as_of, standard_bond, usd_ois_market};
    use time::Duration;

    #[test]
    fn test_generic_hvar_creation() {
        let var_calc = GenericHVar::new(VarConfig::var_95());
        assert_eq!(var_calc.config.confidence_level, 0.95);

        let var_calc = GenericHVar::new(VarConfig::var_99());
        assert_eq!(var_calc.config.confidence_level, 0.99);

        let var_calc = GenericHVar::new(VarConfig::new(0.975));
        assert_eq!(var_calc.config.confidence_level, 0.975);
    }

    #[test]
    fn test_hvar_via_metrics_framework() -> Result<()> {
        let as_of = sample_as_of();
        let maturity = as_of + Duration::days(365 * 5);
        let bond = standard_bond("TEST-BOND", as_of, maturity);

        // Use enough scenarios so ES != VaR at 95% (tail size >= 2).
        let mut shifts: Vec<(finstack_core::dates::Date, f64)> = Vec::new();
        for i in 1..=25_i64 {
            let d = as_of - Duration::days(i);
            // Mix signs and magnitudes to ensure a non-degenerate tail.
            let shift = if i % 2 == 0 {
                0.0004 * (i as f64)
            } else {
                -0.0003 * (i as f64)
            };
            shifts.push((d, shift));
        }
        let history = Arc::new(history_from_rate_shifts(as_of, &shifts));

        let market = Arc::new(usd_ois_market(as_of)?);

        // Compute a reference result directly from the VaR engine.
        let expected = calculate_var(
            &[&bond],
            market.as_ref(),
            history.as_ref(),
            as_of,
            &VarConfig::var_95(),
        )?;

        // Calculate VaR + ES via metrics framework
        use crate::instruments::PricingOptions;
        let opts = PricingOptions::default().with_market_history(history);
        let result_ordered = bond.price_with_options(
            market.as_ref(),
            as_of,
            &[MetricId::HVAR, MetricId::EXPECTED_SHORTFALL],
            opts,
        )?;

        let var = *result_ordered
            .measures
            .get("hvar")
            .expect("VaR should be computed");
        let es = *result_ordered
            .measures
            .get("expected_shortfall")
            .expect("ES should be computed");
        assert!(var > 0.0, "VaR should be positive");
        assert!(es >= var, "ES should be >= VaR");
        assert!(
            (var - expected.var).abs() < 1e-10,
            "VaR mismatch: got {var}, expected {}",
            expected.var
        );
        assert!(
            (es - expected.expected_shortfall).abs() < 1e-10,
            "ES mismatch: got {es}, expected {}",
            expected.expected_shortfall
        );

        // Also verify reversed ordering doesn't break ES vs VaR wiring.
        let history2 = Arc::new(history_from_rate_shifts(as_of, &shifts));
        let opts2 = PricingOptions::default().with_market_history(history2);
        let result_reversed = bond.price_with_options(
            market.as_ref(),
            as_of,
            &[MetricId::EXPECTED_SHORTFALL, MetricId::HVAR],
            opts2,
        )?;
        let var2 = *result_reversed
            .measures
            .get("hvar")
            .expect("VaR should be computed");
        let es2 = *result_reversed
            .measures
            .get("expected_shortfall")
            .expect("ES should be computed");
        assert!((var2 - expected.var).abs() < 1e-10);
        assert!((es2 - expected.expected_shortfall).abs() < 1e-10);

        Ok(())
    }
}
