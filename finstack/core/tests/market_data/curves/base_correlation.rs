//! Tests for BaseCorrelationCurve functionality.
//!
//! This module covers:
//! - Builder validation and construction
//! - Serialization roundtrips

use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use time::{Date, Month};

fn _test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

// =============================================================================
// Serialization Tests
// =============================================================================

mod serde_tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let curve = BaseCorrelationCurve::builder("CDX")
            .knots([(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
            .build()
            .unwrap();

        let json = serde_json::to_string_pretty(&curve).unwrap();
        let deserialized: BaseCorrelationCurve = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id(), curve.id());
        assert_eq!(deserialized.detachment_points(), curve.detachment_points());
        assert_eq!(deserialized.correlations(), curve.correlations());
    }
}
