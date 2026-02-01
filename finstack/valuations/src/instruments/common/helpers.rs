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
    use finstack_core::cashflow::npv;

    let flows = S::build_dated_flows(instrument, curves, as_of)?;
    let disc = curves.get_discount(discount_curve_id.as_str())?;
    // Use None to use the curve's day count for consistent pricing with metrics
    npv(disc.as_ref(), as_of, None, &flows)
}

/// Schedule → PV helper that uses the curve's own day count convention (raw f64).
///
/// Returns unrounded NPV for high-precision calibration/risk.
///
/// # Cashflow-on-as_of Policy: PRICING-VIEW (Includes `date == as_of`)
///
/// This helper uses **pricing-view** semantics:
/// - Cashflows where `date < as_of` are excluded (truly past)
/// - Cashflows where `date == as_of` are **included** at DF=1 (t=0)
/// - Future cashflows (`date > as_of`) are discounted
///
/// This is critical for:
/// - **Calibration instruments**: T+0 deposits require initial exchange for bracketing
/// - **FRAs and same-day settling instruments**: Payment on as_of is part of value
///
/// For holder-view semantics (excludes `date <= as_of`), see
/// [`crate::instruments::common_impl::discountable::npv_by_date`].
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
        // PRICING-VIEW: Include cashflows on `as_of` (t=0, df=1).
        // Only exclude truly past cashflows (date < as_of).
        // This ensures calibration bracketing works for T+0 instruments.
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
/// * `market_history` - Optional market history for Historical VaR / Expected Shortfall metrics.
///   When `None`, these metrics will not be available.
///
/// # Performance
///
/// Accepts Arc-wrapped arguments to avoid cloning on every call. Callers should
/// clone the instrument and market context once into Arc at the call boundary.
///
/// # Thread Safety
///
/// The `curves` parameter is wrapped in `Arc` for efficiency, not thread synchronization.
/// Callers must ensure the market context is not mutated concurrently. For multi-threaded
/// pricing with market data updates, create a new `MarketContext` snapshot for each
/// pricing batch.
///
/// The `instrument` parameter is also `Arc`-wrapped. Instruments are generally immutable
/// after construction, so this is safe for concurrent reads.
pub(crate) fn build_with_metrics_dyn(
    instrument: Arc<dyn crate::instruments::common_impl::traits::Instrument>,
    curves: Arc<MarketContext>,
    as_of: Date,
    base_value: Money,
    metrics: &[crate::metrics::MetricId],
    cfg: Option<Arc<FinstackConfig>>,
    market_history: Option<Arc<MarketHistory>>,
) -> finstack_core::Result<crate::results::ValuationResult> {
    let finstack_config = cfg.unwrap_or_else(MetricContext::default_config);
    let mut context = MetricContext::new(
        instrument.clone(),
        curves,
        as_of,
        base_value,
        finstack_config,
    );

    // Attach market history if provided (for Historical VaR / Expected Shortfall metrics)
    if let Some(history) = market_history {
        context = context.with_market_history(history);
    }

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

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::{Attributes, Instrument};
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
            None,
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

    #[test]
    fn black_scholes_inputs_df_r_eff_consistency() {
        use super::BlackScholesInputsDf;

        // Test that r_eff is consistent with df and t
        // Given df = exp(-r * t), we should have r_eff = -ln(df) / t
        let inputs = BlackScholesInputsDf {
            spot: 100.0,
            df: 0.95, // ~5% discount over the period
            q: 0.02,
            sigma: 0.20,
            t: 1.0, // 1 year
        };

        let r_eff = inputs.r_eff();
        // r_eff should be approximately -ln(0.95) / 1.0 ≈ 0.0513
        let expected_r = -0.95_f64.ln() / 1.0;
        assert!(
            (r_eff - expected_r).abs() < 1e-10,
            "r_eff = {}, expected = {}",
            r_eff,
            expected_r
        );

        // Verify round-trip: exp(-r_eff * t) should equal df
        let reconstructed_df = (-r_eff * inputs.t).exp();
        assert!(
            (reconstructed_df - inputs.df).abs() < 1e-10,
            "reconstructed_df = {}, original df = {}",
            reconstructed_df,
            inputs.df
        );
    }

    #[test]
    fn black_scholes_inputs_df_edge_cases() {
        use super::BlackScholesInputsDf;

        // At expiry (t = 0), r_eff should return 0.0
        let at_expiry = BlackScholesInputsDf {
            spot: 100.0,
            df: 1.0,
            q: 0.02,
            sigma: 0.20,
            t: 0.0,
        };
        assert_eq!(at_expiry.r_eff(), 0.0, "r_eff at expiry should be 0");

        // Very short time horizon
        let short_horizon = BlackScholesInputsDf {
            spot: 100.0,
            df: 0.9999,
            q: 0.0,
            sigma: 0.20,
            t: 0.001,
        };
        let r_short = short_horizon.r_eff();
        // Should be approximately -ln(0.9999) / 0.001 ≈ 0.1 (10%)
        assert!(r_short > 0.0, "r_eff should be positive for df < 1");
        assert!(r_short.is_finite(), "r_eff should be finite");
    }
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
pub(crate) fn instrument_to_arc(
    instrument: &dyn crate::instruments::common_impl::traits::Instrument,
) -> Arc<dyn crate::instruments::common_impl::traits::Instrument> {
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

/// Black-Scholes inputs with discount factor (DF-first approach).
///
/// This struct provides the source-of-truth inputs for Black-Scholes/Garman-Kohlhagen
/// pricing where discounting is done via date-based discount factors rather than rates.
/// This avoids day-count basis mismatches between the rate `r` and time `t`.
///
/// # Fields
///
/// - `spot`: Current spot price
/// - `df`: Discount factor from `as_of` to `expiry` (date-based, no year-fraction ambiguity)
/// - `q`: Dividend yield / foreign rate (continuous)
/// - `sigma`: Implied volatility from the vol surface
/// - `t`: Time to expiry using the vol surface day count basis (for vol interpolation and Greeks)
#[derive(Clone, Copy, Debug)]
pub struct BlackScholesInputsDf {
    /// Current spot price
    pub spot: f64,
    /// Discount factor from as_of to expiry (date-based)
    pub df: f64,
    /// Dividend yield / foreign rate (continuous)
    pub q: f64,
    /// Implied volatility
    pub sigma: f64,
    /// Time to expiry in years (vol surface basis)
    pub t: f64,
}

impl BlackScholesInputsDf {
    /// Derive an effective continuously-compounded rate consistent with `t` and `df`.
    ///
    /// This returns `r_eff = -ln(df) / t` such that `exp(-r_eff * t) = df`.
    /// Use this when analytical formulas require a scalar rate.
    ///
    /// # Returns
    ///
    /// `r_eff` if `t > 0`, otherwise returns 0.0 (at expiry, rate is irrelevant).
    #[inline]
    pub fn r_eff(&self) -> f64 {
        if self.t > 0.0 && self.df > 0.0 {
            -self.df.ln() / self.t
        } else {
            0.0
        }
    }
}

/// Collect Black-Scholes inputs with discount factor (DF-first approach).
///
/// This is the preferred helper for option pricing as it avoids day-count basis
/// mismatches. The discount factor is computed directly from dates, ensuring
/// `exp(-r_eff * t) = df` when `r_eff` is derived via [`BlackScholesInputsDf::r_eff`].
///
/// # Arguments
///
/// * `spot_id` - ID of the spot price scalar
/// * `discount_curve_id` - ID of the discount curve
/// * `div_yield_id` - Optional ID of the dividend yield scalar (defaults to 0.0 if None)
/// * `vol_surface_id` - ID of the volatility surface
/// * `strike` - Strike price for volatility lookup
/// * `expiry` - Expiry date
/// * `day_count` - Day count convention for vol surface time calculation
/// * `curves` - Market data context
/// * `as_of` - Valuation date
///
/// # Returns
///
/// [`BlackScholesInputsDf`] containing (spot, df, q, sigma, t_vol).
#[allow(clippy::too_many_arguments)]
pub fn collect_black_scholes_inputs_df(
    spot_id: &str,
    discount_curve_id: &finstack_core::types::CurveId,
    div_yield_id: Option<&finstack_core::types::CurveId>,
    vol_surface_id: &str,
    strike: f64,
    expiry: Date,
    day_count: DayCount,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<BlackScholesInputsDf> {
    // Get discount curve
    let disc_curve = curves.get_discount(discount_curve_id.as_str())?;

    // Time to expiry for vol surface lookup (using instrument's day count, which should
    // match how the vol surface was calibrated - typically ACT/365F for equity options)
    let t_vol = day_count.year_fraction(as_of, expiry, DayCountCtx::default())?;

    // Discount factor from as_of to expiry (date-based, no year-fraction ambiguity)
    // This is the source of truth for discounting.
    let df = disc_curve.df_between_dates(as_of, expiry)?;

    // Validate DF is usable
    if !df.is_finite() || df <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "Invalid discount factor ({:.6e}) between {} and {}",
            df, as_of, expiry
        )));
    }

    // Spot price (S)
    let spot_scalar = curves.price(spot_id)?;
    let spot = match spot_scalar {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(m) => m.amount(),
    };

    // Dividend yield (q)
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

    Ok(BlackScholesInputsDf {
        spot,
        df,
        q,
        sigma,
        t: t_vol,
    })
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
/// # Time-Consistency
///
/// This function derives `r` from the discount factor such that `exp(-r * t) = df`.
/// This ensures the rate and time are on the same basis, avoiding day-count mismatches
/// that can cause pricing errors in barrier options and other path-dependent derivatives.
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
/// - `r`: Effective continuously compounded rate such that `exp(-r*t) = df`
/// - `q`: Dividend yield (0.0 if not provided)
/// - `sigma`: Implied volatility from the vol surface at (t_vol, strike)
/// - `t`: Time to expiry using the vol surface day count basis (t_vol)
#[allow(clippy::too_many_arguments)]
pub fn collect_black_scholes_inputs(
    spot_id: &str,
    discount_curve_id: &finstack_core::types::CurveId,
    div_yield_id: Option<&finstack_core::types::CurveId>,
    vol_surface_id: &str,
    strike: f64,
    expiry: Date,
    day_count: DayCount,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64)> {
    // Delegate to DF-based helper and derive r_eff
    let inputs = collect_black_scholes_inputs_df(
        spot_id,
        discount_curve_id,
        div_yield_id,
        vol_surface_id,
        strike,
        expiry,
        day_count,
        curves,
        as_of,
    )?;

    // Derive effective rate: r_eff = -ln(df) / t such that exp(-r_eff * t) = df
    let r_eff = inputs.r_eff();

    Ok((inputs.spot, r_eff, inputs.q, inputs.sigma, inputs.t))
}
