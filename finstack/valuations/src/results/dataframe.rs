//! DataFrame export helpers for valuation results.
//!
//! Provides row-oriented helpers to convert valuation results into
//! flat structures suitable for Polars/Pandas DataFrame construction.

use super::ValuationResult;
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
    /// Convert this result to a flat row for DataFrame export.
    ///
    /// Selected measures (dv01, convexity, duration, ytm) are promoted
    /// to top-level columns if present.
    pub fn to_row(&self) -> ValuationRow {
        ValuationRow {
            instrument_id: self.instrument_id.clone(),
            as_of_date: self.as_of.to_string(),
            pv: self.value.amount(),
            currency: self.value.currency().to_string(),
            dv01: self.measures.get("dv01").copied(),
            convexity: self.measures.get("convexity").copied(),
            duration: self
                .measures
                .get("duration")
                .or_else(|| self.measures.get("modified_duration"))
                .copied(),
            ytm: self.measures.get("ytm").copied(),
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
mod tests {
    use super::*;
    use finstack_core::config::results_meta;
    use finstack_core::config::FinstackConfig;
    use finstack_core::money::Money;
    use finstack_core::prelude::*;
    use indexmap::IndexMap;

    #[test]
    fn test_valuation_result_to_row() {
        let mut measures = IndexMap::new();
        measures.insert("dv01".to_string(), 1250.0);
        measures.insert("convexity".to_string(), 125.5);

        let result = ValuationResult::stamped_with_meta(
            "BOND-001",
            Date::from_calendar_date(2025, time::Month::January, 15).unwrap(),
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
            Date::from_calendar_date(2025, time::Month::January, 15).unwrap(),
            Money::new(1_000_000.0, Currency::USD),
        );

        let result2 = ValuationResult::stamped(
            "BOND-002",
            Date::from_calendar_date(2025, time::Month::January, 15).unwrap(),
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
            Date::from_calendar_date(2025, time::Month::January, 15).unwrap(),
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
}
