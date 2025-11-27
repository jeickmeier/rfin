//! Statistical forecast helpers that produce deterministic sequences.
//!
//! Each algorithm consumes a pre-seeded pseudo-random number generator so that
//! repeated calls with identical parameters return the same series. This makes
//! them suitable for scenario analysis where reproducibility matters.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use finstack_core::math::random::{RandomNumberGenerator, TestRng};
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
    // Extract parameters
    let mean = params.get("mean").and_then(|v| v.as_f64()).ok_or_else(|| {
        Error::forecast(
            "Missing or invalid 'mean' parameter for Normal forecast. \
             Expected a number (e.g., 100000.0).",
        )
    })?;

    let std_dev = params
        .get("std_dev")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            Error::forecast(
                "Missing or invalid 'std_dev' parameter for Normal forecast. \
                 Expected a positive number (e.g., 15000.0).",
            )
        })?;

    let seed = params.get("seed").and_then(|v| v.as_u64()).ok_or_else(|| {
        Error::forecast(
            "Missing or invalid 'seed' parameter for Normal forecast. \
             A seed is required for deterministic sampling (e.g., 42).",
        )
    })?;

    if std_dev < 0.0 {
        return Err(Error::forecast(format!(
            "Standard deviation must be non-negative, got {}",
            std_dev
        )));
    }

    // Initialize RNG with seed
    // Note: TestRng is for deterministic testing; for production Monte Carlo,
    // implement RandomNumberGenerator with a robust RNG (e.g., PCG64)
    let mut rng = TestRng::new(seed);

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
    // Extract parameters
    let mean = params.get("mean").and_then(|v| v.as_f64()).ok_or_else(|| {
        Error::forecast(
            "Missing or invalid 'mean' parameter for LogNormal forecast. \
             Expected a number (e.g., 11.5).",
        )
    })?;

    let std_dev = params
        .get("std_dev")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            Error::forecast(
                "Missing or invalid 'std_dev' parameter for LogNormal forecast. \
                 Expected a positive number (e.g., 0.15).",
            )
        })?;

    let seed = params.get("seed").and_then(|v| v.as_u64()).ok_or_else(|| {
        Error::forecast(
            "Missing or invalid 'seed' parameter for LogNormal forecast. \
             A seed is required for deterministic sampling (e.g., 42).",
        )
    })?;

    if std_dev < 0.0 {
        return Err(Error::forecast(format!(
            "Standard deviation must be non-negative, got {}",
            std_dev
        )));
    }

    // Initialize RNG with seed
    // Note: TestRng is for deterministic testing; for production Monte Carlo,
    // implement RandomNumberGenerator with a robust RNG (e.g., PCG64)
    let mut rng = TestRng::new(seed);

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
