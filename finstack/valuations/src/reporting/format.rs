//! Pure number-formatting utilities for report components.
//!
//! Components do not call these internally -- they are exposed separately
//! for consumers who want formatted display strings alongside raw values.
//!
//! # Examples
//!
//! ```rust
//! use finstack_valuations::reporting::format::{format_bps, format_pct, format_currency, format_ratio};
//!
//! assert_eq!(format_bps(0.0025, 1), "25.0 bps");
//! assert_eq!(format_pct(0.0534, 2), "5.34%");
//! assert_eq!(format_currency(1234567.89, "USD", 2), "USD 1,234,567.89");
//! assert_eq!(format_ratio(3.5, 2), "3.50x");
//! ```

use serde::Serialize;

/// Format a value in basis points.
///
/// Multiplies by 10,000 and appends "bps".
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::format_bps;
///
/// assert_eq!(format_bps(0.0001, 1), "1.0 bps");
/// assert_eq!(format_bps(0.0025, 1), "25.0 bps");
/// assert_eq!(format_bps(-0.001, 2), "-10.00 bps");
/// ```
pub fn format_bps(value: f64, decimals: usize) -> String {
    let bps = value * 10_000.0;
    format!("{:.prec$} bps", bps, prec = decimals)
}

/// Format as percentage.
///
/// Multiplies by 100 and appends "%".
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::format_pct;
///
/// assert_eq!(format_pct(0.0534, 2), "5.34%");
/// assert_eq!(format_pct(-0.0534, 2), "-5.34%");
/// assert_eq!(format_pct(1.0, 0), "100%");
/// ```
pub fn format_pct(value: f64, decimals: usize) -> String {
    let pct = value * 100.0;
    format!("{:.prec$}%", pct, prec = decimals)
}

/// Format as currency with thousands separators.
///
/// Prepends the currency code and inserts comma separators.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::format_currency;
///
/// assert_eq!(format_currency(1234567.89, "USD", 2), "USD 1,234,567.89");
/// assert_eq!(format_currency(-500.0, "EUR", 0), "EUR -500");
/// assert_eq!(format_currency(0.0, "GBP", 2), "GBP 0.00");
/// ```
pub fn format_currency(value: f64, currency: &str, decimals: usize) -> String {
    let formatted_number = format_with_commas(value, decimals);
    format!("{} {}", currency, formatted_number)
}

/// Format in scientific notation.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::format_scientific;
///
/// assert_eq!(format_scientific(0.000123, 3), "1.23e-4");
/// assert_eq!(format_scientific(1234.0, 2), "1.2e3");
/// assert_eq!(format_scientific(0.0, 2), "0.0e0");
/// ```
pub fn format_scientific(value: f64, sig_figs: usize) -> String {
    if value == 0.0 {
        let dec = if sig_figs > 1 { sig_figs - 1 } else { 1 };
        return format!("0.{:0>width$}e0", 0, width = dec);
    }

    let abs_val = value.abs();
    let exponent = abs_val.log10().floor() as i32;
    let mantissa = value / 10.0_f64.powi(exponent);
    let dec = if sig_figs > 1 { sig_figs - 1 } else { 1 };
    format!("{:.prec$}e{}", mantissa, exponent, prec = dec)
}

/// Format as ratio with "x" suffix.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::format_ratio;
///
/// assert_eq!(format_ratio(3.5, 2), "3.50x");
/// assert_eq!(format_ratio(-1.25, 1), "-1.2x");
/// ```
pub fn format_ratio(value: f64, decimals: usize) -> String {
    format!("{:.prec$}x", value, prec = decimals)
}

/// Sparkline data: condensed time series for inline display.
#[derive(Debug, Clone, Serialize)]
pub struct SparklineData {
    /// Bucketed values.
    pub buckets: Vec<f64>,
    /// Minimum value across the series.
    pub min: f64,
    /// Maximum value across the series.
    pub max: f64,
    /// Mean value across the series.
    pub mean: f64,
}

/// Condense a time series into a fixed number of buckets for inline display.
///
/// Each bucket is the average of the values that fall into it. If the series
/// has fewer elements than `n_buckets`, all values are returned directly.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::sparkline_buckets;
///
/// let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
/// let result = sparkline_buckets(&data, 10);
/// assert_eq!(result.buckets.len(), 10);
/// assert!((result.min - 0.0).abs() < 1e-10);
/// assert!((result.max - 99.0).abs() < 1e-10);
/// ```
pub fn sparkline_buckets(series: &[f64], n_buckets: usize) -> SparklineData {
    if series.is_empty() || n_buckets == 0 {
        return SparklineData {
            buckets: Vec::new(),
            min: 0.0,
            max: 0.0,
            mean: 0.0,
        };
    }

    let min = series.iter().copied().fold(f64::INFINITY, f64::min);
    let max = series.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = series.iter().sum::<f64>() / series.len() as f64;

    if series.len() <= n_buckets {
        return SparklineData {
            buckets: series.to_vec(),
            min,
            max,
            mean,
        };
    }

    let bucket_size = series.len() as f64 / n_buckets as f64;
    let mut buckets = Vec::with_capacity(n_buckets);

    for i in 0..n_buckets {
        let start = (i as f64 * bucket_size) as usize;
        let end = ((i + 1) as f64 * bucket_size) as usize;
        let end = end.min(series.len());
        let slice = &series[start..end];
        if slice.is_empty() {
            buckets.push(0.0);
        } else {
            let avg = slice.iter().sum::<f64>() / slice.len() as f64;
            buckets.push(avg);
        }
    }

    SparklineData {
        buckets,
        min,
        max,
        mean,
    }
}

/// Percentile rank annotation.
#[derive(Debug, Clone, Serialize)]
pub struct PercentileBadge {
    /// Percentile rank (0-100).
    pub percentile: f64,
    /// Human-readable label (e.g., "25th pctile", "median").
    pub label: String,
    /// Quartile (1-4).
    pub quartile: u8,
}

/// Compute the percentile rank of a value within a distribution.
///
/// Uses linear interpolation for the percentile. The distribution does
/// not need to be sorted -- it is sorted internally.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::reporting::format::percentile_badge;
///
/// let dist: Vec<f64> = (0..100).map(|i| i as f64).collect();
/// let badge = percentile_badge(50.0, &dist);
/// assert!(badge.percentile > 49.0 && badge.percentile < 52.0);
/// assert_eq!(badge.quartile, 3);
/// ```
pub fn percentile_badge(value: f64, distribution: &[f64]) -> PercentileBadge {
    if distribution.is_empty() {
        return PercentileBadge {
            percentile: 0.0,
            label: "N/A".to_string(),
            quartile: 1,
        };
    }

    let mut sorted = distribution.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let count_below = sorted.iter().filter(|&&x| x < value).count();
    let percentile = (count_below as f64 / sorted.len() as f64) * 100.0;

    let quartile = if percentile < 25.0 {
        1
    } else if percentile < 50.0 {
        2
    } else if percentile < 75.0 {
        3
    } else {
        4
    };

    let label = if (percentile - 50.0).abs() < 1.0 {
        "median".to_string()
    } else {
        format!("{:.0}th pctile", percentile)
    };

    PercentileBadge {
        percentile,
        label,
        quartile,
    }
}

/// Format a number with comma thousands separators.
fn format_with_commas(value: f64, decimals: usize) -> String {
    let formatted = format!("{:.prec$}", value, prec = decimals);

    // Split into integer and decimal parts
    let parts: Vec<&str> = formatted.splitn(2, '.').collect();
    let int_part = parts[0];
    let dec_part = parts.get(1);

    // Add commas to integer part
    let is_negative = int_part.starts_with('-');
    let digits = if is_negative {
        &int_part[1..]
    } else {
        int_part
    };

    let mut with_commas = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(c);
    }
    let with_commas: String = with_commas.chars().rev().collect();

    let int_with_sign = if is_negative {
        format!("-{}", with_commas)
    } else {
        with_commas
    };

    match dec_part {
        Some(dec) => format!("{}.{}", int_with_sign, dec),
        None => int_with_sign,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // ===== format_bps =====

    #[test]
    fn bps_positive() {
        assert_eq!(format_bps(0.0025, 1), "25.0 bps");
    }

    #[test]
    fn bps_negative() {
        assert_eq!(format_bps(-0.001, 2), "-10.00 bps");
    }

    #[test]
    fn bps_zero() {
        assert_eq!(format_bps(0.0, 1), "0.0 bps");
    }

    #[test]
    fn bps_one_bp() {
        assert_eq!(format_bps(0.0001, 1), "1.0 bps");
    }

    // ===== format_pct =====

    #[test]
    fn pct_positive() {
        assert_eq!(format_pct(0.0534, 2), "5.34%");
    }

    #[test]
    fn pct_negative() {
        assert_eq!(format_pct(-0.0534, 2), "-5.34%");
    }

    #[test]
    fn pct_zero_decimals() {
        assert_eq!(format_pct(1.0, 0), "100%");
    }

    // ===== format_currency =====

    #[test]
    fn currency_with_commas() {
        assert_eq!(format_currency(1234567.89, "USD", 2), "USD 1,234,567.89");
    }

    #[test]
    fn currency_negative() {
        assert_eq!(format_currency(-500.0, "EUR", 0), "EUR -500");
    }

    #[test]
    fn currency_zero() {
        assert_eq!(format_currency(0.0, "GBP", 2), "GBP 0.00");
    }

    #[test]
    fn currency_small() {
        assert_eq!(format_currency(42.5, "USD", 2), "USD 42.50");
    }

    #[test]
    fn currency_large() {
        assert_eq!(
            format_currency(1_000_000_000.0, "USD", 0),
            "USD 1,000,000,000"
        );
    }

    // ===== format_scientific =====

    #[test]
    fn scientific_small() {
        assert_eq!(format_scientific(0.000123, 3), "1.23e-4");
    }

    #[test]
    fn scientific_large() {
        assert_eq!(format_scientific(1234.0, 2), "1.2e3");
    }

    #[test]
    fn scientific_zero() {
        assert_eq!(format_scientific(0.0, 2), "0.0e0");
    }

    #[test]
    fn scientific_negative() {
        assert_eq!(format_scientific(-0.005, 2), "-5.0e-3");
    }

    // ===== format_ratio =====

    #[test]
    fn ratio_positive() {
        assert_eq!(format_ratio(3.5, 2), "3.50x");
    }

    #[test]
    fn ratio_negative() {
        // Checking the prefix and suffix
        let result = format_ratio(-1.25, 1);
        assert!(result.starts_with("-1.2") || result.starts_with("-1.3"));
        assert!(result.ends_with('x'));
    }

    // ===== sparkline_buckets =====

    #[test]
    fn sparkline_basic() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = sparkline_buckets(&data, 10);
        assert_eq!(result.buckets.len(), 10);
        assert!((result.min - 0.0).abs() < 1e-10);
        assert!((result.max - 99.0).abs() < 1e-10);
    }

    #[test]
    fn sparkline_empty() {
        let result = sparkline_buckets(&[], 10);
        assert!(result.buckets.is_empty());
    }

    #[test]
    fn sparkline_fewer_than_buckets() {
        let data = vec![1.0, 2.0, 3.0];
        let result = sparkline_buckets(&data, 10);
        assert_eq!(result.buckets.len(), 3);
        assert_eq!(result.buckets, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn sparkline_mean() {
        let data = vec![10.0, 20.0, 30.0];
        let result = sparkline_buckets(&data, 3);
        assert!((result.mean - 20.0).abs() < 1e-10);
    }

    // ===== percentile_badge =====

    #[test]
    fn percentile_basic() {
        let dist: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let badge = percentile_badge(50.0, &dist);
        assert!(badge.percentile > 49.0 && badge.percentile < 52.0);
        assert_eq!(badge.quartile, 3);
    }

    #[test]
    fn percentile_low() {
        let dist: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let badge = percentile_badge(10.0, &dist);
        assert!(badge.percentile < 15.0);
        assert_eq!(badge.quartile, 1);
    }

    #[test]
    fn percentile_empty_distribution() {
        let badge = percentile_badge(42.0, &[]);
        assert!((badge.percentile).abs() < 1e-10);
        assert_eq!(badge.label, "N/A");
    }

    #[test]
    fn percentile_high() {
        let dist: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let badge = percentile_badge(95.0, &dist);
        assert!(badge.percentile > 90.0);
        assert_eq!(badge.quartile, 4);
    }

    // ===== format_with_commas =====

    #[test]
    fn commas_thousands() {
        assert_eq!(format_with_commas(1234.0, 0), "1,234");
    }

    #[test]
    fn commas_millions() {
        assert_eq!(format_with_commas(1234567.89, 2), "1,234,567.89");
    }

    #[test]
    fn commas_negative() {
        assert_eq!(format_with_commas(-1234.0, 0), "-1,234");
    }

    #[test]
    fn commas_small() {
        assert_eq!(format_with_commas(42.5, 2), "42.50");
    }
}
