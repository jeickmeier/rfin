//! Tests for MarketContext container functionality.

use finstack_core::{
    market_data::{
        context::MarketContext,
        multicurve::CurveSet,
        primitives::{MarketScalar, ScalarTimeSeries},
        surfaces::vol_surface::VolSurface,
        id::CurveId,
    },
    money::fx::{FxMatrix, FxProvider, FxConversionPolicy, FxRate},
    currency::Currency,
    dates::Date,
};

// Simple test FX provider
#[derive(Debug, Clone)]
struct TestFxProvider;

impl FxProvider for TestFxProvider {
    fn rate(
        &self,
        _from: Currency,
        _to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<FxRate> {
        #[cfg(feature = "decimal128")]
        return Ok(rust_decimal::Decimal::ONE);
        #[cfg(not(feature = "decimal128"))]
        return Ok(1.0);
    }
}

#[test]
fn test_market_context_new() {
    let ctx: MarketContext<TestFxProvider> = MarketContext::new();
    
    // Should start empty
    assert!(ctx.fx.is_none());
    assert!(ctx.surfaces.is_empty());
    assert!(ctx.prices.is_empty());
    assert!(ctx.series.is_empty());
}

#[test]
fn test_market_context_from_curve_set() {
    let curve_set = CurveSet::new();
    let ctx: MarketContext<TestFxProvider> = MarketContext::from_curve_set(curve_set);
    
    // Should have the curve set but other fields empty
    assert!(ctx.fx.is_none());
    assert!(ctx.surfaces.is_empty());
    assert!(ctx.prices.is_empty());
    assert!(ctx.series.is_empty());
}

#[test]
fn test_market_context_with_fx() {
    let fx_matrix = FxMatrix::new(TestFxProvider);
    let ctx = MarketContext::new().with_fx(fx_matrix);
    
    // Should have FX matrix
    assert!(ctx.fx.is_some());
}

#[test]
fn test_market_context_with_surface() {
    // Create a minimal vol surface
    let strikes = [90.0, 100.0, 110.0];
    let expiries = [0.25, 0.5, 1.0];
    
    let surface = VolSurface::builder("TEST_VOL")
        .strikes(&strikes)
        .expiries(&expiries)
        .row(&[0.20, 0.18, 0.16]) // 3M
        .row(&[0.22, 0.20, 0.18]) // 6M
        .row(&[0.24, 0.22, 0.20]) // 1Y
        .build()
        .unwrap();
    
    let ctx: MarketContext<TestFxProvider> = MarketContext::new().with_surface(surface);
    
    // Should have the surface
    assert_eq!(ctx.surfaces.len(), 1);
    assert!(ctx.surfaces.contains_key(&CurveId::new("TEST_VOL")));
}

#[test]
fn test_market_context_with_price() {
    let price = MarketScalar::Unitless(100.0);
    let ctx: MarketContext<TestFxProvider> = MarketContext::new().with_price("SPOT_PRICE", price);
    
    // Should have the price
    assert_eq!(ctx.prices.len(), 1);
    assert!(ctx.prices.contains_key(&CurveId::new("SPOT_PRICE")));
}

#[test]
fn test_market_context_with_series() {
    // Create a simple time series
    let dates = vec![
        Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
        Date::from_calendar_date(2025, time::Month::February, 1).unwrap(),
        Date::from_calendar_date(2025, time::Month::March, 1).unwrap(),
    ];
    let values = vec![100.0, 101.0, 102.0];
    
    let observations: Vec<(Date, f64)> = dates.into_iter().zip(values).collect();
    let series = ScalarTimeSeries::new("TEST_SERIES", observations, None).unwrap();
    
    let ctx: MarketContext<TestFxProvider> = MarketContext::new().with_series(series);
    
    // Should have the series
    assert_eq!(ctx.series.len(), 1);
    assert!(ctx.series.contains_key(&CurveId::new("TEST_SERIES")));
}

#[test]
fn test_market_context_vol_surface_getter() {
    // Create and add a vol surface (need at least 2 points)
    let strikes = [90.0, 100.0];
    let expiries = [0.5, 1.0];
    
    let surface = VolSurface::builder("TEST_VOL")
        .strikes(&strikes)
        .expiries(&expiries)
        .row(&[0.22, 0.20]) // 6M
        .row(&[0.24, 0.22]) // 1Y
        .build()
        .unwrap();
    
    let ctx: MarketContext<TestFxProvider> = MarketContext::new().with_surface(surface);
    
    // Should be able to retrieve the surface
    let retrieved = ctx.vol_surface("TEST_VOL");
    assert!(retrieved.is_ok());
    
    // Should error for non-existent surface
    let missing = ctx.vol_surface("MISSING");
    assert!(missing.is_err());
}

#[test]
fn test_market_context_market_scalar_getter() {
    let price = MarketScalar::Unitless(123.45);
    let ctx: MarketContext<TestFxProvider> = MarketContext::new().with_price("TEST_PRICE", price);
    
    // Should be able to retrieve the scalar
    let retrieved = ctx.market_scalar("TEST_PRICE");
    assert!(retrieved.is_ok());
    // Check the value based on the enum variant
    match retrieved.unwrap() {
        MarketScalar::Unitless(val) => assert_eq!(*val, 123.45),
        _ => panic!("Expected Unitless variant"),
    }
    
    // Should error for non-existent scalar
    let missing = ctx.market_scalar("MISSING");
    assert!(missing.is_err());
}

#[test]
fn test_market_context_scalar_time_series_getter() {
    let dates = vec![
        Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
        Date::from_calendar_date(2025, time::Month::February, 1).unwrap(),
    ];
    let values = vec![42.0, 43.0];
    
    let observations: Vec<(Date, f64)> = dates.into_iter().zip(values).collect();
    let series = ScalarTimeSeries::new("TEST_SERIES", observations, None).unwrap();
    
    let ctx: MarketContext<TestFxProvider> = MarketContext::new().with_series(series);
    
    // Should be able to retrieve the series
    let retrieved = ctx.scalar_time_series("TEST_SERIES");
    assert!(retrieved.is_ok());
    
    // Should error for non-existent series
    let missing = ctx.scalar_time_series("MISSING");
    assert!(missing.is_err());
}

#[test]
fn test_market_context_chaining() {
    // Test that all builder methods can be chained
    let fx_matrix = FxMatrix::new(TestFxProvider);
    let price = MarketScalar::Unitless(100.0);
    
    let strikes = [90.0, 100.0];
    let expiries = [0.5, 1.0];
    
    let surface = VolSurface::builder("VOL")
        .strikes(&strikes)
        .expiries(&expiries)
        .row(&[0.22, 0.20]) // 6M
        .row(&[0.24, 0.22]) // 1Y
        .build()
        .unwrap();
    
    let dates = vec![
        Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
        Date::from_calendar_date(2025, time::Month::February, 1).unwrap(),
    ];
    let values = vec![50.0, 51.0];
    
    let observations: Vec<(Date, f64)> = dates.into_iter().zip(values).collect();
    let series = ScalarTimeSeries::new("SERIES", observations, None).unwrap();
    
    let ctx: MarketContext<TestFxProvider> = MarketContext::new()
        .with_fx(fx_matrix)
        .with_surface(surface)
        .with_price("PRICE", price)
        .with_series(series);
    
    // Should have all components
    assert!(ctx.fx.is_some());
    assert_eq!(ctx.surfaces.len(), 1);
    assert_eq!(ctx.prices.len(), 1);
    assert_eq!(ctx.series.len(), 1);
}
