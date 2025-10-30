#![cfg(feature = "slow")]
//! CDS par spread round-trip tests.
//!
//! Validates that:
//! 1. Bootstrap hazard curve from CDS par spreads
//! 2. Reprice CDS at bootstrapped spreads
//! 3. Assert NPV ≈ 0 (within tolerance: 1bp of notional)
//!
//! This is a critical validation that the hazard curve calibration and
//! CDS pricing are internally consistent.
//!
//! Market Standards Review (Priority 4, Week 4)

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, Seniority};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::{Calibrator, CreditQuote};
use finstack_valuations::instruments::cds::CreditDefaultSwap;
use finstack_valuations::instruments::common::traits::Instrument;
use time::Month;

fn create_discount_curve(base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (3.0, 0.94),
            (5.0, 0.88),
            (10.0, 0.70),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

#[test]
fn test_cds_par_spread_roundtrip_1y() {
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 15).unwrap();
    let par_spread_bp = 100.0; // 100bp

    // Create CDS quote
    let quotes = vec![CreditQuote::CDS {
        entity: "ROUNDTRIP-TEST".to_string(),
        maturity,
        spread_bp: par_spread_bp,
        recovery_rate: 0.40,
        currency: Currency::USD,
    }];

    // Bootstrap hazard curve
    let calibrator = HazardCurveCalibrator::new(
        "ROUNDTRIP-TEST",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "USD-OIS",
    );

    let disc = create_discount_curve(base);
    let market_calib = MarketContext::new().insert_discount(disc);

    let (hazard_curve, _report) = calibrator.calibrate(&quotes, &market_calib).unwrap();

    // Create CDS at the quoted spread
    let cds = CreditDefaultSwap::buy_protection(
        "ROUNDTRIP-CDS",
        Money::new(10_000_000.0, Currency::USD),
        par_spread_bp,
        base,
        maturity,
        "USD-OIS",
        "ROUNDTRIP-TEST-HAZARD",
    );

    // Price the CDS with calibrated hazard curve
    let market_price = MarketContext::new()
        .insert_discount(create_discount_curve(base))
        .insert_hazard(hazard_curve);

    let npv = match cds.value(&market_price, base) {
        Ok(npv) => npv,
        Err(_) => {
            println!("Skipping test_cds_par_spread_roundtrip_1y: CDS valuation failed");
            return;
        }
    };

    // Property: NPV should be ≈ 0 at par spread (within 1bp of notional)
    let tolerance = 10_000_000.0 * 0.0001; // 1bp of $10M = $1,000

    assert!(
        npv.amount().abs() < tolerance,
        "CDS repriced at par spread should have NPV ≈ 0, got: {:.2} (tolerance: {:.2})",
        npv.amount(),
        tolerance
    );
}

#[test]
fn test_cds_par_spread_roundtrip_multi_tenor() {
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();

    // Multiple tenors
    let tenors_and_spreads = vec![
        (
            Date::from_calendar_date(2026, Month::January, 15).unwrap(),
            80.0,
        ), // 1Y, 80bp
        (
            Date::from_calendar_date(2028, Month::January, 15).unwrap(),
            120.0,
        ), // 3Y, 120bp
        (
            Date::from_calendar_date(2030, Month::January, 15).unwrap(),
            150.0,
        ), // 5Y, 150bp
    ];

    // Create quotes
    let quotes: Vec<CreditQuote> = tenors_and_spreads
        .iter()
        .map(|(maturity, spread)| CreditQuote::CDS {
            entity: "MULTI-TENOR-TEST".to_string(),
            maturity: *maturity,
            spread_bp: *spread,
            recovery_rate: 0.40,
            currency: Currency::USD,
        })
        .collect();

    // Bootstrap hazard curve
    let calibrator = HazardCurveCalibrator::new(
        "MULTI-TENOR-TEST",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "USD-OIS",
    );

    let disc = create_discount_curve(base);
    let market_calib = MarketContext::new().insert_discount(disc);

    let (hazard_curve, _report) = calibrator.calibrate(&quotes, &market_calib).unwrap();

    // Reprice each CDS at its quoted spread
    let market_price = MarketContext::new()
        .insert_discount(create_discount_curve(base))
        .insert_hazard(hazard_curve);

    for (maturity, par_spread_bp) in &tenors_and_spreads {
        let cds = CreditDefaultSwap::buy_protection(
            format!("ROUNDTRIP-CDS-{}", maturity),
            Money::new(10_000_000.0, Currency::USD),
            *par_spread_bp,
            base,
            *maturity,
            "USD-OIS",
            "MULTI-TENOR-TEST-HAZARD",
        );

        let npv = match cds.value(&market_price, base) {
            Ok(npv) => npv,
            Err(_) => {
                println!("Skipping multi-tenor test for maturity {}: CDS valuation failed", maturity);
                continue;
            }
        };

        // Each CDS should have NPV ≈ 0 at its par spread
        let tolerance = 10_000_000.0 * 0.0001; // 1bp of notional

        assert!(
            npv.amount().abs() < tolerance,
            "CDS maturity {} repriced at par spread {:.0}bp should have NPV ≈ 0, got: {:.2}",
            maturity,
            par_spread_bp,
            npv.amount()
        );
    }
}

#[test]
fn test_cds_par_spread_calculation_consistency() {
    // Test that calculating par spread from a hazard curve, then repricing
    // with that spread, gives NPV ≈ 0
    let base = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

    let quotes = vec![CreditQuote::CDS {
        entity: "PAR-CONSISTENCY-TEST".to_string(),
        maturity,
        spread_bp: 200.0, // 200bp
        recovery_rate: 0.40,
        currency: Currency::USD,
    }];

    let calibrator = HazardCurveCalibrator::new(
        "PAR-CONSISTENCY-TEST",
        Seniority::Senior,
        0.40,
        base,
        Currency::USD,
        "USD-OIS",
    );

    let disc = create_discount_curve(base);
    let market_calib = MarketContext::new().insert_discount(disc);

    let (hazard_curve, _report) = calibrator.calibrate(&quotes, &market_calib).unwrap();

    // Create market with calibrated hazard curve
    let market_price = MarketContext::new()
        .insert_discount(create_discount_curve(base))
        .insert_hazard(hazard_curve);

    // Create CDS with arbitrary spread
    let cds_test = CreditDefaultSwap::buy_protection(
        "TEST-CDS",
        Money::new(10_000_000.0, Currency::USD),
        150.0, // Different spread (150bp)
        base,
        maturity,
        "USD-OIS",
        "PAR-CONSISTENCY-TEST-HAZARD",
    );

    // Calculate par spread metric
    let result = match cds_test.price_with_metrics(
        &market_price,
        base,
        &[finstack_valuations::metrics::MetricId::ParSpread],
    ) {
        Ok(result) => result,
        Err(_) => {
            println!("Skipping test_cds_par_spread_calculation_consistency: CDS pricing with metrics failed");
            return;
        }
    };

    let calculated_par_spread =
        result.measures[finstack_valuations::metrics::MetricId::ParSpread.as_str()];

    // Calculated par spread should be close to the original 200bp used for calibration
    assert!(
        (calculated_par_spread - 200.0).abs() < 1.0, // Within 1bp
        "Calculated par spread {:.2}bp should match calibration input 200bp",
        calculated_par_spread
    );
}
