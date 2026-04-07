//! Taylor-expansion P&L attribution.
//!
//! Decomposes P&L into risk-factor contributions using first-order sensitivities
//! computed via bump-and-reprice:
//!
//!   ΔP&L ≈ Σ DV01ᵢ × Δrateᵢ + Σ Fwd01ₖ × Δfwdₖ + Σ CS01ⱼ × Δspreadⱼ + vega × Δvol + FX01 × ΔFX + theta
//!
//! Optionally includes second-order (gamma/convexity) terms:
//!
//!   + ½ Σ Gammaᵢ × Δrateᵢ² + ½ CsGamma × Δspread² + ½ Volga × Δvol²
//!
//! This is complementary to the waterfall (full-reval) approach: it produces a
//! factor-level explained/unexplained decomposition without sequential market
//! state construction.

use super::helpers::*;
use super::types::*;
use crate::instruments::common_impl::traits::Instrument;
use crate::metrics::{
    bump_discount_curve_parallel, bump_forward_curve_parallel, bump_surface_vol_absolute,
};
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::diff::{
    measure_discount_curve_shift, measure_hazard_curve_shift, measure_vol_surface_shift,
    TenorSamplingMethod, STANDARD_TENORS,
};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for Taylor-based P&L attribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaylorAttributionConfig {
    /// Include second-order (gamma/convexity) terms.
    #[serde(default)]
    pub include_gamma: bool,

    /// Rate bump size for DV01 computation (basis points).
    #[serde(default = "default_rate_bump_bp")]
    pub rate_bump_bp: f64,

    /// Credit spread bump size for CS01 computation (basis points).
    #[serde(default = "default_credit_bump_bp")]
    pub credit_bump_bp: f64,

    /// Vol bump size for vega computation (absolute vol points, e.g. 0.01 = 1%).
    #[serde(default = "default_vol_bump")]
    pub vol_bump: f64,
}

fn default_rate_bump_bp() -> f64 {
    1.0
}
fn default_credit_bump_bp() -> f64 {
    1.0
}
fn default_vol_bump() -> f64 {
    0.01
}

impl Default for TaylorAttributionConfig {
    fn default() -> Self {
        Self {
            include_gamma: false,
            rate_bump_bp: default_rate_bump_bp(),
            credit_bump_bp: default_credit_bump_bp(),
            vol_bump: default_vol_bump(),
        }
    }
}

/// Per-factor result from Taylor attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaylorFactorResult {
    /// Human-readable factor name (e.g. "Rates:USD-OIS").
    pub factor_name: String,
    /// First-order sensitivity (DV01, CS01, vega, theta, etc.).
    pub sensitivity: f64,
    /// Observed market move between T0 and T1.
    pub market_move: f64,
    /// First-order explained P&L: sensitivity × move.
    pub explained_pnl: f64,
    /// Second-order (gamma) P&L if requested: ½ × gamma × move².
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamma_pnl: Option<f64>,
}

/// Complete result of Taylor-based attribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaylorAttributionResult {
    /// Actual P&L (PV_T1 - PV_T0).
    pub actual_pnl: f64,
    /// Sum of all first-order (+ optional second-order) explained P&L.
    pub total_explained: f64,
    /// Unexplained residual: actual - explained.
    pub unexplained: f64,
    /// Unexplained as percentage of actual P&L.
    pub unexplained_pct: f64,
    /// Per-factor breakdown.
    pub factors: Vec<TaylorFactorResult>,
    /// Number of repricings performed (bump-and-reprice calls).
    pub num_repricings: usize,
    /// Present value at T0 (cached to avoid redundant repricing in compat layer).
    pub pv_t0: Money,
    /// Present value at T1 (cached to avoid redundant repricing in compat layer).
    pub pv_t1: Money,
}

/// Compute Taylor-based P&L attribution.
///
/// Uses bump-and-reprice at T0 to compute first-order sensitivities, then
/// multiplies by the observed market move between T0 and T1 to obtain
/// factor-level explained P&L.
///
/// # Arguments
///
/// * `instrument` - Instrument to attribute
/// * `market_t0` - Market context at T0
/// * `market_t1` - Market context at T1
/// * `as_of_t0` - Valuation date T0
/// * `as_of_t1` - Valuation date T1
/// * `config` - Taylor attribution configuration
///
/// # Returns
///
/// `TaylorAttributionResult` with per-factor decomposition and residual.
pub fn attribute_pnl_taylor(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    config: &TaylorAttributionConfig,
) -> Result<TaylorAttributionResult> {
    let pv_t0 = reprice_instrument(instrument, market_t0, as_of_t0)?;
    let pv_t1 = reprice_instrument(instrument, market_t1, as_of_t1)?;
    let actual_pnl = pv_t1.amount() - pv_t0.amount();

    let mut factors = Vec::new();
    let mut total_explained = 0.0;
    let mut num_repricings: usize = 2;

    // Rate sensitivities (parallel DV01 per discount curve)
    let market_deps = instrument.market_dependencies()?;
    for curve_id in &market_deps.curve_dependencies().discount_curves {
        match compute_rate_factor(
            instrument, market_t0, market_t1, as_of_t0, pv_t0, curve_id, config,
        ) {
            Ok(result) => {
                total_explained += result.explained_pnl;
                if let Some(g) = result.gamma_pnl {
                    total_explained += g;
                }
                num_repricings += 2;
                factors.push(result);
            }
            Err(e) => {
                tracing::warn!(
                    curve_id = %curve_id,
                    error = %e,
                    "Taylor attribution: rate factor computation failed"
                );
            }
        }
    }

    // Forward curve sensitivities (parallel bump per forward curve)
    for curve_id in &market_deps.curve_dependencies().forward_curves {
        match compute_forward_factor(
            instrument, market_t0, market_t1, as_of_t0, pv_t0, curve_id, config,
        ) {
            Ok(result) => {
                total_explained += result.explained_pnl;
                if let Some(g) = result.gamma_pnl {
                    total_explained += g;
                }
                num_repricings += 2;
                factors.push(result);
            }
            Err(e) => {
                tracing::warn!(
                    curve_id = %curve_id,
                    error = %e,
                    "Taylor attribution: forward factor computation failed"
                );
            }
        }
    }

    // Credit sensitivities (CS01 per hazard curve)
    for curve_id in &market_deps.curve_dependencies().credit_curves {
        match compute_credit_factor(
            instrument, market_t0, market_t1, as_of_t0, pv_t0, curve_id, config,
        ) {
            Ok(result) => {
                total_explained += result.explained_pnl;
                if let Some(g) = result.gamma_pnl {
                    total_explained += g;
                }
                num_repricings += 2;
                factors.push(result);
            }
            Err(e) => {
                tracing::warn!(
                    curve_id = %curve_id,
                    error = %e,
                    "Taylor attribution: credit factor computation failed"
                );
            }
        }
    }

    // Volatility sensitivity (vega)
    if let Some(ref surface_id_str) = market_deps.equity_dependencies().vol_surface_id {
        let surface_id = CurveId::new(surface_id_str.as_str());
        match compute_vol_factor(
            instrument,
            market_t0,
            market_t1,
            as_of_t0,
            pv_t0,
            &surface_id,
            config,
        ) {
            Ok(result) => {
                total_explained += result.explained_pnl;
                if let Some(g) = result.gamma_pnl {
                    total_explained += g;
                }
                num_repricings += 2;
                factors.push(result);
            }
            Err(e) => {
                tracing::warn!(
                    surface_id = %surface_id,
                    error = %e,
                    "Taylor attribution: vol factor computation failed"
                );
            }
        }
    }

    // Theta (time decay): reprice at T1 date with T0 market
    match compute_theta_factor(instrument, market_t0, as_of_t0, as_of_t1, pv_t0) {
        Ok(result) => {
            total_explained += result.explained_pnl;
            num_repricings += 1;
            factors.push(result);
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Taylor attribution: theta factor computation failed"
            );
        }
    }

    let unexplained = actual_pnl - total_explained;
    let unexplained_pct = if actual_pnl.abs() > 1e-10 {
        (unexplained / actual_pnl) * 100.0
    } else {
        0.0
    };

    Ok(TaylorAttributionResult {
        actual_pnl,
        total_explained,
        unexplained,
        unexplained_pct,
        factors,
        num_repricings,
        pv_t0,
        pv_t1,
    })
}

/// Also produce a `PnlAttribution` compatible with the existing attribution framework.
///
/// This maps Taylor results into the standard `PnlAttribution` struct so that
/// Taylor output can be used interchangeably with parallel/waterfall results.
pub fn attribute_pnl_taylor_standard(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    config: &TaylorAttributionConfig,
) -> Result<PnlAttribution> {
    let taylor =
        attribute_pnl_taylor(instrument, market_t0, market_t1, as_of_t0, as_of_t1, config)?;

    let total_pnl = compute_pnl(
        taylor.pv_t0,
        taylor.pv_t1,
        taylor.pv_t1.currency(),
        market_t1,
        as_of_t1,
    )?;

    let ccy = total_pnl.currency();
    let mut attribution = PnlAttribution::new(
        total_pnl,
        instrument.id(),
        as_of_t0,
        as_of_t1,
        AttributionMethod::Taylor(config.clone()),
    );

    for factor in &taylor.factors {
        let pnl_amount = factor.explained_pnl + factor.gamma_pnl.unwrap_or(0.0);
        let factor_money = Money::new(pnl_amount, ccy);

        if factor.factor_name.starts_with("Rates:") {
            attribution.rates_curves_pnl = Money::new(
                attribution.rates_curves_pnl.amount() + factor_money.amount(),
                ccy,
            );
        } else if factor.factor_name.starts_with("Credit:") {
            attribution.credit_curves_pnl = Money::new(
                attribution.credit_curves_pnl.amount() + factor_money.amount(),
                ccy,
            );
        } else if factor.factor_name.starts_with("Vol:") {
            attribution.vol_pnl =
                Money::new(attribution.vol_pnl.amount() + factor_money.amount(), ccy);
        } else if factor.factor_name == "Theta" {
            attribution.carry = factor_money;
            attribution.carry_detail = Some(CarryDetail {
                total: factor_money,
                coupon_income: None,
                pull_to_par: None,
                theta: Some(factor_money),
                roll_down: None,
                funding_cost: None,
            });
        }
    }

    if let Err(e) = attribution.compute_residual() {
        tracing::warn!(
            error = %e,
            instrument_id = %instrument.id(),
            "Taylor compat: residual computation failed"
        );
    }

    attribution.meta.num_repricings = taylor.num_repricings;
    attribution.meta.tolerance_abs = 10.0;
    attribution.meta.tolerance_pct = 5.0;
    attribution.meta.notes.push(format!(
        "Taylor attribution: {:.2}% unexplained ({} factors, {} repricings)",
        taylor.unexplained_pct,
        taylor.factors.len(),
        taylor.num_repricings,
    ));

    Ok(attribution)
}

// ─── Helper functions ──────────────────────────────────────────────────────

/// Measure average parallel forward rate shift between two markets (basis points).
fn measure_forward_curve_shift(
    curve_id: &CurveId,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    method: TenorSamplingMethod,
) -> Result<f64> {
    let curve_t0 = market_t0.get_forward(curve_id.as_str())?;
    let curve_t1 = market_t1.get_forward(curve_id.as_str())?;
    let tenors: &[f64] = match method {
        TenorSamplingMethod::Standard => STANDARD_TENORS,
        TenorSamplingMethod::Dynamic => {
            let knots = curve_t0.knots();
            if knots.is_empty() {
                STANDARD_TENORS
            } else {
                knots
            }
        }
        TenorSamplingMethod::Custom(ref tenors) => tenors.as_slice(),
    };
    Ok(measure_average_rate_shift(
        tenors,
        |t| curve_t0.rate(t),
        |t| curve_t1.rate(t),
    ))
}

fn measure_average_rate_shift(
    sample_points: &[f64],
    mut value_t0: impl FnMut(f64) -> f64,
    mut value_t1: impl FnMut(f64) -> f64,
) -> f64 {
    let mut total_shift = 0.0;
    let mut count = 0;

    for &t in sample_points {
        if t <= 0.0 {
            continue;
        }
        let v0 = value_t0(t);
        let v1 = value_t1(t);
        let shift = (v1 - v0) * 10_000.0;
        total_shift += shift;
        count += 1;
    }

    if count == 0 {
        return 0.0;
    }
    total_shift / count as f64
}

/// Compute rate (DV01) attribution for a single discount curve.
fn compute_rate_factor(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    pv_t0: Money,
    curve_id: &CurveId,
    config: &TaylorAttributionConfig,
) -> Result<TaylorFactorResult> {
    let bumped_up = bump_discount_curve_parallel(market_t0, curve_id, config.rate_bump_bp)?;
    let pv_up = reprice_instrument(instrument, &bumped_up, as_of_t0)?;

    let bumped_down = bump_discount_curve_parallel(market_t0, curve_id, -config.rate_bump_bp)?;
    let pv_down = reprice_instrument(instrument, &bumped_down, as_of_t0)?;

    // Central difference DV01: O(h²) accuracy
    let dv01 = (pv_up.amount() - pv_down.amount()) / (2.0 * config.rate_bump_bp);

    let rate_move_bp = measure_discount_curve_shift(
        curve_id.as_str(),
        market_t0,
        market_t1,
        TenorSamplingMethod::Standard,
    )?;

    let explained = dv01 * rate_move_bp;

    let gamma_pnl = if config.include_gamma {
        let gamma = (pv_up.amount() - 2.0 * pv_t0.amount() + pv_down.amount())
            / (config.rate_bump_bp * config.rate_bump_bp);
        Some(0.5 * gamma * rate_move_bp * rate_move_bp)
    } else {
        None
    };

    Ok(TaylorFactorResult {
        factor_name: format!("Rates:{}", curve_id),
        sensitivity: dv01,
        market_move: rate_move_bp,
        explained_pnl: explained,
        gamma_pnl,
    })
}

/// Compute forward-curve sensitivity attribution for a single forward curve.
///
/// Uses the same parallel bump convention as [`compute_rate_factor`], but applies
/// to the forward curve entry in [`MarketContext`] and measures the realized
/// move using forward rates (not discount zeros).
fn compute_forward_factor(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    pv_t0: Money,
    curve_id: &CurveId,
    config: &TaylorAttributionConfig,
) -> Result<TaylorFactorResult> {
    let bumped_up = bump_forward_curve_parallel(market_t0, curve_id, config.rate_bump_bp)?;
    let pv_up = reprice_instrument(instrument, &bumped_up, as_of_t0)?;

    let bumped_down = bump_forward_curve_parallel(market_t0, curve_id, -config.rate_bump_bp)?;
    let pv_down = reprice_instrument(instrument, &bumped_down, as_of_t0)?;

    let dv01 = (pv_up.amount() - pv_down.amount()) / (2.0 * config.rate_bump_bp);

    let rate_move_bp = measure_forward_curve_shift(
        curve_id,
        market_t0,
        market_t1,
        TenorSamplingMethod::Standard,
    )?;

    let explained = dv01 * rate_move_bp;

    let gamma_pnl = if config.include_gamma {
        let gamma = (pv_up.amount() - 2.0 * pv_t0.amount() + pv_down.amount())
            / (config.rate_bump_bp * config.rate_bump_bp);
        Some(0.5 * gamma * rate_move_bp * rate_move_bp)
    } else {
        None
    };

    Ok(TaylorFactorResult {
        factor_name: format!("Forward:{}", curve_id),
        sensitivity: dv01,
        market_move: rate_move_bp,
        explained_pnl: explained,
        gamma_pnl,
    })
}

/// Compute credit (CS01) attribution for a single hazard curve.
fn compute_credit_factor(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    pv_t0: Money,
    curve_id: &CurveId,
    config: &TaylorAttributionConfig,
) -> Result<TaylorFactorResult> {
    let bumped_up = market_t0.bump([MarketBump::Curve {
        id: curve_id.clone(),
        spec: BumpSpec::parallel_bp(config.credit_bump_bp),
    }])?;
    let pv_up = reprice_instrument(instrument, &bumped_up, as_of_t0)?;

    let bumped_down = market_t0.bump([MarketBump::Curve {
        id: curve_id.clone(),
        spec: BumpSpec::parallel_bp(-config.credit_bump_bp),
    }])?;
    let pv_down = reprice_instrument(instrument, &bumped_down, as_of_t0)?;

    // Central difference CS01: O(h²) accuracy
    let cs01 = (pv_up.amount() - pv_down.amount()) / (2.0 * config.credit_bump_bp);

    let spread_move_bp = measure_hazard_curve_shift(
        curve_id.as_str(),
        market_t0,
        market_t1,
        TenorSamplingMethod::Standard,
    )?;

    let explained = cs01 * spread_move_bp;

    let gamma_pnl = if config.include_gamma {
        let gamma = (pv_up.amount() - 2.0 * pv_t0.amount() + pv_down.amount())
            / (config.credit_bump_bp * config.credit_bump_bp);
        Some(0.5 * gamma * spread_move_bp * spread_move_bp)
    } else {
        None
    };

    Ok(TaylorFactorResult {
        factor_name: format!("Credit:{}", curve_id),
        sensitivity: cs01,
        market_move: spread_move_bp,
        explained_pnl: explained,
        gamma_pnl,
    })
}

/// Compute volatility (vega) attribution for a vol surface.
fn compute_vol_factor(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    market_t1: &MarketContext,
    as_of_t0: Date,
    pv_t0: Money,
    surface_id: &CurveId,
    config: &TaylorAttributionConfig,
) -> Result<TaylorFactorResult> {
    let bumped_up = bump_surface_vol_absolute(market_t0, surface_id.as_str(), config.vol_bump)?;
    let pv_up = reprice_instrument(instrument, &bumped_up, as_of_t0)?;

    let bumped_down = bump_surface_vol_absolute(market_t0, surface_id.as_str(), -config.vol_bump)?;
    let pv_down = reprice_instrument(instrument, &bumped_down, as_of_t0)?;

    // Central difference vega: O(h²) accuracy
    let vega_per_point = (pv_up.amount() - pv_down.amount()) / (2.0 * config.vol_bump);

    let vol_move =
        measure_vol_surface_shift(surface_id.as_str(), market_t0, market_t1, None, None)?;

    let explained = vega_per_point * vol_move;

    let gamma_pnl = if config.include_gamma {
        let volga = (pv_up.amount() - 2.0 * pv_t0.amount() + pv_down.amount())
            / (config.vol_bump * config.vol_bump);
        Some(0.5 * volga * vol_move * vol_move)
    } else {
        None
    };

    Ok(TaylorFactorResult {
        factor_name: format!("Vol:{}", surface_id),
        sensitivity: vega_per_point,
        market_move: vol_move,
        explained_pnl: explained,
        gamma_pnl,
    })
}

/// Compute theta (time decay) by repricing at T1 date with T0 market.
fn compute_theta_factor(
    instrument: &Arc<dyn Instrument>,
    market_t0: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
    pv_t0: Money,
) -> Result<TaylorFactorResult> {
    let pv_t0_at_t1 = reprice_instrument(instrument, market_t0, as_of_t1)?;
    let theta_pnl = pv_t0_at_t1.amount() - pv_t0.amount();
    let days = (as_of_t1 - as_of_t0).whole_days() as f64;

    let theta_per_day = if days.abs() > 0.0 {
        theta_pnl / days
    } else {
        0.0
    };

    Ok(TaylorFactorResult {
        factor_name: "Theta".to_string(),
        sensitivity: theta_per_day,
        market_move: days,
        explained_pnl: theta_pnl,
        gamma_pnl: None,
    })
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
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::money::Money;
    use std::sync::Arc;
    use test_utils::TestInstrument;
    use time::macros::date;

    #[test]
    fn test_taylor_config_default() {
        let config = TaylorAttributionConfig::default();
        assert!(!config.include_gamma);
        assert_eq!(config.rate_bump_bp, 1.0);
        assert_eq!(config.credit_bump_bp, 1.0);
        assert_eq!(config.vol_bump, 0.01);
    }

    #[test]
    fn test_taylor_config_serde_roundtrip() {
        let config = TaylorAttributionConfig {
            include_gamma: true,
            rate_bump_bp: 0.5,
            credit_bump_bp: 2.0,
            vol_bump: 0.005,
        };

        let json = serde_json::to_string(&config).expect("serialize should succeed");
        let parsed: TaylorAttributionConfig =
            serde_json::from_str(&json).expect("deserialize should succeed");

        assert_eq!(parsed, config);
    }

    #[test]
    fn test_taylor_attribution_empty_market() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        let instrument: Arc<dyn Instrument> = Arc::new(TestInstrument::new(
            "TEST-001",
            Money::new(1000.0, Currency::USD),
        ));

        let market_t0 = MarketContext::new();
        let market_t1 = MarketContext::new();
        let config = TaylorAttributionConfig::default();

        let result = attribute_pnl_taylor(
            &instrument,
            &market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
            &config,
        )
        .expect("taylor attribution should succeed for simple instrument");

        // TestInstrument returns the same value regardless of market → actual_pnl ≈ 0
        assert!(result.actual_pnl.abs() < 1e-10);
    }

    #[test]
    fn test_taylor_compat_produces_pnl_attribution() {
        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        let instrument: Arc<dyn Instrument> = Arc::new(TestInstrument::new(
            "TEST-001",
            Money::new(1000.0, Currency::USD),
        ));

        let market_t0 = MarketContext::new();
        let market_t1 = MarketContext::new();
        let config = TaylorAttributionConfig::default();

        let attribution = attribute_pnl_taylor_standard(
            &instrument,
            &market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
            &config,
        )
        .expect("taylor compat attribution should succeed");

        assert_eq!(attribution.meta.instrument_id, "TEST-001");
        assert!(matches!(
            attribution.meta.method,
            AttributionMethod::Taylor(_)
        ));
    }

    #[test]
    fn taylor_attribution_includes_forward_curve_factors() {
        use finstack_core::dates::DayCount;
        use finstack_core::market_data::term_structures::ForwardCurve;
        use finstack_core::types::CurveId;

        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        let fwd_t0 = ForwardCurve::builder(CurveId::new("TEST-FWD"), 0.25)
            .base_date(as_of_t0)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.03), (10.0, 0.03)])
            .build()
            .expect("forward curve");
        let fwd_t1 = ForwardCurve::builder(CurveId::new("TEST-FWD"), 0.25)
            .base_date(as_of_t0)
            .day_count(DayCount::Act360)
            .knots([(0.0, 0.04), (10.0, 0.04)])
            .build()
            .expect("forward curve");

        let market_t0 = MarketContext::new().insert(fwd_t0);
        let market_t1 = MarketContext::new().insert(fwd_t1);

        let instrument: Arc<dyn Instrument> = Arc::new(
            TestInstrument::new("FWDI", Money::new(0.0, Currency::USD))
                .with_forward_curves(&["TEST-FWD"]),
        );

        let config = TaylorAttributionConfig::default();
        let result = attribute_pnl_taylor(
            &instrument,
            &market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
            &config,
        )
        .expect("taylor attribution should succeed");

        assert!(
            result
                .factors
                .iter()
                .any(|f| f.factor_name.starts_with("Forward:")),
            "expected forward curve factor, got {:?}",
            result.factors
        );
    }
}
