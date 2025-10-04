//! Override forecast method for explicit period values.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Override: Use explicit period-specific values, forward fill for remaining periods.
///
/// # Parameters
///
/// * `overrides` - Map of period_id string → value
///
/// # Example
///
/// ```
/// // base_value = 100
/// // overrides = {"2025Q1": 120, "2025Q3": 130}
/// // Period 2025Q1: 120 (override)
/// // Period 2025Q2: 120 (forward fill from Q1)
/// // Period 2025Q3: 130 (override)
/// // Period 2025Q4: 130 (forward fill from Q3)
/// ```
pub fn apply_override(
    base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Extract overrides parameter
    let overrides_json = params.get("overrides").ok_or_else(|| {
        Error::forecast(
            "Missing 'overrides' parameter for Override method. \
             Expected a JSON object mapping period IDs to values (e.g., {\"2025Q2\": 120000}).",
        )
    })?;

    let overrides_map: IndexMap<String, f64> = serde_json::from_value(overrides_json.clone())
        .map_err(|e| {
            Error::forecast(format!(
                "Invalid 'overrides' parameter: expected map of period_id → value. Error: {}",
                e
            ))
        })?;

    let mut results = IndexMap::new();
    let mut current_value = base_value;

    for period_id in forecast_periods {
        // Check if there's an override for this period
        let period_str = period_id.to_string();
        if let Some(&override_value) = overrides_map.get(&period_str) {
            current_value = override_value;
        }
        // Otherwise forward fill current_value

        results.insert(*period_id, current_value);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_override_all_periods() {
        let periods = vec![PeriodId::quarter(2025, 1), PeriodId::quarter(2025, 2)];

        let mut params = IndexMap::new();
        let overrides = serde_json::json!({
            "2025Q1": 120.0,
            "2025Q2": 130.0,
        });
        params.insert("overrides".to_string(), overrides);

        let results = apply_override(100.0, &periods, &params).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[&PeriodId::quarter(2025, 1)], 120.0);
        assert_eq!(results[&PeriodId::quarter(2025, 2)], 130.0);
    }

    #[test]
    fn test_override_sparse_with_forward_fill() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
            PeriodId::quarter(2025, 4),
        ];

        let mut params = IndexMap::new();
        let overrides = serde_json::json!({
            "2025Q1": 120.0,
            "2025Q3": 130.0,
        });
        params.insert("overrides".to_string(), overrides);

        let results = apply_override(100.0, &periods, &params).unwrap();

        assert_eq!(results.len(), 4);
        assert_eq!(results[&PeriodId::quarter(2025, 1)], 120.0);
        assert_eq!(results[&PeriodId::quarter(2025, 2)], 120.0); // Forward fill from Q1
        assert_eq!(results[&PeriodId::quarter(2025, 3)], 130.0);
        assert_eq!(results[&PeriodId::quarter(2025, 4)], 130.0); // Forward fill from Q3
    }

    #[test]
    fn test_override_no_overrides_forward_fill_base() {
        let periods = vec![PeriodId::quarter(2025, 1), PeriodId::quarter(2025, 2)];

        let mut params = IndexMap::new();
        let overrides = serde_json::json!({});
        params.insert("overrides".to_string(), overrides);

        let results = apply_override(100.0, &periods, &params).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[&PeriodId::quarter(2025, 1)], 100.0);
        assert_eq!(results[&PeriodId::quarter(2025, 2)], 100.0);
    }

    #[test]
    fn test_override_missing_parameter_error() {
        let periods = vec![PeriodId::quarter(2025, 1)];
        let params = IndexMap::new();

        let result = apply_override(100.0, &periods, &params);
        assert!(result.is_err());
    }
}
