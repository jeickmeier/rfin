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

use super::factors::*;
use super::helpers::*;
use super::model_params;
use super::types::*;
use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::sensitivities::theta::collect_cashflows_in_period;
use finstack_core::config::FinstackConfig;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use indexmap::IndexMap;
use std::sync::Arc;

fn cross_interaction_pnl(
    val_t1: Money,
    val_with_t0_a: Money,
    val_with_t0_b: Money,
    val_with_t0_ab: Money,
) -> Result<Money> {
    val_t1
        .checked_sub(val_with_t0_a)?
        .checked_sub(val_with_t0_b)?
        .checked_add(val_with_t0_ab)
}

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
#[tracing::instrument(skip_all, fields(instrument_id = %instrument.id(), method = "parallel"))]
pub fn attribute_pnl_parallel(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    _config: &FinstackConfig,
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
        model_params::with_model_params(instrument, params)?
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

    // Initialize attribution result with configured rounding context
    let rounding = finstack_core::config::rounding_context_from(_config);
    let mut attribution = PnlAttribution::new_with_rounding(
        total_pnl,
        instrument.id(),
        as_of_t0,
        as_of_t1,
        AttributionMethod::Parallel,
        rounding,
    );

    let mut val_with_t0_credit = None;
    let mut val_with_t0_fx = None;
    let mut val_with_t0_vol = None;
    let mut val_with_t0_scalars = None;

    // Step 2: Carry attribution (time decay + accruals + roll-down)
    //
    // METHODOLOGY: Price at T₁ date with T₀ market (frozen curves).
    // This captures the combined effect of:
    //   - Theta (pure time decay): coupon accrual, option decay, funding cost
    //   - Roll-down: benefit from moving down a positively-sloped curve
    //
    // These sub-components are separated in metrics-based attribution (where
    // Theta is pre-computed), but in parallel attribution the total carry
    // is reported. Use `carry_detail` for the decomposition when available.
    //
    // FX CONVENTION: Both T₀ and carry values are converted at T₁ FX rates
    // (via `compute_pnl`). This isolates the pricing effect of time passage
    // from FX translation effects. The FX factor (Step 7) captures all
    // translation P&L, ensuring consistent summation.
    let market_frozen = market_t0.clone();
    let val_carry = reprice_instrument(instrument, &market_frozen, as_of_t1)?;
    num_repricings += 1;

    attribution.carry = compute_pnl(val_t0, val_carry, val_t1.currency(), market_t1, as_of_t1)?;
    let coupon_income = collect_cashflows_in_period(
        instrument.as_ref(),
        &market_frozen,
        as_of_t0,
        as_of_t1,
        val_t1.currency(),
    )
    .ok()
    .map(|value| Money::new(value, val_t1.currency()));
    attribution.carry_detail = Some(CarryDetail {
        total: attribution.carry,
        coupon_income,
        pull_to_par: None,
        roll_down: None,
        funding_cost: None,
        theta: Some(attribution.carry),
    });

    // Step 3: Rates curves attribution (discount + forward)
    let rates_snapshot = MarketSnapshot::extract(market_t0, CurveRestoreFlags::RATES);
    let market_with_t0_rates =
        MarketSnapshot::restore_market(market_t1, &rates_snapshot, CurveRestoreFlags::RATES);
    let rates_reprice = reprice_instrument(instrument, &market_with_t0_rates, as_of_t1)?;
    num_repricings += 1;
    let val_with_t0_rates = rates_reprice;

    // Rates P&L = impact of moving from T₀ rates to T₁ rates
    // val_t1 (with T₁ rates) - val_with_t0_rates (with T₀ rates)
    attribution.rates_curves_pnl = compute_pnl(
        rates_reprice,
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
        let credit_reprice = reprice_instrument(instrument, &market_with_t0_credit, as_of_t1)?;
        num_repricings += 1;
        val_with_t0_credit = Some(credit_reprice);

        attribution.credit_curves_pnl = compute_pnl(
            credit_reprice,
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
    //
    // FX P&L has two conceptually distinct components that are COMBINED in this attribution:
    //
    // 1. **FX EXPOSURE P&L** (Pricing Impact):
    //    Impact of FX rate changes on instrument pricing for cross-currency products.
    //    Example: Cross-currency swap pricing depends on FX rates for basis adjustment.
    //    This is isolated by repricing with T₀ FX rates but T₁ curves.
    //
    // 2. **FX TRANSLATION P&L** (Reporting Impact):
    //    Impact of converting instrument PV from native currency to reporting currency.
    //    Example: EUR-denominated bond held by USD reporter. If EUR/USD moves, the
    //    reported USD value changes even if EUR PV is unchanged.
    //    This is captured by using date-appropriate FX rates for conversion.
    //
    // FX CONVERSION METHODOLOGY:
    //   - Non-FX factors (carry, rates, credit, vol, etc.) use `compute_pnl` which
    //     converts BOTH T₀ and T₁ values at T₁ FX. This isolates pricing effects.
    //   - Total P&L uses `compute_pnl_with_fx` which converts T₀ at T₀ FX and T₁
    //     at T₁ FX (actual P&L).
    //   - The FX factor uses `compute_pnl_with_fx` to capture both exposure and
    //     translation effects, absorbing the V₀×(X₁-X₀) translation term that
    //     the non-FX factors exclude.
    //   - Any residual cross-terms (ΔV×ΔX) are inherent in the parallel approach
    //     and appear in the residual by design.
    //
    // For single-currency instruments in their native reporting currency, this step
    // produces zero P&L as expected.
    let fx_t0 = extract_fx(market_t0);
    if fx_t0.is_some() {
        let market_with_t0_fx = restore_fx(market_t1, fx_t0.clone());
        let fx_reprice = reprice_instrument(instrument, &market_with_t0_fx, as_of_t1)?;
        num_repricings += 1;
        val_with_t0_fx = Some(fx_reprice);

        // Compute combined FX P&L (exposure + translation)
        // Uses T₀ FX for converting T₀ PV and T₁ FX for converting T₁ PV
        attribution.fx_pnl = compute_pnl_with_fx(
            fx_reprice,
            val_t1,
            val_t1.currency(),
            market_t0,
            market_t1,
            as_of_t0,
            as_of_t1,
        )?;

        // Stamp FX policy metadata for audit trail
        attribution.meta.fx_policy = Some(finstack_core::money::fx::FxPolicyMeta {
            strategy: finstack_core::money::fx::FxConversionPolicy::CashflowDate,
            target_ccy: Some(val_t1.currency()),
            notes: "Combined FX exposure and translation P&L (see parallel.rs for details)"
                .to_string(),
        });
    }

    // Step 8: Volatility attribution
    let vol_snapshot = VolatilitySnapshot::extract(market_t0);
    if !vol_snapshot.surfaces.is_empty() {
        let market_with_t0_vol = restore_volatility(market_t1, &vol_snapshot);
        let vol_reprice = reprice_instrument(instrument, &market_with_t0_vol, as_of_t1)?;
        num_repricings += 1;
        val_with_t0_vol = Some(vol_reprice);

        attribution.vol_pnl =
            compute_pnl(vol_reprice, val_t1, val_t1.currency(), market_t1, as_of_t1)?;
    }

    // Step 9: Model parameters attribution
    let params_t0 = model_params_t0
        .cloned()
        .unwrap_or_else(|| model_params::extract_model_params(instrument));
    if !matches!(params_t0, model_params::ModelParamsSnapshot::None) {
        // Create instrument with T₀ parameters
        match model_params::with_model_params(instrument, &params_t0) {
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
        let scalars_reprice = reprice_instrument(instrument, &market_with_t0_scalars, as_of_t1)?;
        num_repricings += 1;
        val_with_t0_scalars = Some(scalars_reprice);

        attribution.market_scalars_pnl = compute_pnl(
            scalars_reprice,
            val_t1,
            val_t1.currency(),
            market_t1,
            as_of_t1,
        )?;
    }

    // Step 10b: Explicit cross-factor interaction repricings.
    let mut cross_total = 0.0;
    let mut cross_by_pair = IndexMap::new();

    if let Some(credit_reprice) = val_with_t0_credit {
        let market_with_t0_rates_credit = MarketSnapshot::restore_market(
            &market_with_t0_rates,
            &credit_snapshot,
            CurveRestoreFlags::CREDIT,
        );
        let val_with_t0_rates_credit =
            reprice_instrument(instrument, &market_with_t0_rates_credit, as_of_t1)?;
        num_repricings += 1;

        let pnl = cross_interaction_pnl(
            val_t1,
            val_with_t0_rates,
            credit_reprice,
            val_with_t0_rates_credit,
        )?;
        if pnl.amount().abs() > 1e-12 {
            cross_total += pnl.amount();
            cross_by_pair.insert("Rates×Credit".to_string(), pnl);
        }
    }

    if let Some(vol_reprice) = val_with_t0_vol {
        let market_with_t0_rates_vol = restore_volatility(&market_with_t0_rates, &vol_snapshot);
        let val_with_t0_rates_vol =
            reprice_instrument(instrument, &market_with_t0_rates_vol, as_of_t1)?;
        num_repricings += 1;

        let pnl = cross_interaction_pnl(
            val_t1,
            val_with_t0_rates,
            vol_reprice,
            val_with_t0_rates_vol,
        )?;
        if pnl.amount().abs() > 1e-12 {
            cross_total += pnl.amount();
            cross_by_pair.insert("Rates×Vol".to_string(), pnl);
        }
    }

    if let (Some(scalars_reprice), Some(vol_reprice)) = (val_with_t0_scalars, val_with_t0_vol) {
        let market_with_t0_spot_vol = restore_volatility(
            &restore_scalars(market_t1, &scalars_snapshot),
            &vol_snapshot,
        );
        let val_with_t0_spot_vol =
            reprice_instrument(instrument, &market_with_t0_spot_vol, as_of_t1)?;
        num_repricings += 1;

        let pnl =
            cross_interaction_pnl(val_t1, scalars_reprice, vol_reprice, val_with_t0_spot_vol)?;
        if pnl.amount().abs() > 1e-12 {
            cross_total += pnl.amount();
            cross_by_pair.insert("Spot×Vol".to_string(), pnl);
        }
    }

    if let (Some(scalars_reprice), Some(credit_reprice)) = (val_with_t0_scalars, val_with_t0_credit)
    {
        let market_with_t0_spot_credit = restore_scalars(
            &MarketSnapshot::restore_market(market_t1, &credit_snapshot, CurveRestoreFlags::CREDIT),
            &scalars_snapshot,
        );
        let val_with_t0_spot_credit =
            reprice_instrument(instrument, &market_with_t0_spot_credit, as_of_t1)?;
        num_repricings += 1;

        let pnl = cross_interaction_pnl(
            val_t1,
            scalars_reprice,
            credit_reprice,
            val_with_t0_spot_credit,
        )?;
        if pnl.amount().abs() > 1e-12 {
            cross_total += pnl.amount();
            cross_by_pair.insert("Spot×Credit".to_string(), pnl);
        }
    }

    if let (Some(fx_reprice), Some(vol_reprice)) = (val_with_t0_fx, val_with_t0_vol) {
        let market_with_t0_fx_vol =
            restore_volatility(&restore_fx(market_t1, fx_t0.clone()), &vol_snapshot);
        let val_with_t0_fx_vol = reprice_instrument(instrument, &market_with_t0_fx_vol, as_of_t1)?;
        num_repricings += 1;

        let pnl = cross_interaction_pnl(val_t1, fx_reprice, vol_reprice, val_with_t0_fx_vol)?;
        if pnl.amount().abs() > 1e-12 {
            cross_total += pnl.amount();
            cross_by_pair.insert("FX×Vol".to_string(), pnl);
        }
    }

    if let Some(fx_reprice) = val_with_t0_fx {
        let market_with_t0_fx_rates = MarketSnapshot::restore_market(
            &restore_fx(market_t1, fx_t0),
            &rates_snapshot,
            CurveRestoreFlags::RATES,
        );
        let val_with_t0_fx_rates =
            reprice_instrument(instrument, &market_with_t0_fx_rates, as_of_t1)?;
        num_repricings += 1;

        let pnl =
            cross_interaction_pnl(val_t1, fx_reprice, val_with_t0_rates, val_with_t0_fx_rates)?;
        if pnl.amount().abs() > 1e-12 {
            cross_total += pnl.amount();
            cross_by_pair.insert("FX×Rates".to_string(), pnl);
        }
    }

    if !cross_by_pair.is_empty() {
        attribution.cross_factor_pnl = Money::new(cross_total, val_t1.currency());
        attribution.cross_factor_detail = Some(CrossFactorDetail {
            total: attribution.cross_factor_pnl,
            by_pair: cross_by_pair,
        });
    }

    // Step 11: Compute residual
    if let Err(e) = attribution.compute_residual() {
        tracing::warn!(
            error = %e,
            instrument_id = %instrument.id(),
            "Residual computation failed; attribution may be incomplete"
        );
    }

    // Update metadata
    attribution.meta.num_repricings = num_repricings;
    attribution.meta.tolerance_abs = 1.0;
    attribution.meta.tolerance_pct = 0.1;

    Ok(attribution)
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
    use finstack_core::dates::Date;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::HazardCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use std::sync::OnceLock;
    use test_utils::TestInstrument;
    use time::macros::date;

    #[derive(Clone)]
    struct RatesCreditInteractionInstrument {
        id: String,
    }

    impl RatesCreditInteractionInstrument {
        fn new(id: &str) -> Self {
            Self { id: id.to_string() }
        }
    }

    impl Instrument for RatesCreditInteractionInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> crate::pricer::InstrumentType {
            crate::pricer::InstrumentType::Bond
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
            static ATTRS: OnceLock<crate::instruments::common_impl::traits::Attributes> =
                OnceLock::new();
            ATTRS.get_or_init(crate::instruments::common_impl::traits::Attributes::default)
        }

        fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
            unreachable!("RatesCreditInteractionInstrument::attributes_mut should not be called")
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn market_dependencies(
            &self,
        ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
        {
            let mut deps = crate::instruments::common_impl::dependencies::MarketDependencies::new();
            deps.add_curves(
                crate::instruments::common_impl::traits::InstrumentCurves::builder()
                    .discount(finstack_core::types::CurveId::new("USD-OIS"))
                    .credit(finstack_core::types::CurveId::new("ACME-HAZ"))
                    .build()?,
            );
            Ok(deps)
        }

        fn value(&self, market: &MarketContext, _as_of: Date) -> Result<Money> {
            let rate = market.get_discount("USD-OIS")?.zero(1.0);
            let hazard = market.get_hazard("ACME-HAZ")?.hazard_rate(1.0);
            Ok(Money::new(1_000_000.0 * rate * hazard, Currency::USD))
        }

        fn price_with_metrics(
            &self,
            market: &MarketContext,
            as_of: Date,
            _metrics: &[crate::metrics::MetricId],
        ) -> Result<crate::results::ValuationResult> {
            Ok(crate::results::ValuationResult::stamped(
                self.id(),
                as_of,
                self.value(market, as_of)?,
            ))
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
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let curve_t1 = DiscountCurve::builder("USD-OIS")
            .base_date(as_of_t1)
            .knots(vec![(0.0, 1.0), (1.0, 0.97)]) // Rates increased (curve lower)
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let market_t0 = MarketContext::new().insert(curve_t0);
        let market_t1 = MarketContext::new().insert(curve_t1);

        // Extract and verify snapshots work
        let rates_snapshot = MarketSnapshot::extract(&market_t0, CurveRestoreFlags::RATES);
        assert_eq!(rates_snapshot.discount_curves.len(), 1);

        let restored =
            MarketSnapshot::restore_market(&market_t1, &rates_snapshot, CurveRestoreFlags::RATES);
        assert!(restored.get_discount("USD-OIS").is_ok());
    }

    #[test]
    fn test_parallel_attribution_extracts_rates_credit_cross_factor() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);
        let config = FinstackConfig::default();
        let instrument: Arc<dyn Instrument> =
            Arc::new(RatesCreditInteractionInstrument::new("TEST-RATES-CREDIT"));

        let market_t0 = MarketContext::new()
            .insert(
                DiscountCurve::builder("USD-OIS")
                    .base_date(as_of_t0)
                    .knots(vec![(0.0, 1.0), (1.0, 0.99)])
                    .interp(InterpStyle::Linear)
                    .build()
                    .expect("discount curve should build"),
            )
            .insert(
                HazardCurve::builder("ACME-HAZ")
                    .base_date(as_of_t0)
                    .knots(vec![(1.0, 0.01)])
                    .build()
                    .expect("hazard curve should build"),
            );

        let market_t1 = MarketContext::new()
            .insert(
                DiscountCurve::builder("USD-OIS")
                    .base_date(as_of_t1)
                    .knots(vec![(0.0, 1.0), (1.0, 0.98)])
                    .interp(InterpStyle::Linear)
                    .build()
                    .expect("discount curve should build"),
            )
            .insert(
                HazardCurve::builder("ACME-HAZ")
                    .base_date(as_of_t1)
                    .knots(vec![(1.0, 0.02)])
                    .build()
                    .expect("hazard curve should build"),
            );

        let attribution = attribute_pnl_parallel(
            &instrument,
            &market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
            &config,
            None,
        )
        .expect("parallel attribution should succeed");

        assert!(attribution.cross_factor_pnl.amount().abs() > 0.0);
        let detail = attribution
            .cross_factor_detail
            .expect("cross factor detail should be populated");
        assert!(
            detail
                .by_pair
                .get("Rates×Credit")
                .expect("rates-credit entry")
                .amount()
                .abs()
                > 0.0
        );
    }
}
