//! DataFrame export helpers for valuation results.
//!
//! Provides row-oriented helpers to convert valuation results into
//! flat structures suitable for Polars/Pandas DataFrame construction.

use super::ValuationResult;
use crate::metrics::MetricId;
use serde::{Deserialize, Serialize};

/// Flat row representation of a ValuationResult for DataFrame export.
///
/// This structure flattens the nested ValuationResult into a single row
/// with selected measures promoted to top-level columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationRow {
    /// Instrument identifier
    pub instrument_id: String,
    /// Valuation date (ISO 8601 format)
    pub as_of_date: String,
    /// Present value (amount only, as f64)
    pub pv: f64,
    /// Currency code
    pub currency: String,
    /// DV01 risk measure (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dv01: Option<f64>,
    /// Convexity risk measure (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convexity: Option<f64>,
    /// Duration risk measure (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Yield to maturity (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ytm: Option<f64>,
}

impl ValuationResult {
    /// Helper method to get a measure value using MetricId constant.
    ///
    /// This ensures we use the correct, canonical metric key strings
    /// from the metrics registry instead of hardcoded strings.
    fn get_measure(&self, id: MetricId) -> Option<f64> {
        self.measures.get(id.as_str()).copied()
    }

    /// Convert this result to a flat row for DataFrame export.
    ///
    /// Selected measures (dv01, convexity, duration, ytm) are promoted
    /// to top-level columns if present. Uses correct MetricId constants
    /// to ensure reliable measure extraction.
    ///
    /// For duration, prefers ModifiedDuration but falls back to Macaulay
    /// if modified is not available.
    pub fn to_row(&self) -> ValuationRow {
        ValuationRow {
            instrument_id: self.instrument_id.clone(),
            as_of_date: self.as_of.to_string(),
            pv: self.value.amount(),
            currency: self.value.currency().to_string(),
            dv01: self.get_measure(MetricId::Dv01),
            convexity: self.get_measure(MetricId::Convexity),
            duration: self
                .get_measure(MetricId::DurationMod)
                .or_else(|| self.get_measure(MetricId::DurationMac)),
            ytm: self.get_measure(MetricId::Ytm),
        }
    }

    /// Convert this result to a vector of rows (typically just one).
    ///
    /// This is useful for consistent batch processing where each result
    /// produces a Vec that can be concatenated.
    pub fn to_rows(&self) -> Vec<ValuationRow> {
        vec![self.to_row()]
    }
}

/// Convert multiple valuation results to rows for DataFrame construction.
///
/// See unit tests and `examples/` for usage.
pub fn results_to_rows(results: &[ValuationResult]) -> Vec<ValuationRow> {
    results.iter().flat_map(|r| r.to_rows()).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::config::results_meta;
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::money::Money;
    use indexmap::IndexMap;

    #[test]
    fn test_valuation_result_to_row() {
        let mut measures = IndexMap::new();
        measures.insert("dv01".to_string(), 1250.0);
        measures.insert("convexity".to_string(), 125.5);

        let result = ValuationResult::stamped_with_meta(
            "BOND-001",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_042_315.67, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(row.instrument_id, "BOND-001");
        assert_eq!(row.currency, "USD");
        assert!((row.pv - 1_042_315.67).abs() < 0.01);
        assert_eq!(row.dv01, Some(1250.0));
        assert_eq!(row.convexity, Some(125.5));
        assert!(row.duration.is_none());
        assert!(row.ytm.is_none());
    }

    #[test]
    fn test_results_to_rows_batch() {
        let result1 = ValuationResult::stamped(
            "BOND-001",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
        );

        let result2 = ValuationResult::stamped(
            "BOND-002",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(500_000.0, Currency::EUR),
        );

        let rows = results_to_rows(&[result1, result2]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].instrument_id, "BOND-001");
        assert_eq!(rows[1].instrument_id, "BOND-002");
    }

    #[test]
    fn test_row_serialization() {
        let result = ValuationResult::stamped(
            "BOND-001",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
        );

        let row = result.to_row();
        let json = serde_json::to_string(&row).expect("Should serialize");

        assert!(json.contains("BOND-001"));
        assert!(json.contains("USD"));

        // Roundtrip
        let deserialized: ValuationRow = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.instrument_id, "BOND-001");
    }

    #[test]
    fn test_to_row_dv01_mapping() {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::Dv01.as_str().to_string(), 500.0);

        let result = ValuationResult::stamped_with_meta(
            "BOND-DV01",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(row.dv01, Some(500.0), "DV01 should be extracted correctly");
        assert!(row.convexity.is_none(), "Convexity should be None");
        assert!(row.duration.is_none(), "Duration should be None");
        assert!(row.ytm.is_none(), "YTM should be None");
    }

    #[test]
    fn test_to_row_convexity_mapping() {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::Convexity.as_str().to_string(), 125.5);

        let result = ValuationResult::stamped_with_meta(
            "BOND-CVX",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(
            row.convexity,
            Some(125.5),
            "Convexity should be extracted correctly"
        );
        assert!(row.dv01.is_none(), "DV01 should be None");
        assert!(row.duration.is_none(), "Duration should be None");
        assert!(row.ytm.is_none(), "YTM should be None");
    }

    #[test]
    fn test_to_row_duration_mod_mapping() {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::DurationMod.as_str().to_string(), 7.5);

        let result = ValuationResult::stamped_with_meta(
            "BOND-DURMOD",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(
            row.duration,
            Some(7.5),
            "Modified Duration should be extracted correctly"
        );
        assert!(row.dv01.is_none(), "DV01 should be None");
        assert!(row.convexity.is_none(), "Convexity should be None");
        assert!(row.ytm.is_none(), "YTM should be None");
    }

    #[test]
    fn test_to_row_duration_mac_fallback() {
        let mut measures = IndexMap::new();
        // Only provide Macaulay duration (no Modified duration)
        measures.insert(MetricId::DurationMac.as_str().to_string(), 8.2);

        let result = ValuationResult::stamped_with_meta(
            "BOND-DURMAC",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(
            row.duration,
            Some(8.2),
            "Macaulay Duration should be used as fallback"
        );
        assert!(row.dv01.is_none(), "DV01 should be None");
        assert!(row.convexity.is_none(), "Convexity should be None");
        assert!(row.ytm.is_none(), "YTM should be None");
    }

    #[test]
    fn test_to_row_duration_mod_preferred_over_mac() {
        let mut measures = IndexMap::new();
        // Provide both Modified and Macaulay - Modified should win
        measures.insert(MetricId::DurationMod.as_str().to_string(), 7.5);
        measures.insert(MetricId::DurationMac.as_str().to_string(), 8.2);

        let result = ValuationResult::stamped_with_meta(
            "BOND-DURBOTH",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(
            row.duration,
            Some(7.5),
            "Modified Duration should be preferred over Macaulay"
        );
    }

    #[test]
    fn test_to_row_ytm_mapping() {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::Ytm.as_str().to_string(), 0.0425);

        let result = ValuationResult::stamped_with_meta(
            "BOND-YTM",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(row.ytm, Some(0.0425), "YTM should be extracted correctly");
        assert!(row.dv01.is_none(), "DV01 should be None");
        assert!(row.convexity.is_none(), "Convexity should be None");
        assert!(row.duration.is_none(), "Duration should be None");
    }

    #[test]
    fn test_to_row_all_metrics_populated() {
        let mut measures = IndexMap::new();
        measures.insert(MetricId::Dv01.as_str().to_string(), 500.0);
        measures.insert(MetricId::Convexity.as_str().to_string(), 125.5);
        measures.insert(MetricId::DurationMod.as_str().to_string(), 7.5);
        measures.insert(MetricId::Ytm.as_str().to_string(), 0.0425);

        let result = ValuationResult::stamped_with_meta(
            "BOND-FULL",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(row.dv01, Some(500.0), "DV01 should be populated");
        assert_eq!(row.convexity, Some(125.5), "Convexity should be populated");
        assert_eq!(row.duration, Some(7.5), "Duration should be populated");
        assert_eq!(row.ytm, Some(0.0425), "YTM should be populated");
    }

    #[test]
    fn test_to_row_legacy_keys_not_used() {
        // Verify that old incorrect keys like "duration" and "modified_duration" don't work
        let mut measures = IndexMap::new();
        measures.insert("duration".to_string(), 7.5);
        measures.insert("modified_duration".to_string(), 7.5);
        measures.insert("dv01".to_string(), 500.0); // Still works (correct key)

        let result = ValuationResult::stamped_with_meta(
            "BOND-LEGACY",
            Date::from_calendar_date(2025, time::Month::January, 15).expect("valid date"),
            Money::new(1_000_000.0, Currency::USD),
            results_meta(&FinstackConfig::default()),
        )
        .with_measures(measures);

        let row = result.to_row();

        assert_eq!(row.dv01, Some(500.0), "DV01 still works with correct key");
        assert!(
            row.duration.is_none(),
            "Duration should be None with legacy keys (not duration_mod or duration_mac)"
        );
    }
}
