//! Common test utilities and fixtures for portfolio tests.
//!
//! This module provides shared testing infrastructure including helper functions
//! for creating market contexts, discount curves, and FX providers used across
//! multiple test files.

use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
use finstack_core::prelude::*;
use std::sync::Arc;
use time::macros::date;

pub fn base_date() -> Date {
    date!(2024 - 01 - 01)
}

#[allow(dead_code)]
fn usd_curve() -> DiscountCurve {
    // Flat curve for testing - requires allow_non_monotonic()
    DiscountCurve::builder("USD")
        .base_date(base_date())
        .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .allow_non_monotonic()
        .build()
        .unwrap()
}

#[allow(dead_code)]
fn eur_curve() -> DiscountCurve {
    // Flat curve for testing - requires allow_non_monotonic()
    DiscountCurve::builder("EUR")
        .base_date(base_date())
        .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .allow_non_monotonic()
        .build()
        .unwrap()
}

#[allow(dead_code)]
pub fn market_with_usd() -> MarketContext {
    MarketContext::new().insert_discount(usd_curve())
}

/// Create a USD discount curve with a flat rate in basis points.
/// DF(t) = exp(-rate * t) where rate = bp / 10000
#[allow(dead_code)]
pub fn usd_curve_at_rate(rate_bp: f64) -> DiscountCurve {
    let rate = rate_bp / 10000.0;
    let mut builder = DiscountCurve::builder("USD")
        .base_date(base_date())
        .knots(vec![
            (0.0, 1.0),
            (1.0, (-rate * 1.0_f64).exp()),
            (5.0, (-rate * 5.0_f64).exp()),
        ])
        .set_interp(InterpStyle::Linear);

    // For flat or near-zero rates, discount factors may be non-monotonic
    if rate_bp.abs() < 1.0 {
        builder = builder.allow_non_monotonic();
    }

    builder.build().unwrap()
}

/// Create a market context with USD curve at a specific rate level (in basis points).
#[allow(dead_code)]
pub fn market_with_usd_at_rate(rate_bp: f64) -> MarketContext {
    MarketContext::new().insert_discount(usd_curve_at_rate(rate_bp))
}

#[allow(dead_code)]
pub fn market_with_eur() -> MarketContext {
    MarketContext::new().insert_discount(eur_curve())
}

pub struct StaticFx {
    pub rate: f64,
}

impl FxProvider for StaticFx {
    fn rate(
        &self,
        _from: Currency,
        _to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<f64> {
        Ok(self.rate)
    }
}

#[allow(dead_code)]
fn fx_matrix(rate: f64) -> FxMatrix {
    FxMatrix::new(Arc::new(StaticFx { rate }))
}

#[allow(dead_code)]
pub fn market_with_eur_and_fx(rate: f64) -> MarketContext {
    market_with_eur().insert_fx(fx_matrix(rate))
}
