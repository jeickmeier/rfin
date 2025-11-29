//! Integration tests for model parameters attribution.
//!
//! Tests attribution of P&L from changes in model-specific parameters like
//! prepayment speeds, default rates, recovery rates, and conversion ratios.

use finstack_valuations::attribution::model_params::{
    measure_conversion_shift, measure_default_shift, measure_prepayment_shift,
    measure_recovery_shift, ModelParamsSnapshot,
};
use finstack_valuations::instruments::convertible::{
    AntiDilutionPolicy, ConversionPolicy, ConversionSpec, DividendAdjustment,
};
use finstack_valuations::cashflow::builder::{
    DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
};

#[test]
fn test_prepayment_shift_measurement_psa() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.0),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.5),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let shift = measure_prepayment_shift(&params_t0, &params_t1);

    // PSA increased from 100% to 150% (0.5 increase)
    // 0.5 * 600bp = 300bp
    assert_eq!(shift, 300.0);
}

#[test]
fn test_prepayment_shift_measurement_cpr() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::constant_cpr(0.06),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::constant_cpr(0.08),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let shift = measure_prepayment_shift(&params_t0, &params_t1);

    // CPR increased from 6% to 8% (2% increase = 200bp)
    assert!((shift - 200.0).abs() < 0.01);
}

#[test]
fn test_default_shift_measurement() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.0),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.0),
        default_spec: DefaultModelSpec::constant_cdr(0.03),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let shift = measure_default_shift(&params_t0, &params_t1);

    // CDR increased from 2% to 3% (1% increase = 100bp)
    assert!((shift - 100.0).abs() < 0.01);
}

#[test]
fn test_recovery_shift_measurement() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.0),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.0),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.65, 12),
    };

    let shift = measure_recovery_shift(&params_t0, &params_t1);

    // Recovery rate increased from 60% to 65% (5 percentage points)
    assert!((shift - 5.0).abs() < 0.01);
}

#[test]
fn test_conversion_shift_measurement() {
    let params_t0 = ModelParamsSnapshot::Convertible {
        conversion_spec: ConversionSpec {
            ratio: Some(10.0),
            price: None,
            policy: ConversionPolicy::Voluntary,
            anti_dilution: AntiDilutionPolicy::None,
            dividend_adjustment: DividendAdjustment::None,
        },
    };

    let params_t1 = ModelParamsSnapshot::Convertible {
        conversion_spec: ConversionSpec {
            ratio: Some(12.0),
            price: None,
            policy: ConversionPolicy::Voluntary,
            anti_dilution: AntiDilutionPolicy::None,
            dividend_adjustment: DividendAdjustment::None,
        },
    };

    let shift = measure_conversion_shift(&params_t0, &params_t1);

    // Conversion ratio increased from 10 to 12 (20% increase)
    assert_eq!(shift, 20.0);
}

#[test]
fn test_model_params_none_for_plain_instruments() {
    use finstack_core::currency::Currency;
    use finstack_core::dates::create_date;
    use finstack_core::money::Money;
    use finstack_valuations::attribution::model_params::extract_model_params;
    use finstack_valuations::instruments::bond::Bond;
    use finstack_valuations::instruments::Instrument;
    use std::sync::Arc;
    use time::Month;

    // Plain bonds don't have model parameters (no prepayment/default/recovery models)
    let bond = Bond::fixed(
        "PLAIN-BOND-001",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        create_date(2024, Month::January, 1).unwrap(),
        create_date(2029, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let bond_instrument: Arc<dyn Instrument> = Arc::new(bond);
    let params = extract_model_params(&bond_instrument);

    // Plain bond should return None for model parameters
    assert!(
        matches!(params, ModelParamsSnapshot::None),
        "Plain bond should have ModelParamsSnapshot::None, got {:?}",
        params
    );
}
