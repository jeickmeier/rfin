//! Statistical forecast methods with deterministic seeding.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use finstack_core::math::random::{RandomNumberGenerator, SimpleRng};
use indexmap::IndexMap;

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
/// ```
/// // mean = 100_000, std_dev = 15_000, seed = 42
/// // Generates deterministic samples from N(100_000, 15_000^2)
/// ```
pub fn normal_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Extract parameters
    let mean = params.get("mean").and_then(|v| v.as_f64()).ok_or_else(|| {
        Error::Forecast("Missing or invalid 'mean' parameter for Normal forecast".to_string())
    })?;

    let std_dev = params
        .get("std_dev")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            Error::Forecast(
                "Missing or invalid 'std_dev' parameter for Normal forecast".to_string(),
            )
        })?;

    let seed = params.get("seed").and_then(|v| v.as_u64()).ok_or_else(|| {
        Error::Forecast(
            "Missing or invalid 'seed' parameter for Normal forecast (required for determinism)"
                .to_string(),
        )
    })?;

    if std_dev < 0.0 {
        return Err(Error::Forecast(
            "Standard deviation must be non-negative".to_string(),
        ));
    }

    // Initialize RNG with seed
    let mut rng = SimpleRng::new(seed);

    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        // Box-Muller transform for normal distribution
        let u1 = rng.uniform();
        let u2 = rng.uniform();

        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let value = mean + std_dev * z;

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
/// ```
/// // mean = 11.5, std_dev = 0.15, seed = 42
/// // Generates deterministic samples from LogNormal(11.5, 0.15^2)
/// // Results are always positive (suitable for revenue, prices)
/// ```
pub fn lognormal_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Extract parameters
    let mean = params.get("mean").and_then(|v| v.as_f64()).ok_or_else(|| {
        Error::Forecast("Missing or invalid 'mean' parameter for LogNormal forecast".to_string())
    })?;

    let std_dev = params
        .get("std_dev")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            Error::Forecast(
                "Missing or invalid 'std_dev' parameter for LogNormal forecast".to_string(),
            )
        })?;

    let seed = params.get("seed").and_then(|v| v.as_u64()).ok_or_else(|| {
        Error::Forecast(
            "Missing or invalid 'seed' parameter for LogNormal forecast (required for determinism)"
                .to_string(),
        )
    })?;

    if std_dev < 0.0 {
        return Err(Error::Forecast(
            "Standard deviation must be non-negative".to_string(),
        ));
    }

    // Initialize RNG with seed
    let mut rng = SimpleRng::new(seed);

    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        // Box-Muller transform for normal distribution
        let u1 = rng.uniform();
        let u2 = rng.uniform();

        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let normal_value = mean + std_dev * z;

        // Exponentiate to get log-normal
        let value = normal_value.exp();

        results.insert(*period_id, value);
    }

    Ok(results)
}

#[cfg(test)]
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

        let results1 = normal_forecast(0.0, &periods, &params).unwrap();
        let results2 = normal_forecast(0.0, &periods, &params).unwrap();

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

        let results1 = normal_forecast(0.0, &periods, &params1).unwrap();
        let results2 = normal_forecast(0.0, &periods, &params2).unwrap();

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

        let results = lognormal_forecast(0.0, &periods, &params).unwrap();

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

        let results1 = lognormal_forecast(0.0, &periods, &params).unwrap();
        let results2 = lognormal_forecast(0.0, &periods, &params).unwrap();

        // Same seed should produce identical results
        assert_eq!(
            results1[&PeriodId::quarter(2025, 1)],
            results2[&PeriodId::quarter(2025, 1)]
        );
    }
}
