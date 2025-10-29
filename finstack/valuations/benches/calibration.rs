//! Calibration benchmarks.
//!
//! Measures performance of critical calibration operations:
//! - Discount curve bootstrapping from deposits and swaps
//! - Forward curve calibration from FRAs and futures
//! - Complete market calibration with SimpleCalibration
//! - Volatility surface calibration
//!
//! Market Standards Review (Week 5)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use finstack_core::currency::Currency;
use finstack_core::dates::{add_months, Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve, Seniority};
use finstack_core::math::interp::InterpStyle;
use finstack_valuations::calibration::methods::swaption_vol::{
    AtmStrikeConvention, SwaptionVolCalibrator, SwaptionVolConvention,
};
use finstack_valuations::calibration::methods::{
    BaseCorrelationCalibrator, DiscountCurveCalibrator, ForwardCurveCalibrator,
    HazardCurveCalibrator, InflationCurveCalibrator, VolSurfaceCalibrator,
};
use finstack_valuations::calibration::{
    CalibrationConfig, Calibrator, CreditQuote, InflationQuote, MarketQuote, RatesQuote,
    SimpleCalibration, SolverKind, VolQuote,
};
use time::Month;

// ================================
// Helper Functions
// ================================

/// Create a basic market context with a discount curve
fn create_base_market() -> MarketContext {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.88),
            (10.0, 0.70),
            (30.0, 0.40),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    MarketContext::new().insert_discount(curve)
}

/// Create deposit quotes for discount curve calibration
fn create_deposit_quotes(base_date: Date, num_deposits: usize) -> Vec<RatesQuote> {
    let mut quotes = Vec::with_capacity(num_deposits);
    let base_rate = 0.045;

    for i in 1..=num_deposits {
        let maturity = add_months(base_date, i as i32);
        let rate = base_rate + (i as f64 * 0.0005); // Slight upward slope

        quotes.push(RatesQuote::Deposit {
            maturity,
            rate,
            day_count: DayCount::Act360,
        });
    }

    quotes
}

/// Create swap quotes for discount curve calibration
fn create_swap_quotes(base_date: Date, tenors: &[i32]) -> Vec<RatesQuote> {
    let mut quotes = Vec::with_capacity(tenors.len());
    let base_rate = 0.045;

    for &tenor_years in tenors {
        let maturity = add_months(base_date, tenor_years * 12);
        let rate = base_rate + (tenor_years as f64 * 0.002); // Upward sloping curve

        quotes.push(RatesQuote::Swap {
            maturity,
            rate,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            fixed_dc: DayCount::Thirty360,
            float_dc: DayCount::Act360,
            index: "USD-SOFR-3M".to_string(),
        });
    }

    quotes
}

/// Create FRA quotes for forward curve calibration
fn create_fra_quotes(base_date: Date, num_fras: usize) -> Vec<RatesQuote> {
    let mut quotes = Vec::with_capacity(num_fras);
    let base_rate = 0.047;

    for i in 1..=num_fras {
        let start = add_months(base_date, (i * 3) as i32);
        let end = add_months(start, 3);
        let rate = base_rate + (i as f64 * 0.0003);

        quotes.push(RatesQuote::FRA {
            start,
            end,
            rate,
            day_count: DayCount::Act360,
        });
    }

    quotes
}

/// Create CDS quotes for hazard curve calibration
fn create_cds_quotes(base_date: Date, tenors: &[i32]) -> Vec<CreditQuote> {
    let mut quotes = Vec::with_capacity(tenors.len());
    let base_spread = 150.0; // 150 bps

    for &tenor_years in tenors {
        let maturity = add_months(base_date, tenor_years * 12);
        let spread_bp = base_spread + (tenor_years as f64 * 10.0); // Upward sloping

        quotes.push(CreditQuote::CDS {
            entity: "CORP-A".to_string(),
            maturity,
            spread_bp,
            recovery_rate: 0.40,
            currency: Currency::USD,
        });
    }

    quotes
}

/// Create option volatility quotes
fn create_vol_quotes(base_date: Date, num_expiries: usize) -> Vec<VolQuote> {
    let mut quotes = Vec::with_capacity(num_expiries * 3);
    let strikes = [90.0, 100.0, 110.0]; // ATM and wings
    let base_vol = 0.25;

    for i in 1..=num_expiries {
        let expiry = add_months(base_date, (i * 3) as i32);

        for &strike in &strikes {
            let vol = base_vol + ((strike - 100.0_f64).abs() * 0.001); // Smile effect
            quotes.push(VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry,
                strike,
                vol,
                option_type: "Call".to_string(),
            });
        }
    }

    quotes
}

/// Create a complete market quote set for SimpleCalibration
fn create_complete_market_quotes(base_date: Date) -> Vec<MarketQuote> {
    let mut quotes = Vec::new();

    // Add deposits (1-12 months)
    for deposit in create_deposit_quotes(base_date, 12) {
        quotes.push(MarketQuote::Rates(deposit));
    }

    // Add swaps (2Y, 5Y, 10Y, 30Y)
    for swap in create_swap_quotes(base_date, &[2, 5, 10, 30]) {
        quotes.push(MarketQuote::Rates(swap));
    }

    // Add CDS quotes
    for cds in create_cds_quotes(base_date, &[1, 3, 5, 10]) {
        quotes.push(MarketQuote::Credit(cds));
    }

    // Add vol quotes
    for vol in create_vol_quotes(base_date, 4) {
        quotes.push(MarketQuote::Vol(vol));
    }

    quotes
}

/// Create CDS tranche quotes for base correlation calibration
fn create_tranche_quotes(base_date: Date, detachment_points: &[f64]) -> Vec<CreditQuote> {
    let mut quotes = Vec::with_capacity(detachment_points.len());
    let maturity = add_months(base_date, 5 * 12); // 5Y maturity
    let base_upfront = 10.0; // 10% base upfront

    for (i, &detachment) in detachment_points.iter().enumerate() {
        let attachment = if i == 0 {
            0.0
        } else {
            detachment_points[i - 1]
        };
        let upfront_pct = base_upfront + (detachment * 2.0); // Increasing upfront with detachment

        quotes.push(CreditQuote::CDSTranche {
            index: "CDX.NA.IG".to_string(),
            attachment,
            detachment,
            maturity,
            upfront_pct,
            running_spread_bp: 500.0,
        });
    }

    quotes
}

/// Create inflation swap quotes
fn create_inflation_quotes(base_date: Date, num_tenors: usize) -> Vec<InflationQuote> {
    let mut quotes = Vec::with_capacity(num_tenors);
    let base_rate = 0.025; // 2.5% base inflation

    for i in 1..=num_tenors {
        let maturity = add_months(base_date, (i * 12) as i32);
        let rate = base_rate - (i as f64 * 0.0002); // Slightly declining inflation

        quotes.push(InflationQuote::InflationSwap {
            maturity,
            rate,
            index: "US-CPI-U".to_string(),
        });
    }

    quotes
}

/// Create option volatility quotes for SABR surface calibration
fn create_sabr_vol_quotes(
    base_date: Date,
    num_expiries: usize,
    strikes_per_expiry: usize,
) -> Vec<VolQuote> {
    let mut quotes = Vec::with_capacity(num_expiries * strikes_per_expiry);
    let base_vol = 0.25;
    let atm_strike = 100.0;

    for i in 1..=num_expiries {
        let expiry = add_months(base_date, (i * 3) as i32);

        for j in 0..strikes_per_expiry {
            let strike_offset = (j as f64 - (strikes_per_expiry as f64 / 2.0)) * 5.0;
            let strike = atm_strike + strike_offset;
            let vol = base_vol + ((strike - atm_strike).abs() * 0.001); // Smile

            quotes.push(VolQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry,
                strike,
                vol,
                option_type: "Call".to_string(),
            });
        }
    }

    quotes
}

/// Create swaption volatility quotes
fn create_swaption_vol_quotes(base_date: Date, expiries: &[f64], tenors: &[f64]) -> Vec<VolQuote> {
    let mut quotes = Vec::with_capacity(expiries.len() * tenors.len() * 3); // 3 strikes per combo
    let base_vol = 0.50; // 50% normal vol (basis points)

    for &exp_years in expiries {
        let expiry_date = add_months(base_date, (exp_years * 12.0) as i32);

        for &ten_years in tenors {
            let tenor_date = add_months(expiry_date, (ten_years * 12.0) as i32);

            // ATM and wings
            for strike_offset in [-0.005_f64, 0.0, 0.005] {
                let strike = 0.04 + strike_offset; // 4% base rate
                let vol = base_vol + (strike_offset.abs() * 10.0); // Vol smile

                quotes.push(VolQuote::SwaptionVol {
                    expiry: expiry_date,
                    tenor: tenor_date,
                    strike,
                    vol,
                    quote_type: "ATM".to_string(),
                });
            }
        }
    }

    quotes
}

/// Create market context with credit index for base correlation
fn create_market_with_credit_index(base_date: Date) -> MarketContext {
    use finstack_core::market_data::term_structures::{BaseCorrelationCurve, CreditIndexData};
    use std::sync::Arc;

    // Discount curve
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.88), (10.0, 0.70)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    // Index hazard curve
    let hazard_curve = HazardCurve::builder("CDX.NA.IG")
        .base_date(base_date)
        .recovery_rate(0.40)
        .knots(vec![(1.0, 0.01), (5.0, 0.02), (10.0, 0.025)])
        .par_spreads(vec![(1.0, 60.0), (5.0, 100.0), (10.0, 140.0)])
        .build()
        .unwrap();

    // Dummy base correlation curve
    let base_corr = BaseCorrelationCurve::builder("CDX.NA.IG")
        .points(vec![(3.0, 0.25), (30.0, 0.60)])
        .build()
        .unwrap();

    let index_data = CreditIndexData::builder()
        .num_constituents(125)
        .recovery_rate(0.40)
        .index_credit_curve(Arc::new(hazard_curve))
        .base_correlation_curve(Arc::new(base_corr))
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_credit_index("CDX.NA.IG", index_data)
}

// ================================
// Benchmark Functions
// ================================

fn bench_discount_curve_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("discount_curve_small");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Small curve: 6 deposits + 2 swaps
    let mut quotes = create_deposit_quotes(base_date, 6);
    quotes.extend(create_swap_quotes(base_date, &[2, 5]));

    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex);

    let base_context = MarketContext::new();

    group.bench_function("8_instruments", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&base_context))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_discount_curve_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("discount_curve_medium");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Medium curve: 12 deposits + 4 swaps
    let mut quotes = create_deposit_quotes(base_date, 12);
    quotes.extend(create_swap_quotes(base_date, &[2, 5, 10, 30]));

    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex);

    let base_context = MarketContext::new();

    group.bench_function("16_instruments", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&base_context))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_discount_curve_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("discount_curve_large");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Large curve: 11 deposits (up to 11 months) + 11 swaps (1Y+)
    let mut quotes = create_deposit_quotes(base_date, 11);
    quotes.extend(create_swap_quotes(
        base_date,
        &[1, 2, 3, 4, 5, 6, 7, 10, 15, 20, 30],
    ));

    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex);

    let base_context = MarketContext::new();

    group.bench_function("22_instruments", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&base_context))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_discount_curve_interpolation(c: &mut Criterion) {
    let mut group = c.benchmark_group("discount_curve_interp");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let mut quotes = create_deposit_quotes(base_date, 12);
    quotes.extend(create_swap_quotes(base_date, &[2, 5, 10, 30]));

    let base_context = MarketContext::new();

    for interp in [
        InterpStyle::Linear,
        InterpStyle::MonotoneConvex,
        InterpStyle::CubicHermite,
    ] {
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
            .with_solve_interp(interp);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", interp)),
            &interp,
            |b, _| {
                b.iter(|| {
                    calibrator
                        .calibrate(black_box(&quotes), black_box(&base_context))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_forward_curve(c: &mut Criterion) {
    let mut group = c.benchmark_group("forward_curve");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create base market with discount curve
    let market = create_base_market();

    for num_fras in [4, 8, 16] {
        let quotes = create_fra_quotes(base_date, num_fras);

        let calibrator = ForwardCurveCalibrator::new(
            "USD-SOFR-3M-FWD",
            0.25,
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_fras", num_fras)),
            &num_fras,
            |b, _| {
                b.iter(|| {
                    calibrator
                        .calibrate(black_box(&quotes), black_box(&market))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_hazard_curve(c: &mut Criterion) {
    let mut group = c.benchmark_group("hazard_curve");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create base market with discount curve
    let market = create_base_market();

    for tenors in [&[1, 3, 5][..], &[1, 2, 3, 5, 7, 10][..]] {
        let quotes = create_cds_quotes(base_date, tenors);

        let calibrator = HazardCurveCalibrator::new(
            "CORP-A",
            Seniority::Senior,
            0.40, // 40% recovery rate
            base_date,
            Currency::USD,
            "USD-OIS",
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_tenors", tenors.len())),
            &tenors.len(),
            |b, _| {
                b.iter(|| {
                    calibrator
                        .calibrate(black_box(&quotes), black_box(&market))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_simple_calibration_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_calibration_small");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Small market: minimal quotes
    let mut quotes = Vec::new();
    for deposit in create_deposit_quotes(base_date, 6) {
        quotes.push(MarketQuote::Rates(deposit));
    }
    for swap in create_swap_quotes(base_date, &[2, 5]) {
        quotes.push(MarketQuote::Rates(swap));
    }

    let calibration = SimpleCalibration::new(base_date, Currency::USD);

    group.bench_function("minimal_market", |b| {
        b.iter(|| calibration.calibrate(black_box(&quotes)).unwrap());
    });

    group.finish();
}

fn bench_simple_calibration_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_calibration_medium");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Medium market: rates + credit
    let mut quotes = Vec::new();
    for deposit in create_deposit_quotes(base_date, 12) {
        quotes.push(MarketQuote::Rates(deposit));
    }
    for swap in create_swap_quotes(base_date, &[2, 5, 10, 30]) {
        quotes.push(MarketQuote::Rates(swap));
    }
    for cds in create_cds_quotes(base_date, &[1, 3, 5, 10]) {
        quotes.push(MarketQuote::Credit(cds));
    }

    let calibration = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("CORP-A", Seniority::Senior);

    group.bench_function("rates_and_credit", |b| {
        b.iter(|| calibration.calibrate(black_box(&quotes)).unwrap());
    });

    group.finish();
}

fn bench_simple_calibration_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_calibration_full");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Full market: rates + credit + vol
    let quotes = create_complete_market_quotes(base_date);

    let calibration = SimpleCalibration::new(base_date, Currency::USD)
        .with_entity_seniority("CORP-A", Seniority::Senior);

    group.bench_function("complete_market", |b| {
        b.iter(|| calibration.calibrate(black_box(&quotes)).unwrap());
    });

    group.finish();
}

fn bench_calibration_solver_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("calibration_solver");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let mut quotes = create_deposit_quotes(base_date, 12);
    quotes.extend(create_swap_quotes(base_date, &[2, 5, 10, 30]));

    let base_context = MarketContext::new();

    for solver in [SolverKind::Newton, SolverKind::Brent, SolverKind::Hybrid] {
        let config = CalibrationConfig {
            solver_kind: solver.clone(),
            tolerance: 1e-8,
            ..CalibrationConfig::default()
        };

        let calibrator =
            DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD).with_config(config);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{:?}", solver)),
            &solver,
            |b, _| {
                b.iter(|| {
                    calibrator
                        .calibrate(black_box(&quotes), black_box(&base_context))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_base_correlation_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("base_correlation_small");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Small: 3 tranches (standard equity tranches)
    let quotes = create_tranche_quotes(base_date, &[3.0, 7.0, 10.0]);
    let market = create_market_with_credit_index(base_date);

    let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG", 42, 5.0, base_date);

    group.bench_function("3_tranches", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_base_correlation_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("base_correlation_full");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Full: 5 standard tranches
    let quotes = create_tranche_quotes(base_date, &[3.0, 7.0, 10.0, 15.0, 30.0]);
    let market = create_market_with_credit_index(base_date);

    let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG", 42, 5.0, base_date);

    group.bench_function("5_tranches", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_inflation_curve(c: &mut Criterion) {
    let mut group = c.benchmark_group("inflation_curve");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Create market with discount curve
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.88), (10.0, 0.70)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc_curve);

    for num_tenors in [3, 5, 10] {
        let quotes = create_inflation_quotes(base_date, num_tenors);

        let calibrator = InflationCurveCalibrator::new(
            "US-CPI-U",
            base_date,
            Currency::USD,
            290.0, // Base CPI
            "USD-OIS",
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_tenors", num_tenors)),
            &num_tenors,
            |b, _| {
                b.iter(|| {
                    calibrator
                        .calibrate(black_box(&quotes), black_box(&market))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_sabr_surface_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("sabr_surface_small");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Small: 2 expiries × 5 strikes
    let quotes = create_sabr_vol_quotes(base_date, 2, 5);

    let calibrator = VolSurfaceCalibrator::new(
        "SPY-VOL",
        1.0, // Lognormal beta for equity
        vec![0.25, 0.5],
        vec![90.0, 95.0, 100.0, 105.0, 110.0],
    )
    .with_base_date(base_date);

    // Create market context
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.78)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_price(
            "SPY",
            finstack_core::market_data::scalars::MarketScalar::Unitless(100.0),
        )
        .insert_price(
            "SPY-DIVYIELD",
            finstack_core::market_data::scalars::MarketScalar::Unitless(0.02),
        );

    group.bench_function("2exp_5strikes", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_sabr_surface_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("sabr_surface_medium");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Medium: 4 expiries × 7 strikes
    let quotes = create_sabr_vol_quotes(base_date, 4, 7);

    let calibrator = VolSurfaceCalibrator::new(
        "SPY-VOL",
        1.0,
        vec![0.25, 0.5, 1.0, 2.0],
        vec![80.0, 90.0, 95.0, 100.0, 105.0, 110.0, 120.0],
    )
    .with_base_date(base_date);

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([(0.0, 1.0), (5.0, 0.78)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_price(
            "SPY",
            finstack_core::market_data::scalars::MarketScalar::Unitless(100.0),
        )
        .insert_price(
            "SPY-DIVYIELD",
            finstack_core::market_data::scalars::MarketScalar::Unitless(0.02),
        );

    group.bench_function("4exp_7strikes", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_swaption_vol_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("swaption_vol_small");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Small: 2 expiries × 2 tenors
    let quotes = create_swaption_vol_quotes(base_date, &[0.25, 0.5], &[2.0, 5.0]);

    let calibrator = SwaptionVolCalibrator::new(
        "USD-SWAPTION-VOL",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    );

    // Create market with discount curve
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.88),
            (10.0, 0.70),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc_curve);

    group.bench_function("2exp_2ten", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_swaption_vol_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("swaption_vol_medium");
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Medium: 3 expiries × 3 tenors
    let quotes = create_swaption_vol_quotes(base_date, &[0.25, 0.5, 1.0], &[2.0, 5.0, 10.0]);

    let calibrator = SwaptionVolCalibrator::new(
        "USD-SWAPTION-VOL",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        "USD-OIS",
        Currency::USD,
    );

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots([
            (0.0, 1.0),
            (1.0, 0.98),
            (2.0, 0.96),
            (5.0, 0.88),
            (10.0, 0.70),
        ])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc_curve);

    group.bench_function("3exp_3ten", |b| {
        b.iter(|| {
            calibrator
                .calibrate(black_box(&quotes), black_box(&market))
                .unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_discount_curve_small,
    bench_discount_curve_medium,
    bench_discount_curve_large,
    bench_discount_curve_interpolation,
    bench_forward_curve,
    bench_hazard_curve,
    bench_simple_calibration_small,
    bench_simple_calibration_medium,
    bench_simple_calibration_full,
    bench_calibration_solver_comparison,
    bench_base_correlation_small,
    bench_base_correlation_full,
    bench_inflation_curve,
    bench_sabr_surface_small,
    bench_sabr_surface_medium,
    bench_swaption_vol_small,
    bench_swaption_vol_medium,
);
criterion_main!(benches);
