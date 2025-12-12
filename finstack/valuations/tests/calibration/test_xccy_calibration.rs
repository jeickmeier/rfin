use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::types::CurveId;
use finstack_valuations::calibration::create_simple_solver;
use finstack_valuations::calibration::methods::{
    XccyBasisCalibrator, XccyBasisQuote, XccySpreadOn,
};
use finstack_valuations::instruments::xccy_swap::{
    LegSide, NotionalExchange, XccySwap, XccySwapLeg,
};
use std::sync::Arc;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn market() -> MarketContext {
    let base = d(2025, 1, 2);

    let usd_disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
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
        .knots(vec![(0.0, 0.01), (2.0, 0.01)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let provider = Arc::new(SimpleFxProvider::new());
    provider.set_quote(Currency::EUR, Currency::USD, 1.10);
    let fx = FxMatrix::new(provider);

    MarketContext::new()
        .insert_discount(usd_disc)
        .insert_forward(usd_fwd)
        .insert_forward(eur_fwd)
        .insert_fx(fx)
}

#[test]
fn xccy_bootstrap_reprices_quote() {
    let base = d(2025, 1, 2);
    // T+2 spot (joint USNY/TARGET2) from 2025-01-02 is 2025-01-06.
    let start = d(2025, 1, 6);
    let maturity = d(2026, 1, 2);

    let calibrator = XccyBasisCalibrator::new("EUR-XCCY-DISC", base, Currency::EUR);
    let solver = create_simple_solver(&calibrator.config);
    let ctx = market();

    let quote = XccyBasisQuote {
        maturity,
        spread_bp: 0.0,
        domestic_currency: Currency::USD,
        foreign_currency: Currency::EUR,
        domestic_notional: 1_100_000.0,
        foreign_notional: 1_000_000.0,
        domestic_discount_curve_id: CurveId::new("USD-OIS"),
        domestic_forward_curve_id: CurveId::new("USD-SOFR-3M"),
        foreign_forward_curve_id: CurveId::new("EUR-EURIBOR-3M"),
        domestic_freq: Frequency::quarterly(),
        foreign_freq: Frequency::quarterly(),
        domestic_dc: DayCount::Act360,
        foreign_dc: DayCount::Act360,
        domestic_bdc: BusinessDayConvention::ModifiedFollowing,
        foreign_bdc: BusinessDayConvention::ModifiedFollowing,
        domestic_payment_lag_days: 0,
        foreign_payment_lag_days: 0,
        spot_lag_days: 2,
        spot_bdc: BusinessDayConvention::Following,
        domestic_calendar_id: "usny".to_string(),
        foreign_calendar_id: "target2".to_string(),
        spread_on: XccySpreadOn::Foreign,
        notional_exchange: NotionalExchange::InitialAndFinal,
    };

    let (eur_disc, report) = calibrator.bootstrap(&[quote], &solver, &ctx).unwrap();
    assert!(!report.residuals.is_empty());

    let ctx = ctx.insert_discount(eur_disc);

    let swap = XccySwap::new(
        "XCCY-REPRICE",
        start,
        maturity,
        XccySwapLeg {
            currency: Currency::USD,
            notional: finstack_core::money::Money::new(1_100_000.0, Currency::USD),
            side: LegSide::Receive,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            discount_curve_id: CurveId::new("USD-OIS"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            calendar_id: Some("usny".to_string()),
            allow_calendar_fallback: false,
        },
        XccySwapLeg {
            currency: Currency::EUR,
            notional: finstack_core::money::Money::new(1_000_000.0, Currency::EUR),
            side: LegSide::Pay,
            forward_curve_id: CurveId::new("EUR-EURIBOR-3M"),
            discount_curve_id: CurveId::new("EUR-XCCY-DISC"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            calendar_id: Some("target2".to_string()),
            allow_calendar_fallback: false,
        },
        Currency::USD,
    )
    .with_notional_exchange(NotionalExchange::InitialAndFinal);

    let pv = swap.npv(&ctx, base).unwrap().amount();
    assert!(pv.is_finite());
    assert!(pv.abs() < 1e-6, "reprice pv={}", pv);
}
