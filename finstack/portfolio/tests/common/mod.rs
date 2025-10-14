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
    DiscountCurve::builder("USD")
        .base_date(base_date())
        .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

#[allow(dead_code)]
fn eur_curve() -> DiscountCurve {
    DiscountCurve::builder("EUR")
        .base_date(base_date())
        .knots(vec![(0.0, 1.0), (1.0, 1.0), (5.0, 1.0)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap()
}

#[allow(dead_code)]
pub fn market_with_usd() -> MarketContext {
    MarketContext::new().insert_discount(usd_curve())
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


