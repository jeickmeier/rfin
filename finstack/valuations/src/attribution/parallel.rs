//! Parallel P&L attribution methodology.
//!
//! Independent factor isolation approach where each factor is analyzed separately
//! by restoring T₀ values for that factor while keeping all other factors at T₁.
//!
//! # Algorithm
//!
//! 1. Price at T₀ and T₁ with actual markets → total_pnl
//! 2. **Carry**: Price at T₁ date with T₀ market (frozen) → isolate time/accrual effect
//! 3. **RatesCurves**: Restore T₀ discount/forward curves, reprice → rates P&L
//! 4. **CreditCurves**: Restore T₀ hazard curves, reprice → credit P&L
//! 5. **InflationCurves**: Restore T₀ inflation curves, reprice → inflation P&L
//! 6. **Correlations**: Restore T₀ base correlation curves, reprice → correlation P&L
//! 7. **Fx**: Restore T₀ FX matrix, reprice → fx P&L
//! 8. **Volatility**: Restore T₀ vol surfaces, reprice → vol P&L
//! 9. **ModelParameters**: Restore T₀ model parameters, reprice → model params P&L
//! 10. **MarketScalars**: Restore T₀ market scalars, reprice → scalars P&L
//! 11. **Residual**: total_pnl - sum(all attributed factors)
//!
//! # Notes
//!
//! - Factors are isolated independently, so cross-effects appear in residual
//! - Model parameters attribution requires instrument-specific support (see model_params.rs)

use crate::attribution::factors::*;
use crate::attribution::helpers::*;
use crate::attribution::types::*;
use crate::instruments::common::traits::Instrument;
use finstack_core::config::FinstackConfig;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;
use std::sync::Arc;

/// Perform parallel P&L attribution for an instrument.
///
/// Each factor is isolated independently by restoring T₀ values for that
/// factor while keeping all others at T₁. Cross-effects and non-linearities
/// appear in the residual.
///
/// # Arguments
///
/// * `instrument` - Instrument to attribute
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `as_of_t0` - Valuation date at T₀
/// * `as_of_t1` - Valuation date at T₁
/// * `config` - Finstack configuration (for rounding, etc.)
///
/// # Returns
///
/// Complete P&L attribution with factor decomposition.
///
/// # Errors
///
/// Returns error if:
/// - Pricing fails at T₀ or T₁
/// - Currency conversion fails
/// - Market data is missing
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::attribution::attribute_pnl_parallel;
/// use finstack_valuations::instruments::deposit::Deposit;
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
///         .start(as_of_t0)
///         .end(as_of_t1)
///         .day_count(finstack_core::dates::DayCount::Act360)
///         .discount_curve_id("USD-OIS".into())
///         .build()
///         .expect("deposit builder should succeed"),
/// ) as Arc<dyn finstack_valuations::instruments::common::traits::Instrument>;
///
/// let attribution = attribute_pnl_parallel(
///     &instrument,
///     &market_t0,
///     &market_t1,
///     as_of_t0,
///     as_of_t1,
///     &config,
///     None,
/// )?;
///
/// println!("Total P&L: {}", attribution.total_pnl);
/// println!("Carry: {}", attribution.carry);
/// println!("Rates: {}", attribution.rates_curves_pnl);
/// println!("Residual: {} ({:.2}%)",
///     attribution.residual,
///     attribution.meta.residual_pct
/// );
/// # Ok(())
/// # }
/// ```
pub fn attribute_pnl_parallel(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    _config: &FinstackConfig,
    model_params_t0: Option<&crate::attribution::model_params::ModelParamsSnapshot>,
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
        strict_validation: false,
    };
    attribute_pnl_parallel_impl(&input)
}

/// Internal implementation of parallel attribution using `AttributionInput`.
///
/// This is the core implementation that uses the context struct pattern
/// to reduce parameter count and improve maintainability.
fn attribute_pnl_parallel_impl(input: &AttributionInput) -> Result<PnlAttribution> {
    let instrument = input.instrument;
    let market_t0 = input.market_t0;
    let market_t1 = input.market_t1;
    let as_of_t0 = input.as_of_t0;
    let as_of_t1 = input.as_of_t1;
    let model_params_t0 = input.model_params_t0;
    let _config = input.config.ok_or_else(|| {
        finstack_core::Error::Validation("config required for parallel attribution".to_string())
    })?;

    let mut num_repricings = 0;

    // Step 1: Price at T₀ and T₁
    // Use T₀ model parameters for T₀ valuation if available
    let instrument_t0 = if let Some(params) = model_params_t0 {
        crate::attribution::model_params::with_model_params(instrument, params)?
    } else {
        Arc::clone(instrument)
    };
    let val_t0 = reprice_instrument(&instrument_t0, market_t0, as_of_t0)?;
    num_repricings += 1;

    let val_t1 = reprice_instrument(instrument, market_t1, as_of_t1)?;
    num_repricings += 1;

    // Total P&L (with FX translation)
    let total_pnl = compute_pnl_with_fx(
        val_t0,
        val_t1,
        val_t1.currency(),
        market_t0,
        market_t1,
        as_of_t0,
        as_of_t1,
    )?;

    // Initialize attribution result
    let mut attribution = PnlAttribution::new(
        total_pnl,
        instrument.id(),
        as_of_t0,
        as_of_t1,
        AttributionMethod::Parallel,
    );

    // Step 2: Carry attribution (time decay + accruals)
    // Price at T₁ date with T₀ market
    let market_frozen = market_t0.clone();
    let val_carry = reprice_instrument(instrument, &market_frozen, as_of_t1)?;
    num_repricings += 1;

    attribution.carry = compute_pnl(val_t0, val_carry, val_t1.currency(), market_t1, as_of_t1)?;

    // Step 3: Rates curves attribution (discount + forward)
    let rates_snapshot = MarketSnapshot::extract(market_t0, CurveRestoreFlags::RATES);
    let market_with_t0_rates =
        MarketSnapshot::restore_market(market_t1, &rates_snapshot, CurveRestoreFlags::RATES);
    let val_with_t0_rates = reprice_instrument(instrument, &market_with_t0_rates, as_of_t1)?;
    num_repricings += 1;

    // Rates P&L = impact of moving from T₀ rates to T₁ rates
    // val_t1 (with T₁ rates) - val_with_t0_rates (with T₀ rates)
    attribution.rates_curves_pnl = compute_pnl(
        val_with_t0_rates,
        val_t1,
        val_t1.currency(),
        market_t1,
        as_of_t1,
    )?;

    // Step 4: Credit curves attribution (hazard curves)
    let credit_snapshot = MarketSnapshot::extract(market_t0, CurveRestoreFlags::CREDIT);
    if !credit_snapshot.hazard_curves.is_empty() {
        let market_with_t0_credit =
            MarketSnapshot::restore_market(market_t1, &credit_snapshot, CurveRestoreFlags::CREDIT);
        let val_with_t0_credit = reprice_instrument(instrument, &market_with_t0_credit, as_of_t1)?;
        num_repricings += 1;

        attribution.credit_curves_pnl = compute_pnl(
            val_with_t0_credit,
            val_t1,
            val_t1.currency(),
            market_t1,
            as_of_t1,
        )?;
    }

    // Step 5: Inflation curves attribution
    let inflation_snapshot = MarketSnapshot::extract(market_t0, CurveRestoreFlags::INFLATION);
    if !inflation_snapshot.inflation_curves.is_empty() {
        let market_with_t0_inflation = MarketSnapshot::restore_market(
            market_t1,
            &inflation_snapshot,
            CurveRestoreFlags::INFLATION,
        );
        let val_with_t0_inflation =
            reprice_instrument(instrument, &market_with_t0_inflation, as_of_t1)?;
        num_repricings += 1;

        attribution.inflation_curves_pnl = compute_pnl(
            val_with_t0_inflation,
            val_t1,
            val_t1.currency(),
            market_t1,
            as_of_t1,
        )?;
    }

    // Step 6: Correlations attribution (base correlation curves)
    let correlations_snapshot = MarketSnapshot::extract(market_t0, CurveRestoreFlags::CORRELATION);
    if !correlations_snapshot.base_correlation_curves.is_empty() {
        let market_with_t0_corr = MarketSnapshot::restore_market(
            market_t1,
            &correlations_snapshot,
            CurveRestoreFlags::CORRELATION,
        );
        let val_with_t0_corr = reprice_instrument(instrument, &market_with_t0_corr, as_of_t1)?;
        num_repricings += 1;

        attribution.correlations_pnl = compute_pnl(
            val_with_t0_corr,
            val_t1,
            val_t1.currency(),
            market_t1,
            as_of_t1,
        )?;
    }

    // Step 7: FX attribution
    // Measures internal FX exposure (pricing-side) effects
    // For cross-currency instruments, this captures how changes in FX rates affect
    // the instrument's value in its native currency
    let fx_t0 = extract_fx(market_t0);
    if fx_t0.is_some() {
        let market_with_t0_fx = restore_fx(market_t1, fx_t0.clone());
        let val_with_t0_fx = reprice_instrument(instrument, &market_with_t0_fx, as_of_t1)?;
        num_repricings += 1;

        // Use full FX translation attribution
        // This captures both internal pricing impact and translation P&L
        attribution.fx_pnl = compute_pnl_with_fx(
            val_with_t0_fx,
            val_t1,
            val_t1.currency(),
            market_t0,
            market_t1,
            as_of_t0,
            as_of_t1,
        )?;

        // Stamp FX policy if conversions were applied
        if attribution.fx_pnl.currency() != val_t1.currency() {
            attribution.meta.fx_policy = Some(finstack_core::money::fx::FxPolicyMeta {
                strategy: finstack_core::money::fx::FxConversionPolicy::CashflowDate,
                target_ccy: Some(val_t1.currency()),
                notes: "Parallel FX attribution with full translation".to_string(),
            });
        }
    }

    // Step 8: Volatility attribution
    let vol_snapshot = VolatilitySnapshot::extract(market_t0);
    if !vol_snapshot.surfaces.is_empty() {
        let market_with_t0_vol = restore_volatility(market_t1, &vol_snapshot);
        let val_with_t0_vol = reprice_instrument(instrument, &market_with_t0_vol, as_of_t1)?;
        num_repricings += 1;

        attribution.vol_pnl = compute_pnl(
            val_with_t0_vol,
            val_t1,
            val_t1.currency(),
            market_t1,
            as_of_t1,
        )?;
    }

    // Step 9: Model parameters attribution
    let params_t0 = model_params_t0
        .cloned()
        .unwrap_or_else(|| crate::attribution::model_params::extract_model_params(instrument));
    if !matches!(
        params_t0,
        crate::attribution::model_params::ModelParamsSnapshot::None
    ) {
        // Create instrument with T₀ parameters
        match crate::attribution::model_params::with_model_params(instrument, &params_t0) {
            Ok(instrument_with_t0_params) => {
                // Reprice with T₁ market
                match reprice_instrument(&instrument_with_t0_params, market_t1, as_of_t1) {
                    Ok(val_with_t0_params) => {
                        num_repricings += 1;

                        attribution.model_params_pnl = compute_pnl(
                            val_with_t0_params,
                            val_t1,
                            val_t1.currency(),
                            market_t1,
                            as_of_t1,
                        )?;
                    }
                    Err(e) => {
                        attribution.meta.notes.push(format!(
                            "Model parameters attribution: repricing failed - {}",
                            e
                        ));
                    }
                }
            }
            Err(e) => {
                attribution.meta.notes.push(format!(
                    "Model parameters attribution: parameter modification failed - {}",
                    e
                ));
            }
        }
    }

    // Step 10: Market scalars attribution
    let scalars_snapshot = ScalarsSnapshot::extract(market_t0);
    let has_scalars = !scalars_snapshot.prices.is_empty()
        || !scalars_snapshot.series.is_empty()
        || !scalars_snapshot.inflation_indices.is_empty()
        || !scalars_snapshot.dividends.is_empty();

    if has_scalars {
        let market_with_t0_scalars = restore_scalars(market_t1, &scalars_snapshot);
        let val_with_t0_scalars =
            reprice_instrument(instrument, &market_with_t0_scalars, as_of_t1)?;
        num_repricings += 1;

        attribution.market_scalars_pnl = compute_pnl(
            val_with_t0_scalars,
            val_t1,
            val_t1.currency(),
            market_t1,
            as_of_t1,
        )?;
    }

    // Step 11: Compute residual
    // Ignore error as notes will be populated
    let _ = attribution.compute_residual();

    // Update metadata
    attribution.meta.num_repricings = num_repricings;
    attribution.meta.tolerance_abs = 1.0;
    attribution.meta.tolerance_pct = 0.1;
    attribution.meta.rounding = finstack_core::config::rounding_context_from(_config);

    Ok(attribution)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::attribution::test_utils::TestInstrument;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use time::macros::date;

    #[test]
    fn test_parallel_attribution_simple() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        // Create test instrument with different values at T0 and T1
        let _instrument_t0 = Arc::new(TestInstrument::new(
            "TEST-001",
            Money::new(1000.0, Currency::USD),
        ));

        // Simulate P&L by creating a different value for T1
        // In practice, the same instrument would be repriced with different markets
        let val_t0 = Money::new(1000.0, Currency::USD);
        let val_t1 = Money::new(1100.0, Currency::USD);

        // Create minimal markets
        let _market_t0 = MarketContext::new();
        let _market_t1 = MarketContext::new();
        let _config = FinstackConfig::default();

        // For this test, we'll manually construct the attribution since our test
        // instrument returns fixed values
        let total_pnl = val_t1
            .checked_sub(val_t0)
            .expect("PNL calculation should succeed in test");
        let attribution = PnlAttribution::new(
            total_pnl,
            "TEST-001",
            as_of_t0,
            as_of_t1,
            AttributionMethod::Parallel,
        );

        assert_eq!(attribution.total_pnl.amount(), 100.0);
        assert_eq!(attribution.residual.amount(), 100.0); // Initially all in residual
    }

    #[test]
    fn test_parallel_attribution_with_curve_change() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        // Create discount curves at T0 and T1
        let curve_t0 = DiscountCurve::builder("USD-OIS")
            .base_date(as_of_t0)
            .knots(vec![(0.0, 1.0), (1.0, 0.98)])
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let curve_t1 = DiscountCurve::builder("USD-OIS")
            .base_date(as_of_t1)
            .knots(vec![(0.0, 1.0), (1.0, 0.97)]) // Rates increased (curve lower)
            .set_interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let market_t0 = MarketContext::new().insert_discount(curve_t0);
        let market_t1 = MarketContext::new().insert_discount(curve_t1);

        // Extract and verify snapshots work
        let rates_snapshot = MarketSnapshot::extract(&market_t0, CurveRestoreFlags::RATES);
        assert_eq!(rates_snapshot.discount_curves.len(), 1);

        let restored =
            MarketSnapshot::restore_market(&market_t1, &rates_snapshot, CurveRestoreFlags::RATES);
        assert!(restored.get_discount("USD-OIS").is_ok());
    }
}
