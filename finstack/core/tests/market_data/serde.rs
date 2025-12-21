//! Serialization tests for market data types.
//!
//! This module contains serde roundtrip tests for market data types
//! that don't have their own dedicated test modules, as well as
//! cross-cutting serialization tests like MarketContext.
//!
//! Note: Curve-specific serde tests are in their respective modules
//! (curves/discount.rs, curves/forward.rs, etc.)

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::context::MarketContextState;
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex, InflationInterpolation, InflationLag,
};
use finstack_core::market_data::scalars::{ScalarTimeSeries, SeriesInterpolation};
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
use finstack_core::types::CurveId;
use time::Month;

fn test_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 15).unwrap()
}

// =============================================================================
// VolSurface Tests
// =============================================================================

#[test]
fn vol_surface_roundtrip() {
    let surface = VolSurface::builder("EQ-VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0, 110.0])
        .row(&[0.2, 0.21, 0.22])
        .row(&[0.19, 0.2, 0.21])
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&surface).unwrap();
    let deserialized: VolSurface = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), surface.id());
    assert_eq!(deserialized.expiries(), surface.expiries());
    assert_eq!(deserialized.strikes(), surface.strikes());
    assert_eq!(deserialized.grid_shape(), surface.grid_shape());
}

// =============================================================================
// ScalarTimeSeries Tests
// =============================================================================

#[test]
fn scalar_time_series_roundtrip() {
    let d1 = Date::from_calendar_date(2024, Month::January, 31).unwrap();
    let d2 = Date::from_calendar_date(2024, Month::February, 29).unwrap();

    let series = ScalarTimeSeries::new("VOL-TS", vec![(d1, 0.2), (d2, 0.25)], None)
        .unwrap()
        .with_interpolation(SeriesInterpolation::Linear);

    let json = serde_json::to_string_pretty(&series).unwrap();
    let deserialized: ScalarTimeSeries = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id(), series.id());
    assert_eq!(deserialized.observations(), series.observations());
}

// =============================================================================
// InflationIndex Tests
// =============================================================================

#[test]
fn inflation_index_roundtrip() {
    let d1 = Date::from_calendar_date(2024, Month::January, 31).unwrap();
    let d2 = Date::from_calendar_date(2024, Month::February, 29).unwrap();

    let index = InflationIndex::new("US-CPI", vec![(d1, 300.0), (d2, 301.5)], Currency::USD)
        .unwrap()
        .with_interpolation(InflationInterpolation::Linear)
        .with_lag(InflationLag::Months(3));

    let json = serde_json::to_string_pretty(&index).unwrap();
    let deserialized: InflationIndex = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, index.id);
    assert_eq!(deserialized.currency, index.currency);
    assert_eq!(deserialized.interpolation, index.interpolation);
    assert_eq!(deserialized.lag(), index.lag());
}

// =============================================================================
// MarketContext Tests
// =============================================================================

#[test]
fn market_context_roundtrip() {
    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(test_date())
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build()
        .unwrap();

    let forward = ForwardCurve::builder("USD-SOFR", 0.25)
        .base_date(test_date())
        .knots([(0.0, 0.03), (5.0, 0.04)])
        .build()
        .unwrap();

    let surface = VolSurface::builder("VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0])
        .row(&[0.2, 0.2])
        .row(&[0.2, 0.2])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(discount)
        .insert_forward(forward)
        .insert_surface(surface);

    let json = serde_json::to_string_pretty(&ctx).unwrap();
    let deserialized: MarketContext = serde_json::from_str(&json).unwrap();

    assert!(deserialized.get_discount("USD-OIS").is_ok());
    assert!(deserialized.get_forward("USD-SOFR").is_ok());
    assert!(deserialized.surface("VOL").is_ok());
}

#[test]
fn market_context_state_is_deterministically_sorted_and_roundtrips_full_snapshot() {
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::money::Money;

    let d = test_date();

    // Intentionally insert in "unsorted" order to verify deterministic state ordering.
    let discount_b = DiscountCurve::builder("B-DISC")
        .base_date(d)
        .knots([(0.0, 1.0), (5.0, 0.9)])
        .build()
        .unwrap();
    let discount_a = DiscountCurve::builder("A-DISC")
        .base_date(d)
        .knots([(0.0, 1.0), (5.0, 0.92)])
        .build()
        .unwrap();
    let hazard = HazardCurve::builder("CDX-HAZ")
        .base_date(d)
        .knots([(0.0, 0.01), (5.0, 0.02)])
        .build()
        .unwrap();
    let base_corr = BaseCorrelationCurve::builder("CDX-BC")
        .points([(3.0, 0.25), (7.0, 0.4)])
        .build()
        .unwrap();

    // Add issuer curves/weights/recoveries to cover optional-map serde paths.
    let mut issuer_curves = std::collections::HashMap::new();
    issuer_curves.insert("ISSUER1".to_string(), std::sync::Arc::new(hazard.clone()));
    let issuer2_haz = HazardCurve::builder("ISSUER2-HAZ")
        .base_date(d)
        .knots([(0.0, 0.02), (5.0, 0.03)])
        .build()
        .unwrap();
    issuer_curves.insert(
        "ISSUER2".to_string(),
        std::sync::Arc::new(issuer2_haz.clone()),
    );
    let mut issuer_recovery = std::collections::HashMap::new();
    issuer_recovery.insert("ISSUER1".to_string(), 0.35);
    let mut issuer_weights = std::collections::HashMap::new();
    issuer_weights.insert("ISSUER1".to_string(), 0.6);
    issuer_weights.insert("ISSUER2".to_string(), 0.4);

    let credit_index = CreditIndexData::builder()
        .num_constituents(125)
        .recovery_rate(0.4)
        .index_credit_curve(std::sync::Arc::new(hazard.clone()))
        .base_correlation_curve(std::sync::Arc::new(base_corr.clone()))
        .with_issuer_curves(issuer_curves)
        .with_issuer_recovery_rates(issuer_recovery)
        .with_issuer_weights(issuer_weights)
        .build()
        .unwrap();

    let series = ScalarTimeSeries::new("TS", vec![(d, 1.0)], None)
        .unwrap()
        .with_interpolation(SeriesInterpolation::Linear);

    let surface = VolSurface::builder("VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0])
        .row(&[0.2, 0.2])
        .row(&[0.2, 0.2])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(discount_b)
        .insert_discount(discount_a)
        .insert_hazard(hazard)
        .insert_hazard(issuer2_haz)
        .insert_base_correlation(base_corr)
        .insert_credit_index("CDX", credit_index)
        .insert_series(series)
        .insert_surface(surface)
        .insert_price("EQ-SPOT", MarketScalar::Price(Money::new(100.0, Currency::USD)))
        .map_collateral("USD-CSA", CurveId::from("A-DISC"));

    let state = MarketContextState::from(&ctx);

    // Curves are sorted by id in the state representation.
    let ids: Vec<String> = state
        .curves
        .iter()
        .map(|c| match c {
            finstack_core::market_data::context::CurveState::Discount(dc) => dc.id().to_string(),
            finstack_core::market_data::context::CurveState::Forward(fc) => fc.id().to_string(),
            finstack_core::market_data::context::CurveState::Hazard(hc) => hc.id().to_string(),
            finstack_core::market_data::context::CurveState::Inflation(ic) => ic.id().to_string(),
            finstack_core::market_data::context::CurveState::BaseCorrelation(bc) => bc.id().to_string(),
        })
        .collect();
    assert_eq!(ids, vec!["A-DISC", "B-DISC", "CDX-BC", "CDX-HAZ", "ISSUER2-HAZ"]);

    // Full JSON roundtrip should preserve ability to resolve referenced curves
    let json = serde_json::to_string_pretty(&ctx).unwrap();
    let roundtripped: MarketContext = serde_json::from_str(&json).unwrap();

    assert!(roundtripped.get_discount("A-DISC").is_ok());
    assert!(roundtripped.get_discount("B-DISC").is_ok());
    assert!(roundtripped.get_hazard("CDX-HAZ").is_ok());
    assert!(roundtripped.get_base_correlation("CDX-BC").is_ok());
    assert!(roundtripped.credit_index("CDX").is_ok());
    assert!(roundtripped.series("TS").is_ok());
    assert!(roundtripped.surface("VOL").is_ok());
    assert!(roundtripped.price("EQ-SPOT").is_ok());
    assert!(roundtripped.collateral("USD-CSA").is_ok());
}

#[test]
fn curve_storage_roundtrip_and_market_context_state_error_branch() {
    use finstack_core::market_data::context::{CurveStorage, CurveState};

    // CurveStorage roundtrip through serde (uses CurveState internally)
    let storage = CurveStorage::from(DiscountCurve::builder("USD-OIS")
        .base_date(test_date())
        .knots([(0.0, 1.0), (1.0, 0.98)])
        .build()
        .unwrap());
    let json = serde_json::to_string(&storage).unwrap();
    let de: CurveStorage = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id().as_str(), "USD-OIS");

    // Also exercise CurveState serde directly
    let st = CurveState::Discount(
        DiscountCurve::builder("X")
            .base_date(test_date())
            .knots([(0.0, 1.0), (1.0, 0.99)])
            .build()
            .unwrap(),
    );
    let json = serde_json::to_string(&st).unwrap();
    let _back: CurveState = serde_json::from_str(&json).unwrap();

    // MarketContextState -> MarketContext error branch: credit index references missing curve IDs.
    let bad_state = MarketContextState {
        curves: vec![],
        surfaces: vec![],
        prices: std::collections::BTreeMap::new(),
        series: vec![],
        inflation_indices: vec![],
        credit_indices: vec![finstack_core::market_data::context::CreditIndexState {
            id: "CDX".to_string(),
            num_constituents: 125,
            recovery_rate: 0.4,
            index_credit_curve_id: "MISSING-HAZ".to_string(),
            base_correlation_curve_id: "MISSING-BC".to_string(),
            issuer_credit_curve_ids: None,
            issuer_recovery_rates: None,
            issuer_weights: None,
        }],
        collateral: std::collections::BTreeMap::new(),
    };
    assert!(MarketContext::try_from(bad_state).is_err());
}

#[test]
fn curve_state_and_storage_roundtrip_all_variants() {
    use finstack_core::market_data::context::{CurveState, CurveStorage};
    use finstack_core::market_data::term_structures::{
        base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve, forward_curve::ForwardCurve,
        hazard_curve::HazardCurve, inflation::InflationCurve,
    };

    let d = test_date();

    let disc = DiscountCurve::builder("DISC").base_date(d).knots([(0.0, 1.0), (1.0, 0.99)]).build().unwrap();
    let fwd = ForwardCurve::builder("FWD", 0.25).base_date(d).knots([(0.0, 0.02), (1.0, 0.03)]).build().unwrap();
    let haz = HazardCurve::builder("HAZ").base_date(d).knots([(0.0, 0.01), (5.0, 0.02)]).build().unwrap();
    let inf = InflationCurve::builder("INF").base_cpi(100.0).knots([(0.0, 100.0), (1.0, 101.0)]).build().unwrap();
    let bc = BaseCorrelationCurve::builder("BC").points([(3.0, 0.25), (7.0, 0.4)]).build().unwrap();

    let states = vec![
        CurveState::Discount(disc.clone()),
        CurveState::Forward(fwd.clone()),
        CurveState::Hazard(haz.clone()),
        CurveState::Inflation(inf.clone()),
        CurveState::BaseCorrelation(bc.clone()),
    ];
    for st in states {
        let json = serde_json::to_string(&st).unwrap();
        let _back: CurveState = serde_json::from_str(&json).unwrap();
    }

    let storages: Vec<CurveStorage> = vec![
        disc.into(),
        fwd.into(),
        haz.into(),
        inf.into(),
        bc.into(),
    ];
    for s in storages {
        let json = serde_json::to_string(&s).unwrap();
        let back: CurveStorage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id().as_str(), s.id().as_str());
    }
}

#[test]
fn market_context_state_roundtrip_hits_more_state_serde_lines() {
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::term_structures::{
        base_correlation::BaseCorrelationCurve, discount_curve::DiscountCurve, forward_curve::ForwardCurve,
        hazard_curve::HazardCurve, inflation::InflationCurve, credit_index::CreditIndexData,
    };
    use finstack_core::money::Money;
    use std::sync::Arc;

    let d = test_date();

    // Curves of all variants (exercise CurveState enum coverage)
    let disc = DiscountCurve::builder("USD-OIS").base_date(d).knots([(0.0, 1.0), (5.0, 0.9)]).build().unwrap();
    let fwd = ForwardCurve::builder("USD-SOFR", 0.25).base_date(d).knots([(0.0, 0.03), (5.0, 0.04)]).build().unwrap();
    let haz = HazardCurve::builder("CDX-HAZ").base_date(d).knots([(0.0, 0.01), (5.0, 0.02)]).build().unwrap();
    let issuer_haz = HazardCurve::builder("ISSUER-HAZ").base_date(d).knots([(0.0, 0.02), (5.0, 0.03)]).build().unwrap();
    let inf = InflationCurve::builder("US-CPI-CURVE").base_cpi(100.0).knots([(0.0, 100.0), (1.0, 101.0)]).build().unwrap();
    let bc = BaseCorrelationCurve::builder("CDX-BC").points([(3.0, 0.25), (7.0, 0.4)]).build().unwrap();

    let mut issuer_curves = std::collections::HashMap::new();
    issuer_curves.insert("ISSUER".to_string(), Arc::new(issuer_haz.clone()));

    let credit_index = CreditIndexData::builder()
        .num_constituents(125)
        .recovery_rate(0.4)
        .index_credit_curve(Arc::new(haz.clone()))
        .base_correlation_curve(Arc::new(bc.clone()))
        .with_issuer_curves(issuer_curves)
        .build()
        .unwrap();

    // Inflation index (separate from inflation curve)
    let idx = InflationIndex::new("US-CPI", vec![(d, 300.0)], Currency::USD)
        .unwrap()
        .with_interpolation(InflationInterpolation::Linear);

    let series = ScalarTimeSeries::new("TS", vec![(d, 1.0)], None)
        .unwrap()
        .with_interpolation(SeriesInterpolation::Linear);

    let surface = VolSurface::builder("VOL")
        .expiries(&[1.0, 2.0])
        .strikes(&[90.0, 100.0])
        .row(&[0.2, 0.2])
        .row(&[0.2, 0.2])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
        .insert_hazard(haz)
        .insert_hazard(issuer_haz)
        .insert_inflation(inf)
        .insert_base_correlation(bc)
        .insert_credit_index("CDX", credit_index)
        .insert_inflation_index("US-CPI", idx)
        .insert_series(series)
        .insert_surface(surface)
        .insert_price("EQ-SPOT", MarketScalar::Price(Money::new(100.0, Currency::USD)))
        .map_collateral("USD-CSA", CurveId::from("USD-OIS"));

    // Roundtrip via MarketContextState explicitly
    let state = MarketContextState::from(&ctx);
    let json = serde_json::to_string(&state).unwrap();
    let back_state: MarketContextState = serde_json::from_str(&json).unwrap();
    let rebuilt = MarketContext::try_from(back_state).unwrap();

    assert!(rebuilt.get_discount("USD-OIS").is_ok());
    assert!(rebuilt.get_forward("USD-SOFR").is_ok());
    assert!(rebuilt.get_hazard("CDX-HAZ").is_ok());
    assert!(rebuilt.get_hazard("ISSUER-HAZ").is_ok());
    assert!(rebuilt.get_inflation("US-CPI-CURVE").is_ok());
    assert!(rebuilt.get_base_correlation("CDX-BC").is_ok());
    assert!(rebuilt.credit_index("CDX").is_ok());
    assert!(rebuilt.inflation_index("US-CPI").is_some());
    assert!(rebuilt.series("TS").is_ok());
    assert!(rebuilt.surface("VOL").is_ok());
    assert!(rebuilt.price("EQ-SPOT").is_ok());
    assert!(rebuilt.collateral("USD-CSA").is_ok());
}
