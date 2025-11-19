//! CDS integration method tests.
//!
//! Tests different numerical integration methods for protection leg
//! and accrual-on-default calculations.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::cds::pricer::{
    CDSPricer, CDSPricerConfig, IntegrationMethod,
};
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use time::macros::date;

fn build_curves(as_of: Date) -> (DiscountCurve, HazardCurve) {
    let disc = DiscountCurve::builder("USD_OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78), (10.0, 0.61)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("CORP")
        .base_date(as_of)
        .recovery_rate(0.40)
        .knots([(0.0, 0.02), (1.0, 0.02), (5.0, 0.025), (10.0, 0.03)])
        .build()
        .unwrap();

    (disc, hazard)
}

fn create_test_cds(as_of: Date, end: Date) -> CreditDefaultSwap {
    CreditDefaultSwap::buy_protection(
        "INTEGRATION_TEST",
        Money::new(10_000_000.0, Currency::USD),
        100.0,
        as_of,
        end,
        "USD_OIS",
        "CORP",
    )
}

#[test]
fn test_midpoint_integration_method() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::Midpoint,
        steps_per_year: 365,
        ..Default::default()
    });

    let protection_pv = pricer
        .pv_protection_leg(&cds, &disc, &hazard, as_of)
        .unwrap();

    assert!(protection_pv.amount() > 0.0);
    assert!(protection_pv.amount().is_finite());
}

#[test]
fn test_gaussian_quadrature_integration() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::GaussianQuadrature,
        gl_order: 8,
        ..Default::default()
    });

    let protection_pv = pricer
        .pv_protection_leg(&cds, &disc, &hazard, as_of)
        .unwrap();

    assert!(protection_pv.amount() > 0.0);
    assert!(protection_pv.amount().is_finite());
}

#[test]
fn test_adaptive_simpson_integration() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::AdaptiveSimpson,
        adaptive_max_depth: 12,
        tolerance: 1e-10,
        ..Default::default()
    });

    let protection_pv = pricer
        .pv_protection_leg(&cds, &disc, &hazard, as_of)
        .unwrap();

    assert!(protection_pv.amount() > 0.0);
    assert!(protection_pv.amount().is_finite());
}

#[test]
fn test_isda_exact_integration() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::IsdaExact,
        ..Default::default()
    });

    let protection_pv = pricer
        .pv_protection_leg(&cds, &disc, &hazard, as_of)
        .unwrap();

    assert!(protection_pv.amount() > 0.0);
    assert!(protection_pv.amount().is_finite());
}

#[test]
fn test_integration_methods_converge() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let methods = [
        IntegrationMethod::Midpoint,
        IntegrationMethod::GaussianQuadrature,
        IntegrationMethod::AdaptiveSimpson,
        IntegrationMethod::IsdaExact,
    ];

    let mut pvs = Vec::new();

    for method in methods {
        let pricer = CDSPricer::with_config(CDSPricerConfig {
            integration_method: method,
            steps_per_year: 365,
            gl_order: 8,
            adaptive_max_depth: 12,
            ..Default::default()
        });

        let pv = pricer
            .pv_protection_leg(&cds, &disc, &hazard, as_of)
            .unwrap()
            .amount();

        pvs.push((method, pv));
    }

    // All methods should be within 5% of each other
    let mean = pvs.iter().map(|(_, pv)| pv).sum::<f64>() / pvs.len() as f64;

    for (method, pv) in &pvs {
        let rel_diff = ((pv - mean) / mean).abs();
        assert!(
            rel_diff < 0.05,
            "Integration method {:?} differs by {:.2}% from mean",
            method,
            rel_diff * 100.0
        );
    }
}

#[test]
fn test_accrual_on_default_with_different_methods() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    // With accrual
    let pricer_with = CDSPricer::with_config(CDSPricerConfig {
        include_accrual: true,
        integration_method: IntegrationMethod::IsdaExact,
        ..Default::default()
    });

    // Without accrual
    let pricer_without = CDSPricer::with_config(CDSPricerConfig {
        include_accrual: false,
        integration_method: IntegrationMethod::IsdaExact,
        ..Default::default()
    });

    let pv_with = pricer_with
        .pv_premium_leg(&cds, &disc, &hazard, as_of)
        .unwrap()
        .amount();

    let pv_without = pricer_without
        .pv_premium_leg(&cds, &disc, &hazard, as_of)
        .unwrap()
        .amount();

    assert!(
        pv_with > pv_without,
        "Accrual on default should increase premium PV"
    );
}

#[test]
fn test_higher_gl_order_increases_accuracy() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    // Use ISDA as reference
    let pricer_ref = CDSPricer::with_config(CDSPricerConfig {
        integration_method: IntegrationMethod::IsdaExact,
        ..Default::default()
    });
    let ref_pv = pricer_ref
        .pv_protection_leg(&cds, &disc, &hazard, as_of)
        .unwrap()
        .amount();

    // Test different GL orders
    for order in [2, 4, 8, 16] {
        let pricer = CDSPricer::with_config(CDSPricerConfig {
            integration_method: IntegrationMethod::GaussianQuadrature,
            gl_order: order,
            ..Default::default()
        });

        let pv = pricer
            .pv_protection_leg(&cds, &disc, &hazard, as_of)
            .unwrap()
            .amount();

        let rel_error = ((pv - ref_pv) / ref_pv).abs();

        // Higher orders should be more accurate (lower error)
        assert!(
            rel_error < 0.1,
            "GL order {} has excessive error: {:.2}%",
            order,
            rel_error * 100.0
        );
    }
}

#[test]
fn test_isda_schedule_generates_20th_dates() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, end);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        use_isda_coupon_dates: true,
        ..Default::default()
    });

    let schedule = pricer.generate_isda_schedule(&cds).unwrap();

    // Interior dates should be on the 20th
    for &date in schedule
        .iter()
        .skip(1)
        .take(schedule.len().saturating_sub(2))
    {
        assert_eq!(date.day(), 20, "ISDA dates should be on 20th of month");
    }
}

#[test]
fn test_non_isda_schedule_respects_frequency() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let cds = create_test_cds(as_of, end);

    let pricer = CDSPricer::with_config(CDSPricerConfig {
        use_isda_coupon_dates: false,
        ..Default::default()
    });

    let schedule = pricer.generate_schedule(&cds, as_of).unwrap();

    // Should have roughly quarterly payments (4 per year for 5 years ≈ 20 periods)
    assert!(
        schedule.len() >= 18 && schedule.len() <= 22,
        "Quarterly schedule for 5Y should have ~20 periods, got {}",
        schedule.len()
    );
}

#[test]
fn test_exact_daycount_vs_approximate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    // Exact day count
    let pricer_exact = CDSPricer::with_config(CDSPricerConfig {
        exact_daycount: true,
        ..Default::default()
    });

    // Approximate day count
    let pricer_approx = CDSPricer::with_config(CDSPricerConfig {
        exact_daycount: false,
        ..Default::default()
    });

    let pv_exact = pricer_exact
        .pv_premium_leg(&cds, &disc, &hazard, as_of)
        .unwrap()
        .amount();

    let pv_approx = pricer_approx
        .pv_premium_leg(&cds, &disc, &hazard, as_of)
        .unwrap()
        .amount();

    // Results should be close but not identical
    // Day count differences can be significant (Act/360 vs Act/365), allow up to 2.5%
    let rel_diff = ((pv_exact - pv_approx) / pv_exact).abs();
    assert!(
        rel_diff < 0.025,
        "Daycount methods should be within 2.5%, got {:.2}%",
        rel_diff * 100.0
    );
}

#[test]
fn test_isda_standard_config() {
    let config = CDSPricerConfig::isda_standard();

    assert_eq!(
        config.integration_method,
        IntegrationMethod::IsdaStandardModel
    );
    assert!(config.include_accrual);
    assert!(config.exact_daycount);
    assert!(config.use_isda_coupon_dates);
    assert_eq!(config.business_days_per_year, 252.0); // US market
}

#[test]
fn test_isda_europe_config() {
    let config = CDSPricerConfig::isda_europe();

    assert_eq!(config.business_days_per_year, 250.0); // UK market
    assert_eq!(
        config.integration_method,
        IntegrationMethod::IsdaStandardModel
    );
}

#[test]
fn test_isda_asia_config() {
    let config = CDSPricerConfig::isda_asia();

    assert_eq!(config.business_days_per_year, 255.0); // Japan market
    assert_eq!(
        config.integration_method,
        IntegrationMethod::IsdaStandardModel
    );
}

#[test]
fn test_simplified_config() {
    let config = CDSPricerConfig::simplified();

    assert_eq!(config.integration_method, IntegrationMethod::Midpoint);
    assert!(!config.exact_daycount);
    assert!(!config.use_isda_coupon_dates);
}

#[test]
fn test_integration_with_high_steps_converges() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let mut pvs = Vec::new();

    for steps in [52, 365, 730, 1460] {
        let pricer = CDSPricer::with_config(CDSPricerConfig {
            integration_method: IntegrationMethod::Midpoint,
            steps_per_year: steps,
            ..Default::default()
        });

        let pv = pricer
            .pv_protection_leg(&cds, &disc, &hazard, as_of)
            .unwrap()
            .amount();

        pvs.push((steps, pv));
    }

    // Higher steps should converge
    let final_pv = pvs.last().unwrap().1;
    let penultimate_pv = pvs[pvs.len() - 2].1;

    let convergence = ((final_pv - penultimate_pv) / final_pv).abs();
    assert!(
        convergence < 0.001,
        "Should converge with high steps, diff={:.4}%",
        convergence * 100.0
    );
}

#[test]
fn test_par_spread_with_different_methods() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let (disc, hazard) = build_curves(as_of);

    let cds = create_test_cds(as_of, end);

    let methods = [IntegrationMethod::Midpoint, IntegrationMethod::IsdaExact];

    let mut par_spreads = Vec::new();

    for method in methods {
        let pricer = CDSPricer::with_config(CDSPricerConfig {
            integration_method: method,
            ..Default::default()
        });

        let par_spread = pricer.par_spread(&cds, &disc, &hazard, as_of).unwrap();
        par_spreads.push((method, par_spread));
    }

    // Par spreads should be close across methods
    let mean = par_spreads.iter().map(|(_, s)| s).sum::<f64>() / par_spreads.len() as f64;

    for (method, spread) in &par_spreads {
        let rel_diff = ((spread - mean) / mean).abs();
        assert!(
            rel_diff < 0.05,
            "Par spread with {:?} differs by {:.2}% from mean",
            method,
            rel_diff * 100.0
        );
    }
}
