//! Time-based window operations for expressions.
//!
//! This module provides support for time-based rolling windows (every="30d")
//! with explicit time column specification, enabling operations like
//! rolling_mean(..., every="7d", time_column="timestamp").

use std::collections::HashMap;
use time::Duration;

/// Parse a duration string like "30d", "1h", "2w" into time::Duration.
pub fn parse_duration(period: &str) -> Option<Duration> {
    if period.is_empty() {
        return None;
    }

    let (num_str, unit) = period.split_at(period.len() - 1);
    let num: i64 = num_str.parse().ok()?;

    match unit {
        "d" => Some(Duration::days(num)),
        "h" => Some(Duration::hours(num)),
        "m" => Some(Duration::minutes(num)),
        "s" => Some(Duration::seconds(num)),
        "w" => Some(Duration::weeks(num)),
        "M" => Some(Duration::days(num * 30)), // Approximate month
        "Y" => Some(Duration::days(num * 365)), // Approximate year
        _ => None,
    }
}

/// Time-based rolling window evaluator.
pub struct TimeWindowEvaluator {
    /// The time column data as Unix timestamps.
    pub time_data: Vec<i64>,
    /// Cache for window boundaries to avoid recomputation.
    boundary_cache: HashMap<(usize, String), (usize, usize)>,
}

impl TimeWindowEvaluator {
    /// Create a new time window evaluator with the given time column.
    #[allow(dead_code)]
    pub fn new(time_data: Vec<i64>) -> Self {
        Self {
            time_data,
            boundary_cache: HashMap::new(),
        }
    }

    /// Find the window boundaries for a given index and duration.
    #[allow(dead_code)]
    pub fn window_boundaries(&mut self, index: usize, period: &str) -> Option<(usize, usize)> {
        let cache_key = (index, period.to_string());
        if let Some(&bounds) = self.boundary_cache.get(&cache_key) {
            return Some(bounds);
        }

        let duration = parse_duration(period)?;
        if index >= self.time_data.len() {
            return None;
        }

        let current_time = self.time_data[index];
        let window_start = current_time - duration.whole_seconds();

        // Find the first index where time >= window_start
        let start_idx = self
            .time_data
            .iter()
            .position(|&t| t >= window_start)
            .unwrap_or(0);

        // End index is current index + 1 (exclusive)
        let end_idx = index + 1;

        let bounds = (start_idx, end_idx);
        self.boundary_cache.insert(cache_key, bounds);
        Some(bounds)
    }

    /// Compute rolling mean over a time window.
    #[allow(dead_code)]
    pub fn rolling_mean(&mut self, values: &[f64], period: &str) -> Vec<f64> {
        let mut result = Vec::with_capacity(values.len());

        for i in 0..values.len() {
            if let Some((start, end)) = self.window_boundaries(i, period) {
                let window_data = &values[start..end.min(values.len())];
                if window_data.is_empty() {
                    result.push(f64::NAN);
                } else {
                    let sum: f64 = window_data.iter().sum();
                    result.push(sum / window_data.len() as f64);
                }
            } else {
                result.push(f64::NAN);
            }
        }

        result
    }

    /// Compute rolling sum over a time window.
    #[allow(dead_code)]
    pub fn rolling_sum(&mut self, values: &[f64], period: &str) -> Vec<f64> {
        let mut result = Vec::with_capacity(values.len());

        for i in 0..values.len() {
            if let Some((start, end)) = self.window_boundaries(i, period) {
                let window_data = &values[start..end.min(values.len())];
                let sum: f64 = window_data.iter().sum();
                result.push(sum);
            } else {
                result.push(f64::NAN);
            }
        }

        result
    }

    /// Compute rolling standard deviation over a time window.
    #[allow(dead_code)]
    pub fn rolling_std(&mut self, values: &[f64], period: &str) -> Vec<f64> {
        let mut result = Vec::with_capacity(values.len());

        for i in 0..values.len() {
            if let Some((start, end)) = self.window_boundaries(i, period) {
                let window_data = &values[start..end.min(values.len())];
                if window_data.len() < 2 {
                    result.push(f64::NAN);
                } else {
                    let mean = window_data.iter().sum::<f64>() / window_data.len() as f64;
                    let variance = window_data.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                        / (window_data.len() - 1) as f64;
                    result.push(variance.sqrt());
                }
            } else {
                result.push(f64::NAN);
            }
        }

        result
    }

    /// Compute rolling variance over a time window.
    #[allow(dead_code)]
    pub fn rolling_var(&mut self, values: &[f64], period: &str) -> Vec<f64> {
        let mut result = Vec::with_capacity(values.len());

        for i in 0..values.len() {
            if let Some((start, end)) = self.window_boundaries(i, period) {
                let window_data = &values[start..end.min(values.len())];
                if window_data.len() < 2 {
                    result.push(f64::NAN);
                } else {
                    let mean = window_data.iter().sum::<f64>() / window_data.len() as f64;
                    let variance = window_data.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                        / (window_data.len() - 1) as f64;
                    result.push(variance);
                }
            } else {
                result.push(f64::NAN);
            }
        }

        result
    }

    /// Compute rolling median over a time window.
    #[allow(dead_code)]
    pub fn rolling_median(&mut self, values: &[f64], period: &str) -> Vec<f64> {
        let mut result = Vec::with_capacity(values.len());

        for i in 0..values.len() {
            if let Some((start, end)) = self.window_boundaries(i, period) {
                let mut window_data: Vec<f64> = values[start..end.min(values.len())].to_vec();
                if window_data.is_empty() {
                    result.push(f64::NAN);
                } else {
                    window_data.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let n = window_data.len();
                    let median = if n % 2 == 1 {
                        window_data[n / 2]
                    } else {
                        (window_data[n / 2 - 1] + window_data[n / 2]) * 0.5
                    };
                    result.push(median);
                }
            } else {
                result.push(f64::NAN);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1d"), Some(Duration::days(1)));
        assert_eq!(parse_duration("7d"), Some(Duration::days(7)));
        assert_eq!(parse_duration("2h"), Some(Duration::hours(2)));
        assert_eq!(parse_duration("30m"), Some(Duration::minutes(30)));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_time_window_evaluator() {
        // Create time data: timestamps every hour for 24 hours
        let start_time = 1640995200; // Jan 1, 2022 00:00:00 UTC
        let time_data: Vec<i64> = (0..24).map(|i| start_time + i * 3600).collect();

        let mut evaluator = TimeWindowEvaluator::new(time_data);

        // Test data: simple incremental values
        let values: Vec<f64> = (0..24).map(|i| i as f64).collect();

        // Test 4-hour rolling mean
        let rolling_mean = evaluator.rolling_mean(&values, "4h");

        // The first few values should be the mean of available data
        assert!(!rolling_mean[0].is_nan());
        assert!(!rolling_mean[4].is_nan());
        assert!(!rolling_mean[23].is_nan());
    }
}
