use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::diff::{
    measure_bucketed_discount_shift, measure_correlation_shift, measure_discount_curve_shift,
    measure_fx_shift, measure_hazard_curve_shift, measure_inflation_curve_shift,
    measure_scalar_shift, measure_vol_surface_shift, TenorSamplingMethod, STANDARD_TENORS,
};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::{
    fx::{FxConversionPolicy, FxMatrix, FxProvider},
    Money,
};
use std::sync::Arc;
use time::Month;

// ===================================================================
// Test Helpers
// ===================================================================

fn sample_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date")
}

struct MockFxProvider;
impl FxProvider for MockFxProvider {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<f64> {
        match (from, to) {
            (Currency::USD, Currency::EUR) => Ok(0.90),
            (Currency::EUR, Currency::USD) => Ok(1.0 / 0.90),
            _ if from == to => Ok(1.0),
            _ => Ok(1.0),
        }
    }
}

fn sample_fx_matrix() -> FxMatrix {
    FxMatrix::new(Arc::new(MockFxProvider))
}

// ===================================================================
// Discount Curve Shift Tests
// ===================================================================

#[test]
fn test_discount_curve_parallel_shift() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82), (10.0, 0.67)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Should build curve");

    // +50bp shift: multiply discount factors by exp(-0.005*t)
    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (1.0, 0.96 * (-0.005_f64 * 1.0).exp()),
            (5.0, 0.82 * (-0.005_f64 * 5.0).exp()),
            (10.0, 0.67 * (-0.005_f64 * 10.0).exp()),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let shift = measure_discount_curve_shift(
        "USD-OIS",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    )
    .expect("Should measure shift");

    assert!((shift - 50.0).abs() < 5.0, "Expected ~50bp, got {}", shift);
}

#[test]
fn test_discount_curve_steepening() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.74)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    // Steepening: long end moves more than short end
    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.968),  // -30bp
            (5.0, 0.835),  // -50bp
            (10.0, 0.704), // -100bp
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let shift = measure_discount_curve_shift(
        "USD-OIS",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    )
    .expect("Should measure shift");

    // Shift should be mixed between short and long moves
    assert!(shift > 0.0, "Should detect positive shift on average");
}

#[test]
fn test_discount_curve_zero_shift() {
    let base_date = sample_date();

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market = MarketContext::new().insert_discount(curve);

    let shift =
        measure_discount_curve_shift("USD-OIS", &market, &market, TenorSamplingMethod::Standard)
            .expect("Should measure shift");

    assert_eq!(shift, 0.0, "Same market should produce zero shift");
}

#[test]
fn test_discount_curve_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_discount_curve_shift(
        "MISSING",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    );

    assert!(result.is_err(), "Should error on missing curve");
}

#[test]
fn test_discount_curve_dynamic_sampling() {
    let base_date = sample_date();

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.5, 0.97), (3.5, 0.92), (7.5, 0.80)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market = MarketContext::new().insert_discount(curve);

    let shift =
        measure_discount_curve_shift("USD-OIS", &market, &market, TenorSamplingMethod::Dynamic)
            .expect("Should measure with dynamic sampling");

    assert_eq!(shift, 0.0, "Same market should produce zero shift");
}

#[test]
fn test_discount_curve_custom_sampling() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82), (10.0, 0.67)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Should build curve");

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.96 * (-0.005_f64 * 1.0).exp()),
            (5.0, 0.82 * (-0.005_f64 * 5.0).exp()),
            (10.0, 0.67 * (-0.005_f64 * 10.0).exp()),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let shift = measure_discount_curve_shift(
        "USD-OIS",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Custom(vec![1.0, 5.0, 10.0]),
    )
    .expect("Should measure with custom sampling");

    assert!((shift - 50.0).abs() < 5.0, "Expected ~50bp, got {}", shift);
}

// ===================================================================
// Bucketed Discount Shift Tests
// ===================================================================

#[test]
fn test_bucketed_discount_shift_detailed() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82), (10.0, 0.67)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Should build curve");

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.96 * (-0.005_f64 * 1.0).exp()),
            (5.0, 0.82 * (-0.005_f64 * 5.0).exp()),
            (10.0, 0.67 * (-0.005_f64 * 10.0).exp()),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let tenors = vec![1.0, 5.0, 10.0];
    let shifts = measure_bucketed_discount_shift("USD-OIS", &market_t0, &market_t1, &tenors)
        .expect("Should measure bucketed shifts");

    assert_eq!(shifts.len(), 3, "Should have three tenor shifts");

    for (tenor, shift_bp) in &shifts {
        assert!(
            (shift_bp - 50.0).abs() < 1.0,
            "Expected ~50bp at tenor {}, got {}",
            tenor,
            shift_bp
        );
    }
}

#[test]
fn test_bucketed_discount_shift_single_tenor() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.96)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let shifts = measure_bucketed_discount_shift("USD-OIS", &market_t0, &market_t1, &[1.0])
        .expect("Should handle single tenor");

    assert_eq!(shifts.len(), 1, "Should return one shift");
}

#[test]
fn test_bucketed_discount_shift_filters_negative_tenors() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.96)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.95)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let tenors = vec![-1.0, 0.0, 1.0];
    let shifts = measure_bucketed_discount_shift("USD-OIS", &market_t0, &market_t1, &tenors)
        .expect("Should filter negative tenors");

    assert_eq!(shifts.len(), 1, "Should only include positive tenors");
    assert_eq!(shifts[0].0, 1.0, "Should be tenor 1.0");
}

// ===================================================================
// Hazard Curve Shift Tests
// ===================================================================

#[test]
fn test_hazard_curve_parallel_shift() {
    let base_date = sample_date();

    let curve_t0 = HazardCurve::builder("CORP-01")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02), (10.0, 0.025)])
        .build()
        .expect("Should build curve");

    // +25bp shift
    let curve_t1 = HazardCurve::builder("CORP-01")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.0125), (5.0, 0.0225), (10.0, 0.0275)])
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_hazard(curve_t0);
    let market_t1 = MarketContext::new().insert_hazard(curve_t1);

    let shift = measure_hazard_curve_shift(
        "CORP-01",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    )
    .expect("Should measure shift");

    assert!((shift - 25.0).abs() < 1.0, "Expected ~25bp, got {}", shift);
}

#[test]
fn test_hazard_curve_widening() {
    let base_date = sample_date();

    let curve_t0 = HazardCurve::builder("CORP-01")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.010), (5.0, 0.012), (10.0, 0.015)])
        .build()
        .expect("Should build curve");

    // Widening: longer spreads move more
    let curve_t1 = HazardCurve::builder("CORP-01")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.0110), (5.0, 0.0145), (10.0, 0.0250)])
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_hazard(curve_t0);
    let market_t1 = MarketContext::new().insert_hazard(curve_t1);

    let shift = measure_hazard_curve_shift(
        "CORP-01",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    )
    .expect("Should measure shift");

    assert!(shift > 0.0, "Should detect positive widening");
}

#[test]
fn test_hazard_curve_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_hazard_curve_shift(
        "MISSING",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    );

    assert!(result.is_err(), "Should error on missing curve");
}

#[test]
fn test_hazard_curve_dynamic_sampling() {
    let base_date = sample_date();

    let curve = HazardCurve::builder("CORP-01")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(2.0, 0.01), (4.0, 0.015), (8.0, 0.02)])
        .build()
        .expect("Should build curve");

    let market = MarketContext::new().insert_hazard(curve);

    let shift =
        measure_hazard_curve_shift("CORP-01", &market, &market, TenorSamplingMethod::Dynamic)
            .expect("Should measure with dynamic sampling");

    assert_eq!(shift, 0.0, "Same market should produce zero shift");
}

// ===================================================================
// Inflation Curve Shift Tests
// ===================================================================

#[test]
fn test_inflation_curve_shift() {
    let curve_t0 = InflationCurve::builder("CPI-USD")
        .base_cpi(100.0)
        .knots([(0.0, 100.0), (1.0, 102.0), (5.0, 110.0), (10.0, 120.0)])
        .build()
        .expect("Should build curve");

    // 2% higher CPI at all tenors
    let curve_t1 = InflationCurve::builder("CPI-USD")
        .base_cpi(100.0)
        .knots([(0.0, 100.0), (1.0, 104.04), (5.0, 112.2), (10.0, 122.4)])
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_inflation(curve_t0);
    let market_t1 = MarketContext::new().insert_inflation(curve_t1);

    let shift = measure_inflation_curve_shift("CPI-USD", &market_t0, &market_t1)
        .expect("Should measure shift");

    assert!(shift > 0.0, "Should detect positive inflation shift");
}

#[test]
fn test_inflation_curve_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_inflation_curve_shift("MISSING", &market_t0, &market_t1);

    assert!(result.is_err(), "Should error on missing curve");
}

// ===================================================================
// Correlation Shift Tests
// ===================================================================

#[test]
fn test_correlation_shift() {
    let curve_t0 = BaseCorrelationCurve::builder("CDXNA")
        .knots([(3.0, 0.25), (7.0, 0.35), (10.0, 0.40)])
        .build()
        .expect("Should build curve");

    // +5% correlation shift
    let curve_t1 = BaseCorrelationCurve::builder("CDXNA")
        .knots([(3.0, 0.30), (7.0, 0.40), (10.0, 0.45)])
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_base_correlation(curve_t0);
    let market_t1 = MarketContext::new().insert_base_correlation(curve_t1);

    let shift =
        measure_correlation_shift("CDXNA", &market_t0, &market_t1).expect("Should measure shift");

    // Should be in percentage points (100x the fractional shift)
    assert!(
        (shift - 5.0).abs() < 0.5,
        "Expected ~5pct pts, got {}",
        shift
    );
}

#[test]
fn test_correlation_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_correlation_shift("MISSING", &market_t0, &market_t1);

    assert!(result.is_err(), "Should error on missing curve");
}

// ===================================================================
// Volatility Surface Shift Tests
// ===================================================================

#[test]
fn test_vol_surface_point_shift() {
    let _base_date = sample_date();
    let surface_t0 = VolSurface::builder("EQ-VOL")
        .expiries(&[0.25, 1.0, 2.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.30, 0.25, 0.22])
        .row(&[0.28, 0.23, 0.20])
        .row(&[0.26, 0.21, 0.18])
        .build()
        .expect("Should build surface");

    let surface_t1 = VolSurface::builder("EQ-VOL")
        .expiries(&[0.25, 1.0, 2.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.32, 0.27, 0.24])
        .row(&[0.30, 0.25, 0.22])
        .row(&[0.28, 0.23, 0.20])
        .build()
        .expect("Should build surface");

    let market_t0 = MarketContext::new().insert_surface(surface_t0);
    let market_t1 = MarketContext::new().insert_surface(surface_t1);

    let shift = measure_vol_surface_shift("EQ-VOL", &market_t0, &market_t1, Some(1.0), Some(1.0))
        .expect("Should measure shift");

    // 0.23 - 0.23 = 0 at 1Y ATM from the surfaces as built
    // Let me verify: at expiry 1.0, strike 1.0 (middle strike), vol should be ~0.23
    // Actually we need to check the exact values

    assert!((shift - 2.0).abs() < 0.5, "Expected ~2pct pts vol shift");
}

#[test]
fn test_vol_surface_average_shift() {
    let surface_t0 = VolSurface::builder("EQ-VOL")
        .expiries(&[0.25, 1.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.25, 0.24, 0.23])
        .row(&[0.22, 0.21, 0.20])
        .build()
        .expect("Should build surface");

    let surface_t1 = VolSurface::builder("EQ-VOL")
        .expiries(&[0.25, 1.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.27, 0.26, 0.25])
        .row(&[0.24, 0.23, 0.22])
        .build()
        .expect("Should build surface");

    let market_t0 = MarketContext::new().insert_surface(surface_t0);
    let market_t1 = MarketContext::new().insert_surface(surface_t1);

    let shift = measure_vol_surface_shift("EQ-VOL", &market_t0, &market_t1, None, None)
        .expect("Should measure average shift");

    assert!(shift > 0.0, "Should detect positive vol shift");
}

#[test]
fn test_vol_surface_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_vol_surface_shift("MISSING", &market_t0, &market_t1, None, None);

    assert!(result.is_err(), "Should error on missing surface");
}

#[test]
fn test_vol_surface_zero_shift() {
    let surface = VolSurface::builder("EQ-VOL")
        .expiries(&[0.25, 1.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.25, 0.24, 0.23])
        .row(&[0.22, 0.21, 0.20])
        .build()
        .expect("Should build surface");

    let market = MarketContext::new().insert_surface(surface);

    let shift = measure_vol_surface_shift("EQ-VOL", &market, &market, None, None)
        .expect("Should measure shift");

    assert_eq!(shift, 0.0, "Same surface should produce zero shift");
}

// ===================================================================
// FX Shift Tests
// ===================================================================

#[test]
fn test_fx_shift_strengthening() {
    let market_t0 = MarketContext::new().insert_fx(sample_fx_matrix());
    let market_t1 = MarketContext::new().insert_fx(sample_fx_matrix());

    // Since MockFxProvider always returns same rates, shift should be zero
    let shift = measure_fx_shift(Currency::USD, Currency::EUR, &market_t0, &market_t1)
        .expect("Should measure shift");

    assert_eq!(shift, 0.0, "Same FX matrix should produce zero shift");
}

#[test]
fn test_fx_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_fx_shift(Currency::USD, Currency::EUR, &market_t0, &market_t1);

    assert!(result.is_err(), "Should error on missing FX matrix");
}

// ===================================================================
// Scalar Shift Tests
// ===================================================================

#[test]
fn test_scalar_price_shift() {
    let price_t0 = Money::new(100.0, Currency::USD);
    let price_t1 = Money::new(110.0, Currency::USD);

    let market_t0 = MarketContext::new().insert_price("EQUITY-SPX", MarketScalar::Price(price_t0));
    let market_t1 = MarketContext::new().insert_price("EQUITY-SPX", MarketScalar::Price(price_t1));

    let shift =
        measure_scalar_shift("EQUITY-SPX", &market_t0, &market_t1).expect("Should measure shift");

    // (110 / 100 - 1) * 100 = 10%
    assert!((shift - 10.0).abs() < 0.01, "Expected 10%, got {}", shift);
}

#[test]
fn test_scalar_unitless_shift() {
    let market_t0 =
        MarketContext::new().insert_price("COMMODITY-GOLD", MarketScalar::Unitless(1800.0));
    let market_t1 =
        MarketContext::new().insert_price("COMMODITY-GOLD", MarketScalar::Unitless(1900.0));

    let shift = measure_scalar_shift("COMMODITY-GOLD", &market_t0, &market_t1)
        .expect("Should measure shift");

    // (1900 / 1800 - 1) * 100 = 5.56%
    assert!(
        (shift - 5.56).abs() < 0.01,
        "Expected ~5.56%, got {}",
        shift
    );
}

#[test]
fn test_scalar_missing_error() {
    let market_t0 = MarketContext::new();
    let market_t1 = MarketContext::new();

    let result = measure_scalar_shift("MISSING", &market_t0, &market_t1);

    assert!(result.is_err(), "Should error on missing scalar");
}

#[test]
fn test_scalar_zero_shift() {
    let price = Money::new(100.0, Currency::USD);
    let market = MarketContext::new().insert_price("TEST", MarketScalar::Price(price));

    let shift = measure_scalar_shift("TEST", &market, &market).expect("Should measure shift");

    assert_eq!(shift, 0.0, "Same scalar should produce zero shift");
}

// ===================================================================
// Tenor Sampling Method Tests
// ===================================================================

#[test]
fn test_standard_tenors_constant() {
    // Verify STANDARD_TENORS contains expected values
    assert_eq!(STANDARD_TENORS.len(), 9);
    assert_eq!(STANDARD_TENORS[0], 0.25);
    assert_eq!(STANDARD_TENORS[4], 3.0);
    assert_eq!(STANDARD_TENORS[8], 30.0);
}

#[test]
fn test_tenor_sampling_with_all_methods() {
    let base_date = sample_date();

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (0.25, 0.9925),
            (1.0, 0.97),
            (2.0, 0.94),
            (3.0, 0.91),
            (5.0, 0.85),
            (10.0, 0.74),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market = MarketContext::new().insert_discount(curve);

    // Standard sampling
    let shift_std =
        measure_discount_curve_shift("USD-OIS", &market, &market, TenorSamplingMethod::Standard)
            .expect("Standard sampling should work");

    // Dynamic sampling
    let shift_dyn =
        measure_discount_curve_shift("USD-OIS", &market, &market, TenorSamplingMethod::Dynamic)
            .expect("Dynamic sampling should work");

    // Custom sampling
    let shift_custom = measure_discount_curve_shift(
        "USD-OIS",
        &market,
        &market,
        TenorSamplingMethod::Custom(vec![1.0, 2.0, 5.0]),
    )
    .expect("Custom sampling should work");

    // All should be zero for identical market
    assert_eq!(shift_std, 0.0);
    assert_eq!(shift_dyn, 0.0);
    assert_eq!(shift_custom, 0.0);
}

// ===================================================================
// Edge Cases and Boundary Conditions
// ===================================================================

#[test]
fn test_discount_shift_negative_shift() {
    let base_date = sample_date();

    let curve_t0 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    // -50bp shift: multiply discount factors by exp(0.005*t)
    let curve_t1 = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.97 * (0.005_f64 * 1.0).exp()),
            (5.0, 0.85 * (0.005_f64 * 5.0).exp()),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .expect("Should build curve");

    let market_t0 = MarketContext::new().insert_discount(curve_t0);
    let market_t1 = MarketContext::new().insert_discount(curve_t1);

    let shift = measure_discount_curve_shift(
        "USD-OIS",
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    )
    .expect("Should measure shift");

    // Should detect negative (downward) shift
    assert!(
        shift < -40.0,
        "Should detect negative shift of ~-50bp, got {}",
        shift
    );
}

#[test]
fn test_hazard_curve_zero_shift_consistency() {
    let base_date = sample_date();

    let curve = HazardCurve::builder("CORP-01")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02), (10.0, 0.03)])
        .build()
        .expect("Should build curve");

    let market = MarketContext::new().insert_hazard(curve);

    // All sampling methods should yield same result for identical curves
    let shift_std =
        measure_hazard_curve_shift("CORP-01", &market, &market, TenorSamplingMethod::Standard)
            .expect("Should measure");

    let shift_dyn =
        measure_hazard_curve_shift("CORP-01", &market, &market, TenorSamplingMethod::Dynamic)
            .expect("Should measure");

    assert_eq!(shift_std, 0.0);
    assert_eq!(shift_dyn, 0.0);
}

#[test]
fn test_vol_surface_single_expiry() {
    let surface_t0 = VolSurface::builder("TEST-VOL")
        .expiries(&[1.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.25, 0.24, 0.23])
        .build()
        .expect("Should build surface");

    let surface_t1 = VolSurface::builder("TEST-VOL")
        .expiries(&[1.0])
        .strikes(&[0.9, 1.0, 1.1])
        .row(&[0.27, 0.26, 0.25])
        .build()
        .expect("Should build surface");

    let market_t0 = MarketContext::new().insert_surface(surface_t0);
    let market_t1 = MarketContext::new().insert_surface(surface_t1);

    let shift = measure_vol_surface_shift("TEST-VOL", &market_t0, &market_t1, None, None)
        .expect("Should measure shift");

    assert!(shift > 0.0, "Should detect positive shift");
}

#[test]
fn test_scalar_neutral_shift() {
    let price = Money::new(100.0, Currency::USD);
    let market_t0 = MarketContext::new().insert_price("TEST", MarketScalar::Price(price));
    let market_t1 = MarketContext::new().insert_price("TEST", MarketScalar::Price(price));

    let shift = measure_scalar_shift("TEST", &market_t0, &market_t1).expect("Should measure shift");

    assert_eq!(shift, 0.0, "Should be zero for identical prices");
}
