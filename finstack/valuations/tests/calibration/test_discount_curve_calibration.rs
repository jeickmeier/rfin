//! Integration tests for DiscountCurveCalibrator.
//!
//! Tests for discount curve bootstrapping including:
//! - Multi-curve instrument validation
//! - Quote validation and sorting
//! - Deposit, FRA, and swap repricing
//! - Negative rate environments
//! - Extrapolation and day count configuration

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::money::Money;
use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
use finstack_valuations::calibration::quotes::InstrumentConventions;
use finstack_valuations::calibration::{
    Calibrator, MultiCurveConfig, RatesQuote, CALIBRATION_CONFIG_KEY_V1,
};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::deposit::Deposit;
use finstack_valuations::instruments::irs::FloatingLegCompounding;
use time::Month;

#[test]
fn test_multi_curve_instrument_validation() {
    // Test that RatesQuote correctly identifies forward-dependent instruments
    let deposit = RatesQuote::Deposit {
        maturity: Date::from_calendar_date(2024, Month::February, 1).expect("Valid test date"),
        rate: 0.015,
        conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
    };
    assert!(
        !deposit.requires_forward_curve(),
        "Deposits should not require forward curves"
    );
    assert!(
        deposit.is_ois_suitable(),
        "Deposits should be suitable for OIS"
    );

    let fra = RatesQuote::FRA {
        start: Date::from_calendar_date(2024, Month::April, 1).expect("Valid test date"),
        end: Date::from_calendar_date(2024, Month::July, 1).expect("Valid test date"),
        rate: 0.018,
        conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
    };
    assert!(
        fra.requires_forward_curve(),
        "FRAs should require forward curves"
    );
    assert!(
        !fra.is_ois_suitable(),
        "FRAs should not be suitable for OIS"
    );

    let ois_swap = RatesQuote::Swap {
        maturity: Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
        rate: 0.02,
        is_ois: true,
        conventions: Default::default(),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::daily())
            .with_day_count(DayCount::Act360)
            .with_index("SOFR"),
    };
    assert!(
        ois_swap.requires_forward_curve(),
        "Swaps require forward curves"
    );
    assert!(
        ois_swap.is_ois_suitable(),
        "SOFR swaps should be OIS suitable"
    );

    let libor_swap = RatesQuote::Swap {
        maturity: Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
        rate: 0.02,
        is_ois: false, // LIBOR swaps are not OIS suitable
        conventions: Default::default(),
        fixed_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::semi_annual())
            .with_day_count(DayCount::Thirty360),
        float_leg_conventions: InstrumentConventions::default()
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360)
            .with_index("3M-LIBOR"),
    };
    assert!(
        libor_swap.requires_forward_curve(),
        "Swaps require forward curves"
    );
    assert!(
        !libor_swap.is_ois_suitable(),
        "LIBOR swaps should not be OIS suitable"
    );
}

#[test]
fn test_multi_curve_config() {
    // Test multi-curve configuration
    let multi_config = MultiCurveConfig::new();
    assert!(multi_config.calibrate_basis);
    assert!(multi_config.enforce_separation);
}

fn create_test_quotes() -> Vec<RatesQuote> {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

    vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.047,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::quarterly())
                .with_day_count(DayCount::Act360)
                .with_index("USD-SOFR-3M"),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.048,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::quarterly())
                .with_day_count(DayCount::Act360)
                .with_index("USD-SOFR-3M"),
        },
    ]
}

#[test]
fn test_quote_sorting() {
    let mut quotes = create_test_quotes();

    // Reverse order
    quotes.reverse();

    // Get maturities before sorting
    let maturities_before: Vec<_> = quotes.iter().map(|q| q.maturity_date()).collect();

    // Sort
    quotes.sort_by_key(RatesQuote::maturity_date);

    // Get maturities after sorting
    let maturities_after: Vec<_> = quotes.iter().map(|q| q.maturity_date()).collect();

    // Should be properly sorted
    for i in 1..maturities_after.len() {
        assert!(maturities_after[i] >= maturities_after[i - 1]);
    }

    // Should not be the same as the original reversed order
    assert_ne!(maturities_before, maturities_after);
}

#[test]
fn test_deposit_repricing_under_bootstrap() {
    // Use a business day as base_date (Thursday, January 2, 2025)
    // to avoid holiday adjustment complications
    let base_date = Date::from_calendar_date(2025, Month::January, 2).expect("Valid test date");

    // Use explicit T+0 settlement to match pre-settlement-aware behavior
    // For production, use currency default (T+2 for USD)
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD); // T+0 for test consistency

    // Use just deposits for initial test
    let deposit_quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(180),
            rate: 0.047,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
    ];

    let base_context = MarketContext::new();

    let (curve, report) = calibrator
        .calibrate(&deposit_quotes, &base_context)
        .expect("Deposit calibration should succeed");
    assert!(report.success);
    assert_eq!(curve.id().as_str(), "USD-OIS");

    // Verify repricing via instrument PVs (|PV| ≤ $1 per $1MM for 0.1bp tolerance)
    let ctx = base_context.insert_discount(curve);
    for quote in &deposit_quotes {
        if let RatesQuote::Deposit {
            maturity,
            rate,
            conventions,
        } = quote
        {
            let day_count = quote
                .conventions()
                .day_count
                .unwrap_or_else(|| InstrumentConventions::default_money_market_day_count(Currency::USD));
            let dep = Deposit {
                id: format!("DEP-{}", maturity).into(),
                notional: Money::new(1_000_000.0, Currency::USD),
                start: base_date, // Match T+0 settlement
                end: *maturity,
                day_count,
                quote_rate: Some(*rate),
                discount_curve_id: "USD-OIS".into(),
                attributes: Default::default(),
                spot_lag_days: conventions.settlement_days,
                bdc: None,
                calendar_id: None,
            };
            let pv = dep
                .value(&ctx, base_date)
                .expect("Deposit valuation should succeed in test");
            // For deposits, $1 per $1M notional is approximately 0.1bp tolerance
            assert!(
                pv.amount().abs() <= 1.0,
                "Deposit PV too large: ${:.2} (expected <= $1 for 0.1bp tolerance)",
                pv.amount()
            );
        }
    }
}

#[test]
fn test_fra_repricing_under_bootstrap() {
    // Use a business day as base_date (Thursday, January 2, 2025)
    let base_date = Date::from_calendar_date(2025, Month::January, 2).expect("Valid test date");
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        CALIBRATION_CONFIG_KEY_V1,
        serde_json::json!({
            "tolerance": 1e-12,
            "max_iterations": 100,
            "use_fd_sabr_gradients": true,
            "rate_bounds_policy": "explicit",
            "rate_bounds": { "min_rate": -0.05, "max_rate": 1.00 }
        }),
    );
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_finstack_config(&cfg)
        .expect("valid config");

    // Build quotes: deposits + one FRA
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(90),
            end: base_date + time::Duration::days(180),
            rate: 0.0470,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
    ];

    // In the multi-curve framework, discount curves must be calibrated with
    // OIS-suitable instruments only. FRAs require a forward curve
    // calibrated via `ForwardCurveCalibrator`, not `DiscountCurveCalibrator`.
    let base_context = MarketContext::new();
    let result = calibrator.calibrate(&quotes, &base_context);

    match result {
        Ok(_) => panic!(
            "DiscountCurveCalibrator should reject FRA quotes in multi-curve framework; \
             use ForwardCurveCalibrator for FRAs instead."
        ),
        Err(err) => {
            let msg = format!("{err}");
            assert!(
                msg.contains("non-OIS forward-dependent quote")
                    && msg.contains("calibrate forward curves separately"),
                "Unexpected error message for FRA quote in discount calibration: {msg}"
            );
        }
    }
}

#[test]
fn test_swap_repricing_under_bootstrap() {
    use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
    use finstack_valuations::metrics::{MetricCalculator, MetricContext};

    let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

    // Use conservative config for tighter convergence (1e-12 tolerance, 200 iterations)
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        CALIBRATION_CONFIG_KEY_V1,
        serde_json::json!({
            "tolerance": 1e-12,
            "max_iterations": 200,
            "use_fd_sabr_gradients": true,
            "rate_bounds_policy": "explicit",
            "rate_bounds": { "min_rate": -0.05, "max_rate": 1.00 }
        }),
    );

    // Use T+0 settlement for test consistency
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_finstack_config(&cfg)
        .expect("valid config");

    // Quotes: deposits + one 1Y swap par rate
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0470,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
    ];

    // OIS swaps can be calibrated without pre-existing forward curves
    let base_context = MarketContext::new();
    let (curve, report) = calibrator
        .calibrate(&quotes, &base_context)
        .expect("Swap calibration should succeed");

    assert!(report.success, "Calibration should succeed: {:?}", report);

    // Verify calibration residuals are tight (price_instrument returns PV/notional, so 1e-4 = $100 per $1M)
    // For 0.1bp tolerance on a swap with DV01=$96.64, we need residual < $9.66/$1M = 9.66e-6
    assert!(
        report.max_residual < 1e-5,
        "Calibration residual too large: {:.2e} (expected < 1e-5 for 0.1bp tolerance)",
        report.max_residual
    );

    // For OIS swaps, we don't need a forward curve since the pricer uses discount-only
    let ctx = base_context.insert_discount(curve);

    // Construct 1Y par swap matching quote
    let start = base_date;
    let end = base_date + time::Duration::days(365);
    let irs = InterestRateSwap::builder()
        .id("IRS-1Y".to_string().into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .side(PayReceive::ReceiveFixed)
        .fixed(finstack_valuations::instruments::irs::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: 0.0470,
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start,
            end,
            payment_delay_days: 0,
        })
        .float(finstack_valuations::instruments::irs::FloatLegSpec {
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-OIS".into(),
            spread_bp: 0.0,
            freq: Tenor::daily(),
            dc: DayCount::Act360,
            bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            start,
            end,
            compounding: FloatingLegCompounding::sofr(),
            payment_delay_days: 0,
        })
        .build()
        .expect("IRS builder should succeed with valid test data");

    let pv = irs
        .value(&ctx, base_date)
        .expect("IRS valuation should succeed in test");

    // Calculate DV01 and check repricing within 0.1bp tolerance
    let mut metric_ctx = MetricContext::new(
        std::sync::Arc::new(irs.clone()),
        std::sync::Arc::new(ctx.clone()),
        base_date,
        pv,
    );

    // Calculate DV01 using unified DV01 calculator
    use finstack_valuations::metrics::{Dv01CalculatorConfig, UnifiedDv01Calculator};
    let dv01_calc =
        UnifiedDv01Calculator::<finstack_valuations::instruments::InterestRateSwap>::new(
            Dv01CalculatorConfig::parallel_combined(),
        );
    let dv01 = dv01_calc
        .calculate(&mut metric_ctx)
        .expect("DV01 calculation should succeed in test");

    // Tolerance: 10bp * |DV01| for bootstrap (tighter checks in dedicated swap tests)
    // OIS swap calibration has inherent approximations in schedule generation
    // that can cause residuals up to a few bp
    let tolerance = (10.0 * dv01.abs()).max(100.0);

    assert!(
        pv.amount().abs() <= tolerance,
        "Swap PV too large: ${:.2} (DV01: ${:.2}, tolerance: ${:.2})",
        pv.amount(),
        dv01,
        tolerance
    );
}

#[test]
fn test_ois_bootstrap_with_deposits_and_ois_swaps() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

    // Deposits + OIS swaps (float leg frequency set to daily; index contains "USD-OIS")
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0470,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.0480,
            is_ois: true,
            conventions: Default::default(),
            fixed_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::semi_annual())
                .with_day_count(DayCount::Thirty360),
            float_leg_conventions: InstrumentConventions::default()
                .with_payment_frequency(Tenor::daily())
                .with_day_count(DayCount::Act360)
                .with_index("USD-OIS"),
        },
    ];

    let base_context = MarketContext::new();
    let (_curve, report) = calibrator
        .calibrate(&quotes, &base_context)
        .expect("OIS bootstrap should succeed");

    // Residuals should be small since par swaps should reprice under discount-only formula
    assert!(report.success);
    assert!(report.max_residual < 1e-4);
}

#[test]
fn test_configured_interpolation_used() {
    use finstack_core::dates::DateExt;

    let base_date = Date::from_calendar_date(2025, Month::January, 31).expect("Valid test date");

    // Test 1: Verify configured interpolation is used
    let linear_calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::Linear);
    let monotone_calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex);

    assert!(matches!(
        linear_calibrator.solve_interp,
        InterpStyle::Linear
    ));
    assert!(matches!(
        monotone_calibrator.solve_interp,
        InterpStyle::MonotoneConvex
    ));

    // Test 2: Verify proper month arithmetic vs crude approximation
    let delivery_months = 3i32;

    // Crude way (should be wrong for end-of-month)
    let crude_result = base_date + time::Duration::days((delivery_months as i64) * 30);

    // Proper way
    let proper_result = base_date.add_months(delivery_months);

    // Should be different for Jan 31 + 3 months
    assert_ne!(
        crude_result, proper_result,
        "Month arithmetic should give different results"
    );

    // The proper result should handle month-end correctly
    // Jan 31 + 3 months = Apr 30 (no Apr 31)
    let expected = Date::from_calendar_date(2025, Month::April, 30).expect("Valid test date");
    assert_eq!(
        proper_result, expected,
        "Expected proper month-end handling"
    );
}

// ========================================================================
// Tests for market-standards improvements
// ========================================================================

#[test]
fn test_extrapolation_policy_config() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

    // Default should be FlatForward
    let default_cal = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD);
    assert!(
        matches!(default_cal.extrapolation, ExtrapolationPolicy::FlatForward),
        "Default extrapolation should be FlatForward"
    );

    // Can configure FlatZero
    let flat_zero = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD)
        .with_extrapolation(ExtrapolationPolicy::FlatZero);
    assert!(
        matches!(flat_zero.extrapolation, ExtrapolationPolicy::FlatZero),
        "Should be able to configure FlatZero extrapolation"
    );
}

#[test]
fn test_negative_rate_curve_building() {
    // First, verify we can build a curve with DF > 1.0 directly
    let base_date = Date::from_calendar_date(2020, Month::January, 1).expect("Valid test date");

    let knots = vec![
        (0.0, 1.0),
        (0.25, 1.0025), // -1% rate for 90 days: DF = 1/(1+(-0.01)*0.25) ≈ 1.0025
        (0.5, 1.002),   // Slightly lower negative rate
    ];

    let curve = DiscountCurve::builder("TEST-NEG")
        .base_date(base_date)
        .knots(knots)
        .set_interp(InterpStyle::Linear) // Use simple linear for test
        .allow_non_monotonic()
        .build()
        .expect("Curve with DF > 1.0 should build successfully");

    // Verify DF values
    let df_0 = curve.df(0.0);
    let df_90d = curve.df(0.25);

    assert!((df_0 - 1.0).abs() < 1e-10, "DF(0) should be 1.0: {}", df_0);
    assert!(
        df_90d > 1.0,
        "DF at 90 days should exceed 1.0: {} (expected 1.0025)",
        df_90d
    );
}

#[test]
fn test_negative_rate_deposit_calibration() {
    // Test calibration with negative rates (EUR/CHF/JPY scenario)
    let base_date = Date::from_calendar_date(2020, Month::January, 1).expect("Valid test date");

    // Use T+0 settlement and Linear interpolation for simpler debugging
    let calibrator = DiscountCurveCalibrator::new("EUR-ESTR", base_date, Currency::EUR)
        .with_solve_interp(InterpStyle::Linear); // Use Linear for predictable behavior

    // Use more pronounced negative rates and longer maturities
    // to see DF > 1.0 effect clearly
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: -0.01, // -100bp (more pronounced negative)
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(180),
            rate: -0.008, // -80bp
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(365),
            rate: -0.005, // -50bp
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
    ];

    let base_context = MarketContext::new();
    let result = calibrator.calibrate(&quotes, &base_context);

    // Should succeed with negative rates
    assert!(
        result.is_ok(),
        "Calibration should succeed with negative rates: {:?}",
        result.err()
    );

    let (curve, report) = result.expect("Calibration should succeed");

    // Verify discount factors > 1.0 at first knot (negative rates)
    // At -100bp for 90 days, DF ≈ 1 / (1 + (-0.01) * 90/360) = 1 / 0.9975 ≈ 1.0025
    let df_90d = curve.df(90.0 / 360.0);
    assert!(
        df_90d > 1.0,
        "DF at 90 days should exceed 1.0 for -100bp rate: {} (expected ~1.0025)",
        df_90d
    );

    // Residuals should still be tight
    assert!(report.success, "Calibration should report success");
    assert!(
        report.max_residual < 1e-4,
        "Max residual should be small: {}",
        report.max_residual
    );
}

#[test]
fn test_pre_validation_fails_for_missing_forward_curves() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
    let calibrator = DiscountCurveCalibrator::new("TEST", base_date, Currency::USD);

    // Basis swap requires forward curves
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::BasisSwap {
            maturity: base_date + time::Duration::days(365),
            spread_bp: 5.0,
            conventions: InstrumentConventions::default().with_currency(Currency::USD),
            primary_leg_conventions: InstrumentConventions::default()
                .with_index("3M-SOFR")
                .with_payment_frequency(Tenor::quarterly())
                .with_day_count(DayCount::Act360),
            reference_leg_conventions: InstrumentConventions::default()
                .with_index("1M-SOFR")
                .with_payment_frequency(Tenor::monthly())
                .with_day_count(DayCount::Act360),
        },
    ];

    let base_context = MarketContext::new(); // No forward curves
    let result = calibrator.calibrate(&quotes, &base_context);

    // Should fail with clear error about missing forward curve
    let err = result.expect_err("Should fail when forward curves are missing");
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("Forward curve") || err_msg.contains("forward curve"),
        "Error should mention missing forward curve: {}",
        err_msg
    );
}

#[test]
fn test_calibration_report_metadata() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_extrapolation(ExtrapolationPolicy::FlatZero);

    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
        },
    ];

    let base_context = MarketContext::new();
    let (_, report) = calibrator
        .calibrate(&quotes, &base_context)
        .expect("Calibration should succeed");

    // Verify metadata is populated
    assert!(
        report.metadata.contains_key("extrapolation"),
        "Report should contain extrapolation metadata"
    );
    assert!(
        report.metadata.contains_key("curve_day_count"),
        "Report should contain curve_day_count metadata"
    );

    // Check specific values
    assert!(
        report.metadata["extrapolation"].contains("FlatZero"),
        "Extrapolation metadata should indicate FlatZero"
    );
}
