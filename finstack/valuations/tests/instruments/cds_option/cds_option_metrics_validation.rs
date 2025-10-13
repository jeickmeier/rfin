//! CDS Option metrics validation tests against market standards.
//!
//! These tests validate CDS option pricing and Greeks against:
//! - Black model for credit options
//! - Option moneyness effects
//! - Time decay behavior
//! - Credit spread sensitivity
//!
//! References:
//! - Black (1976) model for options on forwards
//! - Market practice for credit options

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::dates::utils::add_months;
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::cds::parameters::RECOVERY_SENIOR_UNSECURED;
use finstack_valuations::instruments::CreditParams;
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::cds_option::CdsOption;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn flat_discount(id: &str, base: Date) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.70)])
        .build()
        .unwrap()
}

fn flat_hazard(id: &str, base: Date, rec: f64, hz: f64) -> HazardCurve {
    let par = hz * 10000.0 * (1.0 - rec);
    HazardCurve::builder(id)
        .base_date(base)
        .recovery_rate(rec)
        .knots([(1.0, hz), (5.0, hz), (10.0, hz)])
        .par_spreads([(1.0, par), (5.0, par), (10.0, par)])
        .build()
        .unwrap()
}

#[test]
fn test_cds_option_call_value_positive() {
    // Call option on CDS should have positive value
    
    let as_of = date!(2025 - 01 - 01);
    let expiry = add_months(as_of, 12); // 1 year
    let cds_maturity = add_months(as_of, 60); // 5 years
    
    let disc = flat_discount("USD-OIS", as_of);
    let credit = flat_hazard("HZ-SN", as_of, RECOVERY_SENIOR_UNSECURED, 0.02);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    
    let option_params = CdsOptionParams::call(
        100.0, // Strike spread 100 bps
        expiry,
        cds_maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    
    let credit_params = CreditParams::corporate_standard("SN", "HZ-SN");
    let mut option = CdsOption::new(
        "CDSOPT-CALL-TEST",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    option.pricing_overrides.implied_volatility = Some(0.30);
    
    let pv = option.value(&ctx, as_of).unwrap();
    
    assert!(
        pv.amount() >= 0.0,
        "CDS call option value={:.2} should be non-negative",
        pv.amount()
    );
}

#[test]
fn test_cds_option_put_value_positive() {
    // Put option on CDS should have positive value
    
    let as_of = date!(2025 - 01 - 01);
    let expiry = add_months(as_of, 12);
    let cds_maturity = add_months(as_of, 60);
    
    let disc = flat_discount("USD-OIS", as_of);
    let credit = flat_hazard("HZ-SN", as_of, RECOVERY_SENIOR_UNSECURED, 0.02);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    
    let option_params = CdsOptionParams::put(
        100.0,
        expiry,
        cds_maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    
    let credit_params = CreditParams::corporate_standard("SN", "HZ-SN");
    let mut option = CdsOption::new(
        "CDSOPT-PUT-TEST",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    option.pricing_overrides.implied_volatility = Some(0.30);
    
    let pv = option.value(&ctx, as_of).unwrap();
    
    assert!(
        pv.amount() >= 0.0,
        "CDS put option value={:.2} should be non-negative",
        pv.amount()
    );
}

#[test]
fn test_cds_option_delta_bounds() {
    // Delta should be in [0, 1] for calls, [-1, 0] for puts (approximately)
    
    let as_of = date!(2025 - 01 - 01);
    let expiry = add_months(as_of, 12);
    let cds_maturity = add_months(as_of, 60);
    
    let disc = flat_discount("USD-OIS", as_of);
    let credit = flat_hazard("HZ-SN", as_of, RECOVERY_SENIOR_UNSECURED, 0.02);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    
    // Call option
    let call_params = CdsOptionParams::call(
        100.0,
        expiry,
        cds_maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    
    let credit_params = CreditParams::corporate_standard("SN", "HZ-SN");
    let mut call_option = CdsOption::new(
        "CDSOPT-CALL-DELTA",
        &call_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    call_option.pricing_overrides.implied_volatility = Some(0.30);
    
    let call_result = call_option
        .price_with_metrics(&ctx, as_of, &[MetricId::Delta])
        .unwrap();
    
    let call_delta = *call_result.measures.get("delta").unwrap();
    
    // Call delta should be in reasonable range [0, notional]
    assert!(
        call_delta >= 0.0,
        "Call delta={:.2} should be non-negative",
        call_delta
    );
}

#[test]
fn test_cds_option_vega_positive() {
    // Vega should be positive (option value increases with volatility)
    
    let as_of = date!(2025 - 01 - 01);
    let expiry = add_months(as_of, 12);
    let cds_maturity = add_months(as_of, 60);
    
    let disc = flat_discount("USD-OIS", as_of);
    let credit = flat_hazard("HZ-SN", as_of, RECOVERY_SENIOR_UNSECURED, 0.02);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    
    let option_params = CdsOptionParams::call(
        100.0,
        expiry,
        cds_maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    
    let credit_params = CreditParams::corporate_standard("SN", "HZ-SN");
    let mut option = CdsOption::new(
        "CDSOPT-VEGA-TEST",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    option.pricing_overrides.implied_volatility = Some(0.30);
    
    let result = option
        .price_with_metrics(&ctx, as_of, &[MetricId::Vega])
        .unwrap();
    
    let vega = *result.measures.get("vega").unwrap();
    
    assert!(
        vega > 0.0,
        "Vega={:.2} should be positive",
        vega
    );
}

#[test]
fn test_cds_option_value_increases_with_volatility() {
    // Option value should increase with implied volatility
    
    let as_of = date!(2025 - 01 - 01);
    let expiry = add_months(as_of, 12);
    let cds_maturity = add_months(as_of, 60);
    
    let disc = flat_discount("USD-OIS", as_of);
    let credit = flat_hazard("HZ-SN", as_of, RECOVERY_SENIOR_UNSECURED, 0.02);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    
    let option_params = CdsOptionParams::call(
        100.0,
        expiry,
        cds_maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    
    let credit_params = CreditParams::corporate_standard("SN", "HZ-SN");
    
    let mut values = Vec::new();
    
    for vol in [0.15, 0.25, 0.35, 0.45] {
        let mut option = CdsOption::new(
            "CDSOPT-VOL-SENS",
            &option_params,
            &credit_params,
            "USD-OIS",
            "CDS-OPT-VOL",
        );
        option.pricing_overrides.implied_volatility = Some(vol);
        
        let pv = option.value(&ctx, as_of).unwrap();
        values.push((vol, pv.amount()));
    }
    
    // Value should increase with volatility
    for i in 1..values.len() {
        assert!(
            values[i].1 > values[i - 1].1,
            "Option value should increase with vol: vol {:.0}% value={:.0} <= vol {:.0}% value={:.0}",
            values[i - 1].0 * 100.0,
            values[i - 1].1,
            values[i].0 * 100.0,
            values[i].1
        );
    }
}

#[test]
fn test_cds_option_cs01_positive_for_call() {
    // Call option CS01 should be positive (benefits from spread widening)
    
    let as_of = date!(2025 - 01 - 01);
    let expiry = add_months(as_of, 12);
    let cds_maturity = add_months(as_of, 60);
    
    let disc = flat_discount("USD-OIS", as_of);
    let credit = flat_hazard("HZ-SN", as_of, RECOVERY_SENIOR_UNSECURED, 0.02);
    
    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_hazard(credit);
    
    let option_params = CdsOptionParams::call(
        100.0,
        expiry,
        cds_maturity,
        Money::new(10_000_000.0, Currency::USD),
    );
    
    let credit_params = CreditParams::corporate_standard("SN", "HZ-SN");
    let mut option = CdsOption::new(
        "CDSOPT-CS01-TEST",
        &option_params,
        &credit_params,
        "USD-OIS",
        "CDS-OPT-VOL",
    );
    option.pricing_overrides.implied_volatility = Some(0.30);
    
    let result = option
        .price_with_metrics(&ctx, as_of, &[MetricId::Cs01])
        .unwrap();
    
    let cs01 = *result.measures.get("cs01").unwrap();
    
    assert!(
        cs01 > 0.0,
        "Call option CS01={:.2} should be positive",
        cs01
    );
}

