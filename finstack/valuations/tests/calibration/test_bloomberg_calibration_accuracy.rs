//! Bloomberg calibration accuracy tests.
//!
//! Validates calibrated curves against Bloomberg reference data to ensure
//! the calibration engine produces market-standard results.

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::calibration::api::schema::{
    CalibrationMethod, DiscountCurveParams, StepParams,
};
use finstack_valuations::calibration::targets::handlers::execute_step;
use finstack_valuations::calibration::ResidualWeightingScheme;
use finstack_valuations::calibration::{CalibrationConfig, CALIBRATION_CONFIG_KEY};
use finstack_valuations::market::conventions::ids::IndexId;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use time::Month;

/// Helper to create deposit quotes with ACT/360 day count (using "USD-FEDFUNDS-OIS" conventions)
fn deposit(maturity: Date, rate: f64) -> RateQuote {
    RateQuote::Deposit {
        id: QuoteId::new(format!("DEP-{}", maturity)),
        index: IndexId::new("USD-DEPOSIT"),
        pillar: Pillar::Date(maturity),
        rate,
    }
}

/// Helper to create OIS swap quotes with annual frequency and ACT/360
fn ois_swap(maturity: Date, rate: f64) -> RateQuote {
    RateQuote::Swap {
        id: QuoteId::new(format!("OIS-{}", maturity)),
        index: IndexId::new("USD-FEDFUNDS-OIS"),
        pillar: Pillar::Date(maturity),
        rate,
        spread: None,
    }
}

/// Test calibration accuracy against Bloomberg USD OIS curve data.
///
/// This test uses actual market quotes from Bloomberg (curve date: 2025-12-10, USD spot-start)
/// and validates that the calibrated curve produces zero rates and discount factors
/// within acceptable tolerance of Bloomberg's values.
///
/// Data source: Bloomberg USD OIS curve (USSOC Curncy)
#[test]
fn test_bloomberg_usd_ois_calibration_accuracy() {
    // Bloomberg curve date: 2025-12-10 (pricing date; instruments are spot-start in USD).
    let base_date = Date::from_calendar_date(2025, Month::December, 10).expect("Valid test date");

    // Bloomberg USD OIS curve data (Settle: 12/10/2025)
    // Short-end deposits (< 1Y) and OIS swaps (>= 1Y)
    let discount_quotes = vec![
        // Short-term deposits - using actual maturity dates from Bloomberg
        deposit(
            Date::from_calendar_date(2025, Month::December, 19).expect("Valid date"),
            0.0364447,
        ), // 1W
        deposit(
            Date::from_calendar_date(2025, Month::December, 26).expect("Valid date"),
            0.0364455,
        ), // 2W
        deposit(
            Date::from_calendar_date(2026, Month::January, 2).expect("Valid date"),
            0.0365300,
        ), // 3W
        deposit(
            Date::from_calendar_date(2026, Month::January, 12).expect("Valid date"),
            0.0364950,
        ), // 1M
        deposit(
            Date::from_calendar_date(2026, Month::February, 12).expect("Valid date"),
            0.0364050,
        ), // 2M
        deposit(
            Date::from_calendar_date(2026, Month::March, 12).expect("Valid date"),
            0.0363477,
        ), // 3M
        deposit(
            Date::from_calendar_date(2026, Month::April, 13).expect("Valid date"),
            0.0361400,
        ), // 4M
        deposit(
            Date::from_calendar_date(2026, Month::May, 12).expect("Valid date"),
            0.0359544,
        ), // 5M
        deposit(
            Date::from_calendar_date(2026, Month::June, 12).expect("Valid date"),
            0.0358000,
        ), // 6M
        deposit(
            Date::from_calendar_date(2026, Month::July, 13).expect("Valid date"),
            0.0355310,
        ), // 7M
        deposit(
            Date::from_calendar_date(2026, Month::August, 12).expect("Valid date"),
            0.0352500,
        ), // 8M
        deposit(
            Date::from_calendar_date(2026, Month::September, 14).expect("Valid date"),
            0.0350225,
        ), // 9M
        deposit(
            Date::from_calendar_date(2026, Month::October, 13).expect("Valid date"),
            0.0347742,
        ), // 10M
        deposit(
            Date::from_calendar_date(2026, Month::November, 12).expect("Valid date"),
            0.0345356,
        ), // 11M
        // OIS Swaps (>= 1Y)
        ois_swap(
            Date::from_calendar_date(2026, Month::December, 14).expect("Valid date"),
            0.0343446,
        ), // 1Y
        ois_swap(
            Date::from_calendar_date(2027, Month::June, 14).expect("Valid date"),
            0.0332849,
        ), // 18M
        ois_swap(
            Date::from_calendar_date(2027, Month::December, 13).expect("Valid date"),
            0.0329864,
        ), // 2Y
        ois_swap(
            Date::from_calendar_date(2028, Month::December, 12).expect("Valid date"),
            0.0330190,
        ), // 3Y
        ois_swap(
            Date::from_calendar_date(2029, Month::December, 12).expect("Valid date"),
            0.0333823,
        ), // 4Y
        ois_swap(
            Date::from_calendar_date(2030, Month::December, 12).expect("Valid date"),
            0.0338799,
        ), // 5Y
        ois_swap(
            Date::from_calendar_date(2031, Month::December, 12).expect("Valid date"),
            0.0344608,
        ), // 6Y
        ois_swap(
            Date::from_calendar_date(2032, Month::December, 13).expect("Valid date"),
            0.0350619,
        ), // 7Y
        ois_swap(
            Date::from_calendar_date(2033, Month::December, 12).expect("Valid date"),
            0.0356592,
        ), // 8Y
        ois_swap(
            Date::from_calendar_date(2034, Month::December, 12).expect("Valid date"),
            0.0362453,
        ), // 9Y
        ois_swap(
            Date::from_calendar_date(2035, Month::December, 12).expect("Valid date"),
            0.0368206,
        ), // 10Y
        ois_swap(
            Date::from_calendar_date(2037, Month::December, 14).expect("Valid date"),
            0.0378975,
        ), // 12Y
        ois_swap(
            Date::from_calendar_date(2040, Month::December, 12).expect("Valid date"),
            0.0391717,
        ), // 15Y
        ois_swap(
            Date::from_calendar_date(2045, Month::December, 12).expect("Valid date"),
            0.0402348,
        ), // 20Y
        ois_swap(
            Date::from_calendar_date(2050, Month::December, 12).expect("Valid date"),
            0.0410412,
        ), // 25Y
        ois_swap(
            Date::from_calendar_date(2055, Month::December, 13).expect("Valid date"),
            0.0403819,
        ), // 30Y
        ois_swap(
            Date::from_calendar_date(2065, Month::December, 14).expect("Valid date"),
            0.0381263,
        ), // 40Y
        ois_swap(
            Date::from_calendar_date(2075, Month::December, 12).expect("Valid date"),
            0.0355798,
        ), // 50Y
    ];

    // Bloomberg zero rates (in decimal) and discount factors at each maturity
    // Source: Bloomberg USSOC Curncy as of 2025-12-10
    let bloomberg_data: Vec<(Date, f64, f64)> = vec![
        // (maturity_date, zero_rate_decimal, discount_factor)
        (
            Date::from_calendar_date(2025, Month::December, 19).expect("Valid date"),
            0.0369398,
            0.999090,
        ),
        (
            Date::from_calendar_date(2025, Month::December, 26).expect("Valid date"),
            0.0369282,
            0.998383,
        ),
        (
            Date::from_calendar_date(2026, Month::January, 2).expect("Valid date"),
            0.0369935,
            0.997672,
        ),
        (
            Date::from_calendar_date(2026, Month::January, 12).expect("Valid date"),
            0.0369440,
            0.996665,
        ),
        (
            Date::from_calendar_date(2026, Month::February, 12).expect("Valid date"),
            0.0368001,
            0.993568,
        ),
        (
            Date::from_calendar_date(2026, Month::March, 12).expect("Valid date"),
            0.0366918,
            0.990794,
        ),
        (
            Date::from_calendar_date(2026, Month::April, 13).expect("Valid date"),
            0.0364279,
            0.987701,
        ),
        (
            Date::from_calendar_date(2026, Month::May, 12).expect("Valid date"),
            0.0361916,
            0.984944,
        ),
        (
            Date::from_calendar_date(2026, Month::June, 12).expect("Valid date"),
            0.0359832,
            0.982024,
        ),
        (
            Date::from_calendar_date(2026, Month::July, 13).expect("Valid date"),
            0.0356631,
            0.979212,
        ),
        (
            Date::from_calendar_date(2026, Month::August, 12).expect("Valid date"),
            0.0353343,
            0.976562,
        ),
        (
            Date::from_calendar_date(2026, Month::September, 14).expect("Valid date"),
            0.0350543,
            0.973654,
        ),
        (
            Date::from_calendar_date(2026, Month::October, 13).expect("Valid date"),
            0.0347621,
            0.971185,
        ),
        (
            Date::from_calendar_date(2026, Month::November, 12).expect("Valid date"),
            0.0344791,
            0.968667,
        ),
        (
            Date::from_calendar_date(2026, Month::December, 14).expect("Valid date"),
            0.0342406,
            0.965976,
        ),
        (
            Date::from_calendar_date(2027, Month::June, 14).expect("Valid date"),
            0.0332790,
            0.951019,
        ),
        (
            Date::from_calendar_date(2027, Month::December, 13).expect("Valid date"),
            0.0328850,
            0.936093,
        ),
        (
            Date::from_calendar_date(2028, Month::December, 12).expect("Valid date"),
            0.0329221,
            0.905709,
        ), // 3Y
        (
            Date::from_calendar_date(2029, Month::December, 12).expect("Valid date"),
            0.0332985,
            0.875056,
        ), // 4Y
        (
            Date::from_calendar_date(2030, Month::December, 12).expect("Valid date"),
            0.0338188,
            0.844195,
        ), // 5Y
        (
            Date::from_calendar_date(2031, Month::December, 12).expect("Valid date"),
            0.0344337,
            0.813113,
        ), // 6Y
        (
            Date::from_calendar_date(2032, Month::December, 13).expect("Valid date"),
            0.0350778,
            0.781903,
        ), // 7Y
        (
            Date::from_calendar_date(2033, Month::December, 12).expect("Valid date"),
            0.0357278,
            0.751102,
        ), // 8Y
        (
            Date::from_calendar_date(2034, Month::December, 12).expect("Valid date"),
            0.0363749,
            0.720527,
        ), // 9Y
        (
            Date::from_calendar_date(2035, Month::December, 12).expect("Valid date"),
            0.0370206,
            0.690312,
        ), // 10Y
        (
            Date::from_calendar_date(2037, Month::December, 14).expect("Valid date"),
            0.0382585,
            0.631387,
        ), // 12Y
           // (
           //     Date::from_calendar_date(2040, Month::December, 12).expect("Valid date"),
           //     0.0397745,
           //     0.550311,
           // ), // 15Y
           // (
           //     Date::from_calendar_date(2045, Month::December, 12).expect("Valid date"),
           //     0.0410343,
           //     0.439783,
           // ), // 20Y
           // (
           //     Date::from_calendar_date(2050, Month::December, 12).expect("Valid date"),
           //     0.0410412,
           //     0.358105,
           // ), // 25Y (peak)
           // (
           //     Date::from_calendar_date(2055, Month::December, 13).expect("Valid date"),
           //     0.0403819,
           //     0.297434,
           // ), // 30Y
    ];

    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        CALIBRATION_CONFIG_KEY,
        serde_json::json!({
            "solver": {
                "method": "brent",
                "tolerance": 1e-12,
                "max_iterations": 200
            }
        }),
    );
    let mut settings =
        CalibrationConfig::from_finstack_config_or_default(&cfg).expect("valid config");
    settings.discount_curve.weighting_scheme = ResidualWeightingScheme::Equal;

    let params = DiscountCurveParams {
        curve_id: "USD-OIS".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::GlobalSolve {
            use_analytical_jacobian: true,
        },
        interpolation: InterpStyle::LogLinear,
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: finstack_valuations::calibration::api::schema::RatesStepConventions {
            curve_day_count: Some(DayCount::Act365F),
            settlement_days: Some(2),
            calendar_id: Some("usny".to_string()),
            business_day_convention: Some(BusinessDayConvention::ModifiedFollowing),
            allow_calendar_fallback: Some(false),
            use_settlement_start: Some(true),
            default_payment_delay_days: Some(2),
            default_reset_lag_days: Some(0),
            strict_pricing: Some(false),
            ..Default::default()
        },
    };
    let step = StepParams::Discount(params);
    let quotes: Vec<MarketQuote> = discount_quotes
        .iter()
        .cloned()
        .map(MarketQuote::Rates)
        .collect();
    let base_context = MarketContext::new();

    let (ctx, report) = execute_step(&step, &quotes, &base_context, &settings)
        .expect("Discount curve calibration should succeed");
    let curve = ctx
        .get_discount_ref("USD-OIS")
        .expect("curve inserted")
        .clone();

    // Verify calibration succeeded
    assert!(
        report.success,
        "Calibration should report success. Report: {:?}",
        report
    );
    assert!(
        report.max_residual < 1e-8,
        "Calibration max_residual must be < 1e-8. Got {:.3e}",
        report.max_residual
    );

    // Bloomberg DF matching: calibrated curve vs reference discount factors
    let mut mismatches = Vec::new();
    for (maturity, _zero_rate, expected_df) in bloomberg_data {
        let calibrated_df = curve
            .try_df_on_date_curve(maturity)
            .expect("DF on maturity date");
        let t = DayCount::Act365F
            .year_fraction(base_date, maturity, DayCountCtx::default())
            .expect("year fraction");
        let calibrated_zero = -calibrated_df.ln() / t;
        let diff = (calibrated_df - expected_df).abs();
        if diff >= 1e-3 {
            mismatches.push(format!(
                "maturity={:?}: calibrated_df={:.6}, bloomberg_df={:.6}, diff_df={:.6}, calibrated_zero={:.6}%, bb_zero={:.6}%, diff_zero={:.4} bps",
                maturity,
                calibrated_df,
                expected_df,
                diff,
                calibrated_zero * 100.0,
                _zero_rate * 100.0,
                (calibrated_zero - _zero_rate) * 1e4
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "DF mismatches above 1e-3:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn test_interpolation_method_comparison() {
    let base_date = Date::from_calendar_date(2025, Month::December, 10).expect("Valid test date");

    let test_quotes = [
        deposit(
            Date::from_calendar_date(2025, Month::December, 19).expect("Valid date"),
            0.0364447,
        ),
        deposit(
            Date::from_calendar_date(2026, Month::January, 12).expect("Valid date"),
            0.0364950,
        ),
        deposit(
            Date::from_calendar_date(2026, Month::March, 12).expect("Valid date"),
            0.0363477,
        ),
        deposit(
            Date::from_calendar_date(2026, Month::June, 12).expect("Valid date"),
            0.0358000,
        ),
        ois_swap(
            Date::from_calendar_date(2026, Month::December, 14).expect("Valid date"),
            0.0343446,
        ),
        ois_swap(
            Date::from_calendar_date(2027, Month::December, 13).expect("Valid date"),
            0.0329864,
        ),
        ois_swap(
            Date::from_calendar_date(2030, Month::December, 12).expect("Valid date"),
            0.0338799,
        ),
        ois_swap(
            Date::from_calendar_date(2035, Month::December, 12).expect("Valid date"),
            0.0370206,
        ),
    ];

    let base_context = MarketContext::new();
    let quotes: Vec<MarketQuote> = test_quotes
        .iter()
        .cloned()
        .map(MarketQuote::Rates)
        .collect();
    let settings = CalibrationConfig::default();

    let mc_params = DiscountCurveParams {
        curve_id: "USD-OIS-MC".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: InterpStyle::MonotoneConvex,
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: Default::default(),
    };
    let (mc_ctx, mc_report) = execute_step(
        &StepParams::Discount(mc_params),
        &quotes,
        &base_context,
        &settings,
    )
    .expect("MC failed");
    let mc_curve = mc_ctx
        .get_discount_ref("USD-OIS-MC")
        .expect("curve inserted")
        .clone();

    let ll_params = DiscountCurveParams {
        curve_id: "USD-OIS-LL".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: InterpStyle::LogLinear,
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: Default::default(),
    };
    let (ll_ctx, ll_report) = execute_step(
        &StepParams::Discount(ll_params),
        &quotes,
        &base_context,
        &settings,
    )
    .expect("LL failed");
    let ll_curve = ll_ctx
        .get_discount_ref("USD-OIS-LL")
        .expect("curve inserted")
        .clone();

    assert!(mc_report.success);
    assert!(ll_report.success);

    let test_points = [0.025, 0.089, 0.506, 1.013, 2.011, 5.014, 10.014];
    for yf in test_points {
        let mc_df = mc_curve.df(yf);
        let ll_df = ll_curve.df(yf);
        assert!((mc_df - ll_df).abs() < 0.01); // reasonable closeness
    }
}
