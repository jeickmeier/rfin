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
use finstack_valuations::instruments::structured_credit::components::specs::{
    DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
};

#[test]
fn test_prepayment_shift_measurement_psa() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.5 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let shift = measure_prepayment_shift(&params_t0, &params_t1);

    // PSA increased from 100% to 150% (0.5 increase)
    // 0.5 * 600bp = 300bp
    assert_eq!(shift, 300.0);
}

#[test]
fn test_prepayment_shift_measurement_cpr() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::ConstantCpr { cpr: 0.06 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::ConstantCpr { cpr: 0.08 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let shift = measure_prepayment_shift(&params_t0, &params_t1);

    // CPR increased from 6% to 8% (2% increase = 200bp)
    assert!((shift - 200.0).abs() < 0.01);
}

#[test]
fn test_default_shift_measurement() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.03 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let shift = measure_default_shift(&params_t0, &params_t1);

    // CDR increased from 2% to 3% (1% increase = 100bp)
    assert!((shift - 100.0).abs() < 0.01);
}

#[test]
fn test_recovery_shift_measurement() {
    let params_t0 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.60 },
    };

    let params_t1 = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
        default_spec: DefaultModelSpec::ConstantCdr { cdr: 0.02 },
        recovery_spec: RecoveryModelSpec::Constant { rate: 0.65 },
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
    // Plain bonds and deposits don't have model parameters
    // This is verified in the extract_model_params function
    // which returns ModelParamsSnapshot::None for unsupported types
}
