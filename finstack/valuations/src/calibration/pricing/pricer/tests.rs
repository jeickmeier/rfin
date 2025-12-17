use super::*;
use crate::calibration::config::RateBounds;
use crate::calibration::quotes::{FutureSpecs, InstrumentConventions, RatesQuote};
use crate::instruments::common::traits::Instrument;
use finstack_core::dates::{BusinessDayConvention, CalendarRegistry, Date, DateExt, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::money::Money;
use finstack_core::types::Currency;
use time::Month;

use super::super::convention_resolution as conv;

#[test]
fn fra_fixing_date_respects_signed_reset_lag_negative() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let start = Date::from_calendar_date(2024, Month::January, 9).expect("valid start date");
    let calendar = CalendarRegistry::global().resolve_str("usny").expect("usny calendar");
    let expected = start
        .add_business_days(-2, calendar)
        .expect("business day math");

    let (fixing, found) = pricer
        .compute_fra_fixing_date(start, -2, "usny", false)
        .expect("fixing date");
    assert!(found);
    assert_eq!(fixing, expected);
}

#[test]
fn fra_fixing_date_respects_signed_reset_lag_positive() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let start = Date::from_calendar_date(2024, Month::January, 9).expect("valid start date");
    let calendar = CalendarRegistry::global().resolve_str("usny").expect("usny calendar");
    let expected = start
        .add_business_days(2, calendar)
        .expect("business day math");

    let (fixing, found) = pricer
        .compute_fra_fixing_date(start, 2, "usny", false)
        .expect("fixing date");
    assert!(found);
    assert_eq!(fixing, expected);
}

#[test]
fn fra_fixing_date_falls_back_to_calendar_days_when_allowed() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS").with_allow_calendar_fallback(true);
    let start = Date::from_calendar_date(2024, Month::January, 9).expect("valid start date");
    let expected = start + time::Duration::days(-2);

    let (fixing, found) = pricer
        .compute_fra_fixing_date(start, -2, "missing-calendar", true)
        .expect("fixing");
    assert!(!found);
    assert_eq!(fixing, expected);
}

#[test]
fn create_ois_swap_matches_price_swap_pricing() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .expect("discount curve");

    let ctx = MarketContext::new().insert_discount(disc);

    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let maturity = base_date + time::Duration::days(365);
    let quote = RatesQuote::Swap {
        maturity,
        rate: 0.02,
        is_ois: true,
        conventions: Default::default(),
        fixed_leg_conventions: InstrumentConventions::default(),
        float_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR"),
    };

    let pv_norm = pricer
        .price_instrument(&quote, Currency::USD, &ctx)
        .expect("price swap");

    let swap = pricer
        .create_ois_swap(
            &quote,
            Money::new(1_000_000.0, Currency::USD),
            Currency::USD,
        )
        .expect("create ois swap");
    let pv_swap = swap.value(&ctx, base_date).expect("value swap").amount() / swap.notional.amount();

    assert!(
        (pv_norm - pv_swap).abs() < 1e-12,
        "OIS construction mismatch: pricer={} vs builder={}",
        pv_norm,
        pv_swap
    );
}

#[test]
fn validate_quotes_allows_distinct_fras_with_same_end() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let fra1 = RatesQuote::FRA {
        start: base_date + time::Duration::days(30),
        end: base_date + time::Duration::days(60),
        rate: 0.05,
        conventions: Default::default(),
    };
    let fra2 = RatesQuote::FRA {
        start: base_date + time::Duration::days(35),
        end: base_date + time::Duration::days(60),
        rate: 0.051,
        conventions: Default::default(),
    };
    let bounds = RateBounds::default();
    CalibrationPricer::validate_rates_quotes(
        &[fra1, fra2],
        &bounds,
        base_date,
        RatesQuoteUseCase::DiscountCurve {
            enforce_separation: false,
        },
    )
    .expect("distinct FRAs should not be considered duplicates");
}

#[test]
fn validate_quotes_rejects_duplicate_fra() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let fra = RatesQuote::FRA {
        start: base_date + time::Duration::days(30),
        end: base_date + time::Duration::days(60),
        rate: 0.05,
        conventions: Default::default(),
    };
    let bounds = RateBounds::default();
    let err = CalibrationPricer::validate_rates_quotes(
        &[fra.clone(), fra],
        &bounds,
        base_date,
        RatesQuoteUseCase::DiscountCurve {
            enforce_separation: false,
        },
    )
    .expect_err("duplicate FRA should be rejected");

    match err {
        finstack_core::Error::Input(finstack_core::error::InputError::Invalid) => {}
        other => panic!("Unexpected error: {:?}", other),
    }
}

#[test]
fn validate_quotes_rejects_started_fra() {
    let base_date = Date::from_calendar_date(2024, Month::January, 10).expect("valid base date");
    let fra = RatesQuote::FRA {
        start: base_date, // same as base -> invalid
        end: base_date + time::Duration::days(30),
        rate: 0.05,
        conventions: Default::default(),
    };
    let bounds = RateBounds::default();
    let err = CalibrationPricer::validate_rates_quotes(
        &[fra],
        &bounds,
        base_date,
        RatesQuoteUseCase::DiscountCurve {
            enforce_separation: false,
        },
    )
    .expect_err("FRA starting on base date should be rejected");
    assert!(matches!(err, finstack_core::Error::Calibration { .. }));
}

#[test]
fn validate_quotes_rejects_fra_with_inverted_dates() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let fra = RatesQuote::FRA {
        start: base_date + time::Duration::days(40),
        end: base_date + time::Duration::days(10), // ends before start
        rate: 0.05,
        conventions: Default::default(),
    };
    let bounds = RateBounds::default();
    let err = CalibrationPricer::validate_rates_quotes(
        &[fra],
        &bounds,
        base_date,
        RatesQuoteUseCase::DiscountCurve {
            enforce_separation: false,
        },
    )
    .expect_err("FRA with end before start should be rejected");
    assert!(matches!(err, finstack_core::Error::Calibration { .. }));
}

fn sample_swap(base_date: Date, index: &str, is_ois: bool) -> RatesQuote {
    RatesQuote::Swap {
        maturity: base_date + time::Duration::days(365),
        rate: 0.02,
        is_ois,
        conventions: Default::default(),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::semi_annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360)
            .with_index(index),
    }
}

#[test]
fn validate_quotes_rejects_misclassified_ois_swap() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let swap = sample_swap(base_date, "USD-LIBOR-3M", true);
    let bounds = RateBounds::default();
    let err = CalibrationPricer::validate_rates_quotes(
        &[swap],
        &bounds,
        base_date,
        RatesQuoteUseCase::DiscountCurve {
            enforce_separation: true,
        },
    )
    .expect_err("LIBOR swap flagged as OIS should be rejected");
    assert!(matches!(err, finstack_core::Error::Validation(_)));
}

#[test]
fn validate_quotes_accepts_known_ois_swap() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let swap = sample_swap(base_date, "USD-SOFR-OIS", true);
    let bounds = RateBounds::default();
    CalibrationPricer::validate_rates_quotes(
        &[swap],
        &bounds,
        base_date,
        RatesQuoteUseCase::DiscountCurve {
            enforce_separation: true,
        },
    )
    .expect("SOFR swap flagged as OIS should pass validation");
}

#[test]
fn settlement_respects_custom_bdc() {
    // Saturday base date; Preceding should move to prior Friday.
    let base_date = Date::from_calendar_date(2024, Month::January, 6).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let conventions = InstrumentConventions::default()
        .with_settlement_days(0)
        .with_business_day_convention(BusinessDayConvention::Preceding)
        .with_calendar_id("usny");

    let settlement = pricer
        .settlement_date_for_quote(&conventions, Currency::USD)
        .expect("settlement");

    assert_eq!(
        settlement,
        Date::from_calendar_date(2024, Month::January, 5).expect("valid date")
    );
}

#[test]
fn fixing_calendar_override_is_used() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let conventions = InstrumentConventions::default()
        .with_calendar_id("missing-calendar")
        .with_fixing_calendar_id("usny")
        .with_reset_lag(-2);

    let common = conv::resolve_common(&pricer, &conventions, Currency::USD);
    assert_eq!(common.fixing_calendar_id, "usny");

    let (fixing_date, found) = pricer
        .compute_fra_fixing_date(
            base_date + time::Duration::days(5),
            -2,
            common.fixing_calendar_id,
            false,
        )
        .expect("fixing date");
    assert!(
        found,
        "should use fixing calendar even if general calendar is missing"
    );
    assert_eq!(
        fixing_date,
        (base_date + time::Duration::days(5))
            .add_business_days(
                -2,
                CalendarRegistry::global().resolve_str("usny").expect("usny calendar")
            )
            .expect("business day math")
    );
}

#[test]
fn validate_curve_dependencies_allows_missing_calibrated_forward() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::for_forward_curve(base_date, "FWD_USD-SOFR-3M", "USD-OIS", 0.25);

    // Only provide the reference forward curve; primary (the one being calibrated) is absent.
    let ref_curve = ForwardCurve::builder("FWD_USD-SOFR-6M", 0.5)
        .base_date(base_date)
        .knots([(0.0, 0.03), (5.0, 0.03)])
        .build()
        .expect("forward curve");

    let ctx = MarketContext::new().insert_forward(ref_curve);

    let quote = RatesQuote::BasisSwap {
        maturity: base_date + time::Duration::days(365),
        spread_bp: 0.0,
        conventions: InstrumentConventions::default().with_currency(Currency::USD),
        primary_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-3M"),
        reference_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-6M"),
    };

    pricer
        .validate_curve_dependencies(&[quote], &ctx)
        .expect("calibrated forward curve may be missing");
}

#[test]
fn validate_curve_dependencies_rejects_missing_unrelated_forward() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::for_forward_curve(base_date, "FWD_USD-SOFR-3M", "USD-OIS", 0.25);

    // No forward curves in context; reference leg missing and not the calibrated curve.
    let ctx = MarketContext::new();

    let quote = RatesQuote::BasisSwap {
        maturity: base_date + time::Duration::days(365),
        spread_bp: 0.0,
        conventions: InstrumentConventions::default().with_currency(Currency::USD),
        primary_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-3M"),
        reference_leg_conventions: InstrumentConventions::default().with_index("USD-SOFR-6M"),
    };

    let err = pricer
        .validate_curve_dependencies(&[quote], &ctx)
        .expect_err("missing reference forward curve should error");

    match err {
        finstack_core::Error::Input(finstack_core::error::InputError::NotFound { .. }) => {}
        other => panic!("Unexpected error: {:?}", other),
    }
}

#[test]
fn future_convexity_uses_market_vol_if_provided() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let specs_default = FutureSpecs::default();
    let specs_with_vol = FutureSpecs {
        market_implied_vol: Some(0.02),
        ..FutureSpecs::default()
    };

    let t_exp = 0.5;
    let t_mat = 0.75;

    let adj_default = pricer
        .resolve_future_convexity(&specs_default, Currency::USD, t_exp, t_mat)
        .expect("convexity adjustment");
    let adj_market = pricer
        .resolve_future_convexity(&specs_with_vol, Currency::USD, t_exp, t_mat)
        .expect("convexity adjustment with vol");

    assert!(adj_market > adj_default, "market vol should increase convexity");
}

#[test]
fn future_notional_scales_with_multiplier() {
    let base_date = Date::from_calendar_date(2024, Month::January, 2).expect("valid base date");
    let pricer = CalibrationPricer::new(base_date, "USD-OIS");
    let specs = FutureSpecs {
        face_value: 1_000_000.0,
        multiplier: 2.5,
        ..FutureSpecs::default()
    };

    let notional = pricer.future_notional(&specs, Currency::USD);
    assert!((notional.amount() - 2_500_000.0).abs() < 1e-6);
}


