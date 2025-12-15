//! Comprehensive serialization tests for the calibration framework.
//!
//! This test suite verifies that all calibration types can be:
//! - Serialized to JSON
//! - Deserialized from JSON
//! - Round-tripped without data loss

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::term_structures::Seniority;
use finstack_core::math::interp::InterpStyle;
use finstack_core::types::CurveId;
use finstack_valuations::calibration::methods::base_correlation::{
    BaseCorrelationCalibrator, BaseCorrelationSurfaceCalibrator, CorrelationInterp,
};
use finstack_valuations::calibration::pricing::ConvexityParameters;
use finstack_valuations::calibration::methods::discount::DiscountCurveCalibrator;
use finstack_valuations::calibration::methods::forward_curve::ForwardCurveCalibrator;
use finstack_valuations::calibration::methods::hazard_curve::HazardCurveCalibrator;
use finstack_valuations::calibration::methods::inflation_curve::InflationCurveCalibrator;
use finstack_valuations::calibration::methods::sabr_surface::{
    SurfaceInterp, VolSurfaceCalibrator,
};
use finstack_valuations::calibration::quotes::InstrumentConventions;
use finstack_valuations::calibration::methods::swaption_vol::{
    AtmStrikeConvention, PaymentEstimation, SwaptionMarketConvention, SwaptionVolCalibrator,
    SwaptionVolConvention,
};
use finstack_valuations::calibration::{
    CalibrationConfig, CalibrationReport, CreditQuote, FutureSpecs, InflationQuote, MarketQuote,
    MultiCurveConfig, RatesQuote, SABRMarketData, SolverKind, ValidationConfig, VolQuote,
};
use finstack_valuations::instruments::common::models::SABRParameters;
use std::collections::BTreeMap;
use std::path::PathBuf;
use time::Month;

fn maybe_print_json(json: &str) {
    if std::env::var("FINSTACK_TEST_LOG_JSON").is_ok() {
        println!("JSON representation:\n{}\n", json);
    }
}

/// Helper function to perform JSON roundtrip serialization test
fn roundtrip_json<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string_pretty(value).expect("Failed to serialize to JSON");
    maybe_print_json(&json);
    serde_json::from_str(&json).expect("Failed to deserialize from JSON")
}

fn json_example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("calibration")
        .join("json_examples")
        .join(name)
}

fn read_json_example(name: &str) -> String {
    std::fs::read_to_string(json_example_path(name))
        .unwrap_or_else(|e| panic!("Failed to read example file {name}: {e}"))
}

fn assert_envelope_roundtrip(example_file: &str) {
    // Load pipeline example
    let json = read_json_example(example_file);

    // Parse envelope
    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    assert_eq!(envelope.schema, "finstack.calibration/1");

    // Serialize back
    let reserialized =
        serde_json::to_string_pretty(&envelope).expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope =
        serde_json::from_str(&reserialized).expect("Failed to re-parse envelope");
}

#[test]
fn test_solver_kind_serialization() {
    let kinds = vec![
        SolverKind::Newton,
        SolverKind::Brent,
        SolverKind::Brent,
        SolverKind::LevenbergMarquardt,
    ];

    for kind in kinds {
        let restored = roundtrip_json(&kind);
        assert_eq!(kind, restored);
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
        solver_kind: SolverKind::Brent,
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
        ..CalibrationConfig::default()
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
        conventions: InstrumentConventions::default()
            .with_day_count(DayCount::Act360),
    };
    let _ = roundtrip_json(&deposit);

    // FRA quote
    let fra = RatesQuote::FRA {
        start: base_date + time::Duration::days(90),
        end: base_date + time::Duration::days(180),
        rate: 0.047,
        conventions: InstrumentConventions::default()
            .with_day_count(DayCount::Act360),
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
            ..Default::default()
        },
        conventions: Default::default(),
    };
    let _ = roundtrip_json(&future);

    // Swap quote
    let swap = RatesQuote::Swap {
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
    };
    let _ = roundtrip_json(&swap);

    // Basis swap quote
    let basis = RatesQuote::BasisSwap {
        maturity: base_date + time::Duration::days(365 * 5),
        spread_bp: 5.0,
        conventions: InstrumentConventions::default()
            .with_currency(Currency::USD),
        primary_leg_conventions: InstrumentConventions::default()
            .with_index("3M-SOFR")
            .with_payment_frequency(Tenor::quarterly())
            .with_day_count(DayCount::Act360),
        reference_leg_conventions: InstrumentConventions::default()
            .with_index("6M-SOFR")
            .with_payment_frequency(Tenor::semi_annual())
            .with_day_count(DayCount::Act360),
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
        conventions: Default::default(),
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
        conventions: Default::default(),
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
        conventions: Default::default(),
    };
    let _ = roundtrip_json(&tranche);
}

#[test]
fn test_vol_quote_serialization() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    // Option vol quote
    let option_vol = VolQuote::OptionVol {
        underlying: "SPY".to_string().into(),
        expiry: base_date + time::Duration::days(90),
        strike: 450.0,
        vol: 0.20,
        option_type: "Call".to_string(),
        conventions: Default::default(),
    };
    let _ = roundtrip_json(&option_vol);

    // Swaption vol quote
    let swaption_vol = VolQuote::SwaptionVol {
        expiry: base_date + time::Duration::days(365),
        tenor: base_date + time::Duration::days(365 * 5),
        strike: 0.045,
        vol: 0.50,
        quote_type: "ATM".to_string(),
        conventions: Default::default(),
        fixed_leg_conventions: Default::default(),
        float_leg_conventions: Default::default(),
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
        conventions: Default::default(),
    };
    let _ = roundtrip_json(&zc_swap);

    // Year-on-year inflation swap
    let yoy_swap = InflationQuote::YoYInflationSwap {
        maturity: base_date + time::Duration::days(365 * 5),
        rate: 0.023,
        index: "USCPI".to_string(),
        frequency: Tenor::annual(),
        conventions: Default::default(),
        fixed_leg_conventions: Default::default(),
        inflation_leg_conventions: Default::default(),
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
            conventions: InstrumentConventions::default()
                .with_day_count(DayCount::Act360),
        }),
        MarketQuote::Credit(CreditQuote::CDS {
            entity: "AAPL".to_string(),
            maturity: base_date + time::Duration::days(365 * 5),
            spread_bp: 50.0,
            recovery_rate: 0.40,
            currency: Currency::USD,
            conventions: Default::default(),
        }),
        MarketQuote::Vol(VolQuote::OptionVol {
            underlying: "SPY".to_string().into(),
            expiry: base_date + time::Duration::days(90),
            strike: 450.0,
            vol: 0.20,
            option_type: "Call".to_string(),
            conventions: Default::default(),
        }),
        MarketQuote::Inflation(InflationQuote::InflationSwap {
            maturity: base_date + time::Duration::days(365 * 10),
            rate: 0.025,
            index: "USCPI".to_string(),
            conventions: Default::default(),
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
        validation_passed: true,
        validation_error: None,
        convergence_reason: "Tolerance met".to_string(),
        metadata,
        solver_config: Default::default(),
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
fn test_validation_config_serialization() {
    let config = ValidationConfig {
        check_forward_positivity: true,
        min_forward_rate: -0.02,
        max_forward_rate: 0.50,
        check_monotonicity: true,
        check_arbitrage: true,
        tolerance: 1e-10,
        max_hazard_rate: 0.50,
        min_cpi_growth: -0.10,
        max_cpi_growth: 0.50,
        min_fwd_inflation: -0.20,
        max_fwd_inflation: 0.50,
        max_volatility: 5.0,
        allow_negative_rates: true, // Support EUR/JPY/CHF environments
        lenient_arbitrage: false,
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

    let cfg = FinstackConfig::default();
    let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex)
        .with_finstack_config(&cfg)
        .expect("valid config");

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
        assert_eq!(convention.fixed_day_count, restored.fixed_day_count);
        assert_eq!(convention.float_day_count, restored.float_day_count);
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
        shift: None,
    };

    let restored = roundtrip_json(&market_data);
    assert_eq!(market_data.forward, restored.forward);
    assert_eq!(market_data.time_to_expiry, restored.time_to_expiry);
    assert_eq!(market_data.strikes.len(), restored.strikes.len());
    assert_eq!(market_data.market_vols.len(), restored.market_vols.len());
    assert_eq!(market_data.beta, restored.beta);
}

#[test]
fn test_sabr_parameters_serialization() {
    let params = SABRParameters::new(0.15, 0.5, 0.30, -0.1).expect("valid params");

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

    // Create FinstackConfig with calibration extensions
    let mut cfg = FinstackConfig::default();
    cfg.extensions.insert(
        finstack_valuations::calibration::CALIBRATION_CONFIG_KEY_V1,
        serde_json::json!({
            "tolerance": 1e-12,
            "max_iterations": 200,
            "use_parallel": true,
            "random_seed": 12345,
            "verbose": true,
            "solver_kind": "LevenbergMarquardt"
        }),
    );

    // Create multiple calibrators with this config
    let discount_calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
        .with_solve_interp(InterpStyle::MonotoneConvex)
        .with_finstack_config(&cfg)
        .expect("valid config");

    let forward_calibrator =
        ForwardCurveCalibrator::new("USD-SOFR-3M", 0.25, base_date, Currency::USD, "USD-OIS")
            .with_solve_interp(InterpStyle::Linear)
            .with_finstack_config(&cfg)
            .expect("valid config");

    let hazard_calibrator = HazardCurveCalibrator::new(
        "AAPL",
        Seniority::Senior,
        0.40,
        base_date,
        Currency::USD,
        "USD-OIS",
    )
    .with_finstack_config(&cfg)
    .expect("valid config");

    // Serialize each and verify
    let _ = roundtrip_json(&discount_calibrator);
    let _ = roundtrip_json(&forward_calibrator);
    let _ = roundtrip_json(&hazard_calibrator);

    println!("✓ All calibrators successfully serialized and deserialized");
}

// =============================================================================
// CalibrationEnvelope JSON Roundtrip Tests
// (Consolidated from calibration_roundtrip.rs)
// =============================================================================

use finstack_valuations::calibration::CalibrationEnvelope;

#[test]
fn test_full_market_pipeline_roundtrip() {
    assert_envelope_roundtrip("full_market_pipeline.json");
}

#[test]
fn test_rates_only_pipeline_roundtrip() {
    assert_envelope_roundtrip("rates_only_pipeline.json");
}

#[test]
fn test_credit_pipeline_roundtrip() {
    assert_envelope_roundtrip("credit_pipeline.json");
}

#[test]
fn test_vol_pipeline_roundtrip() {
    assert_envelope_roundtrip("vol_pipeline.json");
}

#[test]
#[cfg(feature = "slow")]
fn test_rates_pipeline_execution() {
    // Load and execute a rates-only pipeline calibration
    let envelope = CalibrationEnvelope::from_json(&read_json_example("rates_only_pipeline.json"))
        .expect("Failed to parse calibration envelope");

    // Execute calibration
    use finstack_valuations::calibration::CalibrationResultEnvelope;
    let result_envelope = envelope
        .execute(None)
        .expect("Calibration execution failed");

    assert_eq!(result_envelope.schema, "finstack.calibration/1");

    // Verify final market has at least one curve
    assert!(!result_envelope.result.final_market.curves.is_empty());

    // Serialize result to JSON
    let result_json = result_envelope
        .to_string()
        .expect("Failed to serialize result");

    // Deserialize result back
    let reparsed_result =
        CalibrationResultEnvelope::from_json(&result_json).expect("Failed to reparse result");

    // Verify structural equality
    assert_eq!(reparsed_result.schema, result_envelope.schema);
    assert_eq!(
        reparsed_result.result.final_market.curves.len(),
        result_envelope.result.final_market.curves.len()
    );
}

#[test]
#[cfg(feature = "slow")]
fn test_credit_pipeline_execution() {
    // Load and execute a credit pipeline calibration
    let envelope = CalibrationEnvelope::from_json(&read_json_example("credit_pipeline.json"))
        .expect("Failed to parse calibration envelope");

    // Execute calibration
    let result_envelope = envelope
        .execute(None)
        .expect("Calibration execution failed");

    assert_eq!(result_envelope.schema, "finstack.calibration/1");

    // Verify we have multiple curves (discount + hazard curves)
    assert!(result_envelope.result.final_market.curves.len() >= 2);

    // Verify step reports exist
    assert!(!result_envelope.result.step_reports.is_empty());
}

// =============================================================================
// MarketContextState Roundtrip Tests
// (Consolidated from calibration_state_roundtrip.rs)
// =============================================================================

use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::money::Money;

#[test]
fn test_empty_context_roundtrip() {
    let ctx = MarketContext::new();
    let state: MarketContextState = (&ctx).into();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&state).expect("Failed to serialize empty context");

    // Deserialize back
    let parsed_state: MarketContextState =
        serde_json::from_str(&json).expect("Failed to deserialize context state");

    // Verify empty
    assert!(parsed_state.curves.is_empty());
    assert!(parsed_state.surfaces.is_empty());
    assert!(parsed_state.prices.is_empty());
}

#[test]
fn test_discount_curve_context_roundtrip() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .set_interp(InterpStyle::MonotoneConvex)
        .build()
        .unwrap();

    let ctx = MarketContext::new().insert_discount(curve);
    let state: MarketContextState = (&ctx).into();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&state)
        .expect("Failed to serialize context with discount curve");

    // Deserialize back
    let parsed_state: MarketContextState =
        serde_json::from_str(&json).expect("Failed to deserialize context state");

    // Reconstruct context
    let reconstructed_ctx: MarketContext = parsed_state
        .try_into()
        .expect("Failed to reconstruct context from state");

    // Verify curve exists and matches
    let retrieved_curve = reconstructed_ctx
        .get_discount("USD-OIS")
        .expect("Discount curve not found in reconstructed context");

    assert_eq!(retrieved_curve.id().as_str(), "USD-OIS");
    assert_eq!(retrieved_curve.base_date(), base_date);

    // Verify discount factors match
    assert!((retrieved_curve.df(0.0) - 1.0).abs() < 1e-12);
    assert!((retrieved_curve.df(1.0) - 0.98).abs() < 1e-12);
    assert!((retrieved_curve.df(5.0) - 0.90).abs() < 1e-12);
}

#[test]
fn test_multiple_curves_context_roundtrip() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M-FWD", 0.25)
        .base_date(base_date)
        .knots(vec![(0.0, 0.045), (1.0, 0.046), (5.0, 0.048)])
        .build()
        .unwrap();

    let hazard = HazardCurve::builder("AAPL-Senior")
        .base_date(base_date)
        .seniority(Seniority::Senior)
        .recovery_rate(0.40)
        .knots(vec![(1.0, 0.010), (3.0, 0.012), (5.0, 0.015)])
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
        .insert_hazard(hazard)
        .insert_price(
            "AAPL",
            MarketScalar::Price(Money::new(180.0, Currency::USD)),
        );

    let state: MarketContextState = (&ctx).into();

    // Serialize
    let json = serde_json::to_string_pretty(&state).expect("Failed to serialize complex context");

    // Deserialize
    let parsed_state: MarketContextState =
        serde_json::from_str(&json).expect("Failed to deserialize context state");

    // Reconstruct
    let reconstructed: MarketContext = parsed_state
        .try_into()
        .expect("Failed to reconstruct context");

    // Verify all curves present
    assert!(reconstructed.get_discount("USD-OIS").is_ok());
    assert!(reconstructed.get_forward("USD-SOFR-3M-FWD").is_ok());
    assert!(reconstructed.get_hazard("AAPL-Senior").is_ok());

    // Verify price
    let aapl_price = reconstructed.price("AAPL").expect("AAPL price not found");
    match aapl_price {
        MarketScalar::Price(money) => {
            assert_eq!(money.currency(), Currency::USD);
            assert!((money.amount() - 180.0).abs() < 1e-9);
        }
        _ => panic!("Expected price scalar"),
    }
}

#[test]
fn test_context_stats_preserved() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (5.0, 0.90)])
        .build()
        .unwrap();

    let original_ctx = MarketContext::new().insert_discount(disc);
    let original_stats = original_ctx.stats();

    // Convert to state and back
    let state: MarketContextState = (&original_ctx).into();
    let json = serde_json::to_string_pretty(&state).unwrap();
    let parsed_state: MarketContextState = serde_json::from_str(&json).unwrap();
    let reconstructed_ctx: MarketContext = parsed_state.try_into().unwrap();

    let reconstructed_stats = reconstructed_ctx.stats();

    // Verify stats match
    assert_eq!(
        reconstructed_stats.total_curves,
        original_stats.total_curves
    );
    assert_eq!(
        reconstructed_stats.surface_count,
        original_stats.surface_count
    );
    assert_eq!(reconstructed_stats.price_count, original_stats.price_count);
}
