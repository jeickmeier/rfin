//! Utilities for instrument pricing and metrics assembly.
//!
//! Contains helpers shared across instrument implementations, notably the
//! function to assemble a `ValuationResult` with computed metrics.

use crate::metrics::risk::MarketHistory;
use crate::metrics::{standard_registry, MetricContext, MetricId};
use finstack_core::config::{results_meta_now, FinstackConfig};
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::{context::MarketContext, scalars::MarketScalar};
use finstack_core::money::Money;
use indexmap::IndexMap;
use std::sync::Arc;

/// Schedule → PV helper that uses the curve's own day count convention.
///
/// This variant ensures consistency between:
/// - Metric calculations (e.g., par rate using `df_on_date_curve`)
/// - NPV calculations
///
/// **Use this when pricing at par rate should yield zero PV** (e.g., deposits, FRAs).
///
/// # Arguments
///
/// * `instrument` - The instrument providing cashflows
/// * `curves` - Market data context
/// * `as_of` - Valuation date
/// * `discount_curve_id` - ID of the discount curve to use
pub fn schedule_pv_using_curve_dc<S>(
    instrument: &S,
    curves: &MarketContext,
    as_of: Date,
    discount_curve_id: &finstack_core::types::CurveId,
) -> finstack_core::Result<Money>
where
    S: crate::cashflow::traits::CashflowProvider,
{
    use finstack_core::cashflow::npv_using_curve_dc;

    let flows = S::build_dated_flows(instrument, curves, as_of)?;
    let disc = curves.get_discount(discount_curve_id.as_str())?;
    // Use the curve's day count for consistent pricing with metrics
    npv_using_curve_dc(disc.as_ref(), as_of, &flows)
}

/// Schedule → PV helper that uses the curve's own day count convention (raw f64).
///
/// Returns unrounded NPV for high-precision calibration/risk.
///
/// # Numerical Stability
///
/// Uses Neumaier compensated summation instead of Kahan summation because
/// cashflow schedules often contain mixed-sign values (positive inflows and
/// negative outflows). Neumaier's algorithm handles cases where the sum and
/// the next value have similar magnitudes but opposite signs better than Kahan.
///
/// Reference: Neumaier, A. (1974). "Rundungsfehleranalyse einiger Verfahren
/// zur Summation endlicher Summen." *ZAMM*, 54(1), 39-51.
pub fn schedule_pv_using_curve_dc_raw<S>(
    instrument: &S,
    curves: &MarketContext,
    as_of: Date,
    discount_curve_id: &finstack_core::types::CurveId,
) -> finstack_core::Result<f64>
where
    S: crate::cashflow::traits::CashflowProvider,
{
    use finstack_core::dates::DayCountCtx;
    use finstack_core::math::neumaier_sum;

    let flows = S::build_dated_flows(instrument, curves, as_of)?;
    let disc = curves.get_discount(discount_curve_id.as_str())?;

    let mut terms = Vec::with_capacity(flows.len());
    let dc = disc.day_count();

    for (date, amount) in flows {
        // Include cashflows that occur exactly on `as_of` (t=0, df=1).
        // Skipping them can break calibration bracketing for instruments that settle on `as_of`
        // (e.g. T+0 deposits), because the initial exchange is incorrectly dropped.
        if date < as_of {
            continue;
        }
        // Use relative time from as_of (T+0)
        let t = dc.year_fraction(as_of, date, DayCountCtx::default())?;
        let df = disc.df(t);
        terms.push(amount.amount() * df);
    }

    Ok(neumaier_sum(terms))
}

/// Shared helper to build a ValuationResult with a set of metrics.
///
/// Centralizes the repeated pattern across instruments to compute base value,
/// build metric context, compute metrics and stamp a result.
///
/// This function uses trait objects to avoid generic monomorphization across
/// compilation units, which can cause coverage metadata mismatches.
///
/// # Arguments
///
/// * `instrument` - The instrument to price (wrapped in Arc for efficiency)
/// * `curves` - Market data context (wrapped in Arc for efficiency)
/// * `as_of` - Valuation date
/// * `base_value` - Pre-computed base value (NPV)
/// * `metrics` - List of metrics to compute
/// * `cfg` - Optional FinstackConfig for user-tunable metric defaults (e.g., bump sizes).
///   When `None`, uses global defaults.
///
/// # Performance
///
/// Accepts Arc-wrapped arguments to avoid cloning on every call. Callers should
/// clone the instrument and market context once into Arc at the call boundary.
pub fn build_with_metrics_dyn(
    instrument: Arc<dyn crate::instruments::common::traits::Instrument>,
    curves: Arc<MarketContext>,
    as_of: Date,
    base_value: Money,
    metrics: &[crate::metrics::MetricId],
    cfg: Option<Arc<FinstackConfig>>,
) -> finstack_core::Result<crate::results::ValuationResult> {
    let finstack_config = cfg.unwrap_or_else(MetricContext::default_config);
    let mut context = MetricContext::new(
        instrument.clone(),
        curves,
        as_of,
        base_value,
        finstack_config,
    );
    // Preserve per-instrument pricing overrides (e.g., bump sizes, scenario shocks) for metrics.
    context.set_pricing_overrides(instrument.scenario_overrides().cloned());

    let registry = standard_registry();
    let metric_measures = registry.compute(metrics, &mut context)?;

    // Pre-allocate capacity to avoid reallocations during insertion.
    // Estimate: requested metrics + a few extras from composite keys.
    let mut measures: IndexMap<MetricId, f64> = IndexMap::with_capacity(metrics.len() + 4);

    // Deterministic insertion order: follow the requested metrics slice order
    for metric_id in metrics {
        if let Some(value) = metric_measures.get(metric_id) {
            measures.insert(metric_id.clone(), *value);
        }
    }

    // Include any composite keys (bucketed series, matrices, tensors, etc.) that were stored into
    // `context.computed` during calculation.
    //
    // IMPORTANT:
    // - We only include *custom* (composite) metric IDs to avoid leaking dependency metrics that
    //   were computed internally but not requested by the caller.
    // - We insert in a stable order (sorted by key) to ensure deterministic results.
    let mut extras: Vec<(&crate::metrics::MetricId, f64)> = context
        .computed
        .iter()
        .filter_map(|(metric_id, value)| {
            if metric_id.is_custom() && !measures.contains_key(metric_id) {
                Some((metric_id, *value))
            } else {
                None
            }
        })
        .collect();
    extras.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));
    for (metric_id, value) in extras {
        measures.insert(metric_id.clone(), value);
    }

    let meta = results_meta_now(context.config());
    let mut result = crate::results::ValuationResult::stamped_with_meta(
        context.instrument.id(),
        as_of,
        base_value,
        meta,
    );
    result.measures = measures;

    Ok(result)
}

/// Variant of [`build_with_metrics_dyn`] that attaches a [`MarketHistory`] to the metric context.
///
/// This enables Historical VaR / Expected Shortfall metrics to run without storing
/// type-erased runtime data inside `finstack_core::MarketContext`.
pub fn build_with_metrics_dyn_with_market_history(
    instrument: Arc<dyn crate::instruments::common::traits::Instrument>,
    curves: Arc<MarketContext>,
    as_of: Date,
    base_value: Money,
    metrics: &[crate::metrics::MetricId],
    cfg: Option<Arc<FinstackConfig>>,
    market_history: Arc<MarketHistory>,
) -> finstack_core::Result<crate::results::ValuationResult> {
    let finstack_config = cfg.unwrap_or_else(MetricContext::default_config);
    let mut context = MetricContext::new(
        instrument.clone(),
        curves,
        as_of,
        base_value,
        finstack_config,
    )
    .with_market_history(Arc::clone(&market_history));
    context.set_pricing_overrides(instrument.scenario_overrides().cloned());

    let registry = standard_registry();
    let metric_measures = registry.compute(metrics, &mut context)?;

    let mut measures: IndexMap<MetricId, f64> = IndexMap::with_capacity(metrics.len() + 4);
    for metric_id in metrics {
        if let Some(value) = metric_measures.get(metric_id) {
            measures.insert(metric_id.clone(), *value);
        }
    }

    let mut extras: Vec<(&crate::metrics::MetricId, f64)> = context
        .computed
        .iter()
        .filter_map(|(metric_id, value)| {
            if metric_id.is_custom() && !measures.contains_key(metric_id) {
                Some((metric_id, *value))
            } else {
                None
            }
        })
        .collect();
    extras.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));
    for (metric_id, value) in extras {
        measures.insert(metric_id.clone(), value);
    }

    let meta = results_meta_now(context.config());
    let mut result = crate::results::ValuationResult::stamped_with_meta(
        context.instrument.id(),
        as_of,
        base_value,
        meta,
    );
    result.measures = measures;

    Ok(result)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::{Attributes, Instrument};
    use crate::metrics::MetricId;
    use crate::pricer::InstrumentType;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use std::any::Any;
    use std::sync::Arc;
    use time::macros::date;

    #[derive(Clone)]
    struct StubInstrument {
        id: String,
        attrs: Attributes,
    }

    impl StubInstrument {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                attrs: Attributes::default(),
            }
        }
    }

    impl Instrument for StubInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> InstrumentType {
            InstrumentType::Bond
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn value(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
            Ok(Money::new(123.45, Currency::USD))
        }

        fn attributes(&self) -> &Attributes {
            &self.attrs
        }

        fn attributes_mut(&mut self) -> &mut Attributes {
            &mut self.attrs
        }

        fn clone_box(&self) -> Box<dyn Instrument> {
            Box::new(self.clone())
        }

        fn price_with_metrics(
            &self,
            market: &MarketContext,
            as_of: Date,
            metrics: &[MetricId],
        ) -> finstack_core::Result<crate::results::ValuationResult> {
            let base = self.value(market, as_of)?;
            build_with_metrics_dyn(
                Arc::from(self.clone_box()),
                Arc::new(market.clone()),
                as_of,
                base,
                metrics,
                None,
            )
        }
    }

    #[test]
    fn stamped_result_uses_provided_config() -> finstack_core::Result<()> {
        let instrument = Arc::new(StubInstrument::new("STUB"));
        let market = Arc::new(MarketContext::new());
        let as_of = date!(2024 - 01 - 01);
        let base_value = Money::new(10.0, Currency::USD);

        let mut cfg = FinstackConfig::default();
        // Set a non-default output scale to verify it is propagated into meta
        cfg.rounding.output_scale.overrides.insert(Currency::USD, 4);
        let cfg = Arc::new(cfg);

        let result = build_with_metrics_dyn(
            instrument,
            market,
            as_of,
            base_value,
            &[],
            Some(cfg.clone()),
        )?;

        let usd_scale = result
            .meta
            .rounding
            .output_scale_by_ccy
            .get(&Currency::USD)
            .copied();
        assert_eq!(usd_scale, Some(4), "meta should reflect provided config");
        Ok(())
    }
}

/// Price an instrument with metrics using pre-allocated Arc references.
///
/// This is an optimized version of the `Instrument::price_with_metrics` pattern
/// that avoids cloning when the caller already holds Arc-wrapped references.
///
/// # Performance
///
/// Use this function in portfolio valuation loops or batch processing where
/// the same instrument/market context is reused across multiple calculations.
/// The Arc wrapping is done once at the loop boundary rather than per-call,
/// avoiding the cost of `Arc::new(self.clone())` on every iteration.
///
/// # Arguments
///
/// * `instrument` - Arc-wrapped instrument (caller provides, avoids clone)
/// * `market` - Arc-wrapped market context (caller provides, avoids clone)
/// * `as_of` - Valuation date
/// * `metrics` - List of metrics to compute
/// * `cfg` - Optional FinstackConfig for metric defaults
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
///
/// // Wrap once at loop boundary
/// let instrument_arc: Arc<dyn Instrument> = Arc::new(bond.clone());
/// let market_arc = Arc::new(market.clone());
///
/// // Process multiple metrics without re-cloning
/// for metric_set in &[vec![MetricId::Dv01], vec![MetricId::Ytm]] {
///     let result = price_with_metrics_from_arcs(
///         instrument_arc.clone(), // Arc clone is cheap (just ref count)
///         market_arc.clone(),
///         as_of,
///         metric_set,
///         None,
///     )?;
/// }
/// ```
pub fn price_with_metrics_from_arcs(
    instrument: Arc<dyn crate::instruments::common::traits::Instrument>,
    market: Arc<MarketContext>,
    as_of: Date,
    metrics: &[crate::metrics::MetricId],
    cfg: Option<Arc<FinstackConfig>>,
) -> finstack_core::Result<crate::results::ValuationResult> {
    let base_value = instrument.value(&market, as_of)?;
    build_with_metrics_dyn(instrument, market, as_of, base_value, metrics, cfg)
}

/// Convert a trait object reference to Arc-wrapped trait object.
///
/// This helper clones the instrument via `clone_box()` and converts it to Arc.
/// Used by language bindings (Python/WASM) that work with trait object references.
///
/// # Implementation
///
/// Uses `clone_box()` to get a `Box<dyn Instrument>`, then converts it to `Arc`
/// using `Arc::from()`. This works because `Arc::from()` can convert from `Box<T>`
/// when `T: ?Sized` (which trait objects are).
pub fn instrument_to_arc(
    instrument: &dyn crate::instruments::common::traits::Instrument,
) -> Arc<dyn crate::instruments::common::traits::Instrument> {
    // Clone via clone_box() to get Box<dyn Instrument>
    let boxed = instrument.clone_box();
    // Convert Box to Arc using Arc::from()
    // This works because Arc::from() can convert Box<T> to Arc<T> for any T
    Arc::from(boxed)
}

/// Ensure all money amounts in a collection share the same currency.
pub fn validate_currency_consistency(amounts: &[Money]) -> finstack_core::Result<()> {
    if amounts.is_empty() {
        return Ok(());
    }

    let expected_currency = amounts[0].currency();
    for amount in amounts.iter().skip(1) {
        if amount.currency() != expected_currency {
            return Err(finstack_core::Error::CurrencyMismatch {
                expected: expected_currency,
                actual: amount.currency(),
            });
        }
    }
    Ok(())
}

/// Collect standard Black-Scholes inputs (spot, r, q, sigma, t) from market context.
///
/// Retrieves and calculates the 5 standard parameters required for Black-Scholes pricing:
/// - Spot price (S)
/// - Risk-free rate (r) for the period to expiry
/// - Dividend/Continuous yield (q)
/// - Volatility (sigma) at strike/maturity
/// - Time to expiry (t) in years
///
/// # Day Count Convention Handling
///
/// **Important**: This function correctly separates the day count bases:
///
/// - **Discounting (t_disc)**: Uses the discount curve's own day count (`disc_curve.day_count()`).
///   This is used to calculate the zero rate `r` and ensures proper discount factor calculation
///   regardless of instrument or volatility conventions.
///
/// - **Volatility lookup (t_vol)**: Uses the instrument's `day_count` parameter, which should
///   match how the volatility surface was calibrated (typically ACT/365F for equity options).
///   This time is used for vol surface interpolation and returned as the primary `t` output.
///
/// This separation is critical for barrier options and other path-dependent derivatives:
/// - Mixing bases would bias barrier crossing probabilities
/// - Monte Carlo time stepping should use the vol surface basis
/// - Rebate PVs require consistent discounting
///
/// # Arguments
///
/// * `spot_id` - ID of the spot price scalar
/// * `discount_curve_id` - ID of the discount curve
/// * `div_yield_id` - Optional ID of the dividend yield scalar (defaults to 0.0 if None)
/// * `vol_surface_id` - ID of the volatility surface
/// * `strike` - Strike price for volatility lookup
/// * `expiry` - Expiry date
/// * `day_count` - Day count convention for vol surface time calculation (should match vol surface calibration basis)
/// * `curves` - Market data context
/// * `as_of` - Valuation date
///
/// # Returns
///
/// A tuple `(spot, r, q, sigma, t)` where:
/// - `spot`: Current spot price
/// - `r`: Continuously compounded risk-free rate (calculated using disc curve's day count)
/// - `q`: Dividend yield (0.0 if not provided)
/// - `sigma`: Implied volatility from the vol surface at (t_vol, strike)
/// - `t`: Time to expiry using the vol surface day count basis (t_vol)
#[allow(clippy::too_many_arguments)]
pub fn collect_black_scholes_inputs(
    spot_id: &str,
    discount_curve_id: &finstack_core::types::CurveId,
    div_yield_id: Option<&finstack_core::types::CurveId>, // Changed to match Instrument fields often being CurveId or String
    vol_surface_id: &str,
    strike: f64,
    expiry: Date,
    day_count: DayCount,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64)> {
    // Get discount curve first to access its day count
    let disc_curve = curves.get_discount(discount_curve_id.as_str())?;

    // Time to expiry for vol surface lookup (using instrument's day count, which should
    // match how the vol surface was calibrated - typically ACT/365F for equity options)
    let t_vol = day_count.year_fraction(as_of, expiry, DayCountCtx::default())?;

    // Time to expiry for discounting (using the discount curve's own day count basis)
    // This ensures proper DF calculation regardless of instrument/vol conventions
    let t_disc = disc_curve
        .day_count()
        .year_fraction(as_of, expiry, DayCountCtx::default())?;

    // Risk-free rate (r) using the discount curve's time basis
    let r = disc_curve.zero(t_disc);

    // Spot price (S)
    let spot_scalar = curves.price(spot_id)?;
    let spot = match spot_scalar {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(m) => m.amount(),
    };

    // Dividend yield (q)
    //
    // When a dividend yield ID is explicitly provided, we require the lookup to succeed
    // and return a unitless scalar. Silent fallback to 0.0 would mask market data
    // configuration errors.
    let q = if let Some(div_id) = div_yield_id {
        let ms = curves.price(div_id.as_str()).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "Failed to fetch dividend yield '{}': {}",
                div_id, e
            ))
        })?;
        match ms {
            MarketScalar::Unitless(v) => *v,
            MarketScalar::Price(m) => {
                return Err(finstack_core::Error::Validation(format!(
                    "Dividend yield '{}' should be a unitless scalar, got Price({})",
                    div_id,
                    m.currency()
                )));
            }
        }
    } else {
        0.0
    };

    // Volatility (sigma) using vol surface's time basis
    let vol_surface = curves.surface(vol_surface_id)?;
    let sigma = vol_surface.value_clamped(t_vol, strike);

    // Return the vol-surface time as 't' (for backward compatibility with callers
    // expecting t to be used for things like Monte Carlo time stepping, which should
    // align with the vol surface basis)
    Ok((spot, r, q, sigma, t_vol))
}
