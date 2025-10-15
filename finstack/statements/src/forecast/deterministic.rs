//! Deterministic forecast algorithms for projecting values across periods.
//!
//! The helpers in this module operate on raw numeric series and return
//! [`IndexMap`]s keyed by [`PeriodId`]. They are building blocks used by the
//! higher-level forecast engine but can also be invoked directly in custom
//! workflows or extensions.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Forward fill: carry the last observed value into every forecast period.
///
/// # Arguments
///
/// * `base_value` - Value to repeat
/// * `forecast_periods` - Periods that require projected values
///
/// # Example
///
/// ```rust
/// # use finstack_statements::forecast::forward_fill;
/// # use finstack_core::dates::PeriodId;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let periods = [
///     PeriodId::quarter(2025, 3),
///     PeriodId::quarter(2025, 4),
/// ];
/// let projected = forward_fill(125.0, &periods)?;
/// assert_eq!(projected[&periods[0]], 125.0);
/// assert_eq!(projected[&periods[1]], 125.0);
/// # Ok(())
/// # }
/// ```
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

/// Growth percentage: apply a constant compound growth rate each period.
///
/// The recurrence relation is `v[t] = v[t-1] * (1 + rate)`, where `rate`
/// represents the fractional growth between consecutive periods.
///
/// # Parameters
///
/// * `rate` - Growth rate per period (e.g., 0.05 for 5% growth)
///
/// # Example
///
/// ```rust
/// # use finstack_statements::forecast::growth_pct;
/// # use finstack_core::dates::PeriodId;
/// # use indexmap::indexmap;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let periods = [
///     PeriodId::quarter(2025, 1),
///     PeriodId::quarter(2025, 2),
///     PeriodId::quarter(2025, 3),
/// ];
/// let params = indexmap! { "rate".to_string() => serde_json::json!(0.05) };
/// let projected = growth_pct(100.0, &periods, &params)?;
/// assert!((projected[&periods[2]] - 115.7625).abs() < 1e-6);
/// # Ok(())
/// # }
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

/// Curve percentage: apply period-specific growth rates supplied as a curve.
///
/// The recurrence relation is `v[t] = v[t-1] * (1 + curve[t])`, where `curve`
/// contains the growth factor for the *t*-th forecast period.
///
/// # Parameters
///
/// * `curve` - Array of growth rates, one per forecast period
///
/// # Example
///
/// ```rust
/// # use finstack_statements::forecast::curve_pct;
/// # use finstack_core::dates::PeriodId;
/// # use indexmap::indexmap;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let periods = [
///     PeriodId::quarter(2025, 1),
///     PeriodId::quarter(2025, 2),
///     PeriodId::quarter(2025, 3),
/// ];
/// let params = indexmap! {
///     "curve".to_string() => serde_json::json!([0.05, 0.06, 0.05])
/// };
/// let projected = curve_pct(100.0, &periods, &params)?;
/// assert!((projected[&periods[2]] - 116.865).abs() < 1e-6);
/// # Ok(())
/// # }
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
