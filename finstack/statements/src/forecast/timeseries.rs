//! Time-series forecasting methods with trend detection and seasonal decomposition.

use crate::error::{Error, Result};
use crate::types::SeasonalMode;
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Apply time-series forecasting to generate future values.
///
/// Supports multiple methods:
/// - Linear trend detection
/// - Exponential smoothing  
/// - Moving average
///
/// # Parameters
///
/// * `historical` - Array of historical values for trend detection (required)
/// * `method` - "linear", "exponential", "moving_average" (default: "linear")
/// * `alpha` - Smoothing factor for exponential method (0-1, required for exponential)
/// * `beta` - Trend smoothing factor for exponential method (0-1, required for exponential)
/// * `window` - Window size for moving average (required for moving_average)
pub fn timeseries_forecast(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Get historical data
    let historical = params
        .get("historical")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::forecast("'historical' must be an array"))?;

    if historical.len() < 2 {
        return Err(Error::forecast(format!(
            "Need at least 2 historical periods for trend detection, got {}. \
             Provide more historical data in the 'historical' parameter.",
            historical.len()
        )));
    }

    // Convert to f64 array
    let hist_data: Vec<f64> = historical.iter().filter_map(|v| v.as_f64()).collect();

    if hist_data.len() < 2 {
        return Err(Error::forecast(format!(
            "Historical data must contain valid numbers. Got {} valid values out of {} total. \
             Ensure all historical values are valid numbers.",
            hist_data.len(),
            historical.len()
        )));
    }

    // Get method (default to "linear")
    let method = params
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("linear");

    let mut result = IndexMap::new();

    match method {
        "linear" => {
            // Linear trend using least squares
            let (slope, intercept) = calculate_linear_trend(&hist_data);
            let n_hist = hist_data.len() as f64;

            for (i, period_id) in forecast_periods.iter().enumerate() {
                let t = n_hist + i as f64 + 1.0;
                let value = slope * t + intercept;
                result.insert(*period_id, value);
            }
        }

        "exponential" => {
            // Double exponential smoothing (Holt's method)
            // Both parameters are required per market standards (no universal defaults)
            let alpha = params
                .get("alpha")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| {
                    Error::forecast(
                        "'alpha' parameter required for exponential smoothing. \
                     Typical range: 0.05 (slow/stable) to 0.3 (fast/responsive). \
                     Industry standard: alpha = 2/(n+1) where n is smoothing window. \
                     Example: alpha = 0.2 for moderate smoothing.",
                    )
                })?;

            let beta = params.get("beta").and_then(|v| v.as_f64()).ok_or_else(|| {
                Error::forecast(
                    "'beta' parameter required for exponential smoothing trend. \
                     Typical range: 0.05 (slow trend) to 0.2 (fast trend). \
                     Should typically be less than alpha. \
                     Example: beta = 0.1 for moderate trend responsiveness.",
                )
            })?;

            // Validate parameter ranges
            if !(0.0..=1.0).contains(&alpha) {
                return Err(Error::forecast(format!(
                    "alpha must be in (0, 1), got {}. Typical values: 0.05 to 0.3",
                    alpha
                )));
            }
            if !(0.0..=1.0).contains(&beta) {
                return Err(Error::forecast(format!(
                    "beta must be in (0, 1), got {}. Typical values: 0.05 to 0.2",
                    beta
                )));
            }

            let (level, trend) = double_exponential_smoothing(&hist_data, alpha, beta);

            for (i, period_id) in forecast_periods.iter().enumerate() {
                let value = level + trend * (i + 1) as f64;
                result.insert(*period_id, value);
            }
        }

        "moving_average" => {
            // Simple moving average with trend extrapolation
            let window = params
                .get("window")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| {
                    Error::forecast(
                        "'window' parameter required for moving average forecast. \
                     Common values: 3 (short-term), 5-10 (medium-term), 20+ (long-term). \
                     Must be less than the number of historical periods. \
                     Example: window = 3 for 3-period moving average.",
                    )
                })? as usize;

            if window == 0 {
                return Err(Error::forecast("window must be greater than 0"));
            }
            if window > hist_data.len() {
                return Err(Error::forecast(format!(
                    "window ({}) cannot exceed historical data length ({})",
                    window,
                    hist_data.len()
                )));
            }

            let window = window.min(hist_data.len());

            // Calculate moving average of last 'window' periods
            let ma: f64 = hist_data.iter().rev().take(window).sum::<f64>() / window as f64;

            // Calculate trend from moving averages
            if hist_data.len() > window {
                let prev_ma: f64 = hist_data[..hist_data.len() - 1]
                    .iter()
                    .rev()
                    .take(window)
                    .sum::<f64>()
                    / window as f64;
                let trend = ma - prev_ma;

                for (i, period_id) in forecast_periods.iter().enumerate() {
                    let value = ma + trend * (i + 1) as f64;
                    result.insert(*period_id, value);
                }
            } else {
                // No trend, use constant MA
                for period_id in forecast_periods {
                    result.insert(*period_id, ma);
                }
            }
        }

        _ => {
            return Err(Error::forecast(format!(
                "Unknown time series method: {}",
                method
            )));
        }
    }

    Ok(result)
}

/// Calculate linear trend using least squares regression
fn calculate_linear_trend(data: &[f64]) -> (f64, f64) {
    let n = data.len() as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xx = 0.0;
    let mut sum_xy = 0.0;

    for (i, &y) in data.iter().enumerate() {
        let x = (i + 1) as f64;
        sum_x += x;
        sum_y += y;
        sum_xx += x * x;
        sum_xy += x * y;
    }

    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
    let intercept = (sum_y - slope * sum_x) / n;

    (slope, intercept)
}

/// Double exponential smoothing (Holt's method)
fn double_exponential_smoothing(data: &[f64], alpha: f64, beta: f64) -> (f64, f64) {
    if data.is_empty() {
        return (0.0, 0.0);
    }

    let mut level = data[0];
    let mut trend = if data.len() > 1 {
        data[1] - data[0]
    } else {
        0.0
    };

    for &value in data.iter().skip(1) {
        let prev_level = level;
        level = alpha * value + (1.0 - alpha) * (level + trend);
        trend = beta * (level - prev_level) + (1.0 - beta) * trend;
    }

    (level, trend)
}

/// Apply a seasonal forecast with decomposition.
///
/// Uses seasonal decomposition of historical data to extract trend, seasonal,
/// and residual components, then projects forward.
///
/// # Parameters
///
/// * `historical` - Array of historical values (need 2+ seasons, required)
/// * `season_length` - Length of seasonal cycle (required)
/// * `growth` - Growth rate to apply to trend (default: 0.0)
/// * `mode` - SeasonalMode enum: "additive" or "multiplicative" (required)
///
/// Note: `base_value` is provided for API parity with other forecast methods but
/// is not used in the seasonal calculation—the historical series establishes
/// the level for projections.
pub fn seasonal_forecast(
    base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // `base_value` is currently ignored for seasonal forecasts—historical data provides the level.
    seasonal_forecast_with_decomposition(base_value, forecast_periods, params)
}

/// Apply seasonal forecasting with decomposition.
///
/// This is the main implementation of seasonal forecasting, requiring historical
/// data and explicit parameters for decomposition.
fn seasonal_forecast_with_decomposition(
    _base_value: f64,
    forecast_periods: &[PeriodId],
    params: &IndexMap<String, serde_json::Value>,
) -> Result<IndexMap<PeriodId, f64>> {
    // Get historical data
    let historical = params
        .get("historical")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::forecast("'historical' must be an array"))?;

    // Convert to f64 array
    let hist_data: Vec<f64> = historical.iter().filter_map(|v| v.as_f64()).collect();

    // Get season length (required parameter - no default per market standards)
    let season_length = params
        .get("season_length")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            Error::forecast(
                "'season_length' parameter required for seasonal decomposition. \
             Common values: 4 (quarterly), 12 (monthly). \
             Must match the cyclical pattern in your data.",
            )
        })? as usize;

    if hist_data.len() < season_length * 2 {
        return Err(Error::forecast(format!(
            "Need at least 2 full seasons of historical data for seasonal decomposition. \
             Season length: {}, need {} periods, got {}. \
             Provide more historical data or reduce season_length.",
            season_length,
            season_length * 2,
            hist_data.len()
        )));
    }

    // Decompose the series
    let (trend, seasonal, _residual) = decompose_series(&hist_data, season_length);

    // Get growth rate for trend projection
    let growth = params.get("growth").and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Get mode (type-safe enum)
    let mode = params.get("mode").ok_or_else(|| {
        Error::forecast(
            "'mode' parameter required for seasonal forecast. \
             Must be either 'additive' or 'multiplicative'.",
        )
    })?;
    let mode: SeasonalMode = serde_json::from_value(mode.clone()).map_err(|_| {
        Error::forecast("Invalid 'mode' parameter. Must be 'additive' or 'multiplicative'.")
    })?;

    // Project forward
    let mut results = IndexMap::new();
    let last_trend = trend
        .last()
        .copied()
        .unwrap_or_else(|| hist_data.last().copied().unwrap_or(0.0));

    for (i, period_id) in forecast_periods.iter().enumerate() {
        // Calculate trend component with growth
        let trend_value = last_trend * (1.0 + growth).powi(i as i32 + 1);

        // Get seasonal component (cycle through pattern)
        let season_idx = (hist_data.len() + i) % season_length;
        let seasonal_value = seasonal.get(season_idx).copied().unwrap_or(0.0);

        // Combine based on mode (type-safe match)
        let value = match mode {
            SeasonalMode::Additive => trend_value + seasonal_value,
            SeasonalMode::Multiplicative => {
                // For multiplicative, seasonal is a factor
                let seasonal_factor = if trend[0] != 0.0 {
                    1.0 + seasonal_value / trend[0]
                } else {
                    1.0
                };
                trend_value * seasonal_factor
            }
        };

        // Allow negative values for financial metrics (losses, declines, etc.)
        results.insert(*period_id, value);
    }

    Ok(results)
}

/// Decompose a time series into trend, seasonal, and residual components.
/// Uses a simple moving average approach.
fn decompose_series(data: &[f64], season_length: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = data.len();

    // Calculate trend using centered moving average
    let mut trend = vec![0.0; n];
    let half_season = season_length / 2;

    // Use centered moving average for middle values
    for (i, trend_val) in trend
        .iter_mut()
        .enumerate()
        .take(n.saturating_sub(half_season))
        .skip(half_season)
    {
        let window_start = i.saturating_sub(half_season);
        let window_end = (i + half_season + 1).min(n);
        let sum: f64 = data[window_start..window_end].iter().sum();
        *trend_val = sum / (window_end - window_start) as f64;
    }

    // Extrapolate trend to edges using linear extrapolation
    if n > season_length {
        // Front extrapolation
        let slope_front = if trend[half_season + 1] != 0.0 {
            trend[half_season + 1] - trend[half_season]
        } else {
            0.0
        };
        for i in 0..half_season {
            trend[i] = trend[half_season] - slope_front * (half_season - i) as f64;
        }

        // Back extrapolation
        let last_valid = n.saturating_sub(half_season + 1);
        let slope_back = if last_valid > 0 && trend[last_valid] != trend[last_valid - 1] {
            trend[last_valid] - trend[last_valid - 1]
        } else {
            0.0
        };
        for i in (n.saturating_sub(half_season))..n {
            let steps = i - (n - half_season - 1);
            trend[i] = trend[n - half_season - 1] + slope_back * steps as f64;
        }
    } else {
        // For short series, use simple average as trend
        let avg = data.iter().sum::<f64>() / n as f64;
        for trend_val in trend.iter_mut().take(n) {
            *trend_val = avg;
        }
    }

    // Calculate detrended series
    let detrended: Vec<f64> = data.iter().zip(&trend).map(|(d, t)| d - t).collect();

    // Calculate seasonal component (average of same season across years)
    let mut seasonal = vec![0.0; season_length];
    for (season, seasonal_val) in seasonal.iter_mut().enumerate().take(season_length) {
        let mut sum = 0.0;
        let mut count = 0;

        let mut idx = season;
        while idx < n {
            sum += detrended[idx];
            count += 1;
            idx += season_length;
        }

        if count > 0 {
            *seasonal_val = sum / count as f64;
        }
    }

    // Normalize seasonal component (sum to zero for additive)
    let seasonal_mean: f64 = seasonal.iter().sum::<f64>() / season_length as f64;
    for s in &mut seasonal {
        *s -= seasonal_mean;
    }

    // Calculate residual
    let mut residual = vec![0.0; n];
    for i in 0..n {
        let season_idx = i % season_length;
        residual[i] = data[i] - trend[i] - seasonal[season_idx];
    }

    (trend, seasonal, residual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn test_timeseries_forecast() {
        let params = indexmap! {
            "historical".into() => serde_json::json!([100.0, 110.0, 120.0, 130.0]),
            "method".into() => serde_json::json!("linear"),
        };

        let periods = vec![PeriodId::quarter(2025, 1), PeriodId::quarter(2025, 2)];

        let result = timeseries_forecast(100.0, &periods, &params)
            .expect("timeseries_forecast should succeed");

        // Should continue linear trend
        assert!(result[&PeriodId::quarter(2025, 1)] > 130.0);
        assert!(result[&PeriodId::quarter(2025, 2)] > result[&PeriodId::quarter(2025, 1)]);
    }

    #[test]
    fn test_seasonal_forecast_with_historical() {
        // Create historical data with seasonal pattern (2 years of quarterly data)
        let historical = vec![
            100.0, 90.0, 110.0, 85.0, // Year 1
            105.0, 95.0, 115.0, 90.0, // Year 2
        ];

        let params = indexmap! {
            "historical".into() => serde_json::json!(historical),
            "season_length".into() => serde_json::json!(4),
            "mode".into() => serde_json::json!("multiplicative"),
            "growth".into() => serde_json::json!(0.02),
        };

        let periods = vec![
            PeriodId::quarter(2025, 1),
            PeriodId::quarter(2025, 2),
            PeriodId::quarter(2025, 3),
            PeriodId::quarter(2025, 4),
        ];

        let result =
            seasonal_forecast(100.0, &periods, &params).expect("seasonal_forecast should succeed");

        // Should produce forecasts for all 4 quarters with seasonal pattern
        assert!(result.len() == 4);

        // Check that growth is applied
        let q1 = result[&PeriodId::quarter(2025, 1)];
        let q2 = result[&PeriodId::quarter(2025, 2)];
        assert!(q1 > 0.0, "Q1 should be positive");
        assert!(q2 > 0.0, "Q2 should be positive");
    }

    #[test]
    fn test_linear_trend() {
        let data = vec![10.0, 20.0, 30.0, 40.0];
        let (slope, intercept) = calculate_linear_trend(&data);

        // Perfect linear trend: y = 10x + 0
        assert_eq!(slope, 10.0);
        assert_eq!(intercept, 0.0);
    }

    #[test]
    fn test_exponential_smoothing() {
        let data = vec![100.0, 110.0, 120.0, 130.0];
        let (level, trend) = double_exponential_smoothing(&data, 0.5, 0.5);

        // Should detect upward trend
        assert!(level > 120.0);
        assert!(trend > 0.0);
    }

    #[test]
    fn test_seasonal_decomposition() {
        // Create data with clear seasonal pattern
        let data = vec![
            100.0, 90.0, 110.0, 85.0, // Year 1
            105.0, 95.0, 115.0, 90.0, // Year 2
            110.0, 100.0, 120.0, 95.0, // Year 3
        ];

        let (trend, seasonal, _residual) = decompose_series(&data, 4);

        // Check that decomposition produces reasonable results
        assert_eq!(
            trend.len(),
            data.len(),
            "Trend should have same length as data"
        );
        assert_eq!(seasonal.len(), 4, "Seasonal should have 4 components");

        // Check that seasonal components sum to approximately zero (for additive)
        let seasonal_sum: f64 = seasonal.iter().sum();
        assert!(
            seasonal_sum.abs() < 1.0,
            "Seasonal components should sum to near zero"
        );

        // There should be some variation in seasonal components
        let seasonal_max = seasonal.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let seasonal_min = seasonal.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        assert!(
            seasonal_max > seasonal_min,
            "Seasonal should have variation"
        );
    }
}
