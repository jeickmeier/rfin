//! Monte Carlo v0.8 integration tests - Multi-curve framework.
//!
//! Tests separation of OIS discounting from IBOR forwarding,
//! tenor basis spreads, and multi-curve caps/floors pricing.

#![cfg(feature = "mc")]

use finstack_valuations::instruments::common::mc::multi_curve::context::{
    DiscountCurve, FlatCurve, FlatForwardCurve, ForwardCurve, MultiCurveContext, Tenor,
    TenorBasis,
};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Basic Multi-Curve Tests
// ============================================================================

#[test]
fn test_single_curve_vs_multi_curve() {
    // Single curve (pre-crisis): same rate for discount and forward
    let single_curve_ctx = MultiCurveContext::single_curve(0.05);

    // Multi-curve (post-crisis): OIS < IBOR
    let ois = Arc::new(FlatCurve::new(0.04)) as Arc<dyn DiscountCurve>;
    let mut ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>> = HashMap::new();
    ibor_curves.insert(
        Tenor::M3,
        Arc::new(FlatForwardCurve::new(Tenor::M3, 0.045)),
    );

    let multi_curve_ctx = MultiCurveContext::new(ois, ibor_curves, vec![]);

    // Discount factors
    let df_single = single_curve_ctx.discount_factor(1.0);
    let df_multi = multi_curve_ctx.discount_factor(1.0);

    println!("Discount Factors:");
    println!("  Single curve: {:.6}", df_single);
    println!("  Multi-curve:  {:.6}", df_multi);

    // Multi-curve should have higher DF (lower OIS rate)
    assert!(df_multi > df_single);

    // Forward rates
    let fwd_single = single_curve_ctx.forward_rate(Tenor::M3, 0.5);
    let fwd_multi = multi_curve_ctx.forward_rate(Tenor::M3, 0.5);

    println!("Forward Rates (3M):");
    println!("  Single curve: {:.6}", fwd_single);
    println!("  Multi-curve:  {:.6}", fwd_multi);

    assert_eq!(fwd_single, 0.05);
    assert_eq!(fwd_multi, 0.045);
}

#[test]
fn test_tenor_basis_adjustment() {
    let ois = Arc::new(FlatCurve::new(0.04)) as Arc<dyn DiscountCurve>;
    
    let mut ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>> = HashMap::new();
    ibor_curves.insert(
        Tenor::M3,
        Arc::new(FlatForwardCurve::new(Tenor::M3, 0.045)),
    );
    ibor_curves.insert(
        Tenor::M6,
        Arc::new(FlatForwardCurve::new(Tenor::M6, 0.046)),
    );

    // 3M trades 12bp below 6M
    let tenor_basis = vec![TenorBasis::new(Tenor::M6, Tenor::M3, -12.0)];

    let context = MultiCurveContext::new(ois, ibor_curves, tenor_basis);

    let fwd_3m = context.forward_rate(Tenor::M3, 0.5);
    let fwd_6m = context.forward_rate(Tenor::M6, 0.5);

    println!("Forward Rates with Tenor Basis:");
    println!("  3M: {:.6}", fwd_3m);
    println!("  6M: {:.6}", fwd_6m);

    // 3M should be adjusted down by 12bp
    assert!((fwd_3m - (0.045 - 0.0012)).abs() < 1e-10);
    
    // 6M should be unadjusted
    assert_eq!(fwd_6m, 0.046);

    // Basis spread
    let basis = fwd_3m - fwd_6m;
    println!("  3M - 6M basis: {:.1} bp", basis * 10_000.0);
    assert!((basis - (-0.0022)).abs() < 1e-10); // -22bp total
}

#[test]
fn test_ois_discount_vs_ibor_forward() {
    // Key multi-curve property: different curves for different purposes
    let ois_rate = 0.03; // Lower (risk-free)
    let ibor_rate = 0.04; // Higher (credit risk)

    let ois = Arc::new(FlatCurve::new(ois_rate)) as Arc<dyn DiscountCurve>;
    let mut ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>> = HashMap::new();
    ibor_curves.insert(
        Tenor::M6,
        Arc::new(FlatForwardCurve::new(Tenor::M6, ibor_rate)),
    );

    let context = MultiCurveContext::new(ois, ibor_curves, vec![]);

    let df = context.discount_factor(1.0);
    let fwd = context.forward_rate(Tenor::M6, 1.0);

    println!("OIS vs IBOR:");
    println!("  OIS rate:  {:.4}", ois_rate);
    println!("  IBOR rate: {:.4}", ibor_rate);
    println!("  DF(1y):    {:.6}", df);
    println!("  Fwd(6M,1y):{:.6}", fwd);

    // Discount uses OIS
    assert!((df - (-ois_rate).exp()).abs() < 1e-10);
    
    // Forward uses IBOR
    assert_eq!(fwd, ibor_rate);
    
    // IBOR should be above OIS (reflects credit spread)
    assert!(fwd > ois_rate);
}

#[test]
fn test_available_tenors() {
    let context = MultiCurveContext::single_curve(0.05);
    
    let tenors = context.available_tenors();
    assert_eq!(tenors.len(), 4);
    
    assert!(context.has_tenor(Tenor::M1));
    assert!(context.has_tenor(Tenor::M3));
    assert!(context.has_tenor(Tenor::M6));
    assert!(context.has_tenor(Tenor::M12));
}

#[test]
fn test_multiple_tenor_basis_accumulate() {
    let ois = Arc::new(FlatCurve::new(0.04)) as Arc<dyn DiscountCurve>;
    
    let mut ibor_curves: HashMap<Tenor, Arc<dyn ForwardCurve>> = HashMap::new();
    ibor_curves.insert(
        Tenor::M3,
        Arc::new(FlatForwardCurve::new(Tenor::M3, 0.045)),
    );

    // Multiple basis adjustments for same tenor (should accumulate)
    let tenor_basis = vec![
        TenorBasis::new(Tenor::M6, Tenor::M3, -10.0),
        TenorBasis::new(Tenor::M12, Tenor::M3, -5.0),
    ];

    let context = MultiCurveContext::new(ois, ibor_curves, tenor_basis);

    let fwd_3m = context.forward_rate(Tenor::M3, 0.5);

    // Should have both adjustments: -10bp - 5bp = -15bp
    let expected = 0.045 - 0.0010 - 0.0005;
    assert!((fwd_3m - expected).abs() < 1e-10);
}

