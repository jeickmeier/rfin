//! Tests for implied volatility calculation.

use super::helpers::*;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_implied_vol_recovers_surface_vol() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.30;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.02);

    // Get market price
    let market_price = call.value(&market, as_of).unwrap().amount();

    // Set market price in attributes
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    // Calculate implied vol
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged (implied_vol > 0 indicates success)
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(
        implied_vol,
        vol,
        1e-4,
        "Implied vol should recover surface vol",
    );
}

#[test]
fn test_implied_vol_atm_option() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.25;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "ATM implied vol");
}

#[test]
fn test_implied_vol_itm_option() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 90.0;
    let spot = 100.0;
    let vol = 0.28;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "ITM implied vol");
}

#[test]
fn test_implied_vol_otm_option() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 110.0;
    let spot = 100.0;
    let vol = 0.32;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "OTM implied vol");
}

#[test]
fn test_implied_vol_short_dated() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2024 - 02 - 01); // 1 month
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.40;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "Short dated implied vol");
}

#[test]
fn test_implied_vol_long_dated() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2026 - 01 - 01); // 2 years
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.22;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "Long dated implied vol");
}

#[test]
fn test_implied_vol_high_volatility() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.80; // High vol

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    // Solver uses 1e-8 tolerance internally; 1e-4 is achievable for high vol
    assert_approx_eq_tol(implied_vol, vol, 1e-4, "High vol implied vol");
}

#[test]
fn test_implied_vol_low_volatility() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.10; // Low vol

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "Low vol implied vol");
}

#[test]
fn test_implied_vol_returns_zero_for_expired() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = as_of; // Already expired
    let strike = 100.0;
    let spot = 110.0;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, 0.25, 0.05, 0.0);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Expired option should return 0 implied vol
    assert_approx_eq_tol(implied_vol, 0.0, TIGHT_TOL, "Expired implied vol");
}

#[test]
fn test_implied_vol_with_dividends() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let strike = 100.0;
    let spot = 100.0;
    let vol = 0.25;
    let div_yield = 0.03;

    let mut call = create_call(as_of, expiry, strike);
    let market = build_standard_market(as_of, spot, vol, 0.05, div_yield);

    let market_price = call.value(&market, as_of).unwrap().amount();
    call.attributes
        .meta
        .insert("market_price".to_string(), market_price.to_string());

    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    let implied_vol = *result.measures.get("implied_vol").unwrap();

    // Assert solver converged
    assert!(
        implied_vol > 0.0,
        "Implied vol solver failed to converge (returned {})",
        implied_vol
    );
    assert_approx_eq_tol(implied_vol, vol, 1e-5, "Implied vol with dividends");
}
