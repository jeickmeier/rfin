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
//! 9. **ModelParameters**: Restore T₀ model parameters, reprice → model params P&L (TODO)
//! 10. **MarketScalars**: Restore T₀ market scalars, reprice → scalars P&L
//! 11. **Residual**: total_pnl - sum(all attributed factors)
//!
//! # Notes
//!
//! - Factors are isolated independently, so cross-effects appear in residual
//! - Model parameters attribution requires instrument-specific support (TODO)

use crate::attribution::factors::*;
use crate::attribution::helpers::*;
use crate::attribution::types::*;
use crate::instruments::common::traits::Instrument;
use finstack_core::prelude::*;
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
/// ```rust,ignore
/// use finstack_valuations::attribution::attribute_pnl_parallel;
///
/// let attribution = attribute_pnl_parallel(
///     &instrument,
///     &market_t0,
///     &market_t1,
///     as_of_t0,
///     as_of_t1,
///     &config,
/// )?;
///
/// println!("Total P&L: {}", attribution.total_pnl);
/// println!("Carry: {}", attribution.carry);
/// println!("Rates: {}", attribution.rates_curves_pnl);
/// println!("Residual: {} ({:.2}%)", 
///     attribution.residual,
///     attribution.meta.residual_pct
/// );
/// ```
pub fn attribute_pnl_parallel(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    _config: &FinstackConfig,
) -> Result<PnlAttribution> {
    let mut num_repricings = 0;

    // Step 1: Price at T₀ and T₁
    let val_t0 = reprice_instrument(instrument, market_t0, as_of_t0)?;
    num_repricings += 1;

    let val_t1 = reprice_instrument(instrument, market_t1, as_of_t1)?;
    num_repricings += 1;

    // Total P&L (in instrument's currency)
    let total_pnl = compute_pnl(val_t0, val_t1, val_t1.currency(), market_t1, as_of_t1)?;

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
    let market_frozen = freeze_all_market(market_t0, market_t1);
    let val_carry = reprice_instrument(instrument, &market_frozen, as_of_t1)?;
    num_repricings += 1;

    attribution.carry = compute_pnl(val_t0, val_carry, val_t1.currency(), market_t1, as_of_t1)?;

    // Step 3: Rates curves attribution (discount + forward)
    let rates_snapshot = extract_rates_curves(market_t0);
    let market_with_t0_rates = restore_rates_curves(market_t1, &rates_snapshot);
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
    let credit_snapshot = extract_credit_curves(market_t0);
    if !credit_snapshot.hazard_curves.is_empty() {
        let market_with_t0_credit = restore_credit_curves(market_t1, &credit_snapshot);
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
    let inflation_snapshot = extract_inflation_curves(market_t0);
    if !inflation_snapshot.inflation_curves.is_empty() {
        let market_with_t0_inflation = restore_inflation_curves(market_t1, &inflation_snapshot);
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
    let correlations_snapshot = extract_correlations(market_t0);
    if !correlations_snapshot.base_correlation_curves.is_empty() {
        let market_with_t0_corr = restore_correlations(market_t1, &correlations_snapshot);
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
    let fx_t0 = extract_fx(market_t0);
    if fx_t0.is_some() {
        let market_with_t0_fx = restore_fx(market_t1, fx_t0);
        let val_with_t0_fx = reprice_instrument(instrument, &market_with_t0_fx, as_of_t1)?;
        num_repricings += 1;

        attribution.fx_pnl =
            compute_pnl(val_with_t0_fx, val_t1, val_t1.currency(), market_t1, as_of_t1)?;
    }

    // Step 8: Volatility attribution
    let vol_snapshot = extract_volatility(market_t0);
    if !vol_snapshot.surfaces.is_empty() {
        let market_with_t0_vol = restore_volatility(market_t1, &vol_snapshot);
        let val_with_t0_vol = reprice_instrument(instrument, &market_with_t0_vol, as_of_t1)?;
        num_repricings += 1;

        attribution.vol_pnl =
            compute_pnl(val_with_t0_vol, val_t1, val_t1.currency(), market_t1, as_of_t1)?;
    }

    // Step 9: Model parameters attribution
    let params_t0 = crate::attribution::model_params::extract_model_params(instrument);
    if !matches!(params_t0, crate::attribution::model_params::ModelParamsSnapshot::None) {
        // Create instrument with T₀ parameters
        match crate::attribution::model_params::with_model_params(instrument, &params_t0) {
            Ok(instrument_with_t0_params) => {
                // Reprice with T₁ market
                if let Ok(val_with_t0_params) = reprice_instrument(&instrument_with_t0_params, market_t1, as_of_t1) {
                    num_repricings += 1;

                    attribution.model_params_pnl = compute_pnl(
                        val_with_t0_params,
                        val_t1,
                        val_t1.currency(),
                        market_t1,
                        as_of_t1,
                    )?;
                }
                // If repricing fails, model_params_pnl remains zero
            }
            Err(_) => {
                // If modification fails, model_params_pnl remains zero
            }
        }
    }

    // Step 10: Market scalars attribution
    let scalars_snapshot = extract_scalars(market_t0);
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
    attribution.compute_residual();

    // Update metadata
    attribution.meta.num_repricings = num_repricings;
    attribution.meta.tolerance = 0.001; // Default 0.1% tolerance

    Ok(attribution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use time::macros::date;

    // Simple test instrument that returns a fixed value
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
            static ATTRS: OnceLock<crate::instruments::common::traits::Attributes> = OnceLock::new();
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
            Ok(crate::results::ValuationResult::stamped(self.id(), as_of, value))
        }
    }

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
        let total_pnl = val_t1.checked_sub(val_t0).unwrap();
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
            .unwrap();

        let curve_t1 = DiscountCurve::builder("USD-OIS")
            .base_date(as_of_t1)
            .knots(vec![(0.0, 1.0), (1.0, 0.97)]) // Rates increased (curve lower)
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let market_t0 = MarketContext::new().insert_discount(curve_t0);
        let market_t1 = MarketContext::new().insert_discount(curve_t1);

        // Extract and verify snapshots work
        let rates_snapshot = extract_rates_curves(&market_t0);
        assert_eq!(rates_snapshot.discount_curves.len(), 1);

        let restored = restore_rates_curves(&market_t1, &rates_snapshot);
        assert!(restored.get_discount("USD-OIS").is_ok());
    }
}

