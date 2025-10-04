//! Deterministic forecast methods.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Forward fill: Carry the last value forward to all forecast periods.
///
/// # Example
///
/// If base_value = 100, all forecast periods will have value 100.
pub fn forward_fill(
    base_value: f64,
    forecast_periods: &[PeriodId],
) -> Result<IndexMap<PeriodId, f64>> {
    let mut results = IndexMap::new();

    for period_id in forecast_periods {
        results.insert(*period_id, base_value);
    }

    Ok(results)
}

/// Growth percentage: Apply compound growth rate.
///
/// Formula: `v[t] = v[t-1] * (1 + rate)`
///
/// # Parameters
///
/// * `rate` - Growth rate per period (e.g., 0.05 for 5% growth)
///
/// # Example
///
/// ```
/// // base_value = 100, rate = 0.05
/// // Period 1: 100 * 1.05 = 105
/// // Period 2: 105 * 1.05 = 110.25
/// // Period 3: 110.25 * 1.05 = 115.76
/// ```
pub fn growth_pct(
    base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Extract rate parameter
    let rate = params.get("rate").and_then(|v| v.as_f64()).ok_or_else(|| {
        Error::forecast(
            "Missing or invalid 'rate' parameter for GrowthPct forecast. \
             Expected a number (e.g., 0.05 for 5% growth).",
        )
    })?;

    let mut results = IndexMap::new();
    let mut current_value = base_value;

    for period_id in forecast_periods {
        current_value *= 1.0 + rate;
        results.insert(*period_id, current_value);
    }

    Ok(results)
}

/// Curve percentage: Apply period-specific growth rates from a curve.
///
/// Formula: `v[t] = v[t-1] * (1 + curve[t])`
///
/// # Parameters
///
/// * `curve` - Array of growth rates, one per forecast period
///
/// # Example
///
/// ```
/// // base_value = 100, curve = [0.05, 0.06, 0.05]
/// // Period 1: 100 * 1.05 = 105
/// // Period 2: 105 * 1.06 = 111.3
/// // Period 3: 111.3 * 1.05 = 116.865
/// ```
pub fn curve_pct(
    base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Extract curve parameter
    let curve_json = params.get("curve").ok_or_else(|| {
        Error::forecast(
            "Missing 'curve' parameter for CurvePct forecast. \
             Expected an array of growth rates (e.g., [0.05, 0.06, 0.05]).",
        )
    })?;

    let curve: Vec<f64> = serde_json::from_value(curve_json.clone()).map_err(|e| {
        Error::forecast(format!(
            "Invalid 'curve' parameter: expected array of numbers. Error: {}",
            e
        ))
    })?;

    if curve.len() != forecast_periods.len() {
        return Err(Error::forecast(format!(
            "Curve length ({}) does not match number of forecast periods ({}). \
             Provide exactly one growth rate per forecast period.",
            curve.len(),
            forecast_periods.len()
        )));
    }

    let mut results = IndexMap::new();
    let mut current_value = base_value;

    for (i, period_id) in forecast_periods.iter().enumerate() {
        current_value *= 1.0 + curve[i];
        results.insert(*period_id, current_value);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_forward_fill() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
        ];

        let results = forward_fill(100.0, &periods).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[&PeriodId::quarter(2025, 1)], 100.0);
        assert_eq!(results[&PeriodId::quarter(2025, 2)], 100.0);
        assert_eq!(results[&PeriodId::quarter(2025, 3)], 100.0);
    }

    #[test]
    fn test_growth_pct() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
        ];

        let mut params = IndexMap::new();
        params.insert("rate".to_string(), serde_json::json!(0.05));

        let results = growth_pct(100.0, &periods, &params).unwrap();

        assert_eq!(results.len(), 3);
        assert!((results[&PeriodId::quarter(2025, 1)] - 105.0).abs() < 0.01);
        assert!((results[&PeriodId::quarter(2025, 2)] - 110.25).abs() < 0.01);
        assert!((results[&PeriodId::quarter(2025, 3)] - 115.7625).abs() < 0.01);
    }

    #[test]
    fn test_growth_pct_negative() {
        let periods = vec![PeriodId::quarter(2025, 1)];

        let mut params = IndexMap::new();
        params.insert("rate".to_string(), serde_json::json!(-0.1)); // -10% decline

        let results = growth_pct(100.0, &periods, &params).unwrap();

        assert!((results[&PeriodId::quarter(2025, 1)] - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_growth_pct_missing_rate_error() {
        let periods = vec![PeriodId::quarter(2025, 1)];
        let params = IndexMap::new();

        let result = growth_pct(100.0, &periods, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_curve_pct() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
        ];

        let mut params = IndexMap::new();
        params.insert("curve".to_string(), serde_json::json!([0.05, 0.06, 0.05]));

        let results = curve_pct(100.0, &periods, &params).unwrap();

        assert_eq!(results.len(), 3);
        assert!((results[&PeriodId::quarter(2025, 1)] - 105.0).abs() < 0.01);
        assert!((results[&PeriodId::quarter(2025, 2)] - 111.3).abs() < 0.01);
        assert!((results[&PeriodId::quarter(2025, 3)] - 116.865).abs() < 0.01);
    }

    #[test]
    fn test_curve_pct_length_mismatch_error() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
        ];

        let mut params = IndexMap::new();
        params.insert("curve".to_string(), serde_json::json!([0.05, 0.06])); // Too short

        let result = curve_pct(100.0, &periods, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_curve_pct_missing_curve_error() {
        let periods = vec![PeriodId::quarter(2025, 1)];
        let params = IndexMap::new();

        let result = curve_pct(100.0, &periods, &params);
        assert!(result.is_err());
    }
}
