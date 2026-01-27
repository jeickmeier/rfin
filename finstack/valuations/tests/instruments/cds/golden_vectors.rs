//! Golden vector validation tests for CDS pricing.
//!
//! These tests validate our CDS implementation against ISDA CDS Standard Model
//! reference values stored in JSON fixture files.
//!
//! Golden vectors provide:
//! - Externally validated expected values (not regression values from our code)
//! - Tight tolerances (0.5bp for par spread, 0.25% for PVs)
//! - Clear documentation of assumptions and conventions
//!
//! To regenerate golden vectors, run: `uv run scripts/generate_cds_golden_vectors.py`
//!
//! # Migration Note
//!
//! This module now uses `finstack_core::golden::ExpectedValue` for tolerance handling,
//! which provides consistent comparison semantics across all golden tests.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::golden::ExpectedValue;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::credit_derivatives::cds::CDSPricer;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use time::Month;

/// Discount curve specification in golden vector
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DiscountSpec {
    Flat { flat_rate: f64 },
    Knots { knots: Vec<DiscountKnot> },
}

#[derive(Debug, Deserialize)]
struct DiscountKnot {
    time: f64,
    df: f64,
}

/// Hazard curve specification in golden vector
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HazardSpec {
    Flat { flat_rate: f64 },
    Knots { knots: Vec<HazardKnot> },
}

#[derive(Debug, Deserialize)]
struct HazardKnot {
    time: f64,
    hazard: f64,
}

/// Curves specification
#[derive(Debug, Deserialize)]
struct CurvesSpec {
    discount: DiscountSpec,
    hazard: HazardSpec,
}

/// Contract specification in golden vector
#[derive(Debug, Deserialize)]
struct ContractSpec {
    as_of: String,
    #[allow(dead_code)]
    start_date: String,
    maturity_date: String,
    notional: f64,
    #[allow(dead_code)]
    currency: String,
    spread_bp: f64,
    recovery_rate: f64,
    #[allow(dead_code)]
    convention: Option<String>,
    #[allow(dead_code)]
    side: Option<String>,
}

// Note: Using finstack_core::golden::ExpectedValue for consistency

/// Expected outputs
#[derive(Debug, Deserialize)]
struct ExpectedSpec {
    par_spread_bp: Option<ExpectedValue>,
    risky_pv01: Option<ExpectedValue>,
    protection_leg_pv: Option<ExpectedValue>,
    #[allow(dead_code)]
    premium_leg_pv: Option<ExpectedValue>,
    npv: Option<ExpectedValue>,
}

/// Golden vector test case
#[derive(Debug, Deserialize)]
struct GoldenVector {
    id: String,
    #[allow(dead_code)]
    source: String,
    #[allow(dead_code)]
    description: Option<String>,
    contract: ContractSpec,
    curves: CurvesSpec,
    expected: ExpectedSpec,
}

fn parse_date(s: &str) -> Date {
    let parts: Vec<u16> = s.split('-').map(|p| p.parse().unwrap()).collect();
    Date::from_calendar_date(
        parts[0] as i32,
        Month::try_from(parts[1] as u8).unwrap(),
        parts[2] as u8,
    )
    .unwrap()
}

fn build_discount_curve(spec: &DiscountSpec, base_date: Date) -> DiscountCurve {
    match spec {
        DiscountSpec::Flat { flat_rate } => {
            let knots: Vec<(f64, f64)> = [0.0, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]
                .iter()
                .map(|&t| (t, (-flat_rate * t).exp()))
                .collect();

            DiscountCurve::builder("USD_DISC")
                .base_date(base_date)
                .day_count(DayCount::Act360)
                .knots(knots)
                .build()
                .unwrap()
        }
        DiscountSpec::Knots { knots } => {
            let knot_vec: Vec<(f64, f64)> = knots.iter().map(|k| (k.time, k.df)).collect();

            DiscountCurve::builder("USD_DISC")
                .base_date(base_date)
                .day_count(DayCount::Act360)
                .knots(knot_vec)
                .build()
                .unwrap()
        }
    }
}

fn build_hazard_curve(spec: &HazardSpec, recovery: f64, base_date: Date) -> HazardCurve {
    match spec {
        HazardSpec::Flat { flat_rate } => {
            let knots: Vec<(f64, f64)> = [0.0, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]
                .iter()
                .map(|&t| (t, *flat_rate))
                .collect();

            HazardCurve::builder("CREDIT")
                .base_date(base_date)
                .recovery_rate(recovery)
                .knots(knots)
                .build()
                .unwrap()
        }
        HazardSpec::Knots { knots } => {
            let knot_vec: Vec<(f64, f64)> = knots.iter().map(|k| (k.time, k.hazard)).collect();

            HazardCurve::builder("CREDIT")
                .base_date(base_date)
                .recovery_rate(recovery)
                .knots(knot_vec)
                .build()
                .unwrap()
        }
    }
}

fn load_golden_vectors() -> Vec<GoldenVector> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let golden_dir = PathBuf::from(manifest_dir)
        .join("tests")
        .join("instruments")
        .join("cds")
        .join("golden");

    let mut vectors = Vec::new();

    for entry in fs::read_dir(&golden_dir).expect("golden directory should exist") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json")
            && !path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("schema")
        {
            let content = fs::read_to_string(&path).expect("should read fixture file");
            let vector: GoldenVector =
                serde_json::from_str(&content).expect("should parse golden vector");
            vectors.push(vector);
        }
    }

    vectors
}

/// Assert using the core golden framework's ExpectedValue type.
///
/// This wraps `finstack_core::golden::assert_expected_value` with a panic on failure
/// for use in tests.
fn assert_within_tolerance(actual: f64, expected: &ExpectedValue, metric: &str, case_id: &str) {
    use finstack_core::golden::assert_expected_value;
    if let Err(e) = assert_expected_value("cds_golden", case_id, metric, actual, expected) {
        panic!("{}", e);
    }
}

#[test]
fn test_cds_golden_vectors_par_spread() {
    let vectors = load_golden_vectors();
    assert!(
        !vectors.is_empty(),
        "Should have at least one golden vector"
    );

    for vector in vectors {
        let as_of = parse_date(&vector.contract.as_of);
        let maturity = parse_date(&vector.contract.maturity_date);

        let disc = build_discount_curve(&vector.curves.discount, as_of);
        let hazard =
            build_hazard_curve(&vector.curves.hazard, vector.contract.recovery_rate, as_of);

        let market = MarketContext::new()
            .insert_discount(disc)
            .insert_hazard(hazard);

        let mut cds = finstack_valuations::test_utils::cds_buy_protection(
            vector.id.clone(),
            Money::new(vector.contract.notional, Currency::USD),
            vector.contract.spread_bp,
            as_of,
            maturity,
            "USD_DISC",
            "CREDIT",
        )
        .expect("CDS construction should succeed");
        cds.protection.recovery_rate = vector.contract.recovery_rate;

        // Test par spread
        if let Some(expected_par_spread) = &vector.expected.par_spread_bp {
            let result = cds
                .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
                .unwrap();
            let par_spread = *result.measures.get("par_spread").unwrap();
            assert_within_tolerance(par_spread, expected_par_spread, "par_spread_bp", &vector.id);
        }
    }
}

#[test]
fn test_cds_golden_vectors_pv() {
    let vectors = load_golden_vectors();

    for vector in vectors {
        let as_of = parse_date(&vector.contract.as_of);
        let maturity = parse_date(&vector.contract.maturity_date);

        let disc = build_discount_curve(&vector.curves.discount, as_of);
        let hazard =
            build_hazard_curve(&vector.curves.hazard, vector.contract.recovery_rate, as_of);

        let market = MarketContext::new()
            .insert_discount(disc.clone())
            .insert_hazard(hazard.clone());

        // For NPV test, set spread to par to test zero NPV
        let mut cds = finstack_valuations::test_utils::cds_buy_protection(
            vector.id.clone(),
            Money::new(vector.contract.notional, Currency::USD),
            vector.contract.spread_bp,
            as_of,
            maturity,
            "USD_DISC",
            "CREDIT",
        )
        .expect("CDS construction should succeed");
        cds.protection.recovery_rate = vector.contract.recovery_rate;

        // Test protection leg PV
        if let Some(expected) = &vector.expected.protection_leg_pv {
            let pricer = CDSPricer::new();
            let pv_prot = pricer
                .pv_protection_leg(&cds, &disc, &hazard, as_of)
                .unwrap();
            assert_within_tolerance(pv_prot.amount(), expected, "protection_leg_pv", &vector.id);
        }

        // Test NPV at stated spread
        if let Some(expected_npv) = &vector.expected.npv {
            let npv = cds.value(&market, as_of).unwrap();
            assert_within_tolerance(npv.amount(), expected_npv, "npv", &vector.id);
        }
    }
}

#[test]
fn test_cds_golden_vectors_risky_pv01() {
    let vectors = load_golden_vectors();

    for vector in vectors {
        let as_of = parse_date(&vector.contract.as_of);
        let maturity = parse_date(&vector.contract.maturity_date);

        let disc = build_discount_curve(&vector.curves.discount, as_of);
        let hazard =
            build_hazard_curve(&vector.curves.hazard, vector.contract.recovery_rate, as_of);

        let mut cds = finstack_valuations::test_utils::cds_buy_protection(
            vector.id.clone(),
            Money::new(vector.contract.notional, Currency::USD),
            vector.contract.spread_bp,
            as_of,
            maturity,
            "USD_DISC",
            "CREDIT",
        )
        .expect("CDS construction should succeed");
        cds.protection.recovery_rate = vector.contract.recovery_rate;

        // Test risky PV01
        if let Some(expected) = &vector.expected.risky_pv01 {
            let pricer = CDSPricer::new();
            let risky_pv01 = pricer.risky_pv01(&cds, &disc, &hazard, as_of).unwrap();
            // risky_pv01 is typically in currency units per bp
            assert_within_tolerance(risky_pv01, expected, "risky_pv01", &vector.id);
        }
    }
}

#[test]
fn test_cds_par_spread_npv_zero_invariant() {
    // When CDS spread equals par spread, NPV should be approximately zero
    let vectors = load_golden_vectors();

    for vector in vectors {
        let as_of = parse_date(&vector.contract.as_of);
        let maturity = parse_date(&vector.contract.maturity_date);

        let disc = build_discount_curve(&vector.curves.discount, as_of);
        let hazard =
            build_hazard_curve(&vector.curves.hazard, vector.contract.recovery_rate, as_of);

        let market = MarketContext::new()
            .insert_discount(disc.clone())
            .insert_hazard(hazard.clone());

        let mut cds = finstack_valuations::test_utils::cds_buy_protection(
            format!("{}_par_npv", vector.id),
            Money::new(vector.contract.notional, Currency::USD),
            100.0, // placeholder
            as_of,
            maturity,
            "USD_DISC",
            "CREDIT",
        )
        .expect("CDS construction should succeed");
        cds.protection.recovery_rate = vector.contract.recovery_rate;

        // Calculate par spread
        let par_spread = cds.par_spread(&disc, &hazard, as_of).unwrap();

        // Set CDS to trade at par
        cds.premium.spread_bp = Decimal::try_from(par_spread).expect("valid par_spread");

        // NPV should be near zero
        let npv = cds.value(&market, as_of).unwrap();

        // Tolerance: $1000 on $10M notional = 1bp (accounts for discrete vs continuous integration)
        // Note: There can be small discrepancies between par spread calculation and NPV
        // calculation due to different integration methods and accrual handling.
        assert!(
            npv.amount().abs() < 1000.0,
            "[{}] NPV at par spread should be ~0, got ${:.2} (par_spread={:.4} bp)",
            vector.id,
            npv.amount(),
            par_spread
        );
    }
}
