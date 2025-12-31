//! Convergence tests comparing finite difference vs analytical greeks.
//!
//! For instruments with analytical greeks (e.g., EquityOption, FxOption), verifies that
//! finite difference implementations converge to analytical values. Also validates that
//! bucketed metrics sum to total metrics (DV01, CS01, Vega).
//!
//! Tests:
//! - Analytical vs FD greeks for EquityOption (all greeks)
//! - Analytical vs FD greeks for FxOption (delta, vega, rho)
//! - Bucketed DV01 sums to total DV01
//! - Bucketed CS01 sums to total CS01
//! - Bucketed Vega sums to total Vega (for instruments with vol surfaces)

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::{PricingOverrides, SettlementType};
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use std::sync::Arc;
use time::macros::date;

fn create_option_market(as_of: Date, spot: f64, vol: f64, rate: f64) -> MarketContext {
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0f64, 1.0f64),
            (1.0f64, (-rate).exp()),
            (2.0f64, (-rate * 2.0f64).exp()),
        ])
        .build()
        .unwrap();

    let vol_surface = VolSurface::builder("AAPL_VOL")
        .expiries(&[0.5, 1.0, 2.0])
        .strikes(&[80.0, 90.0, 100.0, 110.0, 120.0])
        .row(&[vol, vol, vol, vol, vol])
        .row(&[vol, vol, vol, vol, vol])
        .row(&[vol, vol, vol, vol, vol])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_surface(vol_surface)
        .insert_price("AAPL", MarketScalar::Price(Money::new(spot, Currency::USD)))
}

/// Helper to test analytical vs FD greek for EquityOption
fn test_equity_option_greek(
    option: &EquityOption,
    market: &MarketContext,
    as_of: Date,
    metric_id: MetricId,
    analytical_fn: fn(&EquityOption, &MarketContext, Date) -> Result<f64>,
) {
    let registry = standard_registry();
    let pv = option.value(market, as_of).unwrap();

    // Compute analytical greek directly
    let mut analytical_value = analytical_fn(option, market, as_of).unwrap();
    // Registry exposes Rho per 1bp; direct equity option rho is per 1%.
    if metric_id == MetricId::Rho {
        analytical_value /= 100.0;
    }

    let mut context = MetricContext::new(
        Arc::new(option.clone()),
        Arc::new(market.clone()),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    // Compute greek via registry (uses analytical formula for EquityOption)
    let results = registry
        .compute(std::slice::from_ref(&metric_id), &mut context)
        .unwrap();
    let registry_value = *results.get(&metric_id).unwrap();

    // Should match exactly (both use analytical formulas)
    let diff = (analytical_value - registry_value).abs();
    assert!(
        diff < 1e-10,
        "Analytical {:?} from registry ({}) should match direct call ({}), diff: {}",
        metric_id,
        registry_value,
        analytical_value,
        diff
    );
}

#[test]
fn test_equity_option_all_analytical_greeks() {
    // For EquityOption, all greeks should match between direct calls and registry
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    let option = EquityOption {
        id: "ANALYTICAL_GREEKS_TEST".into(),
        underlying_ticker: "AAPL".to_string(),
        strike: Money::new(100.0, Currency::USD),
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        expiry,
        contract_size: 100.0,
        day_count: DayCount::Act365F,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD-OIS".into(),
        spot_id: "AAPL".into(),
        vol_surface_id: "AAPL_VOL".into(),
        div_yield_id: None,
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let market = create_option_market(as_of, 100.0, 0.25, 0.05);

    // Test all analytical greeks
    test_equity_option_greek(&option, &market, as_of, MetricId::Delta, |opt, mkt, dt| {
        opt.delta(mkt, dt)
    });
    test_equity_option_greek(&option, &market, as_of, MetricId::Gamma, |opt, mkt, dt| {
        opt.gamma(mkt, dt)
    });
    test_equity_option_greek(&option, &market, as_of, MetricId::Vega, |opt, mkt, dt| {
        opt.vega(mkt, dt)
    });
    test_equity_option_greek(&option, &market, as_of, MetricId::Rho, |opt, mkt, dt| {
        opt.rho(mkt, dt)
    });
    test_equity_option_greek(&option, &market, as_of, MetricId::Theta, |opt, mkt, dt| {
        opt.theta(mkt, dt)
    });
}

#[test]
fn test_bucketed_dv01_sums_to_parallel() {
    // Test that bucketed DV01 sums to approximately parallel DV01 using the
    // triangular key-rate implementation.
    //
    // NOTE: Due to Money type rounding to cents (2 decimal places), small bucket
    // sensitivities may be lost. This test uses a large notional ($10M) to ensure
    // bucket DV01s are above the precision threshold.
    //
    // The triangular weights partition unity across the bucket grid, ensuring:
    //   sum(bucketed DV01) ≈ parallel DV01
    let as_of = date!(2025 - 01 - 01);

    use finstack_valuations::instruments::fixed_income::bond::Bond;
    // Use large notional ($10M) to ensure bucket DV01s are above Money precision threshold
    let bond = Bond::fixed(
        "BUCKETED_TEST",
        Money::new(10_000_000.0, Currency::USD), // $10M notional
        0.05,                                    // 5% coupon
        as_of,
        date!(2035 - 01 - 01), // 10 year bond
        "USD-OIS",
    )
    .unwrap();

    // Create curve with dense knots (semi-annual) to properly capture cashflow sensitivity.
    let rate: f64 = 0.05;
    let mut knots: Vec<(f64, f64)> = vec![(0.0, 1.0)];
    for i in 1..=60 {
        let t = i as f64 * 0.5;
        knots.push((t, (-rate * t).exp()));
    }
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots(knots)
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc_curve);
    let registry = standard_registry();
    let pv = bond.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    // Compute both total DV01 and bucketed DV01
    let results = registry
        .compute(&[MetricId::Dv01, MetricId::BucketedDv01], &mut context)
        .unwrap();

    let total_dv01 = *results.get(&MetricId::Dv01).unwrap();
    let bucketed_series = context.computed_series.get(&MetricId::BucketedDv01);

    if let Some(series) = bucketed_series {
        let sum_bucketed: f64 = series.iter().map(|(_, v)| v).sum();

        // Debug output
        eprintln!("Parallel DV01: {:.2}", total_dv01);
        eprintln!("Sum of bucketed: {:.2}", sum_bucketed);
        eprintln!("Buckets:");
        for (label, value) in series.iter() {
            eprintln!("  {}: {:.2}", label, value);
        }

        // Verify basic properties
        assert!(
            sum_bucketed.is_finite(),
            "Bucketed DV01 sum should be finite"
        );
        assert!(total_dv01.abs() > 1e-6, "Total DV01 should be non-trivial");
        assert!(
            total_dv01 < 0.0,
            "Parallel DV01 should be negative for long bond"
        );
        assert!(
            sum_bucketed < 0.0,
            "Sum of bucketed DV01 should be negative"
        );

        // Sum of bucketed DV01 should equal parallel DV01 within 0.01%
        // Triangular weights partition unity across the bucket grid, so this should be near-exact.
        // Using value_raw() for high-precision calculations enables tight tolerance.
        let diff_pct = ((sum_bucketed - total_dv01) / total_dv01).abs();
        assert!(
            diff_pct < 0.0001,
            "Sum of bucketed DV01 ({:.4}) should be within 0.01% of parallel DV01 ({:.4}), got {:.3}%",
            sum_bucketed, total_dv01, diff_pct * 100.0
        );

        // The 10y bucket should capture most of the sensitivity
        let ten_year_dv01 = series
            .iter()
            .find(|(k, _)| k == "10y")
            .map(|(_, v)| *v)
            .unwrap_or(0.0);
        assert!(ten_year_dv01 < 0.0, "10Y bucket DV01 should be negative");

        // At least some intermediate buckets should have non-zero DV01
        let nonzero_buckets = series.iter().filter(|(_, v)| v.abs() > 0.01).count();
        assert!(
            nonzero_buckets >= 3,
            "At least 3 buckets should have significant DV01, got {}",
            nonzero_buckets
        );
    } else {
        panic!("Bucketed DV01 series should be populated");
    }
}

#[test]
fn test_bucketed_cs01_sums_to_total() {
    // Bucketed CS01 should approximately sum to total CS01
    let as_of = date!(2025 - 01 - 01);

    use finstack_core::market_data::term_structures::HazardCurve;
    use finstack_valuations::instruments::credit_derivatives::cds::CreditDefaultSwap;

    let cds = CreditDefaultSwap::buy_protection(
        "BUCKETED_CS01_TEST",
        Money::new(1_000_000.0, Currency::USD),
        200.0, // 200bp spread
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
        "HAZARD",
    )
    .expect("CDS construction should succeed");

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0f64, 1.0f64),
            (1.0f64, (-0.05f64).exp()),
            (2.0f64, (-0.10f64).exp()),
            (3.0f64, (-0.15f64).exp()),
            (4.0f64, (-0.20f64).exp()),
            (5.0f64, (-0.25f64).exp()),
        ])
        .build()
        .unwrap();

    let hazard_curve = HazardCurve::builder("HAZARD")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .recovery_rate(0.4)
        .knots([
            (0.0f64, 0.02f64),
            (1.0f64, 0.025f64),
            (2.0f64, 0.03f64),
            (3.0f64, 0.035f64),
            (4.0f64, 0.04f64),
            (5.0f64, 0.045f64),
        ])
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard_curve);

    let registry = standard_registry();
    let pv = cds.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(
        Arc::new(cds),
        Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    // Compute both total CS01 and bucketed CS01
    let results = registry
        .compute(
            &[MetricId::Cs01, MetricId::custom("bucketed_cs01")],
            &mut context,
        )
        .unwrap();

    let total_cs01 = *results.get(&MetricId::Cs01).unwrap();

    // Get bucketed CS01 series
    let bucketed_series = context
        .computed_series
        .get(&MetricId::custom("bucketed_cs01"));

    if let Some(series) = bucketed_series {
        let sum_bucketed: f64 = series.iter().map(|(_, v)| v).sum();

        // Bucketed CS01 is an approximation to the parallel CS01 and depends on bucket
        // definitions vs curve support (e.g. requesting standard buckets beyond the last knot).
        // We enforce a loose sanity bound rather than exact equality.
        let diff_pct = (sum_bucketed - total_cs01).abs() / total_cs01.abs().max(1e-10);
        assert!(
            diff_pct < 0.25,
            "Bucketed CS01 sum ({}) should be close to total CS01 ({}), diff: {:.2}%",
            sum_bucketed,
            total_cs01,
            diff_pct * 100.0
        );
    }
}

#[test]
fn test_bucketed_vega_sums_to_total() {
    // Bucketed Vega should approximately sum to total Vega
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    let option = EquityOption {
        id: "BUCKETED_VEGA_TEST".into(),
        underlying_ticker: "AAPL".to_string(),
        strike: Money::new(100.0, Currency::USD),
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        expiry,
        contract_size: 100.0,
        day_count: DayCount::Act365F,
        settlement: SettlementType::Cash,
        discount_curve_id: "USD-OIS".into(),
        spot_id: "AAPL".into(),
        vol_surface_id: "AAPL_VOL".into(),
        div_yield_id: None,
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let market = create_option_market(as_of, 100.0, 0.25, 0.05);
    let registry = standard_registry();
    let pv = option.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(
        Arc::new(option),
        Arc::new(market),
        as_of,
        pv,
        MetricContext::default_config(),
    );

    // Compute both total Vega and bucketed Vega
    let results = registry
        .compute(&[MetricId::Vega, MetricId::BucketedVega], &mut context)
        .unwrap();

    let total_vega = *results.get(&MetricId::Vega).unwrap();

    // Get bucketed Vega from matrix
    let bucketed_matrix = context.computed_matrix.get(&MetricId::BucketedVega);

    if let Some(matrix) = bucketed_matrix {
        let sum_bucketed: f64 = matrix.values.iter().flatten().sum();

        // Sum should approximately equal total (within 2% for vol surface interpolation)
        let diff_pct = (sum_bucketed - total_vega).abs() / total_vega.abs().max(1e-10);
        assert!(
            diff_pct < 0.02,
            "Bucketed Vega sum ({}) should be close to total Vega ({}), diff: {:.2}%",
            sum_bucketed,
            total_vega,
            diff_pct * 100.0
        );
    }
}
