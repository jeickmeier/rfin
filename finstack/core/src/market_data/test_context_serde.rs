//! Tests for MarketContext serialization

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests {
    use crate::market_data::context::MarketContext;
    use crate::market_data::surfaces::vol_surface::VolSurface;
    use crate::market_data::term_structures::{
        base_correlation::BaseCorrelationCurve,
        hazard_curve::HazardCurve,
    };
    use crate::market_data::primitives::{MarketScalar, ScalarTimeSeries};
    use crate::dates::Date;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
    }

    #[test]
    fn test_empty_context_serialization() {
        let context = MarketContext::new();
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify empty collections
        assert!(data.disc_curves.is_empty());
        assert!(data.fwd_curves.is_empty());
        assert!(data.hazard_curves.is_empty());
        assert!(data.surfaces.is_empty());
        assert!(data.prices.is_empty());
    }

    #[test]
    fn test_context_with_hazard_curve() {
        let hazard_curve = HazardCurve::builder("TEST-HAZARD")
            .base_date(test_date())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015), (5.0, 0.02)])
            .build()
            .unwrap();
        
        let context = MarketContext::new().insert_hazard(hazard_curve);
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify hazard curve was serialized
        assert_eq!(data.hazard_curves.len(), 1);
        let (_id, state) = &data.hazard_curves[0];
        assert_eq!(state.id, "TEST-HAZARD");
        assert_eq!(state.recovery_rate, 0.4);
        
        // Reconstruct from data
        let reconstructed = MarketContext::from_data(data)
            .expect("Should reconstruct from data");
        
        // Verify the curve exists
        let curve = reconstructed.hazard("TEST-HAZARD")
            .expect("Should have hazard curve");
        assert_eq!(curve.recovery_rate(), 0.4);
    }

    #[test]
    fn test_context_with_base_correlation() {
        let base_corr = BaseCorrelationCurve::builder("TEST-CORR")
            .points(vec![(3.0, 0.25), (7.0, 0.45), (10.0, 0.60)])
            .build()
            .unwrap();
        
        let context = MarketContext::new().insert_base_correlation(base_corr);
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify base correlation curve was serialized
        assert_eq!(data.base_correlation_curves.len(), 1);
        
        // Reconstruct from data
        let reconstructed = MarketContext::from_data(data)
            .expect("Should reconstruct from data");
        
        // Verify the curve exists
        let curve = reconstructed.base_correlation("TEST-CORR")
            .expect("Should have base correlation curve");
        assert!((curve.correlation(5.0) - 0.35).abs() < 1e-10);
    }

    #[test]
    fn test_context_with_vol_surface() {
        let surface = VolSurface::builder("TEST-VOL")
            .expiries(&[0.25, 1.0])
            .strikes(&[90.0, 100.0])
            .row(&[0.20, 0.22])
            .row(&[0.18, 0.19])
            .build()
            .unwrap();
        
        let context = MarketContext::new().insert_surface(surface);
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify surface was serialized
        assert_eq!(data.surfaces.len(), 1);
        let (_id, state) = &data.surfaces[0];
        assert_eq!(state.id, "TEST-VOL");
        
        // Reconstruct from data
        let reconstructed = MarketContext::from_data(data)
            .expect("Should reconstruct from data");
        
        // Verify the surface exists
        let surface = reconstructed.surface("TEST-VOL")
            .expect("Should have vol surface");
        assert!((surface.value(0.5, 95.0) - 0.205).abs() < 0.01); // Interpolated value
    }

    #[test]
    fn test_context_with_prices() {
        let context = MarketContext::new()
            .insert_price("SPOT_GOLD", MarketScalar::Unitless(2000.0))
            .insert_price("USD_RATE", MarketScalar::Unitless(0.05));
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify prices were serialized
        assert_eq!(data.prices.len(), 2);
        
        // Reconstruct from data
        let reconstructed = MarketContext::from_data(data)
            .expect("Should reconstruct from data");
        
        // Verify prices exist
        let gold = reconstructed.price("SPOT_GOLD")
            .expect("Should have gold price");
        if let MarketScalar::Unitless(val) = gold {
            assert_eq!(*val, 2000.0);
        } else {
            panic!("Expected unitless scalar");
        }
    }

    #[test]
    fn test_context_with_time_series() {
        let series = ScalarTimeSeries::new(
            "TEST_SERIES",
            vec![
                (Date::from_calendar_date(2025, Month::January, 1).unwrap(), 100.0),
                (Date::from_calendar_date(2025, Month::February, 1).unwrap(), 102.0),
            ],
            None,
        ).unwrap();
        
        let context = MarketContext::new().insert_series(series);
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify series was serialized
        assert_eq!(data.series.len(), 1);
        
        // Reconstruct from data
        let reconstructed = MarketContext::from_data(data)
            .expect("Should reconstruct from data");
        
        // Verify series exists
        let series = reconstructed.series("TEST_SERIES")
            .expect("Should have time series");
        let val = series.value_on(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .expect("Should have value");
        assert_eq!(val, 100.0);
    }

    #[test]
    fn test_context_comprehensive() {
        // Build a complex market context with multiple curve types
        let hazard_curve = HazardCurve::builder("CORP-HAZARD")
            .base_date(test_date())
            .recovery_rate(0.4)
            .knots([(0.0, 0.01), (1.0, 0.015)])
            .build()
            .unwrap();
        
        let base_corr = BaseCorrelationCurve::builder("CDX-CORR")
            .points(vec![(3.0, 0.25), (7.0, 0.45)])
            .build()
            .unwrap();
        
        let surface = VolSurface::builder("SPX-VOL")
            .expiries(&[0.25, 1.0])
            .strikes(&[90.0, 100.0])
            .row(&[0.20, 0.22])
            .row(&[0.18, 0.19])
            .build()
            .unwrap();
        
        let series = ScalarTimeSeries::new(
            "INDEX_LEVELS",
            vec![
                (Date::from_calendar_date(2025, Month::January, 1).unwrap(), 5000.0),
                (Date::from_calendar_date(2025, Month::February, 1).unwrap(), 5100.0),
            ],
            None,
        ).unwrap();
        
        let context = MarketContext::new()
            .insert_hazard(hazard_curve)
            .insert_base_correlation(base_corr)
            .insert_surface(surface)
            .insert_series(series)
            .insert_price("SPOT_SPX", MarketScalar::Unitless(5000.0))
            .insert_price("USD_RATE", MarketScalar::Unitless(0.05));
        
        // Convert to data
        let data = context.to_data().expect("Should convert to data");
        
        // Verify all components were serialized
        assert_eq!(data.hazard_curves.len(), 1);
        assert_eq!(data.base_correlation_curves.len(), 1);
        assert_eq!(data.surfaces.len(), 1);
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.prices.len(), 2);
        
        // Reconstruct from data
        let reconstructed = MarketContext::from_data(data)
            .expect("Should reconstruct from data");
        
        // Verify all components exist in reconstructed context
        assert!(reconstructed.hazard("CORP-HAZARD").is_ok());
        assert!(reconstructed.base_correlation("CDX-CORR").is_ok());
        assert!(reconstructed.surface("SPX-VOL").is_ok());
        assert!(reconstructed.series("INDEX_LEVELS").is_ok());
        assert!(reconstructed.price("SPOT_SPX").is_ok());
        assert!(reconstructed.price("USD_RATE").is_ok());
    }

    #[test]
    fn test_json_round_trip() {
        // Create a simple context
        let context = MarketContext::new()
            .insert_price("TEST_PRICE", MarketScalar::Unitless(100.0));
        
        // Serialize to JSON
        let json = serde_json::to_string(&context)
            .expect("Should serialize to JSON");
        
        // Deserialize from JSON
        let reconstructed: MarketContext = serde_json::from_str(&json)
            .expect("Should deserialize from JSON");
        
        // Verify data is preserved
        let price = reconstructed.price("TEST_PRICE")
            .expect("Should have price");
        if let MarketScalar::Unitless(val) = price {
            assert_eq!(*val, 100.0);
        } else {
            panic!("Expected unitless scalar");
        }
    }
}
