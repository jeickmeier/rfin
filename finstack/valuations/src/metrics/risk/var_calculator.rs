//! Historical VaR calculation engine.
//!
//! Implements Historical Value-at-Risk using historical simulation methodology.
//! Supports both full revaluation and Taylor approximation (Greeks-based) approaches.

use crate::instruments::common::helpers::instrument_to_arc;
use crate::instruments::common::traits::Instrument;
use crate::metrics::core::registry::StrictMode;
use crate::metrics::risk::MarketHistory;
use crate::metrics::sensitivities::dv01::format_bucket_label;
use crate::metrics::{standard_registry, MetricContext, MetricId};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use finstack_core::Result;
use std::sync::Arc;

/// VaR calculation method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VarMethod {
    /// Full revaluation of instrument under each historical scenario.
    ///
    /// Most accurate method - reprices the instrument under each historical
    /// market scenario. Captures all non-linearities and path dependencies.
    FullRevaluation,

    /// Taylor approximation using sensitivities (Greeks).
    ///
    /// Faster method - approximates P&L using pre-computed sensitivities.
    /// Good for linear instruments and large portfolios, but may be
    /// inaccurate for highly non-linear instruments (deep OTM options).
    TaylorApproximation,
}

/// Configuration for VaR calculation.
///
/// Controls statistical properties such as confidence level and pricing method.
/// The historical window/observation count is derived from [`MarketHistory`].
#[derive(Clone, Debug)]
pub struct VarConfig {
    /// Confidence level (e.g., 0.95 for 95% VaR, 0.99 for 99% VaR)
    pub confidence_level: f64,

    /// VaR calculation method
    pub method: VarMethod,
}

impl VarConfig {
    /// Create a new VaR configuration with standard settings.
    ///
    /// # Arguments
    ///
    /// * `confidence_level` - Confidence level (e.g., 0.95, 0.99)
    pub fn new(confidence_level: f64) -> Self {
        Self {
            confidence_level,
            method: VarMethod::FullRevaluation,
        }
    }

    /// Standard 95% VaR configuration.
    pub fn var_95() -> Self {
        Self::new(0.95)
    }

    /// Standard 99% VaR configuration.
    pub fn var_99() -> Self {
        Self::new(0.99)
    }

    /// Set the calculation method.
    pub fn with_method(mut self, method: VarMethod) -> Self {
        self.method = method;
        self
    }
}

/// VaR calculation results.
#[derive(Clone, Debug)]
pub struct VarResult {
    /// Value-at-Risk at specified confidence level (always positive)
    pub var: f64,

    /// Expected Shortfall (CVaR) at specified confidence level (always positive)
    ///
    /// Average of all losses exceeding VaR threshold.
    pub expected_shortfall: f64,

    /// Full P&L distribution from historical simulation (sorted, worst first)
    pub pnl_distribution: Vec<f64>,

    /// Number of scenarios used in calculation
    pub num_scenarios: usize,

    /// Confidence level used
    pub confidence_level: f64,
}

impl VarResult {
    /// Create VaR result from P&L distribution.
    ///
    /// # Arguments
    ///
    /// * `pnl_distribution` - Unsorted P&L values from historical simulation
    /// * `confidence_level` - Confidence level for VaR/ES calculation
    pub fn from_distribution(
        mut pnl_distribution: Vec<f64>,
        confidence_level: f64,
    ) -> Result<Self> {
        if pnl_distribution.iter().any(|v| !v.is_finite()) {
            return Err(finstack_core::Error::Validation(
                "VaR P&L distribution contains non-finite values (NaN or inf)".to_string(),
            ));
        }

        // Sort P&L distribution (ascending = worst losses first)
        pnl_distribution.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let num_scenarios = pnl_distribution.len();

        // Handle empty distribution
        if num_scenarios == 0 {
            return Ok(Self {
                var: 0.0,
                expected_shortfall: 0.0,
                pnl_distribution,
                num_scenarios,
                confidence_level,
            });
        }

        // Calculate VaR at confidence level
        let var_index = ((1.0 - confidence_level) * num_scenarios as f64).ceil() as usize;
        let var_index = var_index.saturating_sub(1).min(num_scenarios - 1);
        let var = -pnl_distribution[var_index]; // Negative because losses are negative P&Ls

        // Calculate Expected Shortfall (average of tail losses)
        let tail_size = var_index + 1;
        let expected_shortfall = if tail_size > 0 {
            let sum: f64 = pnl_distribution.iter().take(tail_size).sum();
            -(sum / tail_size as f64) // Negative because losses are negative P&Ls
        } else {
            0.0
        };

        Ok(Self {
            var,
            expected_shortfall,
            pnl_distribution,
            num_scenarios,
            confidence_level,
        })
    }
}

/// Calculate Historical VaR for a single instrument using full revaluation.
///
/// # Arguments
///
/// * `instrument` - The instrument to calculate VaR for
/// * `base_market` - Current market context (base case)
/// * `history` - Historical market scenarios
/// * `as_of` - Valuation date
/// * `config` - VaR configuration
///
/// # Returns
///
/// VaR result including VaR, ES, and full P&L distribution
///
/// This function revalues the instrument under every scenario contained in
/// [`MarketHistory`]. If the history is empty, the returned VaR/ES will be zero.
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::{Bond, Instrument};
/// use finstack_valuations::metrics::risk::{calculate_var, MarketHistory, MarketScenario, VarConfig};
/// use finstack_core::market_data::context::MarketContext;
/// use time::macros::date;
///
/// # fn main() -> finstack_core::Result<()> {
/// let bond = Bond::example();
/// let market = MarketContext::new();
/// let as_of = date!(2025-01-01);
/// let history = MarketHistory::new(as_of, 0, Vec::<MarketScenario>::new());
/// let config = VarConfig::var_95();
///
/// let instruments = [&bond];
/// let result = calculate_var(&instruments, &market, &history, as_of, &config)?;
/// println!("95% VaR: ${:.2}", result.var);
/// println!("95% ES: ${:.2}", result.expected_shortfall);
/// # Ok(())
/// # }
/// ```
pub fn calculate_var<I>(
    instruments: &[&I],
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult>
where
    I: Instrument + ?Sized,
{
    if instruments.is_empty() {
        return VarResult::from_distribution(Vec::new(), config.confidence_level);
    }

    match config.method {
        VarMethod::FullRevaluation => {
            calculate_var_full_revaluation(instruments, base_market, history, as_of, config)
        }
        VarMethod::TaylorApproximation => {
            calculate_var_taylor(instruments, base_market, history, as_of, config)
        }
    }
}

/// Utility helper to aggregate scenario P&Ls for both single-instrument and portfolio VaR.
fn aggregate_scenario_pnls<F>(
    history: &MarketHistory,
    base_market: &MarketContext,
    mut scenario_pnl: F,
) -> Result<Vec<f64>>
where
    F: FnMut(&MarketContext) -> Result<f64>,
{
    let mut pnls = Vec::with_capacity(history.len());

    for scenario in history.iter() {
        // Apply historical shifts to create scenario market
        let scenario_market = scenario.apply(base_market)?;

        // Delegate P&L calculation to caller-provided closure
        pnls.push(scenario_pnl(&scenario_market)?);
    }

    Ok(pnls)
}

/// Calculate VaR using full revaluation method.
fn calculate_var_full_revaluation<I>(
    instruments: &[&I],
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult>
where
    I: Instrument + ?Sized,
{
    let instrument_refs: Vec<&I> = instruments.to_vec();
    let base_values: Vec<f64> = instrument_refs
        .iter()
        .map(|inst| inst.value(base_market, as_of).map(|m| m.amount()))
        .collect::<Result<_>>()?;

    let pnls = aggregate_scenario_pnls(history, base_market, move |scenario_market| {
        let mut total = 0.0;
        for (inst, base_amount) in instrument_refs.iter().zip(base_values.iter()) {
            let scenario_amount = inst.value(scenario_market, as_of)?.amount();
            total += scenario_amount - base_amount;
        }
        Ok(total)
    })?;

    // Calculate VaR and ES from P&L distribution
    VarResult::from_distribution(pnls, config.confidence_level)
}

fn calculate_var_taylor<I>(
    instruments: &[&I],
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult>
where
    I: Instrument + ?Sized,
{
    if instruments.len() == 1 {
        // Use clone_box to get a sized type for Taylor approximation
        let boxed = instruments[0].clone_box();
        return calculate_var_taylor_approximation(&*boxed, base_market, history, as_of, config);
    }

    let boxed: Vec<Box<dyn Instrument>> = instruments.iter().map(|inst| inst.clone_box()).collect();
    let refs: Vec<&dyn Instrument> = boxed.iter().map(|b| b.as_ref()).collect();
    calculate_portfolio_var_taylor(&refs, base_market, history, as_of, config)
}

// =============================================================================
// Taylor Approximation
// =============================================================================

#[derive(Default)]
struct BucketedSeries {
    per_curve: HashMap<String, HashMap<String, f64>>,
    fallback: HashMap<String, f64>,
}

impl BucketedSeries {
    fn get(&self, curve_id: &str, bucket: &str) -> Option<f64> {
        if let Some(curve) = self.per_curve.get(curve_id) {
            if let Some(value) = curve.get(bucket) {
                return Some(*value);
            }
        }
        self.fallback.get(bucket).copied()
    }
}

#[derive(Default)]
struct TaylorSensitivities {
    dv01: BucketedSeries,
    cs01: BucketedSeries,
    parallel_dv01: f64,
    parallel_cs01: f64,
    equity_delta: f64,
    equity_gamma: f64,
    vega_rel: f64,
}

fn calculate_var_taylor_approximation(
    instrument: &dyn Instrument,
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult> {
    let base_value = instrument.value(base_market, as_of)?;
    let sensitivities = compute_taylor_sensitivities(instrument, base_market, as_of, base_value)?;

    let mut spot_cache: HashMap<String, f64> = HashMap::default();
    let mut vol_cache: HashMap<String, f64> = HashMap::default();
    let mut pnls = Vec::with_capacity(history.len());

    for scenario in history.iter() {
        let pnl = taylor_pnl_for_scenario(
            &sensitivities,
            base_market,
            scenario,
            &mut spot_cache,
            &mut vol_cache,
        );
        pnls.push(pnl);
    }

    VarResult::from_distribution(pnls, config.confidence_level)
}

fn compute_taylor_sensitivities(
    instrument: &dyn Instrument,
    base_market: &MarketContext,
    as_of: Date,
    base_value: Money,
) -> Result<TaylorSensitivities> {
    let instrument_type = instrument.key();
    let registry = standard_registry();
    let instrument_arc = instrument_to_arc(instrument);
    let mut context = MetricContext::new(
        instrument_arc,
        Arc::new(base_market.clone()),
        as_of,
        base_value,
        MetricContext::default_config(),
    );
    context.set_pricing_overrides(instrument.scenario_overrides().cloned());

    let metrics = [
        MetricId::BucketedDv01,
        MetricId::Dv01,
        MetricId::BucketedCs01,
        MetricId::Cs01,
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::IndexDelta,
        MetricId::EquityShares,
        MetricId::Vega,
    ];

    let computed = registry.compute_with_mode(&metrics, &mut context, StrictMode::BestEffort)?;

    let dv01 = collect_bucketed_series(&context.computed_series, MetricId::BucketedDv01.as_str());
    let cs01 = collect_bucketed_series(&context.computed_series, MetricId::BucketedCs01.as_str());

    let parallel_dv01 = computed.get(&MetricId::Dv01).copied().unwrap_or(0.0);
    let parallel_cs01 = computed.get(&MetricId::Cs01).copied().unwrap_or(0.0);

    let has_delta = registry.is_applicable(&MetricId::Delta, instrument_type);
    let has_gamma = registry.is_applicable(&MetricId::Gamma, instrument_type);
    let has_index_delta = registry.is_applicable(&MetricId::IndexDelta, instrument_type);
    let has_equity_shares = registry.is_applicable(&MetricId::EquityShares, instrument_type);
    let has_vega = registry.is_applicable(&MetricId::Vega, instrument_type);

    let delta = if has_delta {
        computed.get(&MetricId::Delta).copied().unwrap_or(0.0)
    } else if has_index_delta {
        computed.get(&MetricId::IndexDelta).copied().unwrap_or(0.0)
    } else if has_equity_shares {
        computed
            .get(&MetricId::EquityShares)
            .copied()
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let gamma = if has_gamma {
        computed.get(&MetricId::Gamma).copied().unwrap_or(0.0)
    } else {
        0.0
    };

    let vega_rel = if has_vega {
        computed.get(&MetricId::Vega).copied().unwrap_or(0.0)
    } else {
        0.0
    };

    Ok(TaylorSensitivities {
        dv01,
        cs01,
        parallel_dv01,
        parallel_cs01,
        equity_delta: delta,
        equity_gamma: gamma,
        vega_rel,
    })
}

fn taylor_pnl_for_scenario(
    sensitivities: &TaylorSensitivities,
    base_market: &MarketContext,
    scenario: &crate::metrics::risk::MarketScenario,
    spot_cache: &mut HashMap<String, f64>,
    vol_cache: &mut HashMap<String, f64>,
) -> f64 {
    let mut pnl = 0.0;
    for shift in &scenario.shifts {
        match &shift.factor {
            crate::metrics::risk::RiskFactorType::DiscountRate {
                curve_id,
                tenor_years,
            }
            | crate::metrics::risk::RiskFactorType::ForwardRate {
                curve_id,
                tenor_years,
            } => {
                let bucket = format_bucket_label(*tenor_years);
                let dv01 = sensitivities
                    .dv01
                    .get(curve_id.as_str(), bucket.as_str())
                    .unwrap_or(sensitivities.parallel_dv01);
                pnl += dv01 * shift.shift * 10_000.0;
            }
            crate::metrics::risk::RiskFactorType::CreditSpread {
                curve_id,
                tenor_years,
            } => {
                let bucket = format_bucket_label(*tenor_years);
                let cs01 = sensitivities
                    .cs01
                    .get(curve_id.as_str(), bucket.as_str())
                    .unwrap_or(sensitivities.parallel_cs01);
                pnl += cs01 * shift.shift * 10_000.0;
            }
            crate::metrics::risk::RiskFactorType::EquitySpot { ticker } => {
                if sensitivities.equity_delta.abs() > 0.0 || sensitivities.equity_gamma.abs() > 0.0
                {
                    let spot = *spot_cache
                        .entry(ticker.clone())
                        .or_insert_with(|| spot_from_market(base_market, ticker).unwrap_or(0.0));
                    if spot > 0.0 {
                        let d_spot = spot * shift.shift;
                        pnl += sensitivities.equity_delta * d_spot
                            + 0.5 * sensitivities.equity_gamma * d_spot * d_spot;
                    }
                }
            }
            crate::metrics::risk::RiskFactorType::ImpliedVol {
                surface_id,
                expiry_years,
                strike,
            } => {
                if sensitivities.vega_rel.abs() > 0.0 {
                    // Use string key because f64 doesn't implement Hash
                    let key = format!("{}:{}:{}", surface_id, expiry_years, strike);
                    let base_vol = *vol_cache.entry(key).or_insert_with(|| {
                        base_vol_for_factor(base_market, surface_id, *expiry_years, *strike)
                            .unwrap_or(0.0)
                    });
                    if base_vol > 0.0 {
                        let vega_abs = sensitivities.vega_rel / base_vol;
                        pnl += vega_abs * shift.shift;
                    }
                }
            }
        }
    }
    pnl
}

fn collect_bucketed_series(
    series_map: &HashMap<MetricId, Vec<(String, f64)>>,
    base_id: &str,
) -> BucketedSeries {
    let mut result = BucketedSeries::default();

    for (metric_id, series) in series_map {
        let id_str = metric_id.as_str();
        if id_str == base_id {
            result.fallback = series.iter().cloned().collect();
        } else if let Some(curve_id) = id_str.strip_prefix(&format!("{base_id}::")) {
            let entry = result
                .per_curve
                .entry(curve_id.to_string())
                .or_insert_with(HashMap::default);
            for (bucket, value) in series {
                entry.insert(bucket.clone(), *value);
            }
        }
    }

    if result.fallback.is_empty() && !result.per_curve.is_empty() {
        for series in result.per_curve.values() {
            for (bucket, value) in series {
                *result.fallback.entry(bucket.clone()).or_insert(0.0) += *value;
            }
        }
    }

    result
}

fn spot_from_market(market: &MarketContext, ticker: &str) -> Option<f64> {
    match market.price(ticker) {
        Ok(MarketScalar::Unitless(v)) => Some(*v),
        Ok(MarketScalar::Price(m)) => Some(m.amount()),
        _ => None,
    }
}

fn base_vol_for_factor(
    market: &MarketContext,
    surface_id: &CurveId,
    expiry_years: f64,
    strike: f64,
) -> Option<f64> {
    let surface = market.surface(surface_id.as_str()).ok()?;
    Some(surface.value_clamped(expiry_years, strike))
}

fn calculate_portfolio_var_taylor(
    instruments: &[&dyn Instrument],
    base_market: &MarketContext,
    history: &MarketHistory,
    as_of: Date,
    config: &VarConfig,
) -> Result<VarResult> {
    if instruments.is_empty() {
        return Ok(VarResult {
            var: 0.0,
            expected_shortfall: 0.0,
            pnl_distribution: vec![],
            confidence_level: config.confidence_level,
            num_scenarios: 0,
        });
    }

    let mut sensitivities: Vec<TaylorSensitivities> = Vec::with_capacity(instruments.len());
    for instrument in instruments {
        let base_value = instrument.value(base_market, as_of)?;
        sensitivities.push(compute_taylor_sensitivities(
            *instrument,
            base_market,
            as_of,
            base_value,
        )?);
    }

    let mut spot_cache: HashMap<String, f64> = HashMap::default();
    let mut vol_cache: HashMap<String, f64> = HashMap::default();
    let mut pnls = Vec::with_capacity(history.len());

    for scenario in history.iter() {
        let mut total = 0.0;
        for sens in &sensitivities {
            total += taylor_pnl_for_scenario(
                sens,
                base_market,
                scenario,
                &mut spot_cache,
                &mut vol_cache,
            );
        }
        pnls.push(total);
    }

    VarResult::from_distribution(pnls, config.confidence_level)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use crate::metrics::risk::test_utils::{
        history_from_rate_shifts, history_from_scenarios, sample_as_of, standard_bond,
        usd_ois_market,
    };
    use std::sync::Arc;
    use time::macros::date;

    #[test]
    fn test_var_config_creation() {
        let config = VarConfig::var_95();
        assert_eq!(config.confidence_level, 0.95);
        assert_eq!(config.method, VarMethod::FullRevaluation);

        let config = VarConfig::var_99().with_method(VarMethod::TaylorApproximation);
        assert_eq!(config.confidence_level, 0.99);
        assert_eq!(config.method, VarMethod::TaylorApproximation);
    }

    #[test]
    fn test_taylor_method_matches_full_revaluation_small_shifts() -> Result<()> {
        let as_of = sample_as_of();
        let bond = standard_bond("TEST-BOND", as_of, date!(2029 - 01 - 01));
        let base_market = usd_ois_market(as_of)?;
        let history = history_from_rate_shifts(
            as_of,
            &[
                (date!(2023 - 12 - 31), 0.0005),
                (date!(2023 - 12 - 30), -0.0003),
                (date!(2023 - 12 - 29), 0.0002),
            ],
        );
        let full_config = VarConfig::var_95();
        let taylor_config = VarConfig::var_95().with_method(VarMethod::TaylorApproximation);

        let full = calculate_var(&[&bond], &base_market, &history, as_of, &full_config)?;
        let taylor = calculate_var(&[&bond], &base_market, &history, as_of, &taylor_config)?;

        assert!(taylor.var > 0.0, "Taylor VaR should be positive");
        let diff = (taylor.var - full.var).abs();
        let rel = diff / full.var.max(1.0);
        assert!(
            rel < 0.15,
            "Taylor VaR should be close to full revaluation (diff: {:.4}%)",
            rel * 100.0
        );

        Ok(())
    }

    #[test]
    fn test_var_result_from_distribution() {
        // Create synthetic P&L distribution with known values
        let pnls = vec![
            100.0,  // gain
            50.0,   // gain
            0.0,    // no change
            -25.0,  // small loss
            -50.0,  // medium loss
            -100.0, // large loss
            -150.0, // very large loss
            -200.0, // worst loss
        ];

        let result = VarResult::from_distribution(pnls, 0.95).expect("pnl distribution is finite");

        // With 8 scenarios and 95% confidence:
        // Tail size = ceil((1-0.95) * 8) = ceil(0.4) = 1
        // So VaR should be the worst loss = 200
        assert_eq!(result.var, 200.0);
        assert_eq!(result.num_scenarios, 8);

        // ES should be average of tail (just the worst loss in this case)
        assert_eq!(result.expected_shortfall, 200.0);
    }

    #[test]
    fn test_var_result_rejects_nan() {
        let pnls = vec![10.0, f64::NAN, -5.0];
        let err = VarResult::from_distribution(pnls, 0.95).expect_err("should reject NaNs");
        match err {
            finstack_core::Error::Validation(msg) => {
                assert!(
                    msg.contains("non-finite"),
                    "error message should mention non-finite values"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn test_var_calculation_simple_bond() -> Result<()> {
        let as_of = sample_as_of();

        let bond = standard_bond("TEST-BOND", as_of, date!(2029 - 01 - 01));
        let base_market = usd_ois_market(as_of)?;

        let history = history_from_rate_shifts(
            as_of,
            &[
                (date!(2023 - 12 - 31), 0.0050),
                (date!(2023 - 12 - 30), -0.0030),
                (date!(2023 - 12 - 29), 0.0010),
            ],
        );
        let config = VarConfig::var_95();

        // Calculate VaR
        let result = calculate_var(&[&bond], &base_market, &history, as_of, &config)?;

        // Verify results
        assert_eq!(result.num_scenarios, 3);
        assert!(result.var > 0.0, "VaR should be positive");
        assert!(
            result.expected_shortfall >= result.var,
            "ES should be >= VaR"
        );

        // P&L distribution should have 3 values
        assert_eq!(result.pnl_distribution.len(), 3);

        // Distribution should be sorted (worst first)
        for i in 1..result.pnl_distribution.len() {
            assert!(
                result.pnl_distribution[i] >= result.pnl_distribution[i - 1],
                "P&L distribution should be sorted"
            );
        }

        Ok(())
    }

    #[test]
    fn test_var_empty_history() -> Result<()> {
        let as_of = sample_as_of();
        let bond = standard_bond("TEST-BOND", as_of, date!(2029 - 01 - 01));
        let base_market = usd_ois_market(as_of)?;

        // Empty history
        let history = history_from_scenarios(as_of, 0, vec![]);
        let config = VarConfig::var_95();

        let result = calculate_var(&[&bond], &base_market, &history, as_of, &config)?;

        assert_eq!(result.num_scenarios, 0);
        assert_eq!(result.pnl_distribution.len(), 0);

        Ok(())
    }

    #[test]
    fn test_portfolio_var_with_diversification() -> Result<()> {
        let as_of = sample_as_of();

        let bond1 = standard_bond("BOND-5Y", as_of, date!(2029 - 01 - 01));
        let bond2 = standard_bond("BOND-2Y", as_of, date!(2026 - 01 - 01));

        let market = Arc::new(usd_ois_market(as_of)?);

        let history = history_from_rate_shifts(
            as_of,
            &[
                (date!(2023 - 12 - 31), 0.0100),
                (date!(2023 - 12 - 30), -0.0075),
                (date!(2023 - 12 - 29), 0.0025),
                (date!(2023 - 12 - 28), -0.0050),
            ],
        );
        let config = VarConfig::var_95();

        // Calculate individual VaRs
        let var1 = calculate_var(&[&bond1], market.as_ref(), &history, as_of, &config)?;
        let var2 = calculate_var(&[&bond2], market.as_ref(), &history, as_of, &config)?;
        let sum_individual_vars = var1.var.abs() + var2.var.abs();

        // Calculate portfolio VaR
        let instruments: Vec<&dyn Instrument> = vec![&bond1, &bond2];
        let portfolio_var =
            calculate_var(&instruments, market.as_ref(), &history, as_of, &config)?;

        // Verify portfolio VaR <= sum of individual VaRs
        // With only a few scenarios and both bonds having similar rate sensitivity,
        // we might not see diversification benefit in this simple test
        assert!(
            portfolio_var.var.abs() <= sum_individual_vars + 0.01, // Allow small numerical tolerance
            "Portfolio VaR ({}) should be <= sum of individual VaRs ({})",
            portfolio_var.var.abs(),
            sum_individual_vars
        );

        // Calculate diversification benefit (may be zero or small with limited scenarios)
        let diversification_benefit = sum_individual_vars - portfolio_var.var.abs();
        assert!(
            diversification_benefit >= -0.01, // Allow small numerical t olerance
            "Diversification benefit should be non-negative, got {}",
            diversification_benefit
        );

        println!("Individual VaR 1: ${:.2}", var1.var);
        println!("Individual VaR 2: ${:.2}", var2.var);
        println!("Sum of individual VaRs: ${:.2}", sum_individual_vars);
        println!("Portfolio VaR: ${:.2}", portfolio_var.var.abs());
        println!("Diversification benefit: ${:.2}", diversification_benefit);

        Ok(())
    }
}
