use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::types::{Currency, CurveId};
use finstack_valuations::calibration::adapters::discount::{
    DiscountCurveTarget, DiscountCurveTargetParams,
};
use finstack_valuations::calibration::pricing::CalibrationPricer;
use finstack_valuations::calibration::pricing::PreparedRatesQuote;
use finstack_valuations::calibration::quotes::{InstrumentConventions, RatesQuote};
use finstack_valuations::calibration::solver::{BootstrapTarget, GlobalSolveTarget};
use finstack_valuations::calibration::CalibrationConfig;
use time::{Duration, Month};

use super::tolerances::{assert_close_abs, F64_ABS_TOL_STRICT};

fn build_target(
    base_date: Date,
    settlement_date: Date,
    pricer: CalibrationPricer,
) -> DiscountCurveTarget {
    build_target_with_config(
        base_date,
        settlement_date,
        pricer,
        CalibrationConfig::default(),
        Currency::USD,
        InterpStyle::Linear,
    )
}

fn build_target_with_config(
    base_date: Date,
    settlement_date: Date,
    pricer: CalibrationPricer,
    config: CalibrationConfig,
    currency: Currency,
    solve_interp: InterpStyle,
) -> DiscountCurveTarget {
    let curve_id: CurveId = format!("{}-OIS", currency).into();

    DiscountCurveTarget::new(DiscountCurveTargetParams {
        base_date,
        currency,
        curve_id: curve_id.clone(),
        discount_curve_id: curve_id.clone(),
        forward_curve_id: curve_id.clone(),
        solve_interp,
        extrapolation: ExtrapolationPolicy::FlatForward,
        config,
        pricer,
        curve_day_count: DayCount::Act365F,
        spot_knot: None,
        settlement_date,
        base_context: MarketContext::new(),
    })
}

#[test]
fn quote_time_uses_curve_day_count_not_instrument_defaults() {
    let base_date = Date::from_calendar_date(2025, Month::January, 3).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 3).unwrap();

    let conventions = InstrumentConventions::default().with_day_count(DayCount::Act360);

    let quote = RatesQuote::Deposit {
        maturity,
        rate: 0.05,
        conventions,
    };

    let target = build_target(
        base_date,
        base_date,
        CalibrationPricer::new(base_date, CurveId::from("USD-OIS")),
    );

    let strict = target.pricer.conventions.strict_pricing.unwrap_or(false);
    let prepared =
        PreparedRatesQuote::new(&target.pricer, quote, Currency::USD, strict).expect("prepare");
    let actual = target.quote_time(&prepared).expect("quote time");
    let expected = DayCount::Act365F
        .year_fraction(
            base_date,
            maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
        .expect("year fraction");

    assert_close_abs(
        actual,
        expected,
        F64_ABS_TOL_STRICT,
        "quote_time (Act365F) should match DayCount::year_fraction",
    );
}

#[test]
fn deposit_initial_guess_respects_business_day_settlement() {
    let base_date = Date::from_calendar_date(2024, Month::May, 3).unwrap(); // Friday
    let maturity = Date::from_calendar_date(2024, Month::June, 3).unwrap();

    let conventions = InstrumentConventions::default()
        .with_settlement_days(2)
        .with_calendar_id("usny")
        .with_business_day_convention(BusinessDayConvention::ModifiedFollowing)
        .with_day_count(DayCount::Act360);

    let quote = RatesQuote::Deposit {
        maturity,
        rate: 0.05,
        conventions,
    };

    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"))
        .with_market_conventions(Currency::USD);
    let settlement_date = pricer
        .settlement_date_for_quote(quote.conventions(), Currency::USD)
        .expect("business-day settlement date");

    let target = build_target(base_date, settlement_date, pricer);

    let strict = target.pricer.conventions.strict_pricing.unwrap_or(false);
    let prepared =
        PreparedRatesQuote::new(&target.pricer, quote.clone(), Currency::USD, strict).expect("prepare");
    let df = target.initial_guess(&prepared, &[]).expect("initial guess");

    let day_count = quote.conventions().day_count.unwrap();
    let yf = day_count
        .year_fraction(settlement_date, maturity, DayCountCtx::default())
        .expect("year fraction")
        .max(1e-6);
    let expected_df = 1.0 / (1.0 + 0.05 * yf);

    assert_close_abs(
        df,
        expected_df,
        F64_ABS_TOL_STRICT,
        "deposit initial_guess should match simple-money-market DF",
    );
}

#[test]
fn global_solve_time_grid_is_sorted() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let maturity_short = Date::from_calendar_date(2025, Month::July, 2).unwrap();
    let maturity_long = Date::from_calendar_date(2026, Month::January, 2).unwrap();

    let quote_short = RatesQuote::Deposit {
        maturity: maturity_short,
        rate: 0.03,
        conventions: InstrumentConventions::default(),
    };
    let quote_long = RatesQuote::Deposit {
        maturity: maturity_long,
        rate: 0.04,
        conventions: InstrumentConventions::default(),
    };

    let target = build_target(
        base_date,
        base_date,
        CalibrationPricer::new(base_date, CurveId::from("USD-OIS")),
    );

    let strict = target.pricer.conventions.strict_pricing.unwrap_or(false);
    let quote_long_p =
        PreparedRatesQuote::new(&target.pricer, quote_long, Currency::USD, strict).expect("prepare");
    let quote_short_p =
        PreparedRatesQuote::new(&target.pricer, quote_short, Currency::USD, strict).expect("prepare");
    let unsorted = vec![quote_long_p.clone(), quote_short_p.clone()];
    let (times, _, active_quotes) = target
        .build_time_grid_and_guesses(&unsorted)
        .expect("sorted grid");

    assert!(
        times.windows(2).all(|w| w[0] < w[1]),
        "Times were not strictly increasing: {:?}",
        times
    );

    let maturities: Vec<Date> = active_quotes
        .iter()
        .map(|q| q.quote.maturity_date())
        .collect();
    assert_eq!(maturities, vec![maturity_short, maturity_long]);
}

#[test]
fn global_solve_rejects_duplicate_times() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let maturity = Date::from_calendar_date(2025, Month::July, 2).unwrap();

    let quote_a = RatesQuote::Deposit {
        maturity,
        rate: 0.03,
        conventions: InstrumentConventions::default(),
    };
    let quote_b = RatesQuote::Deposit {
        maturity,
        rate: 0.04,
        conventions: InstrumentConventions::default(),
    };

    let target = build_target(
        base_date,
        base_date,
        CalibrationPricer::new(base_date, CurveId::from("USD-OIS")),
    );

    let strict = target.pricer.conventions.strict_pricing.unwrap_or(false);
    let quote_a_p =
        PreparedRatesQuote::new(&target.pricer, quote_a, Currency::USD, strict).expect("prepare");
    let quote_b_p =
        PreparedRatesQuote::new(&target.pricer, quote_b, Currency::USD, strict).expect("prepare");

    let err = target
        .build_time_grid_and_guesses(&[quote_a_p, quote_b_p])
        .expect_err("duplicate times should error");

    if let finstack_core::Error::Calibration { category, .. } = err {
        assert_eq!(category, "global_solve");
    } else {
        panic!("Expected calibration error for duplicate times");
    }
}

#[test]
fn final_curve_monotonic_flag_controls_behavior() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let mut config = CalibrationConfig::default();
    config.discount_curve.allow_non_monotonic_final = Some(false);
    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target_strict = build_target_with_config(
        base_date,
        base_date,
        pricer,
        config,
        Currency::USD,
        InterpStyle::Linear,
    );

    let knots = vec![(0.0, 1.0), (1.0, 1.0005), (2.0, 0.98)];
    assert!(
        target_strict.build_curve_final(&knots).is_err(),
        "non-monotonic knots should fail when flag is false"
    );

    let mut config_allow = CalibrationConfig::default();
    config_allow.discount_curve.allow_non_monotonic_final = Some(true);
    let pricer_allow = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target_allow = build_target_with_config(
        base_date,
        base_date,
        pricer_allow,
        config_allow,
        Currency::USD,
        InterpStyle::Linear,
    );
    target_allow
        .build_curve_final(&knots)
        .expect("flag true should allow non-monotonic knots");
}

#[test]
fn final_curve_policy_follows_rate_bounds_and_interp_guard() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let knots = vec![(0.0, 1.0), (1.0, 1.0005), (2.0, 0.98)];

    // USD defaults to non-negative bounds => policy should reject.
    let pricer_usd = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target_usd = build_target_with_config(
        base_date,
        base_date,
        pricer_usd,
        CalibrationConfig::default(),
        Currency::USD,
        InterpStyle::Linear,
    );
    assert!(
        target_usd.build_curve_final(&knots).is_err(),
        "USD policy should enforce monotonicity by default"
    );

    // EUR supports negative rates => policy allows non-monotonic.
    let pricer_eur = CalibrationPricer::new(base_date, CurveId::from("EUR-OIS"));
    let target_eur = build_target_with_config(
        base_date,
        base_date,
        pricer_eur,
        CalibrationConfig::default(),
        Currency::EUR,
        InterpStyle::Linear,
    );
    target_eur
        .build_curve_final(&knots)
        .expect("EUR policy should allow non-monotonic knots");

    // Guard: cannot combine allow + MonotoneConvex interpolation.
    let mut config_convex = CalibrationConfig::default();
    config_convex.discount_curve.allow_non_monotonic_final = Some(true);
    let pricer_convex = CalibrationPricer::new(base_date, CurveId::from("EUR-OIS"));
    let target_convex = build_target_with_config(
        base_date,
        base_date,
        pricer_convex,
        config_convex,
        Currency::EUR,
        InterpStyle::MonotoneConvex,
    );
    let err = target_convex
        .build_curve_final(&knots)
        .expect_err("MonotoneConvex should reject non-monotonic knots");
    if let finstack_core::Error::Calibration { category, .. } = err {
        assert_eq!(category, "curve_build");
    } else {
        panic!("Expected calibration error for MonotoneConvex guard");
    }
}

#[test]
fn solver_curve_build_respects_requested_interpolation() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target_convex = build_target_with_config(
        base_date,
        base_date,
        pricer,
        CalibrationConfig::default(),
        Currency::USD,
        InterpStyle::MonotoneConvex,
    );

    // Non-monotonic DFs are incompatible with MonotoneConvex.
    let non_monotonic = vec![(0.0, 1.0), (1.0, 1.0005), (2.0, 0.98)];
    assert!(
        target_convex
            .build_curve_for_solver(&non_monotonic)
            .is_err(),
        "MonotoneConvex should reject non-monotonic discount factors"
    );

    // Monotone DFs should be accepted.
    let monotonic = vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.95)];
    target_convex
        .build_curve_for_solver(&monotonic)
        .expect("MonotoneConvex should accept monotonic discount factors");

    // Linear interpolation remains permissive for solver exploration.
    let pricer_lin = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target_lin = build_target_with_config(
        base_date,
        base_date,
        pricer_lin,
        CalibrationConfig::default(),
        Currency::USD,
        InterpStyle::Linear,
    );
    target_lin
        .build_curve_for_solver(&non_monotonic)
        .expect("Linear interpolation should accept non-monotonic discount factors");
}

#[test]
fn scan_points_cover_short_end_range() {
    let base_date = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let maturity = base_date + Duration::days(7);

    let mut config = CalibrationConfig::default();
    config.discount_curve.scan_grid_points = 24;
    config.discount_curve.min_scan_grid_points = 24;

    let pricer = CalibrationPricer::new(base_date, CurveId::from("USD-OIS"));
    let target = build_target_with_config(
        base_date,
        base_date,
        pricer,
        config,
        Currency::USD,
        InterpStyle::Linear,
    );

    let conventions = InstrumentConventions::default().with_day_count(DayCount::Act360);

    let quote = RatesQuote::Deposit {
        maturity,
        rate: 0.04,
        conventions,
    };

    let strict = target.pricer.conventions.strict_pricing.unwrap_or(false);
    let prepared =
        PreparedRatesQuote::new(&target.pricer, quote, Currency::USD, strict).expect("prepare");
    let guess = target.initial_guess(&prepared, &[]).expect("initial guess");
    let grid = target
        .scan_points(&prepared, guess)
        .expect("short-end scan points");

    assert!(
        grid.len() >= 24,
        "expected at least 24 scan points, got {}",
        grid.len()
    );
    assert!(
        grid.iter().any(|&df| df < guess),
        "expected at least one scan point below the initial guess"
    );
    assert!(
        grid.iter().any(|&df| df > guess),
        "expected at least one scan point above the initial guess"
    );
}
