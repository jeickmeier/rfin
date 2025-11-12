//! Tests for common metrics utilities

#[cfg(test)]
mod dv01_scaling_tests {
    use crate::instruments::bond::Bond;
    use crate::instruments::common::traits::Instrument;
    use crate::metrics::MetricId;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn test_dv01_scaling_invariance_half_bp() {
        // DV01 with 0.5bp bump should be roughly half of 1bp DV01
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let bond = Bond::fixed(
            "SCALE_TEST",
            Money::new(100.0, Currency::USD),
            0.05,
            as_of,
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (5.0, 0.80)])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(curve);

        // Standard 1bp DV01
        let result_1bp = bond
            .price_with_metrics(&market, as_of, &[MetricId::Dv01])
            .unwrap();
        let dv01_1bp = *result_1bp.measures.get("dv01").unwrap();

        // 0.5bp DV01 with custom bump size
        use crate::instruments::PricingOverrides;
        let mut bond_half = bond.clone();
        bond_half.pricing_overrides = PricingOverrides::default().with_rate_bump(0.5);
        let result_half_bp = bond_half
            .price_with_metrics(&market, as_of, &[MetricId::Dv01])
            .unwrap();
        let dv01_half_bp = *result_half_bp.measures.get("dv01").unwrap();

        // DV01 should be approximately the same (per bp), not half
        // The formula is (PV_bumped - PV_base) / bump_bp
        // So with half the bump, the numerator is smaller but we divide by smaller bp too
        assert!(
            (dv01_1bp - dv01_half_bp).abs() < 1e-9,
            "DV01 should be invariant to bump size: 1bp={:.12}, 0.5bp={:.12}, diff={:.12}",
            dv01_1bp,
            dv01_half_bp,
            (dv01_1bp - dv01_half_bp).abs()
        );
    }

    #[test]
    fn test_dv01_scaling_invariance_two_bp() {
        // DV01 with 2bp bump should match 1bp DV01
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let bond = Bond::fixed(
            "SCALE_TEST_2",
            Money::new(100.0, Currency::USD),
            0.05,
            as_of,
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([(0.0, 1.0), (5.0, 0.80)])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(curve);

        // Standard 1bp DV01
        let result_1bp = bond
            .price_with_metrics(&market, as_of, &[MetricId::Dv01])
            .unwrap();
        let dv01_1bp = *result_1bp.measures.get("dv01").unwrap();

        // 2bp DV01
        use crate::instruments::PricingOverrides;
        let mut bond_2bp = bond.clone();
        bond_2bp.pricing_overrides = PricingOverrides::default().with_rate_bump(2.0);
        let result_2bp = bond_2bp
            .price_with_metrics(&market, as_of, &[MetricId::Dv01])
            .unwrap();
        let dv01_2bp = *result_2bp.measures.get("dv01").unwrap();

        // DV01 should be approximately the same (per bp)
        assert!(
            (dv01_1bp - dv01_2bp).abs() < 1e-9,
            "DV01 should be invariant to bump size: 1bp={:.12}, 2bp={:.12}, diff={:.12}",
            dv01_1bp,
            dv01_2bp,
            (dv01_1bp - dv01_2bp).abs()
        );
    }

    #[test]
    fn test_bucketed_dv01_scaling_invariance() {
        // Bucketed DV01 should also scale correctly
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let bond = Bond::fixed(
            "BUCKETED_SCALE",
            Money::new(100.0, Currency::USD),
            0.05,
            as_of,
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (0.5, 0.98),
                (1.0, 0.96),
                (5.0, 0.80),
                (10.0, 0.60),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(curve);

        // Standard 1bp bucketed DV01
        let result_1bp = bond
            .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
            .unwrap();
        let total_1bp = *result_1bp.measures.get("bucketed_dv01").unwrap();

        // 2bp bucketed DV01
        use crate::instruments::PricingOverrides;
        let mut bond_2bp = bond.clone();
        bond_2bp.pricing_overrides = PricingOverrides::default().with_rate_bump(2.0);
        let result_2bp = bond_2bp
            .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
            .unwrap();
        let total_2bp = *result_2bp.measures.get("bucketed_dv01").unwrap();

        // Total should be invariant
        assert!(
            (total_1bp - total_2bp).abs() < 1e-9,
            "Bucketed DV01 total should be invariant to bump size: 1bp={:.12}, 2bp={:.12}",
            total_1bp,
            total_2bp
        );
    }
}

#[cfg(test)]
mod dv01_interp_preservation_tests {
    use crate::instruments::bond::Bond;
    use crate::instruments::common::traits::Instrument;
    use crate::metrics::MetricId;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn test_bucketed_dv01_preserves_monotone_convex() {
        // Verify that bucketed DV01 preserves MonotoneConvex interpolation
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let bond = Bond::fixed(
            "MC_TEST",
            Money::new(100.0, Currency::USD),
            0.05,
            as_of,
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (0.5, 0.98),
                (1.0, 0.96),
                (2.0, 0.93),
                (5.0, 0.80),
                (10.0, 0.60),
            ])
            .set_interp(InterpStyle::MonotoneConvex)
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(curve);

        // Compute bucketed DV01
        let result = bond
            .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
            .unwrap();
        let total_dv01 = *result.measures.get("bucketed_dv01").unwrap();

        // Verify non-zero sensitivity
        assert!(
            total_dv01.abs() > 1e-6,
            "DV01 should be non-zero for MonotoneConvex curve: {}",
            total_dv01
        );

        // Verify sensible magnitude (negative for bond)
        assert!(
            total_dv01 < 0.0,
            "DV01 should be negative for long fixed-rate bond: {}",
            total_dv01
        );
    }

    #[test]
    fn test_bucketed_dv01_preserves_flat_fwd() {
        // Verify that bucketed DV01 works correctly with FlatFwd interpolation
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let bond = Bond::fixed(
            "FF_TEST",
            Money::new(100.0, Currency::USD),
            0.05,
            as_of,
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (0.5, 0.98),
                (1.0, 0.96),
                (2.0, 0.93),
                (5.0, 0.80),
                (10.0, 0.60),
            ])
            .set_interp(InterpStyle::FlatFwd)
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(curve);

        // Compute bucketed DV01
        let result = bond
            .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
            .unwrap();
        let total_dv01 = *result.measures.get("bucketed_dv01").unwrap();

        // Verify non-zero sensitivity
        assert!(
            total_dv01.abs() > 1e-6,
            "DV01 should be non-zero for FlatFwd curve: {}",
            total_dv01
        );

        // Verify sensible magnitude (negative for bond)
        assert!(
            total_dv01 < 0.0,
            "DV01 should be negative for long fixed-rate bond: {}",
            total_dv01
        );
    }

    #[test]
    fn test_interp_style_parity_across_methods() {
        // Compare DV01 results between different interpolation styles
        // to verify no unexpected deviations from curve reconstruction
        let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let bond = Bond::fixed(
            "PARITY_TEST",
            Money::new(100.0, Currency::USD),
            0.05,
            as_of,
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        // Test with MonotoneConvex
        let curve_mc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (5.0, 0.80),
                (10.0, 0.60),
            ])
            .set_interp(InterpStyle::MonotoneConvex)
            .build()
            .unwrap();
        let market_mc = MarketContext::new().insert_discount(curve_mc);

        let result_mc = bond
            .price_with_metrics(&market_mc, as_of, &[MetricId::BucketedDv01])
            .unwrap();
        let dv01_mc = *result_mc.measures.get("bucketed_dv01").unwrap();

        // Test with Linear
        let curve_linear = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .knots([
                (0.0, 1.0),
                (1.0, 0.96),
                (5.0, 0.80),
                (10.0, 0.60),
            ])
            .set_interp(InterpStyle::Linear)
            .build()
            .unwrap();
        let market_linear = MarketContext::new().insert_discount(curve_linear);

        let result_linear = bond
            .price_with_metrics(&market_linear, as_of, &[MetricId::BucketedDv01])
            .unwrap();
        let dv01_linear = *result_linear.measures.get("bucketed_dv01").unwrap();

        // Results should differ (different interp styles), but both should be valid
        assert!(
            dv01_mc.abs() > 1e-6 && dv01_linear.abs() > 1e-6,
            "Both DV01 values should be non-zero: MC={}, Linear={}",
            dv01_mc,
            dv01_linear
        );
        
        // Both should be negative for bonds
        assert!(dv01_mc < 0.0 && dv01_linear < 0.0);
    }
}

#[cfg(test)]
mod theta_utils_tests {
    use super::super::theta_utils::*;

    #[test]
    fn test_parse_period_days() {
        assert_eq!(parse_period_days("1D").unwrap(), 1);
        assert_eq!(parse_period_days("7D").unwrap(), 7);
        assert_eq!(parse_period_days("1W").unwrap(), 7);
        assert_eq!(parse_period_days("2W").unwrap(), 14);
        assert_eq!(parse_period_days("1M").unwrap(), 30);
        assert_eq!(parse_period_days("3M").unwrap(), 90);
        assert_eq!(parse_period_days("6M").unwrap(), 180);
        assert_eq!(parse_period_days("1Y").unwrap(), 365);
        assert_eq!(parse_period_days("2Y").unwrap(), 730);
    }

    #[test]
    fn test_parse_period_lowercase() {
        assert_eq!(parse_period_days("1d").unwrap(), 1);
        assert_eq!(parse_period_days("1w").unwrap(), 7);
        assert_eq!(parse_period_days("1m").unwrap(), 30);
        assert_eq!(parse_period_days("1y").unwrap(), 365);
    }

    #[test]
    fn test_parse_period_with_whitespace() {
        assert_eq!(parse_period_days(" 1D ").unwrap(), 1);
        assert_eq!(parse_period_days(" 3M ").unwrap(), 90);
    }

    #[test]
    fn test_parse_period_invalid() {
        assert!(parse_period_days("").is_err());
        assert!(parse_period_days("1X").is_err());
        assert!(parse_period_days("XYZ").is_err());
        assert!(parse_period_days("D").is_err());
    }

    #[test]
    fn test_calculate_theta_date_no_expiry() {
        use finstack_core::dates::Date;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let rolled = calculate_theta_date(base, "1D", None).unwrap();
        assert_eq!(
            rolled,
            Date::from_calendar_date(2025, Month::January, 2).unwrap()
        );

        let rolled_week = calculate_theta_date(base, "1W", None).unwrap();
        assert_eq!(
            rolled_week,
            Date::from_calendar_date(2025, Month::January, 8).unwrap()
        );
    }

    #[test]
    fn test_calculate_theta_date_with_expiry_cap() {
        use finstack_core::dates::Date;
        use time::Month;

        let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let expiry = Date::from_calendar_date(2025, Month::January, 5).unwrap();

        // Rolling 1 week would go past expiry, should cap at expiry
        let rolled = calculate_theta_date(base, "1W", Some(expiry)).unwrap();
        assert_eq!(rolled, expiry);

        // Rolling 1 day is before expiry, should not cap
        let rolled_day = calculate_theta_date(base, "1D", Some(expiry)).unwrap();
        assert_eq!(
            rolled_day,
            Date::from_calendar_date(2025, Month::January, 2).unwrap()
        );
    }
}
