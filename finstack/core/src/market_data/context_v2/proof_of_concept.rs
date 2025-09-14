//! Proof of concept test for MarketContext
//!
//! This module provides a basic test to verify that the new enum-based
//! storage system works correctly and provides the expected benefits.

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests {
    use super::super::core::MarketContext;
    use crate::dates::Date;
    use crate::market_data::{
        interp::InterpStyle,
        scalars::MarketScalar,
        term_structures::{
            discount_curve::DiscountCurve,
            forward_curve::ForwardCurve,
            hazard_curve::HazardCurve,
        },
        traits::TermStructure,
    };

    fn create_test_context() -> MarketContext {
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let fwd_curve = ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        let hazard_curve = HazardCurve::builder("CORP-HAZARD")
            .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015), (5.0, 0.02)])
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve)
            .insert_hazard(hazard_curve)
            .insert_price("SPOT_GOLD", MarketScalar::Unitless(2000.0))
    }

    #[test]
    fn proof_of_concept_basic_functionality() {
        let context = create_test_context();

        // Test direct concrete API - clean and simple!
        let disc = context.discount("USD-OIS").unwrap();
        let fwd = context.forward("USD-SOFR3M").unwrap();
        let hazard = context.hazard("CORP-HAZARD").unwrap();

        // Verify functionality - all concrete types, no trait objects
        assert!((disc.df(1.0) - 0.95).abs() < 1e-12);
        assert!((fwd.rate(1.0) - 0.035).abs() < 1e-12);
        assert!(hazard.sp(1.0) < 1.0);

        // Test introspection
        let stats = context.stats();
        assert_eq!(stats.total_curves, 3);
        assert_eq!(stats.price_count, 1);

        let counts = context.count_by_type();
        assert_eq!(counts.get("Discount"), Some(&1));
        assert_eq!(counts.get("Forward"), Some(&1));
        assert_eq!(counts.get("Hazard"), Some(&1));
    }

    #[test]
    fn proof_of_concept_complete_serialization() {
        let original = create_test_context();

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Should serialize to JSON");

        // Verify JSON structure - no string parsing artifacts
        assert!(json.contains("\"curves\""));
        assert!(json.contains("\"type\":\"discount\""));
        assert!(json.contains("\"type\":\"forward\""));
        assert!(json.contains("\"type\":\"hazard\""));
        assert!(json.contains("\"prices\""));

        // Verify NO string parsing artifacts
        assert!(!json.contains("_bump_"));
        assert!(!json.contains("_spread_"));

        // Deserialize from JSON
        let restored: MarketContext = 
            serde_json::from_str(&json).expect("Should deserialize from JSON");

        // Verify all functionality is preserved
        assert_eq!(original.curves.len(), restored.curves.len());
        assert_eq!(original.prices.len(), restored.prices.len());

        // Test that all curves work identically
        let orig_disc = original.discount("USD-OIS").unwrap();
        let rest_disc = restored.discount("USD-OIS").unwrap();
        assert!((orig_disc.df(1.0) - rest_disc.df(1.0)).abs() < 1e-12);

        let orig_fwd = original.forward("USD-SOFR3M").unwrap();
        let rest_fwd = restored.forward("USD-SOFR3M").unwrap();
        assert!((orig_fwd.rate(1.0) - rest_fwd.rate(1.0)).abs() < 1e-12);

        // Test market data preservation
        let orig_price = original.price("SPOT_GOLD").unwrap();
        let rest_price = restored.price("SPOT_GOLD").unwrap();
        if let (MarketScalar::Unitless(o), MarketScalar::Unitless(r)) = (orig_price, rest_price) {
            assert_eq!(o, r);
        }
    }

    #[test]
    fn proof_of_concept_type_safety() {
        let context = create_test_context();

        // Type-safe access - you get exactly what you expect
        let curve = context.curve("USD-OIS").unwrap();
        assert!(curve.is_discount());
        assert!(!curve.is_forward());

        // Can't accidentally get wrong type
        assert!(curve.forward().is_none());
        assert!(curve.discount().is_some());

        // Direct concrete access with zero overhead
        let disc = context.discount("USD-OIS").unwrap();
        assert_eq!(disc.id().as_str(), "USD-OIS");
    }

    #[test]
    fn proof_of_concept_builder_pattern() {
        let context = MarketContext::builder()
            .discount(DiscountCurve::builder("EUR-OIS")
                .base_date(Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
                .knots([(0.0, 1.0), (1.0, 0.96)])
                .build()
                .unwrap())
            .price("EUR_RATE", MarketScalar::Unitless(0.025))
            .collateral("EUR-CSA", "EUR-OIS")
            .build()
            .unwrap();

        assert_eq!(context.curves.len(), 1);
        assert_eq!(context.prices.len(), 1);
        assert!(context.collateral("EUR-CSA").is_ok());
    }

    #[test]
    fn proof_of_concept_performance_characteristics() {
        let context = create_test_context();

        // Measure direct concrete access performance
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let disc = context.discount("USD-OIS").unwrap();
            let _ = disc.df(1.0);
        }
        let access_time = start.elapsed();

        println!("Direct concrete access: {:?}", access_time);

        // Should be fast and efficient
        assert!(access_time > std::time::Duration::ZERO);
        assert!(access_time < std::time::Duration::from_millis(10)); // Should be very fast
    }
}
