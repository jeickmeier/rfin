//! Comprehensive serialization tests for the calibration framework.
//!
//! This test suite verifies that all calibration types can be:
//! - Serialized to JSON
//! - Deserialized from JSON
//! - Round-tripped without data loss

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::CurveId;
use finstack_valuations::calibration::methods::base_correlation::{
    BaseCorrelationCalibrator, BaseCorrelationSurfaceCalibrator, CorrelationInterp,
};
use finstack_valuations::calibration::methods::convexity::ConvexityParameters;
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::methods::forward_curve::ForwardCurveCalibrator;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::methods::inflation_curve::InflationCurveCalibrator;
use finstack_valuations::calibration::methods::sabr_surface::{
    SurfaceInterp, VolSurfaceCalibrator,
};
use finstack_valuations::calibration::methods::swaption_market_conventions::{
    PaymentEstimation, SwaptionMarketConvention,
};
use finstack_valuations::calibration::methods::swaption_vol::{
    AtmStrikeConvention, SwaptionVolCalibrator, SwaptionVolConvention,
};
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationReport, CreditQuote, FutureSpecs, InflationQuote, MarketQuote,
    MultiCurveConfig, RatesQuote, SABRMarketData, SABRModelParams, SimpleCalibration, SolverKind,
    ValidationConfig, ValidationError, VolQuote,
};
use std::collections::BTreeMap;
use time::Month;

/// Helper function to perform JSON roundtrip serialization test
fn roundtrip_json<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string_pretty(value).expect("Failed to serialize to JSON");
    println!("JSON representation:\n{}\n", json);
    serde_json::from_str(&json).expect("Failed to deserialize from JSON")
}

#[test]
fn test_solver_kind_serialization() {
    let kinds = vec![
        SolverKind::Newton,
        SolverKind::Brent,
        SolverKind::Hybrid,
        SolverKind::LevenbergMarquardt,
        SolverKind::DifferentialEvolution,
    ];

    for kind in kinds {
        let restored = roundtrip_json(&kind);
        assert_eq!(format!("{:?}", kind), format!("{:?}", restored));
    }
}

#[test]
fn test_calibration_config_serialization() {
    let config = CalibrationConfig {
        tolerance: 1e-10,
        max_iterations: 100,
        use_parallel: false,
        random_seed: Some(42),
        verbose: true,
        solver_kind: SolverKind::Hybrid,
        entity_seniority: {
            let mut map = hashbrown::HashMap::new();
            map.insert("AAPL".to_string(), Seniority::Senior);
            map.insert("TSLA".to_string(), Seniority::Subordinated);
            map
        },
        multi_curve: MultiCurveConfig {
            calibrate_basis: true,
            enforce_separation: true,
        },
        use_fd_sabr_gradients: false,
        explain: finstack_core::explain::ExplainOpts::default(),
        progress: finstack_core::progress::ProgressReporter::default(),
    };

    let restored = roundtrip_json(&config);
    assert_eq!(config.tolerance, restored.tolerance);
    assert_eq!(config.max_iterations, restored.max_iterations);
    assert_eq!(config.use_parallel, restored.use_parallel);
    assert_eq!(config.random_seed, restored.random_seed);
    assert_eq!(config.verbose, restored.verbose);
}

#[test]
fn test_multi_curve_config_serialization() {
    let config = MultiCurveConfig {
        calibrate_basis: true,
        enforce_separation: false,
    };

    let restored = roundtrip_json(&config);
    assert_eq!(config.calibrate_basis, restored.calibrate_basis);
    assert_eq!(config.enforce_separation, restored.enforce_separation);
}

#[test]
fn test_rates_quote_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Deposit quote
    let deposit = RatesQuote::Deposit {
        maturity: base_date + time::Duration::days(90),
        rate: 0.045,
        day_count: DayCount::Act360,
    };
    let _ = roundtrip_json(&deposit);

    // FRA quote
    let fra = RatesQuote::FRA {
        start: base_date + time::Duration::days(90),
        end: base_date + time::Duration::days(180),
        rate: 0.047,
        day_count: DayCount::Act360,
    };
    let _ = roundtrip_json(&fra);

    // Future quote
    let future = RatesQuote::Future {
        expiry: base_date + time::Duration::days(90),
        price: 99.25,
        specs: FutureSpecs {
            multiplier: 1.0,
            face_value: 1_000_000.0,
            delivery_months: 3,
            day_count: DayCount::Act360,
            convexity_adjustment: Some(0.0001),
        },
    };
    let _ = roundtrip_json(&future);

    // Swap quote
    let swap = RatesQuote::Swap {
        maturity: base_date + time::Duration::days(365 * 2),
        rate: 0.048,
        fixed_freq: Frequency::semi_annual(),
        float_freq: Frequency::quarterly(),
        fixed_dc: DayCount::Thirty360,
        float_dc: DayCount::Act360,
        index: "USD-SOFR-3M".to_string(),
    };
    let _ = roundtrip_json(&swap);

    // Basis swap quote
    let basis = RatesQuote::BasisSwap {
        maturity: base_date + time::Duration::days(365 * 5),
        primary_index: "3M-SOFR".to_string(),
        reference_index: "6M-SOFR".to_string(),
        spread_bp: 5.0,
        primary_freq: Frequency::quarterly(),
        reference_freq: Frequency::semi_annual(),
        primary_dc: DayCount::Act360,
        reference_dc: DayCount::Act360,
        currency: Currency::USD,
    };
    let _ = roundtrip_json(&basis);
}

#[test]
fn test_credit_quote_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // CDS quote
    let cds = CreditQuote::CDS {
        entity: "AAPL".to_string(),
        maturity: base_date + time::Duration::days(365 * 5),
        spread_bp: 50.0,
        recovery_rate: 0.40,
        currency: Currency::USD,
    };
    let _ = roundtrip_json(&cds);

    // CDS upfront quote
    let cds_upfront = CreditQuote::CDSUpfront {
        entity: "DISTRESSED".to_string(),
        maturity: base_date + time::Duration::days(365),
        upfront_pct: 5.0,
        running_spread_bp: 300.0,
        recovery_rate: 0.25,
        currency: Currency::USD,
    };
    let _ = roundtrip_json(&cds_upfront);

    // CDS tranche quote
    let tranche = CreditQuote::CDSTranche {
        index: "CDX.NA.IG.42".to_string(),
        attachment: 0.0,
        detachment: 3.0,
        maturity: base_date + time::Duration::days(365 * 5),
        upfront_pct: 2.5,
        running_spread_bp: 500.0,
    };
    let _ = roundtrip_json(&tranche);
}

#[test]
fn test_vol_quote_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Option vol quote
    let option_vol = VolQuote::OptionVol {
        underlying: "SPY".to_string(),
        expiry: base_date + time::Duration::days(90),
        strike: 450.0,
        vol: 0.20,
        option_type: "Call".to_string(),
    };
    let _ = roundtrip_json(&option_vol);

    // Swaption vol quote
    let swaption_vol = VolQuote::SwaptionVol {
        expiry: base_date + time::Duration::days(365),
        tenor: base_date + time::Duration::days(365 * 5),
        strike: 0.045,
        vol: 0.50,
        quote_type: "ATM".to_string(),
    };
    let _ = roundtrip_json(&swaption_vol);
}

#[test]
fn test_inflation_quote_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Zero-coupon inflation swap
    let zc_swap = InflationQuote::InflationSwap {
        maturity: base_date + time::Duration::days(365 * 10),
        rate: 0.025,
        index: "USCPI".to_string(),
    };
    let _ = roundtrip_json(&zc_swap);

    // Year-on-year inflation swap
    let yoy_swap = InflationQuote::YoYInflationSwap {
        maturity: base_date + time::Duration::days(365 * 5),
        rate: 0.023,
        index: "USCPI".to_string(),
        frequency: Frequency::annual(),
    };
    let _ = roundtrip_json(&yoy_swap);
}

#[test]
fn test_market_quote_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let quotes = vec![
        MarketQuote::Rates(RatesQuote::Deposit {
            maturity: base_date + time::Duration::days(30),
            rate: 0.045,
            day_count: DayCount::Act360,
        }),
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "AAPL".to_string(),
            maturity: base_date + time::Duration::days(365 * 5),
            spread_bp: 50.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
        }),
        MarketQuote::Vol(VolQuote::OptionVol {
            underlying: "SPY".to_string(),
            expiry: base_date + time::Duration::days(90),
            strike: 450.0,
            vol: 0.20,
            option_type: "Call".to_string(),
        }),
        MarketQuote::Inflation(InflationQuote::InflationSwap {
            maturity: base_date + time::Duration::days(365 * 10),
            rate: 0.025,
            index: "USCPI".to_string(),
        }),
    ];

    for quote in quotes {
        let _ = roundtrip_json(&quote);
    }
}

#[test]
fn test_calibration_report_serialization() {
    let mut residuals = BTreeMap::new();
    residuals.insert("DEP-1M".to_string(), 1e-8);
    residuals.insert("DEP-3M".to_string(), 2e-8);
    residuals.insert("SWAP-2Y".to_string(), 3e-8);

    let mut metadata = BTreeMap::new();
    metadata.insert("curve_id".to_string(), "USD-OIS".to_string());
    metadata.insert("interpolation".to_string(), "MonotoneConvex".to_string());

    let report = CalibrationReport {
        success: true,
        residuals,
        iterations: 15,
        objective_value: 1.5e-15,
        max_residual: 3e-8,
        rmse: 2.1e-8,
        convergence_reason: "Tolerance met".to_string(),
        metadata,
        results_meta: finstack_core::config::results_meta(
            &finstack_core::config::FinstackConfig::default(),
        ),
        explanation: None,
    };

    let restored = roundtrip_json(&report);
    assert_eq!(report.success, restored.success);
    assert_eq!(report.iterations, restored.iterations);
    assert_eq!(report.convergence_reason, restored.convergence_reason);
    assert_eq!(report.residuals.len(), restored.residuals.len());
}

#[test]
fn test_validation_error_serialization() {
    let mut values = BTreeMap::new();
    values.insert("df_t1".to_string(), 0.95);
    values.insert("df_t2".to_string(), 0.94);

    let error = ValidationError {
        constraint: "monotonicity".to_string(),
        location: "t=1.5".to_string(),
        details: "Discount factor increased".to_string(),
        values,
    };

    let restored = roundtrip_json(&error);
    assert_eq!(error.constraint, restored.constraint);
    assert_eq!(error.location, restored.location);
    assert_eq!(error.details, restored.details);
}

#[test]
fn test_validation_config_serialization() {
    let config = ValidationConfig {
        check_forward_positivity: true,
        min_forward_rate: -0.02,
        max_forward_rate: 0.50,
        check_monotonicity: true,
        check_arbitrage: true,
        tolerance: 1e-10,
    };

    let restored = roundtrip_json(&config);
    assert_eq!(
        config.check_forward_positivity,
        restored.check_forward_positivity
    );
    assert_eq!(config.min_forward_rate, restored.min_forward_rate);
    assert_eq!(config.max_forward_rate, restored.max_forward_rate);
}

#[test]
fn test_discount_curve_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex)
        .with_config(CalibrationConfig::default());

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.curve_id, restored.curve_id);
    assert_eq!(calibrator.base_date, restored.base_date);
    assert_eq!(calibrator.currency, restored.currency);
}

#[test]
fn test_forward_curve_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS")
            .with_solve_interp(InterpStyle::Linear)
            .with_time_dc(DayCount::Act360);

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.fwd_curve_id, restored.fwd_curve_id);
    assert_eq!(calibrator.tenor_years, restored.tenor_years);
    assert_eq!(calibrator.base_date, restored.base_date);
    assert_eq!(calibrator.currency, restored.currency);
}

#[test]
fn test_hazard_curve_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = HazardCurveCalibrator::new(
        "AAPL",
        Seniority::Senior,
        0.40,
        base_date,
        Currency::USD,
        "USD-OIS",
    );

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.entity, restored.entity);
    assert_eq!(calibrator.seniority, restored.seniority);
    assert_eq!(calibrator.recovery_rate, restored.recovery_rate);
    assert_eq!(calibrator.base_date, restored.base_date);
}

#[test]
fn test_inflation_curve_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator =
        InflationCurveCalibrator::new("USCPI", base_date, Currency::USD, 280.0, "USD-OIS");

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.curve_id, restored.curve_id);
    assert_eq!(calibrator.base_date, restored.base_date);
    assert_eq!(calibrator.currency, restored.currency);
    assert_eq!(calibrator.base_cpi, restored.base_cpi);
}

#[test]
fn test_base_correlation_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = BaseCorrelationCalibrator::new("CDX.NA.IG.42", 42, 5.0, base_date)
        .with_detachment_points(vec![3.0, 7.0, 10.0, 15.0, 30.0])
        .with_corr_interp(CorrelationInterp::Linear);

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.index_id, restored.index_id);
    assert_eq!(calibrator.series, restored.series);
    assert_eq!(calibrator.maturity_years, restored.maturity_years);
    assert_eq!(calibrator.base_date, restored.base_date);
}

#[test]
fn test_base_correlation_surface_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = BaseCorrelationSurfaceCalibrator::new(
        "CDX.NA.IG.42",
        42,
        base_date,
        vec![3.0, 5.0, 7.0, 10.0],
    );

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.index_id, restored.index_id);
    assert_eq!(calibrator.series, restored.series);
    assert_eq!(calibrator.base_date, restored.base_date);
    assert_eq!(calibrator.target_maturities, restored.target_maturities);
}

#[test]
fn test_vol_surface_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = VolSurfaceCalibrator::new(
        "SPY-VOL",
        1.0, // Lognormal beta for equity
        vec![0.25, 0.5, 1.0, 2.0],
        vec![80.0, 90.0, 100.0, 110.0, 120.0],
    )
    .with_base_date(base_date)
    .with_base_currency(Currency::USD)
    .with_surface_interp(SurfaceInterp::Bilinear);

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.surface_id, restored.surface_id);
    assert_eq!(calibrator.beta, restored.beta);
    assert_eq!(calibrator.base_date, restored.base_date);
    assert_eq!(calibrator.base_currency, restored.base_currency);
}

#[test]
fn test_swaption_market_convention_serialization() {
    let conventions = vec![
        SwaptionMarketConvention::usd(),
        SwaptionMarketConvention::eur(),
        SwaptionMarketConvention::gbp(),
        SwaptionMarketConvention::jpy(),
        SwaptionMarketConvention::chf(),
    ];

    for convention in conventions {
        let restored = roundtrip_json(&convention);
        assert_eq!(convention.day_count, restored.day_count);
        assert_eq!(
            format!("{:?}", convention.payment_estimation),
            format!("{:?}", restored.payment_estimation)
        );
    }
}

#[test]
fn test_swaption_vol_calibrator_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibrator = SwaptionVolCalibrator::new(
        "USD-SWAPTION-VOL",
        SwaptionVolConvention::Normal,
        AtmStrikeConvention::SwapRate,
        base_date,
        CurveId::from("USD-OIS"),
        Currency::USD,
    );

    let restored = roundtrip_json(&calibrator);
    assert_eq!(calibrator.surface_id, restored.surface_id);
    assert_eq!(calibrator.base_date, restored.base_date);
    assert_eq!(calibrator.currency, restored.currency);
}

#[test]
fn test_simple_calibration_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let calibration = SimpleCalibration::new(base_date, Currency::USD)
        .with_config(CalibrationConfig::default())
        .with_entity_seniority("AAPL", Seniority::Senior)
        .with_entity_seniority("TSLA", Seniority::Subordinated);

    let _ = roundtrip_json(&calibration);
}

#[test]
fn test_convexity_parameters_serialization() {
    let params = vec![
        ConvexityParameters::usd_sofr(),
        ConvexityParameters::eur_euribor(),
        ConvexityParameters::gbp_sonia(),
        ConvexityParameters::jpy_tonar(),
    ];

    for param in params {
        let restored = roundtrip_json(&param);
        assert_eq!(param.base_volatility, restored.base_volatility);
        assert_eq!(param.mean_reversion, restored.mean_reversion);
        assert_eq!(param.use_ho_lee, restored.use_ho_lee);
    }
}

#[test]
fn test_sabr_market_data_serialization() {
    let market_data = SABRMarketData {
        forward: 100.0,
        time_to_expiry: 1.0,
        strikes: vec![90.0, 95.0, 100.0, 105.0, 110.0],
        market_vols: vec![0.22, 0.21, 0.20, 0.21, 0.22],
        beta: 0.5,
    };

    let restored = roundtrip_json(&market_data);
    assert_eq!(market_data.forward, restored.forward);
    assert_eq!(market_data.time_to_expiry, restored.time_to_expiry);
    assert_eq!(market_data.strikes.len(), restored.strikes.len());
    assert_eq!(market_data.market_vols.len(), restored.market_vols.len());
    assert_eq!(market_data.beta, restored.beta);
}

#[test]
fn test_sabr_model_params_serialization() {
    let params = SABRModelParams::new(0.15, 0.30, -0.1, 0.5);

    let restored = roundtrip_json(&params);
    assert_eq!(params.alpha, restored.alpha);
    assert_eq!(params.nu, restored.nu);
    assert_eq!(params.rho, restored.rho);
    assert_eq!(params.beta, restored.beta);
}

#[test]
fn test_enums_serialization() {
    // Test CorrelationInterp
    let _ = roundtrip_json(&CorrelationInterp::Linear);

    // Test SurfaceInterp
    let _ = roundtrip_json(&SurfaceInterp::Bilinear);

    // Test PaymentEstimation
    let _ = roundtrip_json(&PaymentEstimation::ProperSchedule);

    // Test SwaptionVolConvention
    let _ = roundtrip_json(&SwaptionVolConvention::Normal);
    let _ = roundtrip_json(&SwaptionVolConvention::Lognormal);
    let _ = roundtrip_json(&SwaptionVolConvention::ShiftedLognormal { shift: 0.01 });

    // Test AtmStrikeConvention
    let _ = roundtrip_json(&AtmStrikeConvention::SwapRate);
    let _ = roundtrip_json(&AtmStrikeConvention::ParRate);
}

#[test]
fn test_complex_calibration_workflow_serialization() {
    // Create a complex calibration configuration
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let config = CalibrationConfig {
        tolerance: 1e-12,
        max_iterations: 200,
        use_parallel: true,
        random_seed: Some(12345),
        verbose: true,
        solver_kind: SolverKind::LevenbergMarquardt,
        entity_seniority: {
            let mut map = hashbrown::HashMap::new();
            map.insert("AAPL".to_string(), Seniority::Senior);
            map.insert("TSLA".to_string(), Seniority::Subordinated);
            map.insert("GOOG".to_string(), Seniority::SeniorSecured);
            map
        },
        multi_curve: MultiCurveConfig {
            calibrate_basis: true,
            enforce_separation: true,
        },
        use_fd_sabr_gradients: true,
        explain: finstack_core::explain::ExplainOpts::default(),
        progress: finstack_core::progress::ProgressReporter::default(),
    };

    // Create multiple calibrators with this config
    let discount_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex)
        .with_config(config.clone());

    let forward_calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS")
            .with_solve_interp(InterpStyle::Linear)
            .with_config(config.clone());

    let hazard_calibrator = HazardCurveCalibrator::new(
        "AAPL",
        Seniority::Senior,
        0.40,
        base_date,
        Currency::USD,
        "USD-OIS",
    )
    .with_config(config.clone());

    // Serialize each and verify
    let _ = roundtrip_json(&discount_calibrator);
    let _ = roundtrip_json(&forward_calibrator);
    let _ = roundtrip_json(&hazard_calibrator);

    println!("✓ All calibrators successfully serialized and deserialized");
}
