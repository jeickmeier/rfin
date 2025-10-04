//! Time series and seasonal forecast methods.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Apply a time series forecast based on external data.
///
/// The time series method allows referencing external data sources
/// for forecasting. The external data should be provided as a map
/// of period_id to values in the params.
///
/// # Parameters
///
/// * `series` - JSON object mapping period IDs to values
/// * `default` - Optional default value for periods not in the series
pub fn timeseries_forecast(
    base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Get the time series data from params
    let series = params
        .get("series")
        .ok_or_else(|| Error::forecast("TimeSeries requires 'series' parameter"))?
        .as_object()
        .ok_or_else(|| Error::forecast("'series' parameter must be a JSON object"))?;
    
    // Get optional default value
    let default_value = params
        .get("default")
        .and_then(|v| v.as_f64())
        .unwrap_or(base_value);
    
    let mut results = IndexMap::new();
    
    for period_id in forecast_periods {
        // Look up value in the series
        let period_str = format!("{}", period_id);
        let value = series
            .get(&period_str)
            .and_then(|v| v.as_f64())
            .unwrap_or(default_value);
        
        results.insert(*period_id, value);
    }
    
    Ok(results)
}

/// Apply a seasonal forecast pattern.
///
/// The seasonal method applies a repeating pattern to forecast values.
/// It can be either additive (base + seasonal factor) or 
/// multiplicative (base * seasonal factor).
///
/// # Parameters
///
/// * `pattern` - Array of seasonal factors (length determines season length)
/// * `mode` - "additive" or "multiplicative" (default: "multiplicative")
/// * `growth` - Optional growth rate to apply on top of seasonality
pub fn seasonal_forecast(
    base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Get the seasonal pattern
    let pattern = params
        .get("pattern")
        .ok_or_else(|| Error::forecast("Seasonal requires 'pattern' parameter"))?
        .as_array()
        .ok_or_else(|| Error::forecast("'pattern' parameter must be an array"))?;
    
    if pattern.is_empty() {
        return Err(Error::forecast("Seasonal pattern cannot be empty"));
    }
    
    // Parse pattern values
    let pattern_values: Vec<f64> = pattern
        .iter()
        .map(|v| {
            v.as_f64()
                .ok_or_else(|| Error::forecast("Pattern values must be numbers"))
        })
        .collect::<Result<Vec<_>>>()?;
    
    // Get mode (additive or multiplicative)
    let mode = params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("multiplicative");
    
    // Get optional growth rate
    let growth_rate = params
        .get("growth")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let mut results = IndexMap::new();
    let mut current_base = base_value;
    
    for (i, period_id) in forecast_periods.iter().enumerate() {
        // Apply growth
        if i > 0 && growth_rate != 0.0 {
            current_base *= 1.0 + growth_rate;
        }
        
        // Get seasonal factor
        let seasonal_factor = pattern_values[i % pattern_values.len()];
        
        // Apply seasonal pattern
        let value = match mode {
            "additive" => current_base + seasonal_factor,
            "multiplicative" => current_base * seasonal_factor,
            _ => {
                return Err(Error::forecast(
                    "Seasonal mode must be 'additive' or 'multiplicative'",
                ))
            }
        };
        
        results.insert(*period_id, value);
    }
    
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_timeseries_forecast() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
            PeriodId::quarter(2025, 4),
        ];

        let mut params = IndexMap::new();
        params.insert(
            "series".into(),
            json!({
                "2025Q1": 100000.0,
                "2025Q2": 105000.0,
                "2025Q3": 110000.0,
                "2025Q4": 115000.0,
            }),
        );
        params.insert("default".into(), json!(100000.0));

        let result = timeseries_forecast(90000.0, &periods, &params).unwrap();

        assert_eq!(result[&PeriodId::quarter(2025, 1)], 100000.0);
        assert_eq!(result[&PeriodId::quarter(2025, 2)], 105000.0);
        assert_eq!(result[&PeriodId::quarter(2025, 3)], 110000.0);
        assert_eq!(result[&PeriodId::quarter(2025, 4)], 115000.0);
    }

    #[test]
    fn test_seasonal_multiplicative() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
            PeriodId::quarter(2025, 4),
        ];

        let mut params = IndexMap::new();
        params.insert("pattern".into(), json!([1.1, 1.2, 0.9, 0.8]));
        params.insert("mode".into(), json!("multiplicative"));

        let result = seasonal_forecast(100000.0, &periods, &params).unwrap();

        assert!((result[&PeriodId::quarter(2025, 1)] - 110000.0).abs() < 0.01); // 100000 * 1.1
        assert!((result[&PeriodId::quarter(2025, 2)] - 120000.0).abs() < 0.01); // 100000 * 1.2
        assert!((result[&PeriodId::quarter(2025, 3)] - 90000.0).abs() < 0.01); // 100000 * 0.9
        assert!((result[&PeriodId::quarter(2025, 4)] - 80000.0).abs() < 0.01); // 100000 * 0.8
    }

    #[test]
    fn test_seasonal_additive() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
        ];

        let mut params = IndexMap::new();
        params.insert("pattern".into(), json!([5000.0, -5000.0]));
        params.insert("mode".into(), json!("additive"));

        let result = seasonal_forecast(100000.0, &periods, &params).unwrap();

        assert_eq!(result[&PeriodId::quarter(2025, 1)], 105000.0); // 100000 + 5000
        assert_eq!(result[&PeriodId::quarter(2025, 2)], 95000.0); // 100000 - 5000
    }

    #[test]
    fn test_seasonal_with_growth() {
        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
        ];

        let mut params = IndexMap::new();
        params.insert("pattern".into(), json!([1.0, 1.0])); // No seasonal variation
        params.insert("mode".into(), json!("multiplicative"));
        params.insert("growth".into(), json!(0.1)); // 10% growth per period

        let result = seasonal_forecast(100000.0, &periods, &params).unwrap();

        assert!((result[&PeriodId::quarter(2025, 1)] - 100000.0).abs() < 0.01); // Base * 1.0
        assert!((result[&PeriodId::quarter(2025, 2)] - 110000.0).abs() < 0.01); // Base * 1.1 * 1.0
        assert!((result[&PeriodId::quarter(2025, 3)] - 121000.0).abs() < 0.01); // Base * 1.1 * 1.1 * 1.0
    }
}
