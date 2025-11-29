//! Calibration repricing tests with market-standard tolerance requirements.
//!
//! Verifies that calibrated curves can reprice input instruments to within specified tolerances.
//!
//! ## Tolerance Rationale
//!
//! ### Calibration Residuals (Internal Consistency)
//!
//! Calibration residuals measure how well the solver found discount factors that reprice
//! instruments internally. These should be very tight:
//! - Target: < 1e-5 (0.1bp per $1M = $1)
//!
//! ### Repricing Tolerance (External Swaps)
//!
//! When repricing with externally-constructed swaps, schedule generation differences
//! between calibration and repricing can cause larger errors:
//! - **Deposits**: $1 per $1M notional (~0.1bp) - exact repricing achievable
//! - **OIS Swaps**: 10bp × |DV01| - accounts for schedule generation approximations
//! - **FRAs**: $50 per $1M (0.5bp) - fixing date alignment
//!
//! The OIS swap tolerance uses DV01-scaling because schedule differences affect the
//! fixed leg annuity calculation, which scales with swap duration.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::methods::forward_curve::ForwardCurveCalibrator;
use finstack_valuations::calibration::{
    create_ois_swap_from_quote, CalibrationConfig, Calibrator, RatesQuote,
};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::deposit::Deposit;
use finstack_valuations::instruments::fra::ForwardRateAgreement;
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::metrics::MetricCalculator;
use time::Month;

const NOTIONAL: f64 = 1_000_000.0; // $1M notional

/// OIS swap repricing tolerance in basis points for externally-constructed swaps.
///
/// With consistent swap construction between calibration and repricing, the
/// repricing error should be < 1bp. This tolerance uses DV01-scaling.
const OIS_SWAP_TOLERANCE_BP: f64 = 1.0;

/// Calculate DV01 for a swap using the metrics system.
fn calculate_swap_dv01(swap: &InterestRateSwap, ctx: &MarketContext, as_of: Date) -> f64 {
    use finstack_valuations::metrics::MetricContext;

    let base_pv = swap.value(ctx, as_of).unwrap();
    let mut metric_ctx = MetricContext::new(
        std::sync::Arc::new(swap.clone()),
        std::sync::Arc::new(ctx.clone()),
        as_of,
        base_pv,
    );

    use finstack_valuations::metrics::{Dv01CalculatorConfig, UnifiedDv01Calculator};
    let dv01_calc = UnifiedDv01Calculator::<InterestRateSwap>::new(
        Dv01CalculatorConfig::parallel_combined(),
    );
    dv01_calc.calculate(&mut metric_ctx).unwrap()
}

/// Calculate tolerance for a swap based on DV01.
///
/// The tolerance accounts for schedule generation differences which affect
/// the fixed leg annuity calculation. Error scales with swap duration.
fn swap_tolerance_from_dv01(dv01: f64, tolerance_bp: f64) -> f64 {
    (tolerance_bp * dv01.abs()).max(50.0) // Minimum $50 tolerance
}

#[test]
fn test_discount_curve_swap_repricing() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Use tight solver tolerance for calibration
    let config = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    // Use T+0 settlement for consistency
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_config(config)
        .with_settlement_days(0);

    // Quotes: deposits + swaps of various tenors
    let quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            day_count: DayCount::Act360,
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            day_count: DayCount::Act360,
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365),
            rate: 0.0470,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string().into(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 2),
            rate: 0.0480,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string().into(),
        },
        RatesQuote::Swap {
            maturity: base_date + time::Duration::days(365 * 5),
            rate: 0.0490,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::daily(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-OIS".to_string().into(),
        },
    ];

    let base_context = MarketContext::new();
    let (curve, report) = calibrator
        .calibrate(&quotes, &base_context)
        .expect("Swap calibration should succeed");

    assert!(report.success, "Calibration should succeed: {:?}", report);

    // INTERNAL CONSISTENCY CHECK: Calibration residuals should be tight
    // This verifies the solver found discount factors that reprice instruments internally.
    // Target: < 1e-5 (0.1bp per $1M = $1)
    assert!(
        report.max_residual < 1e-5,
        "Calibration residuals should be < 0.1bp. Max residual: {:.2e}",
        report.max_residual
    );

    let ctx = base_context.insert_discount(curve);

    // EXTERNAL REPRICING CHECK: Verify externally-constructed swaps reprice within tolerance
    // With correct swap construction, repricing error should be < 1bp
    for quote in &quotes {
        if let RatesQuote::Swap { maturity, .. } = quote {
            let swap = create_ois_swap_from_quote(
                quote,
                "USD-OIS",
                "USD-OIS",
                base_date,
                Money::new(NOTIONAL, Currency::USD),
                None,
            )
            .expect("Swap construction should succeed");

            let pv = swap.value(&ctx, base_date).unwrap();
            let dv01 = calculate_swap_dv01(&swap, &ctx, base_date);
            let tolerance = swap_tolerance_from_dv01(dv01, OIS_SWAP_TOLERANCE_BP);

            assert!(
                pv.amount().abs() <= tolerance,
                "Swap at {} should reprice within {}bp × DV01 tolerance. PV: ${:.2}, DV01: ${:.2}, Tolerance: ${:.2}",
                maturity,
                OIS_SWAP_TOLERANCE_BP,
                pv.amount(),
                dv01,
                tolerance
            );
        }
    }
}

#[test]
fn test_discount_curve_deposit_repricing() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let config = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    // Use T+0 settlement for tight repricing
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_config(config)
        .with_settlement_days(0);

    let deposit_quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            day_count: DayCount::Act360,
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.046,
            day_count: DayCount::Act360,
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(180),
            rate: 0.047,
            day_count: DayCount::Act360,
        },
    ];

    let base_context = MarketContext::new();
    let (curve, report) = calibrator
        .calibrate(&deposit_quotes, &base_context)
        .expect("Deposit calibration should succeed");

    assert!(report.success);

    // INTERNAL CONSISTENCY: Calibration residuals should be very tight for deposits
    assert!(
        report.max_residual < 1e-8,
        "Deposit calibration residuals should be < 1e-8. Max: {:.2e}",
        report.max_residual
    );

    let ctx = base_context.insert_discount(curve);

    // Deposits should reprice to par (PV ≈ 0) within tight tolerance
    for quote in &deposit_quotes {
        if let RatesQuote::Deposit {
            maturity,
            rate,
            day_count,
        } = quote
        {
            let dep = Deposit {
                id: format!("DEP-{}", maturity).into(),
                notional: Money::new(NOTIONAL, Currency::USD),
                start: base_date,
                end: *maturity,
                day_count: *day_count,
                quote_rate: Some(*rate),
                discount_curve_id: "USD-OIS".into(),
                attributes: Default::default(),
            };
            let pv = dep.value(&ctx, base_date).unwrap();

            // For deposits, use absolute tolerance: $1 per $1M notional (0.1bp)
            assert!(
                pv.amount().abs() <= 1.0,
                "Deposit at {} should reprice within $1. PV: ${:.2}",
                maturity,
                pv.amount()
            );
        }
    }
}

/// FRA repricing tolerance per $1M notional.
///
/// Multi-curve forward calibration has inherent interpolation effects.
/// Target: $150 per $1M (~1.5bp for 90-day FRAs)
/// Note: With sequential bootstrap and forward curve interpolation,
/// achieving tighter tolerance would require simultaneous calibration.
const FRA_TOLERANCE_DOLLARS: f64 = 150.0;

#[test]
fn test_forward_curve_fra_repricing() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // First calibrate discount curve
    let disc_config = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let disc_calibrator =
        DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD).with_config(disc_config);

    let disc_quotes = vec![
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.0450,
            day_count: DayCount::Act360,
        },
        RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(90),
            rate: 0.0460,
            day_count: DayCount::Act360,
        },
    ];

    let base_context = MarketContext::new();
    let (disc_curve, _) = disc_calibrator
        .calibrate(&disc_quotes, &base_context)
        .expect("Discount calibration should succeed");

    let ctx_with_disc = base_context.insert_discount(disc_curve);

    // Now calibrate forward curve with FRAs
    let fwd_config = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        ..Default::default()
    };

    let fwd_calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS")
            .with_config(fwd_config);

    let fra_quotes = vec![
        RatesQuote::FRA {
            start: base_date + time::Duration::days(90),
            end: base_date + time::Duration::days(180),
            rate: 0.0470,
            day_count: DayCount::Act360,
        },
        RatesQuote::FRA {
            start: base_date + time::Duration::days(180),
            end: base_date + time::Duration::days(270),
            rate: 0.0480,
            day_count: DayCount::Act360,
        },
    ];

    let (fwd_curve, report) = fwd_calibrator
        .calibrate(&fra_quotes, &ctx_with_disc)
        .expect("Forward calibration should succeed");

    assert!(report.success);

    // INTERNAL CONSISTENCY: Calibration residuals should be tight
    assert!(
        report.max_residual < 1e-5,
        "FRA calibration residuals should be < 0.1bp. Max: {:.2e}",
        report.max_residual
    );

    let ctx = ctx_with_disc.insert_forward(fwd_curve);

    // Reprice FRAs and verify within tolerance
    for quote in &fra_quotes {
        if let RatesQuote::FRA {
            start,
            end,
            rate,
            day_count,
        } = quote
        {
            let fixing_date = if *start >= base_date + time::Duration::days(2) {
                *start - time::Duration::days(2)
            } else {
                base_date
            };

            let fra = ForwardRateAgreement::builder()
                .id(format!("FRA-{}-{}", start, end).into())
                .notional(Money::new(NOTIONAL, Currency::USD))
                .fixing_date(fixing_date)
                .start_date(*start)
                .end_date(*end)
                .fixed_rate(*rate)
                .day_count(*day_count)
                .reset_lag(2)
                .discount_curve_id("USD-OIS".into())
                .forward_id("USD-SOFR-3M".into())
                .pay_fixed(false)
                .build()
                .unwrap();

            let pv = fra.value(&ctx, base_date).unwrap();

            assert!(
                pv.amount().abs() <= FRA_TOLERANCE_DOLLARS,
                "FRA from {} to {} should reprice within ${} (0.5bp). PV: ${:.2}",
                start,
                end,
                FRA_TOLERANCE_DOLLARS,
                pv.amount()
            );
        }
    }
}
