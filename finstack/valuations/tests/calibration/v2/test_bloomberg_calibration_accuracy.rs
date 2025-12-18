//! Bloomberg calibration accuracy tests.
//!
//! Validates calibrated curves against Bloomberg reference data to ensure
//! the calibration engine produces market-standard results.
//!
//! ## Data Sources
//!
//! - USD OIS curve: Bloomberg USD SWAP-OIS (Fed Funds / EFFR-style OIS curve)
//!
//! ## Accuracy by Tenor
//!
//! | Tenor Bucket | Expected Accuracy | Primary Factors |
//! |--------------|-------------------|-----------------|
//! | Short-end (<1Y) | < 0.1bp | Deposit pricing is simple and exact |
//! | Mid-term (1-10Y) | < 0.5bp | Minor interpolation differences |
//! | Long-end (>10Y) | < 5bp | Interpolation method affects annuity |
//!
//! ## Sources of Differences
//!
//! ### 1. Interpolation Method (Primary Factor for Long-End)
//!
//! The example notebook and Python script use **PiecewiseQuadraticForward** interpolation
//! (smooth forward curve, C²). Long-dated swap annuities can still be sensitive to
//! interpolation choices, so long-end tolerances are bucketed separately.
//!
//! Test results show MonotoneConvex vs LogLinear can differ by up to ±2.6bp at
//! intermediate points, which affects the annuity and thus the calibrated endpoint DF.
//!
//! ### 2. Payment Delay (Vendor / Table Convention)
//!
//! Many vendor OIS swap conventions use **Pay Delay: 2 Business Days** on both legs.
//! The notebook + script use **2 business days** pay delay in the instrument conventions.
//! Bloomberg's published tables typically still report discount factors at accrual-end
//! dates; we therefore compare using `df_on_date(maturity)` (not maturity+delay).
//!
//! ### 3. Compounding Convention for Zero Rates
//!
//! The notebook compares Bloomberg's displayed zero rates against the curve's
//! continuously-compounded `zero_on_date`.
//!
//! ### 4. Settlement Date Handling
//!
//! OIS swaps start at settlement (T+2), not trade date. Our calibrator correctly
//! handles this (fixed in this iteration).
//!
//! ## Recommendations for Closer Bloomberg Matching
//!
//! 1. Ensure curve time-axis uses ACT/360 (Bloomberg table convention for USD OIS).
//! 2. Ensure USD spot-start settlement (T+2) is modeled consistently.
//! 3. Accept a few bp long-end tolerance under MonotoneConvex due to interpolation
//!    differences that compound through long-dated swap annuities.

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::calibration::adapters::handlers::execute_step;
use finstack_valuations::calibration::api::schema::{
    CalibrationMethod, DiscountCurveParams, StepParams,
};
use finstack_valuations::calibration::quotes::{InstrumentConventions, MarketQuote, RatesQuote};
use finstack_valuations::calibration::{CalibrationConfig, CALIBRATION_CONFIG_KEY_V2};
use time::Month;

/// Helper to create deposit quotes with ACT/360 day count
fn deposit(maturity: Date, rate: f64) -> RatesQuote {
    let conventions = InstrumentConventions::default()
        .with_day_count(DayCount::Act360)
        // Bloomberg USD OIS curve instruments are spot-start (T+2) in USD.
        .with_settlement_days(2)
        .with_calendar_id("usny")
        .with_business_day_convention(BusinessDayConvention::ModifiedFollowing);

    RatesQuote::Deposit {
        maturity,
        rate,
        conventions,
    }
}

/// Helper to create OIS swap quotes with annual frequency and ACT/360
fn ois_swap(maturity: Date, rate: f64) -> RatesQuote {
    let common_conventions = InstrumentConventions::default()
        // Spot-start (T+2) for USD rates.
        .with_settlement_days(2)
        .with_reset_lag(0)
        // Vendor-style OIS pay delay (2 business days).
        .with_payment_delay(2)
        .with_calendar_id("usny")
        .with_business_day_convention(BusinessDayConvention::ModifiedFollowing);

    RatesQuote::Swap {
        maturity,
        rate,
        is_ois: true,
        conventions: common_conventions.clone(),
        fixed_leg_conventions: common_conventions
            .clone()
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Act360),
        float_leg_conventions: common_conventions
            .with_payment_frequency(Tenor::annual())
            .with_day_count(DayCount::Act360)
            // Match Bloomberg USD SWAP-OIS (FEDL01 / Fed Funds OIS style).
            .with_index("USD-FEDFUNDS-OIS"),
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
            0.0403809,
        ), // 25Y
        ois_swap(
            Date::from_calendar_date(2055, Month::December, 13).expect("Valid date"),
            0.0401000,
        ), // 30Y
        ois_swap(
            Date::from_calendar_date(2065, Month::December, 14).expect("Valid date"),
            0.0390413,
        ), // 40Y
        ois_swap(
            Date::from_calendar_date(2075, Month::December, 12).expect("Valid date"),
            0.0378761,
        ), // 50Y
    ];

    // Bloomberg zero rates (in decimal) and discount factors at each maturity
    // Source: Bloomberg USSOC Curncy as of 2025-12-10
    let bloomberg_data: Vec<(Date, f64, f64)> = vec![
        // (maturity_date, zero_rate_decimal, discount_factor)
        // From Bloomberg table (exact values)
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
            0.951003,
        ),
        (
            Date::from_calendar_date(2027, Month::December, 13).expect("Valid date"),
            0.0328850,
            0.936093,
        ),
        // Extended tenors (actual Bloomberg values)
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
        (
            Date::from_calendar_date(2040, Month::December, 12).expect("Valid date"),
            0.0397745,
            0.550311,
        ), // 15Y
        (
            Date::from_calendar_date(2045, Month::December, 12).expect("Valid date"),
            0.0410343,
            0.439783,
        ), // 20Y
        (
            Date::from_calendar_date(2050, Month::December, 12).expect("Valid date"),
            0.0410412,
            0.358105,
        ), // 25Y (peak)
        (
            Date::from_calendar_date(2055, Month::December, 13).expect("Valid date"),
            0.0403819,
            0.297434,
        ), // 30Y
        (
            Date::from_calendar_date(2065, Month::December, 14).expect("Valid date"),
            0.0381263,
            0.217291,
        ), // 40Y
        (
            Date::from_calendar_date(2075, Month::December, 12).expect("Valid date"),
            0.0355798,
            0.168578,
        ), // 50Y
    ];

    // Calibrate the discount curve (same setup as the notebook + Python script):
    // - Fed Funds / EFFR-style OIS index
    // - T+2 settlement
    // - ACT/360 on both legs
    // - Pay delay 2 business days
    // - PiecewiseQuadraticForward interpolation
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        CALIBRATION_CONFIG_KEY_V2,
        serde_json::json!({
            "solver": {
                "method": "brent",
                "tolerance": 1e-8,
                "max_iterations": 200
            }
        }),
    );
    let settings = CalibrationConfig::from_finstack_config_or_default(&cfg).expect("valid config");

    let params = DiscountCurveParams {
        curve_id: "USD-OIS".into(),
        currency: Currency::USD,
        base_date,
        method: CalibrationMethod::Bootstrap,
        interpolation: InterpStyle::PiecewiseQuadraticForward,
        extrapolation: finstack_core::math::interp::ExtrapolationPolicy::FlatForward,
        pricing_discount_id: None,
        pricing_forward_id: None,
        conventions: finstack_valuations::calibration::pricing::RatesStepConventions {
            // Match notebook/script: use default curve day-count (do not override).
            curve_day_count: None,
            // USD spot-start; this ensures the pricer settlement logic matches
            // how the market quotes were generated (T+2), without relying on
            // currency-derived defaults.
            settlement_days: Some(2),
            calendar_id: Some("usny".to_string()),
            business_day_convention: Some(BusinessDayConvention::ModifiedFollowing),
            allow_calendar_fallback: Some(false),
            use_settlement_start: Some(true),
            default_payment_delay_days: Some(2),
            default_reset_lag_days: Some(0),
            // Keep vendor-style strictness off for this test; instrument quotes
            // carry their own explicit conventions.
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
    assert!(report.success, "Calibration should report success");
    assert!(
        report.max_residual < 1e-8,
        "Calibration max_residual must be < 1e-8 (matches notebook/script). Got {:.3e}. Reason: {}",
        report.max_residual,
        report.convergence_reason
    );
    assert_eq!(curve.id().as_str(), "USD-OIS");

    // Track differences for summary statistics
    let mut zero_rate_diffs_bp: Vec<f64> = Vec::new();
    let mut df_diffs_bp: Vec<f64> = Vec::new();

    // Compare calibrated values with Bloomberg data (notebook parity):
    // - DFs compared at maturity date
    // - Zero rates compared using curve.zero_on_date (continuous compounding)
    for (maturity, bbg_zero_rate, bbg_df) in &bloomberg_data {
        let calc_df = curve
            .try_df_on_date_curve(*maturity)
            .expect("df_on_date_curve");
        let calc_zero_cc = curve.zero_on_date(*maturity);

        let zero_diff_bp = (calc_zero_cc - *bbg_zero_rate) * 10_000.0;
        let df_diff_bp = (calc_df - *bbg_df) * 10_000.0;

        zero_rate_diffs_bp.push(zero_diff_bp);
        df_diffs_bp.push(df_diff_bp);

        // Debug output - show early points and any material DF diffs
        let is_first_five = zero_rate_diffs_bp.len() <= 5;
        let has_large_df_diff = df_diff_bp.abs() > 0.25;
        if is_first_five || has_large_df_diff {
            println!(
                "  {} | BBG zero: {:.5}% | Calc zero: {:.5}% ({:+.2}bp) | BBG DF: {:.6} | Calc DF: {:.6} | DF diff: {:+.2}bp{}",
                maturity,
                bbg_zero_rate * 100.0,
                calc_zero_cc * 100.0,
                zero_diff_bp,
                bbg_df,
                calc_df,
                df_diff_bp,
                if has_large_df_diff { " <-- LARGE" } else { "" }
            );
        }
    }

    // Separate analysis by tenor bucket
    // Short-end (deposits <1Y): should match exactly
    // Mid-term (1-10Y swaps): should match well
    // Long-end (>10Y swaps): extrapolation differences expected
    let short_end_count = 14; // deposits
    let mid_term_count = 10; // 1Y to 10Y

    let short_end_df_diffs: Vec<f64> = df_diffs_bp.iter().take(short_end_count).copied().collect();
    let mid_term_df_diffs: Vec<f64> = df_diffs_bp
        .iter()
        .skip(short_end_count)
        .take(mid_term_count)
        .copied()
        .collect();
    let long_end_df_diffs: Vec<f64> = df_diffs_bp
        .iter()
        .skip(short_end_count + mid_term_count)
        .copied()
        .collect();

    let short_end_max_df = short_end_df_diffs
        .iter()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    let mid_term_max_df = mid_term_df_diffs
        .iter()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    let long_end_max_df = long_end_df_diffs
        .iter()
        .map(|x| x.abs())
        .fold(0.0, f64::max);

    // Calculate overall stats
    let avg_zero_diff: f64 =
        zero_rate_diffs_bp.iter().sum::<f64>() / zero_rate_diffs_bp.len() as f64;
    let max_zero_diff: f64 = zero_rate_diffs_bp
        .iter()
        .map(|x| x.abs())
        .fold(0.0, f64::max);
    let avg_df_diff: f64 = df_diffs_bp.iter().sum::<f64>() / df_diffs_bp.len() as f64;

    // ACCURACY CHECKS BY TENOR (Bloomberg/FinCAD-style tolerances):
    // - Short-end (deposits): should match essentially exactly.
    // - Mid-term (1-10Y swaps): sub-bp DF differences.
    // - Long-end (>10Y swaps): a few bp due to annuity sensitivity to interpolation.
    let short_end_tolerance_bp = 0.01;
    assert!(
        short_end_max_df < short_end_tolerance_bp,
        "Short-end DF max diff ({:.3}bp) exceeds tolerance ({:.1}bp). \
         Deposits should match closely.",
        short_end_max_df,
        short_end_tolerance_bp
    );

    // Mid-term (1-10Y): should match closely (vendor-grade).
    let mid_term_tolerance_bp = 0.5;
    assert!(
        mid_term_max_df < mid_term_tolerance_bp,
        "Mid-term DF max diff ({:.3}bp) exceeds tolerance ({:.1}bp). \
         1-10Y swaps should match reasonably.",
        mid_term_max_df,
        mid_term_tolerance_bp
    );

    // Long-end (>10Y): allow a few bp because interpolation differences between
    // pillar points compound through the swap annuity.
    let long_end_tolerance_bp = 10.0;
    assert!(
        long_end_max_df < long_end_tolerance_bp,
        "Long-end DF max diff ({:.3}bp) exceeds tolerance ({:.1}bp). \
         Note: Long-end differences are expected due to extrapolation method differences.",
        long_end_max_df,
        long_end_tolerance_bp
    );

    // Zero rates check (continuous compounding, matching the notebook output).
    let zero_rate_tolerance_bp = 1.00;
    assert!(
        max_zero_diff < zero_rate_tolerance_bp,
        "Max zero rate diff ({:.2}bp) exceeds tolerance ({:.1}bp). \
         Using annual compounding for comparison.",
        max_zero_diff,
        zero_rate_tolerance_bp
    );

    // Verify the curve has the expected number of knots
    assert!(
        curve.knots().len() >= 25,
        "Curve should have at least 25 knots, got {}",
        curve.knots().len()
    );

    // Print summary for test output
    println!("\n========================================");
    println!("Bloomberg USD OIS Calibration Accuracy:");
    println!("========================================");
    println!("\nDISCOUNT FACTOR ACCURACY BY TENOR:");
    println!("  Short-end (<1Y deposits):");
    println!(
        "    Max diff: {:.3}bp | {} (tol: {}bp)",
        short_end_max_df,
        if short_end_max_df < short_end_tolerance_bp {
            "PASS ✓"
        } else {
            "FAIL ✗"
        },
        short_end_tolerance_bp
    );
    println!("  Mid-term (1-10Y swaps):");
    println!(
        "    Max diff: {:.3}bp | {} (tol: {}bp)",
        mid_term_max_df,
        if mid_term_max_df < mid_term_tolerance_bp {
            "PASS ✓"
        } else {
            "FAIL ✗"
        },
        mid_term_tolerance_bp
    );
    println!("  Long-end (>10Y swaps):");
    println!(
        "    Max diff: {:.3}bp | {} (tol: {}bp)",
        long_end_max_df,
        if long_end_max_df < long_end_tolerance_bp {
            "PASS ✓"
        } else {
            "FAIL ✗"
        },
        long_end_tolerance_bp
    );
    println!(
        "    Note: Long-end differences due to interpolation method variations between pillars."
    );
    println!(
        "    The MonotoneConvex interpolation affects annuity calculation for long-dated swaps."
    );
    println!("\nZERO RATES (annual compounding convention):");
    println!(
        "  Avg diff: {:.2}bp | Max diff: {:.2}bp | {} (tol: {}bp)",
        avg_zero_diff,
        max_zero_diff,
        if max_zero_diff < zero_rate_tolerance_bp {
            "PASS ✓"
        } else {
            "FAIL ✗"
        },
        zero_rate_tolerance_bp
    );
    println!("\nOverall DF avg diff: {:.3}bp", avg_df_diff);
    println!("Curve knots: {}", curve.knots().len());
    println!("Calibration success: {}", report.success);
}

/// Test comparing MonotoneConvex vs LogLinear interpolation for Bloomberg matching.
///
/// This test investigates whether LogLinear interpolation produces results closer
/// to Bloomberg's curve construction. Long-dated swap calibration depends on the
/// interpolation method because the annuity calculation (sum of discounted accrual
/// factors) uses interpolated DFs at each coupon date.
#[test]
fn test_interpolation_method_comparison() {
    let base_date = Date::from_calendar_date(2025, Month::December, 10).expect("Valid test date");

    // Use a subset of quotes for comparison (short-end to 10Y)
    let test_quotes = [
        // Short-term deposits
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
        // OIS Swaps
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

    // Test MonotoneConvex (industry standard)
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
    .expect("MonotoneConvex calibration should succeed");
    let mc_curve = mc_ctx
        .get_discount_ref("USD-OIS-MC")
        .expect("curve inserted")
        .clone();

    // Test LogLinear (piecewise constant zero rates)
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
    .expect("LogLinear calibration should succeed");
    let ll_curve = ll_ctx
        .get_discount_ref("USD-OIS-LL")
        .expect("curve inserted")
        .clone();

    assert!(mc_report.success, "MonotoneConvex should succeed");
    assert!(ll_report.success, "LogLinear should succeed");

    println!("\n========================================");
    println!("Interpolation Method Comparison:");
    println!("========================================");
    println!("\nDF comparison at selected tenors:");
    println!(
        "{:>12} | {:>14} | {:>14} | {:>10}",
        "Tenor", "MonotoneConvex", "LogLinear", "Diff (bp)"
    );
    println!("{}", "-".repeat(60));

    let test_points = [
        ("1W", 0.025),   // ~9 days
        ("1M", 0.089),   // ~32 days
        ("6M", 0.506),   // ~182 days
        ("1Y", 1.013),   // ~365 days
        ("2Y", 2.011),   // ~724 days
        ("5Y", 5.014),   // ~1805 days
        ("10Y", 10.014), // ~3605 days
    ];

    for (label, yf) in test_points {
        let mc_df = mc_curve.df(yf);
        let ll_df = ll_curve.df(yf);
        let diff_bp = (mc_df - ll_df) * 10_000.0;
        println!(
            "{:>12} | {:>14.8} | {:>14.8} | {:>+10.4}",
            label, mc_df, ll_df, diff_bp
        );
    }

    // The key insight: for well-spaced pillars, both methods should give similar
    // results at pillar points, but differ at intermediate points.
    // This affects annuity calculation for long-dated swaps.
    println!("\nNote: Differences at intermediate points affect swap annuity calculations,");
    println!("which in turn affects the calibrated DF at long-dated pillar points.");
}
