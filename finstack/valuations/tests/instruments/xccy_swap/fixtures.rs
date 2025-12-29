use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use std::sync::Arc;
use time::Month;

pub fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

pub const USD_CAL: &str = "usny";
pub const EUR_CAL: &str = "target2";

pub fn market_with_fx() -> MarketContext {
    let base = d(2025, 1, 2);

    let usd_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let eur_disc = DiscountCurve::builder("EUR-OIS")
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, 0.985), (2.0, 0.97)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let usd_fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .knots(vec![(0.0, 0.02), (2.0, 0.02)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let eur_fwd = ForwardCurve::builder("EUR-EURIBOR-3M", 0.25)
        .base_date(base)
        .knots(vec![(0.0, 0.015), (2.0, 0.015)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let provider = Arc::new(SimpleFxProvider::new());
    provider.set_quote(Currency::EUR, Currency::USD, 1.10);
    let fx = FxMatrix::new(provider);

    MarketContext::new()
        .insert_discount(usd_disc)
        .insert_discount(eur_disc)
        .insert_forward(usd_fwd)
        .insert_forward(eur_fwd)
        .insert_fx(fx)
}

pub fn market_without_fx() -> MarketContext {
    let base = d(2025, 1, 2);

    let usd_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let eur_disc = DiscountCurve::builder("EUR-OIS")
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, 0.985), (2.0, 0.97)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let usd_fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base)
        .knots(vec![(0.0, 0.02), (2.0, 0.02)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let eur_fwd = ForwardCurve::builder("EUR-EURIBOR-3M", 0.25)
        .base_date(base)
        .knots(vec![(0.0, 0.015), (2.0, 0.015)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(usd_disc)
        .insert_discount(eur_disc)
        .insert_forward(usd_fwd)
        .insert_forward(eur_fwd)
}

pub fn leg_usd_receive() -> finstack_valuations::instruments::xccy_swap::XccySwapLeg {
    finstack_valuations::instruments::xccy_swap::XccySwapLeg {
        currency: Currency::USD,
        notional: finstack_core::money::Money::new(1_000_000.0, Currency::USD),
        side: finstack_valuations::instruments::xccy_swap::LegSide::Receive,
        forward_curve_id: finstack_core::types::CurveId::new("USD-SOFR-3M"),
        discount_curve_id: finstack_core::types::CurveId::new("USD-OIS"),
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        spread: 0.0,
        payment_lag_days: 0,
        calendar_id: Some(USD_CAL.to_string()),
        allow_calendar_fallback: false,
    }
}

pub fn leg_eur_pay() -> finstack_valuations::instruments::xccy_swap::XccySwapLeg {
    finstack_valuations::instruments::xccy_swap::XccySwapLeg {
        currency: Currency::EUR,
        notional: finstack_core::money::Money::new(900_000.0, Currency::EUR),
        side: finstack_valuations::instruments::xccy_swap::LegSide::Pay,
        forward_curve_id: finstack_core::types::CurveId::new("EUR-EURIBOR-3M"),
        discount_curve_id: finstack_core::types::CurveId::new("EUR-OIS"),
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        spread: 0.0,
        payment_lag_days: 0,
        calendar_id: Some(EUR_CAL.to_string()),
        allow_calendar_fallback: false,
    }
}
