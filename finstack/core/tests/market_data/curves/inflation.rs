//! Tests for InflationCurve functionality.
//!
//! This module covers:
//! - Builder validation and construction
//! - CPI interpolation
//! - Inflation rate calculations
//! - Serialization roundtrips

use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::math::interp::InterpStyle;
use time::{Date, Month};

fn _test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).unwrap()
}

// =============================================================================
// Serialization Tests
// =============================================================================

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn roundtrip_with_all_fields() {
        let original = InflationCurve::builder("US-CPI")
            .base_cpi(300.0)
            .knots([
                (0.0, 300.0),
                (1.0, 306.0),
                (2.0, 312.5),
                (5.0, 330.0),
                (10.0, 360.0),
            ])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: InflationCurve = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id(), deserialized.id());
        assert_eq!(original.base_cpi(), deserialized.base_cpi());
        assert_eq!(original.knots(), deserialized.knots());
        assert_eq!(original.cpi_levels(), deserialized.cpi_levels());

        // Test CPI interpolation
        for t in [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0] {
            let original_cpi = original.cpi(t);
            let deserialized_cpi = deserialized.cpi(t);
            assert!(
                (original_cpi - deserialized_cpi).abs() < 1e-10,
                "CPI mismatch at t={}: {} vs {}",
                t,
                original_cpi,
                deserialized_cpi
            );
        }

        // Test inflation rate calculation
        for (t1, t2) in [(0.0, 1.0), (1.0, 2.0), (2.0, 5.0), (5.0, 10.0)] {
            let original_rate = original.inflation_rate(t1, t2);
            let deserialized_rate = deserialized.inflation_rate(t1, t2);
            assert!(
                (original_rate - deserialized_rate).abs() < 1e-12,
                "Inflation rate mismatch for period {}-{}: {} vs {}",
                t1,
                t2,
                original_rate,
                deserialized_rate
            );
        }
    }

    #[test]
    fn roundtrip_all_interp_styles() {
        // Note: MonotoneConvex requires non-increasing values, so it's not suitable for CPI levels
        let interp_styles = [
            InterpStyle::Linear,
            InterpStyle::LogLinear,
            InterpStyle::CubicHermite,
            InterpStyle::FlatFwd,
        ];

        for style in interp_styles {
            let original = InflationCurve::builder("EUR-HICP")
                .base_cpi(100.0)
                .knots([(0.0, 100.0), (1.0, 102.0), (3.0, 106.5), (5.0, 111.0)])
                .set_interp(style)
                .build()
                .unwrap();

            let json = serde_json::to_string(&original).unwrap();
            let deserialized: InflationCurve = serde_json::from_str(&json).unwrap();

            // Test CPI accuracy for each style
            for t in [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0] {
                let original_cpi = original.cpi(t);
                let deserialized_cpi = deserialized.cpi(t);
                assert!(
                    (original_cpi - deserialized_cpi).abs() < 1e-10,
                    "CPI mismatch for {:?} at t={}: {} vs {}",
                    style,
                    t,
                    original_cpi,
                    deserialized_cpi
                );
            }
        }
    }
}
