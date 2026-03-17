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

use super::factors::*;
use super::helpers::*;
use super::model_params;
use super::types::*;
use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::sensitivities::theta::collect_cashflows_in_period;
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, Result};
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
/// ```rust,no_run
/// use finstack_valuations::attribution::{
///     attribute_pnl_waterfall, default_waterfall_order
/// };
/// use finstack_valuations::instruments::rates::deposit::Deposit;
/// use finstack_core::config::FinstackConfig;
/// use finstack_core::currency::Currency;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::money::Money;
/// use std::sync::Arc;
/// use time::macros::date;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let as_of_t0 = date!(2025-01-15);
/// let as_of_t1 = date!(2025-01-16);
/// let market_t0 = MarketContext::new();
/// let market_t1 = MarketContext::new();
/// let config = FinstackConfig::default();
///
/// let instrument = Arc::new(
///     Deposit::builder()
///         .id("DEP-1D".into())
///         .notional(Money::new(1_000_000.0, Currency::USD))
///         .start_date(as_of_t0)
///         .maturity(as_of_t1)
///         .day_count(finstack_core::dates::DayCount::Act360)
///         .discount_curve_id("USD-OIS".into())
///         .build()
///         .expect("deposit builder should succeed"),
/// ) as Arc<dyn finstack_valuations::instruments::Instrument>;
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
///     None,
/// )?;
///
/// // Residual should be minimal
/// assert!(attribution.residual_within_tolerance(0.01, 1.0));
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip_all, fields(instrument_id = %instrument.id(), method = "waterfall"))]
pub fn attribute_pnl_waterfall(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    _config: &FinstackConfig,
    factor_order: Vec<AttributionFactor>,
    strict_validation: bool,
    model_params_t0: Option<&model_params::ModelParamsSnapshot>,
) -> Result<PnlAttribution> {
    let input = AttributionInput {
        instrument,
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
        config: Some(_config),
        model_params_t0,
        val_t0: None,
        val_t1: None,
        strict_validation,
    };
    attribute_pnl_waterfall_impl(&input, factor_order)
}

/// Internal implementation of waterfall attribution using `AttributionInput`.
///
/// This is the core implementation that uses the context struct pattern
/// to reduce parameter count and improve maintainability.
fn attribute_pnl_waterfall_impl(
    input: &AttributionInput,
    factor_order: Vec<AttributionFactor>,
) -> Result<PnlAttribution> {
    if factor_order.is_empty() {
        return Err(Error::Validation(
            "Waterfall attribution requires non-empty factor_order".to_string(),
        ));
    }

    let instrument = input.instrument;
    let market_t0 = input.market_t0;
    let market_t1 = input.market_t1;
    let as_of_t0 = input.as_of_t0;
    let as_of_t1 = input.as_of_t1;
    let model_params_t0 = input.model_params_t0;
    let _config = input.config.ok_or_else(|| {
        finstack_core::Error::Validation("config required for waterfall attribution".to_string())
    })?;
    let strict_validation = input.strict_validation;

    // Step 1: Price at T₀
    // Use T₀ model parameters for T₀ valuation if available
    let instrument_t0 = if let Some(params) = model_params_t0 {
        model_params::with_model_params(instrument, params)?
    } else {
        Arc::clone(instrument)
    };
    let val_t0 = reprice_instrument(&instrument_t0, market_t0, as_of_t0)?;

    // Also price at T₁ for total P&L calculation
    let val_t1 = reprice_instrument(instrument, market_t1, as_of_t1)?;

    let total_pnl = compute_pnl_with_fx(
        val_t0,
        val_t1,
        val_t1.currency(),
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
    )?;

    // Initialize attribution result with configured rounding context
    let rounding = finstack_core::config::rounding_context_from(_config);
    let mut attribution = PnlAttribution::new_with_rounding(
        total_pnl,
        instrument.id(),
        as_of_t0,
        as_of_t1,
        AttributionMethod::Waterfall(factor_order.clone()),
        rounding,
    );

    // Build hybrid market: start with all T₀, progressively apply T₁
    let mut ctx = WaterfallContext {
        target_instrument: instrument,
        current_instrument: instrument_t0,
        current_market: market_t0.clone(),
        current_val: val_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
        strict_validation,
        num_repricings: 2, // T₀ and T₁ repricings already performed
    };

    // Apply each factor in sequence
    for factor in factor_order {
        let factor_pnl = ctx.apply_factor(&factor)?;

        // Record factor P&L
        match factor {
            AttributionFactor::Carry => {
                attribution.carry = factor_pnl;
                let coupon_income = collect_cashflows_in_period(
                    ctx.current_instrument.as_ref(),
                    &ctx.current_market,
                    ctx.as_of_t0,
                    ctx.as_of_t1,
                    factor_pnl.currency(),
                )
                .ok()
                .map(|value| Money::new(value, factor_pnl.currency()));
                attribution.carry_detail = Some(CarryDetail {
                    total: factor_pnl,
                    coupon_income,
                    pull_to_par: None,
                    roll_down: None,
                    funding_cost: None,
                    theta: Some(factor_pnl),
                });
            }
            AttributionFactor::RatesCurves => attribution.rates_curves_pnl = factor_pnl,
            AttributionFactor::CreditCurves => attribution.credit_curves_pnl = factor_pnl,
            AttributionFactor::InflationCurves => attribution.inflation_curves_pnl = factor_pnl,
            AttributionFactor::Correlations => attribution.correlations_pnl = factor_pnl,
            AttributionFactor::Fx => {
                attribution.fx_pnl = factor_pnl;
                // Stamp FX policy when FX factor is applied
                attribution.meta.fx_policy = Some(finstack_core::money::fx::FxPolicyMeta {
                    strategy: finstack_core::money::fx::FxConversionPolicy::CashflowDate,
                    target_ccy: Some(attribution.fx_pnl.currency()),
                    notes: "Waterfall FX attribution with full translation".to_string(),
                });
            }
            AttributionFactor::Volatility => attribution.vol_pnl = factor_pnl,
            AttributionFactor::ModelParameters => {
                attribution.model_params_pnl = factor_pnl;
                // Add note if factor P&L is zero (likely skipped)
                if factor_pnl.amount().abs() < 1e-10 {
                    attribution
                        .meta
                        .notes
                        .push("Model parameters attribution returned zero".to_string());
                }
            }
            AttributionFactor::MarketScalars => attribution.market_scalars_pnl = factor_pnl,
        }
    }

    // Compute residual (should be minimal for waterfall)
    if let Err(e) = attribution.compute_residual() {
        tracing::warn!(
            error = %e,
            instrument_id = %instrument.id(),
            "Residual computation failed; attribution may be incomplete"
        );
    }

    // Update metadata
    attribution.meta.num_repricings = ctx.num_repricings();
    attribution.meta.tolerance_abs = 0.01;
    attribution.meta.tolerance_pct = 0.001; // Waterfall should have very small residual

    Ok(attribution)
}

struct WaterfallContext<'a> {
    target_instrument: &'a Arc<dyn Instrument>,
    current_instrument: Arc<dyn Instrument>,
    current_market: MarketContext,
    current_val: Money,
    market_t1: &'a MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    strict_validation: bool,
    num_repricings: usize,
}

impl<'a> WaterfallContext<'a> {
    fn num_repricings(&self) -> usize {
        self.num_repricings
    }

    fn apply_factor(&mut self, factor: &AttributionFactor) -> Result<Money> {
        let _span = tracing::info_span!("waterfall_factor", factor = %factor).entered();
        let prev_val = self.current_val;
        let base_currency = prev_val.currency();

        if matches!(factor, AttributionFactor::ModelParameters) {
            return self.apply_model_params(prev_val, base_currency, factor);
        }

        // Carry only changes the date, not the market — skip the clone
        if matches!(factor, AttributionFactor::Carry) {
            let new_val = reprice_instrument(
                &self.current_instrument,
                &self.current_market,
                self.as_of_t1,
            )?;
            self.num_repricings += 1;
            let factor_pnl = compute_pnl(
                prev_val,
                new_val,
                base_currency,
                &self.current_market,
                self.as_of_t1,
            )?;
            self.update_current_value(prev_val, factor_pnl)?;
            return Ok(factor_pnl);
        }

        let new_market = self.build_market_for_factor(factor)?;
        let new_val = reprice_instrument(&self.current_instrument, &new_market, self.as_of_t1)?;
        self.num_repricings += 1;

        let factor_pnl = if matches!(factor, AttributionFactor::Fx) {
            compute_pnl_with_fx(
                prev_val,
                new_val,
                base_currency,
                &self.current_market,
                &new_market,
                self.as_of_t0,
                self.as_of_t1,
            )?
        } else {
            compute_pnl(prev_val, new_val, base_currency, &new_market, self.as_of_t1)?
        };

        self.current_market = new_market;
        self.update_current_value(prev_val, factor_pnl)?;
        Ok(factor_pnl)
    }

    fn apply_model_params(
        &mut self,
        prev_val: Money,
        base_currency: Currency,
        factor: &AttributionFactor,
    ) -> Result<Money> {
        match reprice_instrument(self.target_instrument, &self.current_market, self.as_of_t1) {
            Ok(new_val) => {
                self.num_repricings += 1;
                let factor_pnl = compute_pnl(
                    prev_val,
                    new_val,
                    base_currency,
                    &self.current_market,
                    self.as_of_t1,
                )?;
                self.current_instrument = Arc::clone(self.target_instrument);
                self.update_current_value(prev_val, factor_pnl)?;
                Ok(factor_pnl)
            }
            Err(e) => {
                if self.strict_validation {
                    return Err(e);
                }
                tracing::warn!(
                    error = %e,
                    factor = ?factor,
                    instrument_id = %self.target_instrument.id(),
                    "Waterfall attribution: repricing with T₁ model parameters failed, returning zero P&L"
                );
                Ok(Money::new(0.0, base_currency))
            }
        }
    }

    fn build_market_for_factor(&self, factor: &AttributionFactor) -> Result<MarketContext> {
        match factor {
            AttributionFactor::Carry => Ok(self.current_market.clone()),
            AttributionFactor::RatesCurves => {
                let rates_t1 = MarketSnapshot::extract(self.market_t1, CurveRestoreFlags::RATES);
                Ok(MarketSnapshot::restore_market(
                    &self.current_market,
                    &rates_t1,
                    CurveRestoreFlags::RATES,
                ))
            }
            AttributionFactor::CreditCurves => {
                let credit_t1 = MarketSnapshot::extract(self.market_t1, CurveRestoreFlags::CREDIT);
                Ok(MarketSnapshot::restore_market(
                    &self.current_market,
                    &credit_t1,
                    CurveRestoreFlags::CREDIT,
                ))
            }
            AttributionFactor::InflationCurves => {
                let inflation_t1 =
                    MarketSnapshot::extract(self.market_t1, CurveRestoreFlags::INFLATION);
                Ok(MarketSnapshot::restore_market(
                    &self.current_market,
                    &inflation_t1,
                    CurveRestoreFlags::INFLATION,
                ))
            }
            AttributionFactor::Correlations => {
                let corr_t1 =
                    MarketSnapshot::extract(self.market_t1, CurveRestoreFlags::CORRELATION);
                Ok(MarketSnapshot::restore_market(
                    &self.current_market,
                    &corr_t1,
                    CurveRestoreFlags::CORRELATION,
                ))
            }
            AttributionFactor::Fx => {
                let fx_t1 = extract_fx(self.market_t1);
                Ok(restore_fx(&self.current_market, fx_t1))
            }
            AttributionFactor::Volatility => {
                let vol_t1 = VolatilitySnapshot::extract(self.market_t1);
                Ok(restore_volatility(&self.current_market, &vol_t1))
            }
            AttributionFactor::MarketScalars => {
                let scalars_t1 = ScalarsSnapshot::extract(self.market_t1);
                Ok(restore_scalars(&self.current_market, &scalars_t1))
            }
            AttributionFactor::ModelParameters => Err(Error::Internal),
        }
    }

    /// Update the current accumulated value by adding a factor's P&L delta.
    ///
    /// # Numerical Precision Note
    ///
    /// This method uses simple sequential addition rather than Kahan summation.
    /// For the standard 9-factor waterfall attribution, this is acceptable because:
    ///
    /// - IEEE 754 f64 has ~15-16 significant digits
    /// - With 9 additions, accumulated relative error is bounded by ~9 × ε ≈ 2e-15
    /// - For typical P&L values ($1M or less), this represents sub-cent precision
    ///
    /// If you extend the waterfall to >20 factors or work with very large notionals
    /// ($1B+), consider implementing Kahan summation:
    ///
    /// ```ignore
    /// // Kahan summation pseudocode
    /// let compensation = 0.0;
    /// for delta in deltas {
    ///     let y = delta - compensation;
    ///     let t = sum + y;
    ///     compensation = (t - sum) - y;
    ///     sum = t;
    /// }
    /// ```
    ///
    /// For most production use cases, the current implementation is sufficient.
    fn update_current_value(&mut self, prev_val: Money, delta: Money) -> Result<()> {
        self.current_val = prev_val
            .checked_add(delta)
            .map_err(|_| Error::Validation("Currency mismatch in waterfall".to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    #[allow(clippy::expect_used, dead_code, unused_imports)]
    mod test_utils {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/attribution_test_utils.rs"
        ));
    }

    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use test_utils::TestInstrument;
    use time::macros::date;

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

        let instrument: Arc<dyn crate::instruments::common_impl::traits::Instrument> = Arc::new(
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
            None,
        );

        assert!(result.is_err());
    }
}
