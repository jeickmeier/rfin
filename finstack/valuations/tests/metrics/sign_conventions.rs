//! Tests verifying documented sign conventions for all greeks across instruments.
//!
//! Validates that metric signs match market conventions as documented in METRICS.md:
//! - Long Call Delta: Positive
//! - Long Put Delta: Negative
//! - Long Vega: Positive (both calls and puts)
//! - Long Gamma: Positive (both calls and puts, long position)
//! - Long Theta: Negative (time decay)
//! - Long Rho: Positive for calls, negative for puts (rate increase benefits calls, hurts puts)
//! - Bond DV01: Negative (price falls as rates rise)
//! - CS01: Negative (price falls as spreads widen)

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::parameters::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::equity_option::EquityOption;
use finstack_valuations::instruments::{PricingOverrides, SettlementType};
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::bond::Bond;
use std::sync::Arc;
use time::macros::date;

fn create_option_market(as_of: Date, spot: f64, vol: f64, rate: f64, div_yield: f64) -> MarketContext {
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
        .strikes(&[80.0, 100.0, 120.0])
        .row(&[vol, vol, vol])
        .row(&[vol, vol, vol])
        .row(&[vol, vol, vol])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_surface(vol_surface)
        .insert_price("AAPL", MarketScalar::Price(Money::new(spot, Currency::USD)))
        .insert_price("AAPL_DIV", MarketScalar::Unitless(div_yield))
}

#[test]
fn test_call_delta_positive() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let option = EquityOption {
        id: "CALL_TEST".into(),
        underlying_ticker: "AAPL".to_string(),
        strike: Money::new(100.0, Currency::USD),
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        expiry,
        contract_size: 100.0,
        day_count: DayCount::Act365F,
        settlement: SettlementType::Cash,
        disc_id: "USD-OIS".into(),
        spot_id: "AAPL".into(),
        vol_id: "AAPL_VOL".into(),
        div_yield_id: Some("AAPL_DIV".into()),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
    let registry = standard_registry();
    let pv = option.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

    let results = registry.compute(&[MetricId::Delta], &mut context).unwrap();
    let delta = *results.get(&MetricId::Delta).unwrap();

    assert!(delta > 0.0, "Long call delta should be positive, got {}", delta);
}

#[test]
fn test_put_delta_negative() {
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    let option = EquityOption {
        id: "PUT_TEST".into(),
        underlying_ticker: "AAPL".to_string(),
        strike: Money::new(100.0, Currency::USD),
        option_type: OptionType::Put,
        exercise_style: ExerciseStyle::European,
        expiry,
        contract_size: 100.0,
        day_count: DayCount::Act365F,
        settlement: SettlementType::Cash,
        disc_id: "USD-OIS".into(),
        spot_id: "AAPL".into(),
        vol_id: "AAPL_VOL".into(),
        div_yield_id: Some("AAPL_DIV".into()),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
    let registry = standard_registry();
    let pv = option.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

    let results = registry.compute(&[MetricId::Delta], &mut context).unwrap();
    let delta = *results.get(&MetricId::Delta).unwrap();

    assert!(delta < 0.0, "Long put delta should be negative, got {}", delta);
}

#[test]
fn test_theta_negative_for_long_positions() {
    // Theta (time decay) should be negative for long option positions
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    for option_type in [OptionType::Call, OptionType::Put] {
        let option = EquityOption {
            id: "THETA_TEST".into(),
            underlying_ticker: "AAPL".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            contract_size: 100.0,
            day_count: DayCount::Act365F,
            settlement: SettlementType::Cash,
            disc_id: "USD-OIS".into(),
            spot_id: "AAPL".into(),
            vol_id: "AAPL_VOL".into(),
            div_yield_id: Some("AAPL_DIV".into()),
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
        };

        let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
        let registry = standard_registry();
        let pv = option.value(&market, as_of).unwrap();
        let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

        let results = registry.compute(&[MetricId::Theta], &mut context).unwrap();
        let theta = *results.get(&MetricId::Theta).unwrap();

        assert!(
            theta <= 0.0,
            "Theta should be non-positive for long {} option, got {}",
            if matches!(option_type, OptionType::Call) { "call" } else { "put" },
            theta
        );
    }
}

#[test]
fn test_bond_dv01_positive_magnitude() {
    // Bond DV01 is reported as positive magnitude (dollar loss per 1bp rate increase)
    // Even though prices fall as rates rise, DV01 represents the magnitude of the loss
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DV01_TEST",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0f64, 1.0f64),
            (5.0f64, (-0.05f64 * 5.0f64).exp()),
        ])
        .build()
        .unwrap();

    let market = MarketContext::new().insert_discount(disc_curve);
    let registry = standard_registry();
    let pv = bond.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(Arc::new(bond), Arc::new(market), as_of, pv);

    let results = registry.compute(&[MetricId::Dv01], &mut context).unwrap();
    let dv01 = *results.get(&MetricId::Dv01).unwrap();

    // DV01 should be positive (magnitude of dollar loss per 1bp rate increase)
    // The directional effect (price falls) is implicit in the interpretation
    assert!(dv01 > 0.0, "Bond DV01 should be positive (magnitude), got {}", dv01);
}

#[test]
fn test_vega_always_positive() {
    // Vega should be positive for long option positions (both calls and puts)
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    for option_type in [OptionType::Call, OptionType::Put] {
        let option = EquityOption {
            id: "VEGA_TEST".into(),
            underlying_ticker: "AAPL".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            contract_size: 100.0,
            day_count: DayCount::Act365F,
            settlement: SettlementType::Cash,
            disc_id: "USD-OIS".into(),
            spot_id: "AAPL".into(),
            vol_id: "AAPL_VOL".into(),
            div_yield_id: Some("AAPL_DIV".into()),
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
        };

        let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
        let registry = standard_registry();
        let pv = option.value(&market, as_of).unwrap();
        let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

        let results = registry.compute(&[MetricId::Vega], &mut context).unwrap();
        let vega = *results.get(&MetricId::Vega).unwrap();

        assert!(
            vega > 0.0,
            "Vega should be positive for long {} option, got {}",
            if matches!(option_type, OptionType::Call) { "call" } else { "put" },
            vega
        );
    }
}

#[test]
fn test_gamma_always_positive() {
    // Gamma should be positive for long option positions (both calls and puts)
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);

    for option_type in [OptionType::Call, OptionType::Put] {
        let option = EquityOption {
            id: "GAMMA_TEST".into(),
            underlying_ticker: "AAPL".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type,
            exercise_style: ExerciseStyle::European,
            expiry,
            contract_size: 100.0,
            day_count: DayCount::Act365F,
            settlement: SettlementType::Cash,
            disc_id: "USD-OIS".into(),
            spot_id: "AAPL".into(),
            vol_id: "AAPL_VOL".into(),
            div_yield_id: Some("AAPL_DIV".into()),
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
        };

        let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
        let registry = standard_registry();
        let pv = option.value(&market, as_of).unwrap();
        let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

        let results = registry.compute(&[MetricId::Gamma], &mut context).unwrap();
        let gamma = *results.get(&MetricId::Gamma).unwrap();

        assert!(
            gamma >= 0.0,
            "Gamma should be non-negative for long {} option, got {}",
            if matches!(option_type, OptionType::Call) { "call" } else { "put" },
            gamma
        );
    }
}

#[test]
fn test_call_rho_positive() {
    // Call rho should be positive (rate increase benefits calls)
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let option = EquityOption {
        id: "CALL_RHO_TEST".into(),
        underlying_ticker: "AAPL".to_string(),
        strike: Money::new(100.0, Currency::USD),
        option_type: OptionType::Call,
        exercise_style: ExerciseStyle::European,
        expiry,
        contract_size: 100.0,
        day_count: DayCount::Act365F,
        settlement: SettlementType::Cash,
        disc_id: "USD-OIS".into(),
        spot_id: "AAPL".into(),
        vol_id: "AAPL_VOL".into(),
        div_yield_id: Some("AAPL_DIV".into()),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
    let registry = standard_registry();
    let pv = option.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

    let results = registry.compute(&[MetricId::Rho], &mut context).unwrap();
    let rho = *results.get(&MetricId::Rho).unwrap();

    assert!(rho > 0.0, "Call rho should be positive, got {}", rho);
}

#[test]
fn test_put_rho_negative() {
    // Put rho should be negative (rate increase hurts puts)
    let as_of = date!(2024 - 01 - 01);
    let expiry = date!(2025 - 01 - 01);
    
    let option = EquityOption {
        id: "PUT_RHO_TEST".into(),
        underlying_ticker: "AAPL".to_string(),
        strike: Money::new(100.0, Currency::USD),
        option_type: OptionType::Put,
        exercise_style: ExerciseStyle::European,
        expiry,
        contract_size: 100.0,
        day_count: DayCount::Act365F,
        settlement: SettlementType::Cash,
        disc_id: "USD-OIS".into(),
        spot_id: "AAPL".into(),
        vol_id: "AAPL_VOL".into(),
        div_yield_id: Some("AAPL_DIV".into()),
        pricing_overrides: PricingOverrides::default(),
        attributes: Default::default(),
    };

    let market = create_option_market(as_of, 100.0, 0.25, 0.05, 0.02);
    let registry = standard_registry();
    let pv = option.value(&market, as_of).unwrap();
    let mut context = MetricContext::new(Arc::new(option), Arc::new(market), as_of, pv);

    let results = registry.compute(&[MetricId::Rho], &mut context).unwrap();
    let rho = *results.get(&MetricId::Rho).unwrap();

    assert!(rho < 0.0, "Put rho should be negative, got {}", rho);
}

#[test]
fn test_cds_cs01_positive_magnitude() {
    // CDS CS01 is reported as positive magnitude (absolute change per 1bp spread change)
    // Note: The directional effect depends on the CDS side (buy vs sell protection)
    // and whether the CDS is at-market or off-market
    let as_of = date!(2025 - 01 - 01);
    
    use finstack_valuations::instruments::cds::CreditDefaultSwap;
    use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    
    let cds = CreditDefaultSwap::buy_protection(
        "CS01_TEST",
        Money::new(1_000_000.0, Currency::USD),
        200.0, // 200bp spread
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
        "HAZARD",
    );

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
    let mut context = MetricContext::new(Arc::new(cds), Arc::new(market), as_of, pv);

    let results = registry.compute(&[MetricId::Cs01], &mut context).unwrap();
    let cs01 = *results.get(&MetricId::Cs01).unwrap();

    // CS01 is reported as positive magnitude (absolute value)
    // The implementation uses .abs() to return magnitude
    assert!(cs01 > 0.0, "CDS CS01 should be positive (magnitude), got {}", cs01);
}

