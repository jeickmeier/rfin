//! Statistical forecast helpers that produce deterministic sequences.
//!
//! Each algorithm consumes a pre-seeded pseudo-random number generator so that
//! repeated calls with identical parameters return the same series. This makes
//! them suitable for scenario analysis where reproducibility matters.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use finstack_core::math::random::{RandomNumberGenerator, TestRng};
use indexmap::IndexMap;

/// Common parameters for statistical distribution forecasts.
struct DistributionParams {
    mean: f64,
    std_dev: f64,
    seed: u64,
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

    let seed = params.get("seed").and_then(|v| v.as_u64()).ok_or_else(|| {
        Error::forecast(format!(
            "Missing or invalid 'seed' parameter for {} forecast. \
             A seed is required for deterministic sampling (e.g., 42).",
            method_name
        ))
    })?;

    if std_dev < 0.0 {
        return Err(Error::forecast(format!(
            "Standard deviation must be non-negative, got {}",
            std_dev
        )));
    }

    Ok(DistributionParams {
        mean,
        std_dev,
        seed,
    })
}

/// Box-Muller transform for generating a standard normal sample.
///
/// Uses two uniform samples to produce a normally distributed value.
/// Guards against u1=0.0 which would cause ln(0) = -infinity.
fn box_muller_sample(rng: &mut TestRng) -> f64 {
    let u1 = rng.uniform().max(f64::MIN_POSITIVE);
    let u2 = rng.uniform();
    (-2.0_f64 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// Normal distribution forecast (deterministic with seed).
///
/// Samples from a normal distribution N(mean, std_dev^2) for each forecast period.
///
/// # Parameters
///
/// * `mean` - Mean of the distribution
/// * `std_dev` - Standard deviation
/// * `seed` - Random seed for deterministic sampling (required)
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
pub fn normal_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    let p = extract_distribution_params(params, "Normal")?;

    // Note: TestRng is for deterministic testing; for production Monte Carlo,
    // implement RandomNumberGenerator with a robust RNG (e.g., PCG64)
    let mut rng = TestRng::new(p.seed);
    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        let z = box_muller_sample(&mut rng);
        let value = p.mean + p.std_dev * z;
        results.insert(*period_id, value);
    }

    Ok(results)
}

/// Log-normal distribution forecast (deterministic with seed).
///
/// Samples from a log-normal distribution. All values are positive.
///
/// # Parameters
///
/// * `mean` - Mean of the underlying normal distribution
/// * `std_dev` - Standard deviation of the underlying normal distribution
/// * `seed` - Random seed for deterministic sampling (required)
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
pub fn lognormal_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    let p = extract_distribution_params(params, "LogNormal")?;

    // Warn on degenerate distribution (all values will be identical)
    if p.std_dev == 0.0 {
        log::warn!(
            "LogNormal forecast with std_dev=0.0 produces degenerate distribution (all values identical)"
        );
    }

    // Note: TestRng is for deterministic testing; for production Monte Carlo,
    // implement RandomNumberGenerator with a robust RNG (e.g., PCG64)
    let mut rng = TestRng::new(p.seed);
    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        let z = box_muller_sample(&mut rng);
        let normal_value = p.mean + p.std_dev * z;
        // Exponentiate to get log-normal
        let value = normal_value.exp();
        results.insert(*period_id, value);
    }

    Ok(results)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use finstack_core::dates::PeriodId;

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
}
