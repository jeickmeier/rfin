#![cfg(test)]

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::interp::InterpStyle;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider, FxRate};
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::fx_swap::FxSwap;
use finstack_valuations::instruments::traits::Priceable;
use std::collections::HashMap;
use std::sync::Arc;
use time::Month;

struct MockFxProvider {
    rates: HashMap<(Currency, Currency), f64>,
}

impl FxProvider for MockFxProvider {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<FxRate> {
        if let Some(&rate) = self.rates.get(&(from, to)) {
            #[cfg(feature = "decimal128")]
            return rust_decimal::Decimal::try_from(rate)
                .map_err(|_| finstack_core::Error::Internal);
            #[cfg(not(feature = "decimal128"))]
            return Ok(rate);
        }
        Err(finstack_core::Error::Internal)
    }
}

fn setup_market_data(as_of: Date) -> MarketContext {
    let usd_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.9)]) // ~1% flat rate
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let eur_curve = DiscountCurve::builder("EUR-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.95)]) // ~0.5% flat rate
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let mut rates = HashMap::new();
    rates.insert((Currency::EUR, Currency::USD), 1.1);
    let provider = MockFxProvider { rates };
    let fx_matrix = FxMatrix::new(Arc::new(provider));

    MarketContext::new()
        .with_discount(usd_curve)
        .with_discount(eur_curve)
        .with_fx(fx_matrix)
}

#[test]
fn test_fx_swap_pv() {
    let as_of = Date::from_calendar_date(2024, Month::January, 1).unwrap();
    let near_date = Date::from_calendar_date(2024, Month::January, 3).unwrap();
    let far_date = Date::from_calendar_date(2025, Month::January, 3).unwrap();

    let fx_swap = FxSwap::builder()
        .id("test_swap")
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(near_date)
        .far_date(far_date)
        .base_notional(Money::new(1_000_000.0, Currency::EUR))
        .domestic_disc_id("USD-OIS")
        .foreign_disc_id("EUR-OIS")
        .build()
        .unwrap();

    let market_context = setup_market_data(as_of);

    let pv = fx_swap.value(&market_context, as_of).unwrap();

    // The PV of a fair swap at inception should be close to zero.
    // Due to bid-ask, conventions, etc., it won't be exactly zero.
    // Here we just check it's a reasonable number.
    // A more precise test would calculate the expected PV manually.
    assert!(pv.amount().abs() < 5000.0); // Rough check
}

#[test]
fn test_fx_swap_metrics() {
    let as_of = Date::from_calendar_date(2024, Month::January, 1).unwrap();
    let near_date = Date::from_calendar_date(2024, Month::January, 3).unwrap();
    let far_date = Date::from_calendar_date(2025, Month::January, 3).unwrap();

    let fx_swap = FxSwap::builder()
        .id("test_swap_metrics")
        .base_currency(Currency::EUR)
        .quote_currency(Currency::USD)
        .near_date(near_date)
        .far_date(far_date)
        .base_notional(Money::new(1_000_000.0, Currency::EUR))
        .domestic_disc_id("USD-OIS")
        .foreign_disc_id("EUR-OIS")
        .build()
        .unwrap();

    let market_context = setup_market_data(as_of);
    let result = fx_swap
        .price_with_metrics(
            &market_context,
            as_of,
            &[
                finstack_valuations::metrics::MetricId::custom("forward_points"),
                finstack_valuations::metrics::MetricId::custom("ir01_domestic"),
                finstack_valuations::metrics::MetricId::custom("ir01_foreign"),
                finstack_valuations::metrics::MetricId::custom("fx01"),
            ],
        )
        .unwrap();

    // Check forward points
    let forward_points = *result.measures.get("forward_points").unwrap();
    // far_rate is roughly spot * (1 - 0.005) / (1 - 0.01) = 1.1 * 0.995 / 0.99 ~= 1.1055
    // near_rate is 1.1. Points should be ~0.0055
    assert!(forward_points > 0.005 && forward_points < 0.006);

    // Check IR01s
    let ir01_domestic = *result.measures.get("ir01_domestic").unwrap();
    let ir01_foreign = *result.measures.get("ir01_foreign").unwrap();
    // An increase in domestic rates decreases domestic discount factors, which increases the forward,
    // increasing the value of the far leg domestic cashflow (+). So, domestic IR01 should be positive.
    assert!(ir01_domestic > 0.0);
    // An increase in foreign rates decreases foreign discount factors, which decreases the forward,
    // and also decreases the value of the foreign leg. So, foreign IR01 should be negative.
    assert!(ir01_foreign < 0.0);

    // Check FX01
    let fx01 = *result.measures.get("fx01").unwrap();

    // For an FX swap, when we bump the spot rate:
    // - The near leg domestic payment increases (we pay more domestic currency)
    // - The far leg domestic receipt increases (we receive more domestic currency)
    // - The foreign leg value in domestic terms increases
    // The net effect depends on the specific terms and discount factors
    // For our test case with a 30-day swap, FX01 should be small but non-zero
    assert!(fx01.abs() > 1e-10, "FX01 should be non-zero");

    // The sign of FX01 depends on the swap specifics. For this test case,
    // it's negative (PV decreases when spot increases)
    assert!(
        fx01 < 0.0,
        "FX01 should be negative for this swap configuration"
    );
}
