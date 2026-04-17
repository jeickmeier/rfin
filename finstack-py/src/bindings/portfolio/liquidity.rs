//! Python bindings for the `finstack-portfolio::liquidity` submodule.
//!
//! Exposes a function-based API for market microstructure liquidity modeling:
//! spread estimation (Roll, Amihud), liquidity-adjusted VaR (Bangia et al.),
//! market impact (Almgren-Chriss, Kyle), and tier classification.
//!
//! Inputs are plain `Vec<f64>` so callers can pass numpy arrays directly
//! (PyO3 converts automatically). Results are returned as `PyDict`s rather
//! than opaque `#[pyclass]` wrappers to keep the API numpy-friendly.

use crate::errors::display_to_py;
use finstack_core::math::special_functions::standard_normal_inv_cdf;
use finstack_portfolio::liquidity::{
    self, AlmgrenChrissModel, KyleLambdaModel, LiquidityProfile, MarketImpactModel, TradeParams,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ---------------------------------------------------------------------------
// Spread / illiquidity estimators
// ---------------------------------------------------------------------------

/// Estimate the effective bid-ask spread via Roll's (1984) serial covariance
/// estimator.
///
/// Under Roll's model, observed returns are the sum of an efficient-price
/// innovation and a bid-ask bounce component, giving
/// ``effective_spread = 2 * sqrt(-Cov(r_t, r_{t-1}))``.
///
/// Parameters
/// ----------
/// returns : list[float]
///     Log or arithmetic returns, length >= 2.
///
/// Returns
/// -------
/// float
///     Effective spread in the same units as the returns, or ``NaN`` when
///     the serial covariance is non-negative (violates the Roll assumption)
///     or when ``len(returns) < 2``.
#[pyfunction]
fn roll_effective_spread(returns: Vec<f64>) -> f64 {
    liquidity::roll_effective_spread(&returns).unwrap_or(f64::NAN)
}

/// Compute the Amihud (2002) illiquidity ratio from returns and volumes.
///
/// ``ILLIQ = mean(|r_t| / Volume_t)``. Higher values indicate less liquid
/// instruments (more price impact per unit of volume).
///
/// Parameters
/// ----------
/// returns : list[float]
///     Period returns (absolute value taken internally).
/// volumes : list[float]
///     Period trading volumes, same length as ``returns``. All entries must
///     be strictly positive.
///
/// Returns
/// -------
/// float
///     Average daily illiquidity ratio. Returns ``NaN`` if inputs are empty,
///     mismatched in length, non-finite, or contain a zero/negative volume.
#[pyfunction]
fn amihud_illiquidity(returns: Vec<f64>, volumes: Vec<f64>) -> f64 {
    liquidity::amihud_illiquidity(&returns, &volumes).unwrap_or(f64::NAN)
}

// ---------------------------------------------------------------------------
// Position sizing / tiering
// ---------------------------------------------------------------------------

/// Days required to liquidate a dollar-denominated position at the given
/// participation rate.
///
/// ``days = position_value / (avg_daily_volume * participation_rate)``. Both
/// ``position_value`` and ``avg_daily_volume`` must be in the same units
/// (e.g., USD notional).
///
/// Parameters
/// ----------
/// position_value : float
///     Position size in currency units (absolute value used).
/// avg_daily_volume : float
///     Average daily traded volume in matching currency units.
/// participation_rate : float
///     Fraction of ADV that can be traded per day, typically 0.05 to 0.25.
///
/// Returns
/// -------
/// float
///     Trading days to fully liquidate. ``inf`` if ADV or participation rate
///     is non-positive.
#[pyfunction]
fn days_to_liquidate(position_value: f64, avg_daily_volume: f64, participation_rate: f64) -> f64 {
    liquidity::days_to_liquidate(position_value, avg_daily_volume, participation_rate)
}

/// Classify a position into a liquidity tier from its days-to-liquidate.
///
/// Uses the default :class:`LiquidityConfig` thresholds of
/// ``[1.0, 5.0, 20.0, 60.0]`` trading days.
///
/// Parameters
/// ----------
/// days_to_liquidate : float
///     Estimated trading days required to fully unwind the position.
///
/// Returns
/// -------
/// str
///     One of ``"tier1"``, ``"tier2"``, ``"tier3"``, ``"tier4"``, ``"tier5"``
///     with Tier 1 most liquid and Tier 5 least liquid.
#[pyfunction]
fn liquidity_tier(days_to_liquidate: f64) -> String {
    let thresholds = [1.0, 5.0, 20.0, 60.0];
    let tier = liquidity::classify_tier(days_to_liquidate, &thresholds);
    match tier {
        liquidity::LiquidityTier::Tier1 => "tier1".to_string(),
        liquidity::LiquidityTier::Tier2 => "tier2".to_string(),
        liquidity::LiquidityTier::Tier3 => "tier3".to_string(),
        liquidity::LiquidityTier::Tier4 => "tier4".to_string(),
        liquidity::LiquidityTier::Tier5 => "tier5".to_string(),
    }
}

// ---------------------------------------------------------------------------
// LVaR (Bangia et al. 1999)
// ---------------------------------------------------------------------------

/// Liquidity-adjusted VaR following Bangia, Diebold, Schuermann & Stroughair (1999).
///
/// Uses the loss sign convention: VaR and LVaR are non-positive numbers.
///
/// ``LVaR = VaR - (0.5 * spread_mean + z_alpha * 0.5 * spread_vol) * position_value``
///
/// The ``spread_cost`` add-on is returned as a non-negative magnitude.
///
/// Parameters
/// ----------
/// var : float
///     Standard VaR for the position following the loss sign convention
///     (non-positive number; ``-10_000.0`` means a $10,000 loss). ``0.0`` is
///     accepted for a zero-risk position.
/// spread_mean : float
///     Mean relative bid-ask spread over the lookback window, e.g. ``0.001``
///     for 10bp.
/// spread_vol : float
///     Relative spread volatility (standard deviation of relative spread).
/// confidence : float
///     Confidence level in ``(0, 1)``, e.g. ``0.99``.
/// position_value : float
///     Market value of the position (sign ignored; only magnitude is used).
///
/// Returns
/// -------
/// dict
///     ``{var, spread_cost, lvar, lvar_ratio}`` where ``spread_cost`` is a
///     non-negative magnitude, ``lvar <= var <= 0``, and ``lvar_ratio =
///     lvar / var`` (or ``NaN`` if VaR is zero).
#[pyfunction]
#[pyo3(signature = (var, spread_mean, spread_vol, confidence, position_value))]
fn lvar_bangia<'py>(
    py: Python<'py>,
    var: f64,
    spread_mean: f64,
    spread_vol: f64,
    confidence: f64,
    position_value: f64,
) -> PyResult<Bound<'py, PyDict>> {
    if !var.is_finite() || var > 0.0 {
        return Err(PyValueError::new_err(format!(
            "var must be non-positive and finite (loss sign convention), got {var}"
        )));
    }
    if !spread_mean.is_finite() || spread_mean < 0.0 {
        return Err(PyValueError::new_err(
            "spread_mean must be non-negative and finite",
        ));
    }
    if !spread_vol.is_finite() || spread_vol < 0.0 {
        return Err(PyValueError::new_err(
            "spread_vol must be non-negative and finite",
        ));
    }
    if !(0.0..1.0).contains(&confidence) || !confidence.is_finite() {
        return Err(PyValueError::new_err(
            "confidence must be in the open interval (0, 1)",
        ));
    }
    if !position_value.is_finite() {
        return Err(PyValueError::new_err("position_value must be finite"));
    }

    let pv = position_value.abs();
    let z_alpha = standard_normal_inv_cdf(confidence);
    let spread_cost = (0.5 * spread_mean + 0.5 * z_alpha * spread_vol) * pv;
    let lvar = var - spread_cost;
    let lvar_ratio = if var.abs() > 1e-15 {
        lvar / var
    } else {
        f64::NAN
    };

    let out = PyDict::new(py);
    out.set_item("var", var)?;
    out.set_item("spread_cost", spread_cost)?;
    out.set_item("lvar", lvar)?;
    out.set_item("lvar_ratio", lvar_ratio)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Market impact (Almgren-Chriss)
// ---------------------------------------------------------------------------

/// Almgren-Chriss (2001) market impact decomposition for a uniform execution
/// over a fixed horizon.
///
/// Parameters
/// ----------
/// position_size : float
///     Total quantity to execute in shares/contracts (sign is preserved but
///     cost is symmetric in size).
/// avg_daily_volume : float
///     Average daily volume in shares/contracts (must be positive).
/// volatility : float
///     Daily return volatility (e.g., ``0.02`` for 2%).
/// execution_horizon_days : float
///     Execution horizon in trading days (must be positive).
/// permanent_impact_coef : float
///     Permanent impact coefficient gamma. Non-negative.
/// temporary_impact_coef : float
///     Temporary impact coefficient eta. Strictly positive.
///
/// Returns
/// -------
/// dict
///     ``{permanent_impact, temporary_impact, total_impact, expected_cost_bps}``
///     where impacts are expressed as currency costs and ``expected_cost_bps``
///     is the total as basis points of notional value (mid price computed
///     internally as a placeholder of 100.0; the bps figure scales with
///     size * mid).
#[pyfunction]
#[pyo3(signature = (
    position_size,
    avg_daily_volume,
    volatility,
    execution_horizon_days,
    permanent_impact_coef,
    temporary_impact_coef,
))]
fn almgren_chriss_impact<'py>(
    py: Python<'py>,
    position_size: f64,
    avg_daily_volume: f64,
    volatility: f64,
    execution_horizon_days: f64,
    permanent_impact_coef: f64,
    temporary_impact_coef: f64,
) -> PyResult<Bound<'py, PyDict>> {
    if !avg_daily_volume.is_finite() || avg_daily_volume <= 0.0 {
        return Err(PyValueError::new_err(
            "avg_daily_volume must be finite and positive",
        ));
    }
    if !volatility.is_finite() || volatility <= 0.0 {
        return Err(PyValueError::new_err(
            "volatility must be finite and positive",
        ));
    }

    // Delta fixed at 0.5 (standard square-root market impact).
    let model = AlmgrenChrissModel::new(permanent_impact_coef, temporary_impact_coef, 0.5)
        .map_err(display_to_py)?;

    // Use a unit-price liquidity profile so cost_bps is computed relative to
    // a canonical notional. Callers can rescale externally if needed. We pick
    // a mid of 1.0 so that `notional = |Q| * 1.0 = |Q|`, and all returned
    // currency figures are directly comparable to `position_size`.
    let profile = LiquidityProfile::new(
        "AC_CALIBRATION",
        1.0,
        0.999,
        1.001,
        avg_daily_volume,
        1.0,
        0.0,
    )
    .map_err(display_to_py)?;

    let params = TradeParams {
        quantity: position_size,
        horizon_days: execution_horizon_days,
        daily_volatility: volatility,
        profile,
        risk_aversion: None,
    };
    let est = model.estimate_cost(&params).map_err(display_to_py)?;

    let out = PyDict::new(py);
    out.set_item("permanent_impact", est.permanent_impact)?;
    out.set_item("temporary_impact", est.temporary_impact)?;
    out.set_item("total_impact", est.total_cost)?;
    out.set_item("expected_cost_bps", est.cost_bps)?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Kyle's lambda
// ---------------------------------------------------------------------------

/// Estimate Kyle's (1985) linear price impact coefficient lambda from
/// observed volumes and returns.
///
/// Uses the Amihud-ratio proxy: ``lambda = mean(|r_t| / V_t) * mean(V_t)``.
/// Under the Kyle model, price impact per trade is ``lambda * signed_volume``.
///
/// Parameters
/// ----------
/// volumes : list[float]
///     Period trading volumes, strictly positive.
/// returns : list[float]
///     Period returns, same length as ``volumes``.
///
/// Returns
/// -------
/// float
///     Estimated Kyle lambda. Returns ``NaN`` if inputs are invalid (empty,
///     mismatched length, non-finite, or contain zero volumes).
#[pyfunction]
fn kyle_lambda(volumes: Vec<f64>, returns: Vec<f64>) -> f64 {
    if volumes.is_empty() || volumes.len() != returns.len() {
        return f64::NAN;
    }
    let illiq = match liquidity::amihud_illiquidity(&returns, &volumes) {
        Some(v) => v,
        None => return f64::NAN,
    };
    let mean_vol: f64 = volumes.iter().sum::<f64>() / volumes.len() as f64;
    if !mean_vol.is_finite() || mean_vol <= 0.0 {
        return f64::NAN;
    }
    match KyleLambdaModel::from_amihud(illiq, mean_vol) {
        Ok(m) => m.lambda(),
        Err(_) => f64::NAN,
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register liquidity-risk functions on the portfolio submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(roll_effective_spread, m)?)?;
    m.add_function(wrap_pyfunction!(amihud_illiquidity, m)?)?;
    m.add_function(wrap_pyfunction!(days_to_liquidate, m)?)?;
    m.add_function(wrap_pyfunction!(liquidity_tier, m)?)?;
    m.add_function(wrap_pyfunction!(lvar_bangia, m)?)?;
    m.add_function(wrap_pyfunction!(almgren_chriss_impact, m)?)?;
    m.add_function(wrap_pyfunction!(kyle_lambda, m)?)?;
    Ok(())
}
