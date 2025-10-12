//! Tests for bucket filtering on volatility and base correlation surfaces.

use finstack_core::dates::Date;
use finstack_core::market_data::surfaces::vol_surface::VolSurface;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::MarketContext;
use finstack_scenarios::{
    ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec, VolSurfaceKind,
};
use finstack_statements::FinancialModelSpec;
use time::Month;

#[test]
fn test_vol_bucket_filtering_by_tenor() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create volatility surface with multiple expiries and strikes
    let surface = VolSurface::builder("SPX")
        .expiries(&[0.25, 0.5, 1.0, 2.0]) // 3M, 6M, 1Y, 2Y
        .strikes(&[90.0, 100.0, 110.0])
        .row(&[0.20, 0.18, 0.22]) // 3M row
        .row(&[0.21, 0.19, 0.23]) // 6M row
        .row(&[0.22, 0.20, 0.24]) // 1Y row
        .row(&[0.23, 0.21, 0.25]) // 2Y row
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_surface(surface);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Shock only the 1Y tenor (+10% vol)
    let scenario = ScenarioSpec {
        id: "vol_bucket_tenor".into(),
        name: Some("Vol Bucket by Tenor".into()),
        description: None,
        operations: vec![OperationSpec::VolSurfaceBucketPct {
            surface_kind: VolSurfaceKind::Equity,
            surface_id: "SPX".into(),
            tenors: Some(vec!["1Y".into()]),
            strikes: None, // All strikes at 1Y
            pct: 10.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify shocked surface
    let shocked_surface = market.surface_ref("SPX").unwrap();

    // 1Y expiry should be shocked (+10%)
    let shocked_1y_100k = shocked_surface.value(1.0, 100.0);
    let expected_1y = 0.20 * 1.10;
    assert!(
        (shocked_1y_100k - expected_1y).abs() < 1e-6,
        "Expected 1Y shock: {}, got {}",
        expected_1y,
        shocked_1y_100k
    );

    // Other expiries should be unchanged
    let unchanged_3m_100k = shocked_surface.value(0.25, 100.0);
    assert!(
        (unchanged_3m_100k - 0.18).abs() < 1e-6,
        "3M should be unchanged: expected 0.18, got {}",
        unchanged_3m_100k
    );
}

#[test]
fn test_vol_bucket_filtering_by_strike() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let surface = VolSurface::builder("SPX")
        .expiries(&[0.5, 1.0]) // Need at least 2 expiries
        .strikes(&[90.0, 100.0, 110.0])
        .row(&[0.21, 0.19, 0.23]) // 6M row
        .row(&[0.22, 0.20, 0.24]) // 1Y row
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_surface(surface);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Shock only 100 strike
    let scenario = ScenarioSpec {
        id: "vol_bucket_strike".into(),
        name: Some("Vol Bucket by Strike".into()),
        description: None,
        operations: vec![OperationSpec::VolSurfaceBucketPct {
            surface_kind: VolSurfaceKind::Equity,
            surface_id: "SPX".into(),
            tenors: None,
            strikes: Some(vec![100.0]),
            pct: 20.0, // +20% vol
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify 100 strike is shocked
    let shocked_surface = market.surface_ref("SPX").unwrap();
    let shocked_100 = shocked_surface.value(1.0, 100.0);
    let expected = 0.20 * 1.20;
    assert!((shocked_100 - expected).abs() < 1e-6);

    // 90 and 110 should be unchanged
    let unchanged_90 = shocked_surface.value(1.0, 90.0);
    assert!((unchanged_90 - 0.22).abs() < 1e-6);
}

#[test]
fn test_basecorr_bucket_filtering() {
    // Create base correlation curve
    let basecorr = BaseCorrelationCurve::builder("CDX_IG")
        .points(vec![
            (3.0, 0.25),  // 3% detachment
            (7.0, 0.45),  // 7% detachment
            (10.0, 0.60), // 10% detachment
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_base_correlation(basecorr);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Shock only 7% detachment (+5 correlation points)
    let scenario = ScenarioSpec {
        id: "basecorr_bucket".into(),
        name: Some("Base Corr Bucket".into()),
        description: None,
        operations: vec![OperationSpec::BaseCorrBucketPts {
            surface_id: "CDX_IG".into(),
            detachment_bps: Some(vec![700]), // 7% = 700bp
            maturities: None,
            points: 0.05, // +5 points
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify shocked curve
    let shocked_curve = market.get_base_correlation_ref("CDX_IG").unwrap();

    // 7% should be shocked
    let shocked_7 = shocked_curve.correlation(7.0);
    let expected = (0.45_f64 + 0.05).min(1.0); // Clamped to [0, 1]
    assert!(
        (shocked_7 - expected).abs() < 1e-6,
        "Expected 7% shock: {}, got {}",
        expected,
        shocked_7
    );

    // 3% should be unchanged
    let unchanged_3 = shocked_curve.correlation(3.0);
    assert!(
        (unchanged_3 - 0.25).abs() < 1e-6,
        "3% should be unchanged: expected 0.25, got {}",
        unchanged_3
    );
}
