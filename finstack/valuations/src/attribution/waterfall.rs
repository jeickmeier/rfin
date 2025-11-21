//! Waterfall P&L attribution methodology.
//!
//! Sequential factor application approach where factors are applied one-by-one
//! in a specified order. Each factor's P&L is computed after applying all
//! previous factors.
//!
//! # Algorithm
//!
//! 1. Start with PV at T₀
//! 2. For each factor in `factor_order`:
//!    - Apply that factor's T₁ state while keeping remaining factors at T₀
//!    - Reprice and record delta
//!    - Keep that factor at T₁ for remaining steps
//! 3. Final PV should equal T₁ PV (residual ≈ 0 by construction)
//!
//! # Default Order
//!
//! If no order specified:
//! 1. Carry
//! 2. RatesCurves
//! 3. CreditCurves
//! 4. InflationCurves
//! 5. Correlations
//! 6. Fx
//! 7. Volatility
//! 8. ModelParameters
//! 9. MarketScalars
//!
//! # Notes
//!
//! - Order matters! Different orders produce different factor attributions
//! - Residual is minimal by construction (should be within numeric precision)
//! - Recommended for risk reporting where sum must equal total

use crate::attribution::factors::*;
use crate::attribution::helpers::*;
use crate::attribution::types::*;
use crate::instruments::common::traits::Instrument;
use finstack_core::prelude::*;
use std::sync::Arc;

/// Default waterfall order for factor attribution.
///
/// # Returns
///
/// Vector of attribution factors in recommended sequential order.
pub fn default_waterfall_order() -> Vec<AttributionFactor> {
    vec![
        AttributionFactor::Carry,
        AttributionFactor::RatesCurves,
        AttributionFactor::CreditCurves,
        AttributionFactor::InflationCurves,
        AttributionFactor::Correlations,
        AttributionFactor::Fx,
        AttributionFactor::Volatility,
        AttributionFactor::ModelParameters,
        AttributionFactor::MarketScalars,
    ]
}

/// Perform waterfall P&L attribution for an instrument.
///
/// Factors are applied sequentially in the specified order. Each factor's
/// P&L is computed after applying all previous factors at T₁.
///
/// # Arguments
///
/// * `instrument` - Instrument to attribute
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `as_of_t0` - Valuation date at T₀
/// * `as_of_t1` - Valuation date at T₁
/// * `config` - Finstack configuration
/// * `factor_order` - Ordered list of factors to apply
/// * `strict_validation` - If true, propagate errors instead of soft failures
///
/// # Returns
///
/// Complete P&L attribution with factor decomposition.
///
/// # Errors
///
/// Returns error if:
/// - Pricing fails at any step
/// - Currency conversion fails
/// - Factor order is empty
/// - (If strict_validation) Model parameter modification/repricing fails
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::attribution::{
///     attribute_pnl_waterfall, default_waterfall_order
/// };
///
/// let attribution = attribute_pnl_waterfall(
///     &instrument,
///     &market_t0,
///     &market_t1,
///     as_of_t0,
///     as_of_t1,
///     &config,
///     default_waterfall_order(),
///     true, // Strict validation
/// )?;
///
/// // Residual should be minimal
/// assert!(attribution.residual_within_tolerance(0.01, 1.0));
/// ```
#[allow(clippy::too_many_arguments)]
pub fn attribute_pnl_waterfall(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    _config: &FinstackConfig,
    factor_order: Vec<AttributionFactor>,
    strict_validation: bool,
) -> Result<PnlAttribution> {
    if factor_order.is_empty() {
        return Err(Error::Validation(
            "Waterfall attribution requires non-empty factor_order".to_string(),
        ));
    }

    let mut num_repricings = 0;

    // Step 1: Price at T₀
    let val_t0 = reprice_instrument(instrument, market_t0, as_of_t0)?;
    num_repricings += 1;

    // Also price at T₁ for total P&L calculation
    let val_t1 = reprice_instrument(instrument, market_t1, as_of_t1)?;
    num_repricings += 1;

    let total_pnl = compute_pnl(val_t0, val_t1, val_t1.currency(), market_t1, as_of_t1)?;

    // Initialize attribution result
    let mut attribution = PnlAttribution::new(
        total_pnl,
        instrument.id(),
        as_of_t0,
        as_of_t1,
        AttributionMethod::Waterfall(factor_order.clone()),
    );

    // Build hybrid market: start with all T₀, progressively apply T₁
    let mut current_market = market_t0.clone();
    let mut current_val = val_t0;

    // Apply each factor in sequence
    for factor in factor_order {
        let (new_market, factor_pnl) = apply_factor_to_t1(
            instrument,
            &current_market,
            market_t0,
            market_t1,
            as_of_t1,
            &factor,
            current_val,
            &mut num_repricings,
            strict_validation,
        )?;

        // Record factor P&L
        match factor {
            AttributionFactor::Carry => attribution.carry = factor_pnl,
            AttributionFactor::RatesCurves => attribution.rates_curves_pnl = factor_pnl,
            AttributionFactor::CreditCurves => attribution.credit_curves_pnl = factor_pnl,
            AttributionFactor::InflationCurves => attribution.inflation_curves_pnl = factor_pnl,
            AttributionFactor::Correlations => attribution.correlations_pnl = factor_pnl,
            AttributionFactor::Fx => {
                attribution.fx_pnl = factor_pnl;
                // Stamp FX policy when FX factor is applied
                attribution.meta.fx_policy = Some(finstack_core::money::fx::FxPolicyMeta {
                    strategy: finstack_core::money::fx::FxConversionPolicy::CashflowDate,
                    target_ccy: Some(current_val.currency()),
                    notes: "Waterfall FX attribution using instrument currency".to_string(),
                });
            }
            AttributionFactor::Volatility => attribution.vol_pnl = factor_pnl,
            AttributionFactor::ModelParameters => {
                attribution.model_params_pnl = factor_pnl;
                // Add note if factor P&L is zero (likely skipped)
                if factor_pnl.amount().abs() < 1e-10 {
                    attribution.meta.notes.push(
                        "Model parameters attribution returned zero (may be unsupported for this instrument type)".to_string()
                    );
                }
            }
            AttributionFactor::MarketScalars => attribution.market_scalars_pnl = factor_pnl,
        }

        // Update current market and value for next iteration
        current_market = new_market;
        current_val = current_val
            .checked_add(factor_pnl)
            .map_err(|_| Error::Validation("Currency mismatch in waterfall".to_string()))?;
    }

    // Compute residual (should be minimal for waterfall)
    // Ignore error as notes will be populated
    let _ = attribution.compute_residual();

    // Update metadata
    attribution.meta.num_repricings = num_repricings;
    attribution.meta.tolerance_abs = 0.01;
    attribution.meta.tolerance_pct = 0.001; // Waterfall should have very small residual
    attribution.meta.rounding = finstack_core::config::rounding_context_from(_config);

    Ok(attribution)
}

/// Apply a single factor's T₁ state to the current market.
///
/// # Arguments
///
/// * `instrument` - Instrument to price
/// * `current_market` - Current hybrid market (some factors at T₀, some at T₁)
/// * `market_t0` - Full T₀ market (for reference)
/// * `market_t1` - Full T₁ market (to extract T₁ factor state)
/// * `as_of_t1` - Valuation date at T₁
/// * `factor` - Factor to apply
/// * `current_val` - Current valuation
/// * `num_repricings` - Counter for total repricings
/// * `strict_validation` - If true, propagate errors instead of soft failures
///
/// # Returns
///
/// Tuple of (new market with factor applied, P&L from applying factor)
#[allow(clippy::too_many_arguments)]
fn apply_factor_to_t1(
    instrument: &Arc<dyn Instrument>,
    current_market: &MarketContext,
    _market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t1: Date,
    factor: &AttributionFactor,
    current_val: Money,
    num_repricings: &mut usize,
    strict_validation: bool,
) -> Result<(MarketContext, Money)> {
    // For ModelParameters, we need to modify the instrument, not the market
    if matches!(factor, AttributionFactor::ModelParameters) {
        // Extract T₁ parameters and create modified instrument
        let params_t1 = crate::attribution::model_params::extract_model_params(instrument);

        if !matches!(
            params_t1,
            crate::attribution::model_params::ModelParamsSnapshot::None
        ) {
            match crate::attribution::model_params::with_model_params(instrument, &params_t1) {
                Ok(instrument_with_t1_params) => {
                    // Reprice with T₁ parameters
                    match reprice_instrument(&instrument_with_t1_params, current_market, as_of_t1) {
                        Ok(new_val) => {
                            *num_repricings += 1;
                            let factor_pnl = compute_pnl(
                                current_val,
                                new_val,
                                current_val.currency(),
                                current_market,
                                as_of_t1,
                            )?;
                            return Ok((current_market.clone(), factor_pnl));
                        }
                        Err(e) => {
                            if strict_validation {
                                return Err(e);
                            }
                            // Repricing failed - log warning since we can't access attribution.meta.notes from here
                            tracing::warn!(
                                error = %e,
                                factor = ?factor,
                                instrument_id = %instrument.id(),
                                "Waterfall attribution: repricing with T₁ model parameters failed, returning zero P&L"
                            );
                            return Ok((
                                current_market.clone(),
                                Money::new(0.0, current_val.currency()),
                            ));
                        }
                    }
                }
                Err(e) => {
                    if strict_validation {
                        return Err(e);
                    }
                    // Parameter modification failed - log warning since we can't access attribution.meta.notes from here
                    tracing::warn!(
                        error = %e,
                        factor = ?factor,
                        instrument_id = %instrument.id(),
                        "Waterfall attribution: model parameter modification failed, returning zero P&L"
                    );
                    return Ok((
                        current_market.clone(),
                        Money::new(0.0, current_val.currency()),
                    ));
                }
            }
        }
        // If no params or extraction fails, return zero P&L
        return Ok((
            current_market.clone(),
            Money::new(0.0, current_val.currency()),
        ));
    }

    // For all other factors, modify the market
    let new_market = match factor {
        AttributionFactor::Carry => {
            // Carry: just advances time (already priced at T₁ date)
            // No market change needed as we're already at T₁ date
            current_market.clone()
        }

        AttributionFactor::RatesCurves => {
            let rates_t1 = extract_rates_curves(market_t1);
            restore_rates_curves(current_market, &rates_t1)
        }

        AttributionFactor::CreditCurves => {
            let credit_t1 = extract_credit_curves(market_t1);
            restore_credit_curves(current_market, &credit_t1)
        }

        AttributionFactor::InflationCurves => {
            let inflation_t1 = extract_inflation_curves(market_t1);
            restore_inflation_curves(current_market, &inflation_t1)
        }

        AttributionFactor::Correlations => {
            let corr_t1 = extract_correlations(market_t1);
            restore_correlations(current_market, &corr_t1)
        }

        AttributionFactor::Fx => {
            // Apply T1 FX matrix while keeping other factors at their current state
            // This isolates the internal FX exposure effect
            let fx_t1 = extract_fx(market_t1);
            restore_fx(current_market, fx_t1)
        }

        AttributionFactor::Volatility => {
            let vol_t1 = extract_volatility(market_t1);
            restore_volatility(current_market, &vol_t1)
        }

        AttributionFactor::MarketScalars => {
            let scalars_t1 = extract_scalars(market_t1);
            restore_scalars(current_market, &scalars_t1)
        }

        AttributionFactor::ModelParameters => {
            // Already handled above
            unreachable!()
        }
    };

    // Reprice with new market
    let new_val = reprice_instrument(instrument, &new_market, as_of_t1)?;
    *num_repricings += 1;

    // Compute P&L from this factor
    let factor_pnl = compute_pnl(
        current_val,
        new_val,
        current_val.currency(),
        &new_market,
        as_of_t1,
    )?;

    Ok((new_market, factor_pnl))
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use time::macros::date;

    // Simple test instrument
    struct TestInstrument {
        id: String,
        value: Money,
    }

    impl TestInstrument {
        fn new(id: &str, value: Money) -> Self {
            Self {
                id: id.to_string(),
                value,
            }
        }
    }

    impl crate::instruments::common::traits::Instrument for TestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> crate::pricer::InstrumentType {
            crate::pricer::InstrumentType::Bond // arbitrary choice for test
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
            // Return a static empty attributes for test purposes
            use std::sync::OnceLock;
            static ATTRS: OnceLock<crate::instruments::common::traits::Attributes> =
                OnceLock::new();
            ATTRS.get_or_init(crate::instruments::common::traits::Attributes::default)
        }

        fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
            // Not used in tests, but required by trait
            unreachable!("TestInstrument::attributes_mut should not be called in tests")
        }

        fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
            Box::new(Self {
                id: self.id.clone(),
                value: self.value,
            })
        }

        fn value(&self, _market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.value)
        }

        fn price_with_metrics(
            &self,
            market: &MarketContext,
            as_of: Date,
            _metrics: &[crate::metrics::MetricId],
        ) -> Result<crate::results::ValuationResult> {
            let value = self.value(market, as_of)?;
            Ok(crate::results::ValuationResult::stamped(
                self.id(),
                as_of,
                value,
            ))
        }
    }

    #[test]
    fn test_default_waterfall_order() {
        let order = default_waterfall_order();
        assert_eq!(order.len(), 9);
        assert_eq!(order[0], AttributionFactor::Carry);
        assert_eq!(order[1], AttributionFactor::RatesCurves);
    }

    #[test]
    fn test_waterfall_requires_order() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        let instrument: Arc<dyn crate::instruments::common::traits::Instrument> = Arc::new(
            TestInstrument::new("TEST-001", Money::new(1000.0, Currency::USD)),
        );

        let market_t0 = MarketContext::new();
        let market_t1 = MarketContext::new();
        let config = FinstackConfig::default();

        // Empty order should fail
        let result = attribute_pnl_waterfall(
            &instrument,
            &market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
            &config,
            vec![],
            false, // strict validation off
        );

        assert!(result.is_err());
    }
}
