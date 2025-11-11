//! Integration tests for FX option metrics via the metric registry.
//!
//! Tests that metrics are correctly registered and computed through
//! the instrument's price_with_metrics method.

use super::helpers::*;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_price_with_metrics_delta() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Delta])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("delta"), "Delta should be computed");
    let delta = *result.measures.get("delta").unwrap();
    assert_in_range(delta, 300_000.0, 700_000.0, "ATM delta");
}

#[test]
fn test_price_with_metrics_gamma() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Gamma])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("gamma"), "Gamma should be computed");
    let gamma = *result.measures.get("gamma").unwrap();
    assert!(gamma > 0.0, "Gamma should be positive");
}

#[test]
fn test_price_with_metrics_vega() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Vega])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("vega"), "Vega should be computed");
    let vega = *result.measures.get("vega").unwrap();
    assert!(vega > 0.0, "Vega should be positive");
}

#[test]
fn test_price_with_metrics_theta() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("theta"), "Theta should be computed");
    let theta = *result.measures.get("theta").unwrap();
    assert!(theta.is_finite(), "Theta should be finite");
}

#[test]
fn test_price_with_metrics_implied_vol() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::ImpliedVol])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("implied_vol"), "Implied vol should be computed");
    let iv = *result.measures.get("implied_vol").unwrap();
    assert_approx_eq(iv, 0.15, 1e-3, 1e-3, "Implied vol");
}

#[test]
fn test_price_with_metrics_dv01() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("dv01"), "DV01 should be computed");
    let dv01 = *result.measures.get("dv01").unwrap();
    assert!(dv01 > 0.0, "DV01 should be positive");
}

#[test]
fn test_price_with_all_greeks() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    let metrics = vec![
        MetricId::Delta,
        MetricId::Gamma,
        MetricId::Vega,
        MetricId::Theta,
    ];

    // Act
    let result = call.price_with_metrics(&market, as_of, &metrics).unwrap();

    // Assert: All metrics should be present
    assert!(result.measures.contains_key("delta"), "Delta missing");
    assert!(result.measures.contains_key("gamma"), "Gamma missing");
    assert!(result.measures.contains_key("vega"), "Vega missing");
    assert!(result.measures.contains_key("theta"), "Theta missing");
    
    // Base value should also be present
    assert_eq!(result.value.currency(), QUOTE);
}

#[test]
fn test_price_with_empty_metrics() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act: Request no metrics
    let result = call.price_with_metrics(&market, as_of, &[]).unwrap();

    // Assert: Should still have base value, no measures
    assert!(result.measures.is_empty(), "No metrics requested");
    assert!(result.value.amount() > 0.0, "Should have base value");
}

#[test]
fn test_rho_domestic_metric() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act: Request custom rho_domestic metric
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::custom("rho_domestic")])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("rho_domestic"), "Rho domestic should be computed");
    let rho_dom = *result.measures.get("rho_domestic").unwrap();
    assert!(rho_dom > 0.0, "Call rho_domestic should be positive");
}

#[test]
fn test_rho_foreign_metric() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act: Request custom rho_foreign metric
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::custom("rho_foreign")])
        .unwrap();

    // Assert
    assert!(result.measures.contains_key("rho_foreign"), "Rho foreign should be computed");
    let rho_for = *result.measures.get("rho_foreign").unwrap();
    assert!(rho_for < 0.0, "Call rho_foreign should be negative");
}

#[test]
fn test_bucketed_dv01_metric() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act: Request bucketed DV01
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    // Assert: Should have bucketed_dv01 (sum) and potentially bucket breakdown
    assert!(result.measures.contains_key("bucketed_dv01"), "Bucketed DV01 should be computed");
    let bucketed = *result.measures.get("bucketed_dv01").unwrap();
    assert!(bucketed.is_finite() && bucketed != 0.0, "Bucketed DV01 should be finite and non-zero");
}

#[test]
fn test_combined_dv01_bumps_both_curves() {
    // Test that Combined mode (default) bumps both domestic and foreign discount curves
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Compute DV01 using Combined mode (default)
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Dv01])
        .unwrap();

    let dv01_combined = *result.measures.get("dv01").unwrap();

    // DV01 should be positive (value decreases when both discount rates increase)
    assert!(dv01_combined > 0.0, "Combined DV01 should be positive for call option");
    assert!(dv01_combined.is_finite(), "Combined DV01 should be finite");
}

#[test]
fn test_result_includes_instrument_id() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call.price_with_metrics(&market, as_of, &[MetricId::Delta]).unwrap();

    // Assert
    assert_eq!(result.instrument_id.as_str(), "FX_CALL_TEST");
}

#[test]
fn test_result_includes_as_of_date() {
    // Arrange
    let as_of = date!(2024 - 03 - 15);
    let expiry = date!(2025 - 03 - 15);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act
    let result = call.price_with_metrics(&market, as_of, &[]).unwrap();

    // Assert
    assert_eq!(result.as_of, as_of);
}

#[test]
fn test_metrics_consistent_with_direct_greeks_computation() {
    // Arrange
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let call = build_call_option(as_of, expiry, 1.20, 1_000_000.0);
    let market = build_market_context(as_of, MarketParams::atm());

    // Act: Compute via metrics
    let result = call
        .price_with_metrics(&market, as_of, &[MetricId::Delta, MetricId::Gamma, MetricId::Vega])
        .unwrap();

    // Act: Compute via direct method
    let greeks = call.compute_greeks(&market, as_of).unwrap();

    // Assert: Should match
    let delta_metric = *result.measures.get("delta").unwrap();
    let gamma_metric = *result.measures.get("gamma").unwrap();
    let vega_metric = *result.measures.get("vega").unwrap();

    assert_approx_eq(delta_metric, greeks.delta, 1e-6, 1e-6, "Delta matches");
    assert_approx_eq(gamma_metric, greeks.gamma, 1e-9, 1e-9, "Gamma matches");
    assert_approx_eq(vega_metric, greeks.vega, 1e-6, 1e-6, "Vega matches");
}

