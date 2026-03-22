//! Statistical forecast helpers that produce deterministic sequences.
//!
//! Each algorithm consumes a pre-seeded pseudo-random number generator so that
//! repeated calls with identical parameters return the same series. This makes
//! them suitable for scenario analysis where reproducibility matters.
//!
//! Uses [`Pcg64Rng`] for production-quality random number generation.

use crate::error::{Error, Result};
use crate::types::{ForecastMethod, NodeId};
use finstack_core::dates::PeriodId;
use finstack_core::math::random::{Pcg64Rng, RandomNumberGenerator};
use indexmap::IndexMap;

/// Common parameters for statistical distribution forecasts.
struct DistributionParams {
    mean: f64,
    std_dev: f64,
    seed: u64,
}

fn build_rng(seed: u64, stream_id: Option<u64>) -> Pcg64Rng {
    match stream_id {
        Some(stream_id) => Pcg64Rng::new_with_stream(seed, stream_id),
        None => Pcg64Rng::new(seed),
    }
}

/// Deterministic 64-bit mix of a node identifier for Monte Carlo seeding.
///
/// Used to decorrelate independent stochastic forecasts across nodes while
/// keeping results reproducible for a given `(seed, path_offset, node_id)` tuple.
#[must_use]
pub(crate) fn stable_hash_u64(node_id: &str) -> u64 {
    node_id.as_bytes().iter().fold(0u64, |acc, &b| {
        acc.wrapping_mul(31).wrapping_add(u64::from(b))
    })
}

/// Parse a JSON seed as `u64`, accepting integer JSON numbers stored as floats
/// (e.g. `42.0`) when they represent exact integers.
pub(crate) fn parse_seed_json(value: &serde_json::Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        let f = value.as_f64()?;
        if !f.is_finite() || f.fract() != 0.0 || f < 0.0 || f > u64::MAX as f64 {
            return None;
        }
        Some(f as u64)
    })
}

/// Optional Monte Carlo correlation pair: `(peer_node_id, rho)` in `[-1, 1]`.
///
/// When both `correlation_with` and `correlation` are present in forecast params,
/// Monte Carlo evaluation samples shocks correlated with the peer node's standard
/// normal shocks (same forecast period). The peer node must be evaluated earlier in
/// the dependency order so its Z-scores are available.
pub(crate) fn parse_correlation_params(
    params: &IndexMap<String, serde_json::Value>,
) -> Result<Option<(String, f64)>> {
    let with = params
        .get("correlation_with")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let rho = params.get("correlation").and_then(|v| v.as_f64());

    match (with, rho) {
        (None, None) => Ok(None),
        (Some(peer), Some(rho)) => {
            if !rho.is_finite() || !(-1.0..=1.0).contains(&rho) {
                return Err(Error::forecast(format!(
                    "Monte Carlo 'correlation' must be finite and in [-1, 1], got {rho}"
                )));
            }
            Ok(Some((peer, rho)))
        }
        (None, Some(_)) | (Some(_), None) => Err(Error::forecast(
            "Monte Carlo correlation requires both 'correlation_with' (string) and \
             'correlation' (number in [-1, 1])"
                .to_string(),
        )),
    }
}

/// Extract distribution parameters from the params map.
///
/// Validates that mean, std_dev, and seed are present and valid.
fn extract_distribution_params(
    params: &IndexMap<String, serde_json::Value>,
    method_name: &str,
) -> Result<DistributionParams> {
    let mean = params.get("mean").and_then(|v| v.as_f64()).ok_or_else(|| {
        Error::forecast(format!(
            "Missing or invalid 'mean' parameter for {} forecast. \
             Expected a number.",
            method_name
        ))
    })?;

    let std_dev = params
        .get("std_dev")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            Error::forecast(format!(
                "Missing or invalid 'std_dev' parameter for {} forecast. \
                 Expected a positive number.",
                method_name
            ))
        })?;

    let seed = params
        .get("seed")
        .and_then(parse_seed_json)
        .ok_or_else(|| {
            Error::forecast(format!(
                "Missing or invalid 'seed' parameter for {} forecast. \
                 A non-negative integer seed is required for deterministic sampling (e.g., 42).",
                method_name
            ))
        })?;

    if std_dev < 0.0 {
        return Err(Error::forecast(format!(
            "Standard deviation must be non-negative, got {}",
            std_dev
        )));
    }
    if !mean.is_finite() || !std_dev.is_finite() {
        return Err(Error::forecast(format!(
            "{} forecast requires finite mean and std_dev",
            method_name
        )));
    }

    Ok(DistributionParams {
        mean,
        std_dev,
        seed,
    })
}

/// Normal distribution forecast (deterministic with seed).
///
/// Samples from a normal distribution N(mean, std_dev^2) for each forecast period.
///
/// # Arguments
///
/// * `base_value` - Unused for this method; included for API compatibility with
///   other forecast helpers
/// * `forecast_periods` - Periods to simulate
/// * `params` - JSON parameter map containing `mean`, `std_dev`, and `seed`
///
/// `mean` and `std_dev` are expressed in the same units as the returned
/// series. `seed` must be integer-like and is required for deterministic
/// sampling.
///
/// # Returns
///
/// Returns one simulated scalar per forecast period.
///
/// # Errors
///
/// Returns an error if the parameter map is incomplete, if `std_dev` is
/// negative, or if simulation produces a non-finite value.
///
/// # Example
///
/// ```rust
/// # use finstack_statements::forecast::normal_forecast;
/// # use finstack_core::dates::PeriodId;
/// # use indexmap::indexmap;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let periods = [
///     PeriodId::quarter(2025, 1),
///     PeriodId::quarter(2025, 2),
///     PeriodId::quarter(2025, 3),
/// ];
/// let params = indexmap! {
///     "mean".to_string() => serde_json::json!(100_000.0),
///     "std_dev".to_string() => serde_json::json!(15_000.0),
///     "seed".to_string() => serde_json::json!(42_u64),
/// };
/// let simulated = normal_forecast(0.0, &periods, &params)?;
/// assert_eq!(simulated.len(), periods.len());
/// # Ok(())
/// # }
/// ```
///
/// # References
///
/// - Monte Carlo simulation practice: `docs/REFERENCES.md#glasserman-2004-monte-carlo`
/// - Numerical sampling techniques: `docs/REFERENCES.md#press-numerical-recipes`
pub fn normal_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    normal_forecast_with_stream(_base_value, forecast_periods, params, None)
}

pub(crate) fn normal_forecast_with_stream(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
    stream_id: Option<u64>,
) -> Result<IndexMap<PeriodId, f64>> {
    let p = extract_distribution_params(params, "Normal")?;

    let mut rng = build_rng(p.seed, stream_id);
    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        let z = rng.normal(0.0, 1.0);
        let value = p.mean + p.std_dev * z;
        if !value.is_finite() {
            return Err(Error::forecast(format!(
                "Normal forecast produced a non-finite value at period {:?}",
                period_id
            )));
        }
        results.insert(*period_id, value);
    }

    Ok(results)
}

/// Log-normal distribution forecast (deterministic with seed).
///
/// Samples from a log-normal distribution. All values are positive.
///
/// # Arguments
///
/// * `base_value` - Unused for this method; included for API compatibility with
///   other forecast helpers
/// * `forecast_periods` - Periods to simulate
/// * `params` - JSON parameter map containing `mean`, `std_dev`, and `seed`
///
/// `mean` and `std_dev` describe the underlying normal distribution, so the
/// returned series is always positive after exponentiation.
///
/// # Returns
///
/// Returns one positive simulated scalar per forecast period.
///
/// # Errors
///
/// Returns an error if the parameter map is incomplete, if `std_dev` is
/// negative, or if exponentiation produces a non-finite value.
///
/// # Example
///
/// ```rust
/// # use finstack_statements::forecast::lognormal_forecast;
/// # use finstack_core::dates::PeriodId;
/// # use indexmap::indexmap;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let periods = [
///     PeriodId::quarter(2025, 1),
///     PeriodId::quarter(2025, 2),
/// ];
/// let params = indexmap! {
///     "mean".to_string() => serde_json::json!(11.5),
///     "std_dev".to_string() => serde_json::json!(0.15),
///     "seed".to_string() => serde_json::json!(42_u64),
/// };
/// let simulated = lognormal_forecast(0.0, &periods, &params)?;
/// assert!(simulated.values().all(|v| *v > 0.0));
/// # Ok(())
/// # }
/// ```
///
/// # References
///
/// - Monte Carlo simulation practice: `docs/REFERENCES.md#glasserman-2004-monte-carlo`
/// - Numerical sampling techniques: `docs/REFERENCES.md#press-numerical-recipes`
pub fn lognormal_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    lognormal_forecast_with_stream(_base_value, forecast_periods, params, None)
}

pub(crate) fn lognormal_forecast_with_stream(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
    stream_id: Option<u64>,
) -> Result<IndexMap<PeriodId, f64>> {
    let p = extract_distribution_params(params, "LogNormal")?;

    // Warn on degenerate distribution (all values will be identical)
    if p.std_dev == 0.0 {
        tracing::warn!(
            "LogNormal forecast with std_dev=0.0 produces degenerate distribution (all values identical)"
        );
    }

    let mut rng = build_rng(p.seed, stream_id);
    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        let z = rng.normal(0.0, 1.0);
        let normal_value = p.mean + p.std_dev * z;
        // Exponentiate to get log-normal
        let value = normal_value.exp();
        if !value.is_finite() {
            return Err(Error::forecast(format!(
                "LogNormal forecast produced a non-finite value at period {:?}",
                period_id
            )));
        }
        results.insert(*period_id, value);
    }

    Ok(results)
}

/// Store standard-normal Z scores for independent Monte Carlo forecasts so peers can
/// correlate in a later [`crate::evaluator::forecast_eval::evaluate_forecast`] pass.
pub(crate) fn record_independent_z_scores_for_mc(
    method: ForecastMethod,
    params: &IndexMap<String, serde_json::Value>,
    forecast_periods: &[PeriodId],
    values: &IndexMap<PeriodId, f64>,
    node_id: &NodeId,
    mc_z_cache: &mut IndexMap<NodeId, IndexMap<PeriodId, f64>>,
) -> Result<()> {
    match method {
        ForecastMethod::Normal => {
            let p = extract_distribution_params(params, "Normal")?;
            let entry = mc_z_cache.entry(node_id.clone()).or_default();
            for pid in forecast_periods {
                let v = values.get(pid).ok_or_else(|| {
                    Error::forecast(format!(
                        "Monte Carlo forecast missing value for period {:?}",
                        pid
                    ))
                })?;
                let z = if p.std_dev == 0.0 {
                    0.0
                } else {
                    (*v - p.mean) / p.std_dev
                };
                entry.insert(*pid, z);
            }
        }
        ForecastMethod::LogNormal => {
            let p = extract_distribution_params(params, "LogNormal")?;
            let entry = mc_z_cache.entry(node_id.clone()).or_default();
            for pid in forecast_periods {
                let v = values.get(pid).ok_or_else(|| {
                    Error::forecast(format!(
                        "Monte Carlo forecast missing value for period {:?}",
                        pid
                    ))
                })?;
                let z = if p.std_dev == 0.0 {
                    0.0
                } else {
                    let ln_v = (*v).ln();
                    (ln_v - p.mean) / p.std_dev
                };
                entry.insert(*pid, z);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Inputs for [`monte_carlo_correlated_series`].
pub(crate) struct CorrelatedMonteCarloSeries<'a> {
    /// Forecast method (Normal or LogNormal only).
    pub method: ForecastMethod,
    /// Method parameters.
    pub params: &'a IndexMap<String, serde_json::Value>,
    pub forecast_periods: &'a [PeriodId],
    pub seed_offset: u64,
    pub node_id: &'a str,
    pub peer_id: &'a str,
    pub rho: f64,
    pub mc_z_cache: &'a IndexMap<NodeId, IndexMap<PeriodId, f64>>,
}

/// Correlated Normal / LogNormal series for Monte Carlo: `Z` uses
/// `ρ·Z_peer + sqrt(1-ρ²)·Z_indep` per forecast period.
pub(crate) fn monte_carlo_correlated_series(
    input: CorrelatedMonteCarloSeries<'_>,
) -> Result<(IndexMap<PeriodId, f64>, IndexMap<PeriodId, f64>)> {
    let CorrelatedMonteCarloSeries {
        method,
        params,
        forecast_periods,
        seed_offset,
        node_id,
        peer_id,
        rho,
        mc_z_cache,
    } = input;

    let peer_key = NodeId::new(peer_id);
    let peer_map = mc_z_cache.get(&peer_key).ok_or_else(|| {
        Error::forecast(format!(
            "Monte Carlo correlation peer '{peer_id}' must be evaluated before node '{node_id}' \
             (no Z-scores in cache for peer)"
        ))
    })?;

    let p = match method {
        ForecastMethod::Normal => extract_distribution_params(params, "Normal")?,
        ForecastMethod::LogNormal => extract_distribution_params(params, "LogNormal")?,
        _ => {
            return Err(Error::forecast(
                "Monte Carlo correlation is only supported for Normal and LogNormal forecasts"
                    .to_string(),
            ));
        }
    };

    let mut rng = Pcg64Rng::new_with_stream(p.seed ^ stable_hash_u64(node_id), seed_offset);
    let mut values = IndexMap::new();
    let mut z_out = IndexMap::new();

    for period_id in forecast_periods {
        let z_peer = peer_map.get(period_id).copied().ok_or_else(|| {
            Error::forecast(format!(
                "Monte Carlo correlation: peer '{peer_id}' has no Z-score for period {:?}. \
                 Ensure the peer forecast covers the same forecast periods.",
                period_id
            ))
        })?;

        let z_indep = rng.normal(0.0, 1.0);
        let z = rho * z_peer + (1.0 - rho * rho).sqrt() * z_indep;
        z_out.insert(*period_id, z);

        let value = if matches!(method, ForecastMethod::Normal) {
            let v = p.mean + p.std_dev * z;
            if !v.is_finite() {
                return Err(Error::forecast(format!(
                    "Normal forecast produced a non-finite value at period {:?}",
                    period_id
                )));
            }
            v
        } else {
            let normal_value = p.mean + p.std_dev * z;
            let v = normal_value.exp();
            if !v.is_finite() {
                return Err(Error::forecast(format!(
                    "LogNormal forecast produced a non-finite value at period {:?}",
                    period_id
                )));
            }
            v
        };
        values.insert(*period_id, value);
    }

    Ok((values, z_out))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_parse_seed_accepts_integer_like_json_float() {
        let v = serde_json::json!(42.0);
        assert_eq!(parse_seed_json(&v), Some(42));
    }

    #[test]
    fn test_normal_forecast_deterministic() {
        let periods = vec![PeriodId::quarter(2025, 1), PeriodId::quarter(2025, 2)];

        let mut params = IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(100_000.0));
        params.insert("std_dev".to_string(), serde_json::json!(15_000.0));
        params.insert("seed".to_string(), serde_json::json!(42));

        let results1 =
            normal_forecast(0.0, &periods, &params).expect("normal_forecast should succeed");
        let results2 =
            normal_forecast(0.0, &periods, &params).expect("normal_forecast should succeed");

        // Same seed should produce identical results
        assert_eq!(
            results1[&PeriodId::quarter(2025, 1)],
            results2[&PeriodId::quarter(2025, 1)]
        );
        assert_eq!(
            results1[&PeriodId::quarter(2025, 2)],
            results2[&PeriodId::quarter(2025, 2)]
        );
    }

    #[test]
    fn test_normal_forecast_different_seeds() {
        let periods = vec![PeriodId::quarter(2025, 1)];

        let mut params1 = IndexMap::new();
        params1.insert("mean".to_string(), serde_json::json!(100_000.0));
        params1.insert("std_dev".to_string(), serde_json::json!(15_000.0));
        params1.insert("seed".to_string(), serde_json::json!(42));

        let mut params2 = IndexMap::new();
        params2.insert("mean".to_string(), serde_json::json!(100_000.0));
        params2.insert("std_dev".to_string(), serde_json::json!(15_000.0));
        params2.insert("seed".to_string(), serde_json::json!(43));

        let results1 =
            normal_forecast(0.0, &periods, &params1).expect("normal_forecast should succeed");
        let results2 =
            normal_forecast(0.0, &periods, &params2).expect("normal_forecast should succeed");

        // Different seeds should produce different results
        assert_ne!(
            results1[&PeriodId::quarter(2025, 1)],
            results2[&PeriodId::quarter(2025, 1)]
        );
    }

    #[test]
    fn test_normal_forecast_missing_parameters() {
        let periods = vec![PeriodId::quarter(2025, 1)];

        // Missing mean
        let mut params = IndexMap::new();
        params.insert("std_dev".to_string(), serde_json::json!(15_000.0));
        params.insert("seed".to_string(), serde_json::json!(42));
        assert!(normal_forecast(0.0, &periods, &params).is_err());

        // Missing std_dev
        let mut params = IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(100_000.0));
        params.insert("seed".to_string(), serde_json::json!(42));
        assert!(normal_forecast(0.0, &periods, &params).is_err());

        // Missing seed
        let mut params = IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(100_000.0));
        params.insert("std_dev".to_string(), serde_json::json!(15_000.0));
        assert!(normal_forecast(0.0, &periods, &params).is_err());
    }

    #[test]
    fn test_lognormal_forecast_always_positive() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
            PeriodId::quarter(2025, 4),
        ];

        let mut params = IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(11.5));
        params.insert("std_dev".to_string(), serde_json::json!(0.15));
        params.insert("seed".to_string(), serde_json::json!(42));

        let results =
            lognormal_forecast(0.0, &periods, &params).expect("lognormal_forecast should succeed");

        // All values should be positive
        for value in results.values() {
            assert!(*value > 0.0);
        }
    }

    #[test]
    fn test_lognormal_forecast_deterministic() {
        let periods = vec![PeriodId::quarter(2025, 1)];

        let mut params = IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(11.5));
        params.insert("std_dev".to_string(), serde_json::json!(0.15));
        params.insert("seed".to_string(), serde_json::json!(42));

        let results1 =
            lognormal_forecast(0.0, &periods, &params).expect("lognormal_forecast should succeed");
        let results2 =
            lognormal_forecast(0.0, &periods, &params).expect("lognormal_forecast should succeed");

        // Same seed should produce identical results
        assert_eq!(
            results1[&PeriodId::quarter(2025, 1)],
            results2[&PeriodId::quarter(2025, 1)]
        );
    }

    #[test]
    fn test_lognormal_forecast_rejects_non_finite_output() {
        let periods = vec![PeriodId::quarter(2025, 1)];

        let mut params = IndexMap::new();
        params.insert("mean".to_string(), serde_json::json!(1000.0));
        params.insert("std_dev".to_string(), serde_json::json!(0.0));
        params.insert("seed".to_string(), serde_json::json!(42));

        let result = lognormal_forecast(0.0, &periods, &params);
        assert!(result.is_err(), "overflowing lognormal paths must fail");
    }
}
