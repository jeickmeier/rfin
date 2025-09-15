//! Tests for curve serialization with interpolators

#[cfg(feature = "serde")]
mod tests {
    use finstack_core::dates::Date;
    use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
    use finstack_core::market_data::term_structures::{
        DiscountCurve, ForwardCurve, InflationCurve,
    };
    use time::Month;

    #[test]
    fn test_discount_curve_serialization_linear() {
        let original = DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95), (5.0, 0.88)])
            .set_interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .build()
            .unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Verify reconstruction accuracy
        assert_eq!(original.id(), deserialized.id());
        assert_eq!(original.base_date(), deserialized.base_date());
        assert_eq!(original.knots(), deserialized.knots());
        assert_eq!(original.dfs(), deserialized.dfs());

        // Test interpolation accuracy
        for t in [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 5.0, 7.0] {
            let original_df = original.df(t);
            let deserialized_df = deserialized.df(t);
            assert!(
                (original_df - deserialized_df).abs() < 1e-12,
                "DF mismatch at t={}: {} vs {}",
                t,
                original_df,
                deserialized_df
            );
        }
    }

    #[test]
    fn test_discount_curve_serialization_log_linear() {
        let original = DiscountCurve::builder("EUR-ESTR")
            .base_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
            .knots([(0.0, 1.0), (0.25, 0.995), (0.5, 0.99), (1.0, 0.98)])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test interpolation accuracy
        for t in [0.0, 0.1, 0.25, 0.4, 0.5, 0.75, 1.0, 1.5] {
            let original_df = original.df(t);
            let deserialized_df = deserialized.df(t);
            assert!(
                (original_df - deserialized_df).abs() < 1e-12,
                "LogLinear DF mismatch at t={}: {} vs {}",
                t,
                original_df,
                deserialized_df
            );
        }
    }

    #[test]
    fn test_discount_curve_serialization_monotone_convex() {
        let original = DiscountCurve::builder("GBP-SONIA")
            .base_date(Date::from_calendar_date(2025, Month::March, 15).unwrap())
            .knots([
                (0.0, 1.0),
                (0.5, 0.99),
                (1.0, 0.975),
                (2.0, 0.95),
                (5.0, 0.88),
                (10.0, 0.75),
            ])
            .set_interp(InterpStyle::MonotoneConvex)
            .require_monotonic()
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test interpolation accuracy
        for t in [0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0] {
            let original_df = original.df(t);
            let deserialized_df = deserialized.df(t);
            assert!(
                (original_df - deserialized_df).abs() < 1e-12,
                "MonotoneConvex DF mismatch at t={}: {} vs {}",
                t,
                original_df,
                deserialized_df
            );
        }
    }

    #[test]
    fn test_discount_curve_serialization_cubic_hermite() {
        let original = DiscountCurve::builder("JPY-TONAR")
            .base_date(Date::from_calendar_date(2025, Month::September, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.99), (3.0, 0.96), (5.0, 0.92)])
            .set_interp(InterpStyle::CubicHermite)
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test interpolation accuracy
        for t in [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0] {
            let original_df = original.df(t);
            let deserialized_df = deserialized.df(t);
            assert!(
                (original_df - deserialized_df).abs() < 1e-12,
                "CubicHermite DF mismatch at t={}: {} vs {}",
                t,
                original_df,
                deserialized_df
            );
        }
    }

    #[test]
    fn test_discount_curve_serialization_flat_fwd() {
        let original = DiscountCurve::builder("CHF-SARON")
            .base_date(Date::from_calendar_date(2025, Month::December, 31).unwrap())
            .knots([(0.0, 1.0), (0.5, 0.995), (1.0, 0.988), (2.0, 0.975)])
            .set_interp(InterpStyle::FlatFwd)
            .build()
            .unwrap();

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test interpolation accuracy
        for t in [0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5] {
            let original_df = original.df(t);
            let deserialized_df = deserialized.df(t);
            assert!(
                (original_df - deserialized_df).abs() < 1e-12,
                "FlatFwd DF mismatch at t={}: {} vs {}",
                t,
                original_df,
                deserialized_df
            );
        }
    }

    #[test]
    fn test_forward_curve_serialization() {
        use finstack_core::dates::DayCount;

        let original = ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .reset_lag(2)
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.03),
                (0.25, 0.032),
                (0.5, 0.035),
                (1.0, 0.04),
                (2.0, 0.042),
                (5.0, 0.045),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: ForwardCurve = serde_json::from_str(&json).unwrap();

        // Verify reconstruction
        assert_eq!(original.id(), deserialized.id());
        assert_eq!(original.base_date(), deserialized.base_date());
        assert_eq!(original.reset_lag(), deserialized.reset_lag());
        assert_eq!(original.day_count(), deserialized.day_count());
        assert_eq!(original.tenor(), deserialized.tenor());
        assert_eq!(original.knots(), deserialized.knots());
        assert_eq!(original.forwards(), deserialized.forwards());

        // Test rate interpolation
        for t in [0.0, 0.1, 0.25, 0.4, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0] {
            let original_rate = original.rate(t);
            let deserialized_rate = deserialized.rate(t);
            assert!(
                (original_rate - deserialized_rate).abs() < 1e-12,
                "Forward rate mismatch at t={}: {} vs {}",
                t,
                original_rate,
                deserialized_rate
            );
        }
    }

    #[test]
    fn test_forward_curve_serialization_multiple_interp_styles() {
        use finstack_core::dates::DayCount;

        // Note: MonotoneConvex requires non-increasing values, so it's not suitable for forward rates
        let interp_styles = [
            InterpStyle::Linear,
            InterpStyle::LogLinear,
            InterpStyle::CubicHermite,
            InterpStyle::FlatFwd,
        ];

        for style in interp_styles {
            let original = ForwardCurve::builder("EUR-EURIBOR6M", 0.5)
                .base_date(Date::from_calendar_date(2025, Month::June, 30).unwrap())
                .reset_lag(2)
                .day_count(DayCount::Act360)
                .knots([(0.0, 0.025), (1.0, 0.03), (2.0, 0.035), (5.0, 0.04)])
                .set_interp(style)
                .build()
                .unwrap();

            let json = serde_json::to_string(&original).unwrap();
            let deserialized: ForwardCurve = serde_json::from_str(&json).unwrap();

            // Test interpolation accuracy for each style
            for t in [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 5.0] {
                let original_rate = original.rate(t);
                let deserialized_rate = deserialized.rate(t);
                assert!(
                    (original_rate - deserialized_rate).abs() < 1e-12,
                    "Forward rate mismatch for {:?} at t={}: {} vs {}",
                    style,
                    t,
                    original_rate,
                    deserialized_rate
                );
            }
        }
    }

    #[test]
    fn test_inflation_curve_serialization() {
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

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: InflationCurve = serde_json::from_str(&json).unwrap();

        // Verify reconstruction
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
    fn test_inflation_curve_serialization_all_interp_styles() {
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

    #[test]
    fn test_extrapolation_policy_preservation() {
        // Test FlatZero extrapolation (default)
        let curve_flat_zero = DiscountCurve::builder("TEST-FLAT-ZERO")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatZero)
            .build()
            .unwrap();

        let json = serde_json::to_string(&curve_flat_zero).unwrap();
        let deserialized_flat_zero: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test extrapolation beyond the range
        let t_beyond = 10.0;
        let original_df = curve_flat_zero.df(t_beyond);
        let deserialized_df = deserialized_flat_zero.df(t_beyond);
        assert!(
            (original_df - deserialized_df).abs() < 1e-12,
            "FlatZero extrapolation mismatch at t={}: {} vs {}",
            t_beyond,
            original_df,
            deserialized_df
        );

        // Test FlatForward extrapolation
        let curve_flat_forward = DiscountCurve::builder("TEST-FLAT-FWD")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .build()
            .unwrap();

        let json = serde_json::to_string(&curve_flat_forward).unwrap();
        let deserialized_flat_forward: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test extrapolation beyond the range
        let original_df = curve_flat_forward.df(t_beyond);
        let deserialized_df = deserialized_flat_forward.df(t_beyond);
        assert!(
            (original_df - deserialized_df).abs() < 1e-12,
            "FlatForward extrapolation mismatch at t={}: {} vs {}",
            t_beyond,
            original_df,
            deserialized_df
        );

        // Verify the two policies give different results
        assert!(
            (curve_flat_zero.df(t_beyond) - curve_flat_forward.df(t_beyond)).abs() > 0.01,
            "Different extrapolation policies should produce different results"
        );
    }

    #[test]
    fn test_pretty_json_serialization() {
        // Test with pretty JSON for readability
        let original = DiscountCurve::builder("TEST-PRETTY")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).unwrap())
            .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();

        // Serialize to pretty JSON
        let json = serde_json::to_string_pretty(&original).unwrap();

        // Verify it's valid JSON and can be deserialized
        let deserialized: DiscountCurve = serde_json::from_str(&json).unwrap();

        // Test accuracy
        for t in [0.0, 0.5, 1.0, 1.5, 2.0] {
            let original_df = original.df(t);
            let deserialized_df = deserialized.df(t);
            assert!(
                (original_df - deserialized_df).abs() < 1e-12,
                "Pretty JSON DF mismatch at t={}: {} vs {}",
                t,
                original_df,
                deserialized_df
            );
        }
    }
}
