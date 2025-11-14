//! Model parameters extraction and modification for P&L attribution.
//!
//! Provides functionality to extract model-specific parameters from instruments,
//! create modified versions with different parameters, and measure parameter shifts.

use crate::instruments::common::traits::Instrument;
use crate::instruments::convertible::{ConversionSpec, ConvertibleBond};
use crate::instruments::structured_credit::components::specs::{
    DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
};
use crate::instruments::structured_credit::StructuredCredit;
use finstack_core::prelude::*;
use std::sync::Arc;

/// Snapshot of extractable model parameters from an instrument.
///
/// Different instrument types have different model parameters that affect
/// pricing. This enum captures the relevant parameters for each type.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ModelParamsSnapshot {
    /// Structured credit parameters (prepayment, default, recovery).
    StructuredCredit {
        prepayment_spec: PrepaymentModelSpec,
        default_spec: DefaultModelSpec,
        recovery_spec: RecoveryModelSpec,
    },

    /// Convertible bond parameters (conversion ratio, policies).
    Convertible { conversion_spec: ConversionSpec },

    /// No extractable model parameters.
    None,
}

/// Extract model parameters from an instrument.
///
/// Uses downcasting to identify instrument type and extract relevant parameters.
///
/// # Arguments
///
/// * `instrument` - Instrument to extract parameters from
///
/// # Returns
///
/// Snapshot of model parameters, or `ModelParamsSnapshot::None` if instrument
/// type doesn't have extractable parameters.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::attribution::model_params::extract_model_params;
///
/// let params = extract_model_params(&structured_credit);
/// match params {
///     ModelParamsSnapshot::StructuredCredit { prepayment_spec, .. } => {
///         println!("Prepayment: {:?}", prepayment_spec);
///     }
///     _ => {}
/// }
/// ```
pub fn extract_model_params(instrument: &Arc<dyn Instrument>) -> ModelParamsSnapshot {
    // Try downcasting to StructuredCredit
    if let Some(structured) = instrument.as_any().downcast_ref::<StructuredCredit>() {
        return ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: structured.prepayment_spec.clone(),
            default_spec: structured.default_spec.clone(),
            recovery_spec: structured.recovery_spec.clone(),
        };
    }

    // Try downcasting to ConvertibleBond
    if let Some(convertible) = instrument.as_any().downcast_ref::<ConvertibleBond>() {
        return ModelParamsSnapshot::Convertible {
            conversion_spec: convertible.conversion.clone(),
        };
    }

    // Other instrument types don't have model parameters
    ModelParamsSnapshot::None
}

/// Create a modified instrument with different model parameters.
///
/// Clones the instrument and replaces its model parameters with those from
/// the snapshot. Used for isolating model parameter P&L in attribution.
///
/// # Arguments
///
/// * `instrument` - Original instrument
/// * `params` - Model parameters to apply
///
/// # Returns
///
/// New instrument with modified parameters, or original if no params to modify.
///
/// # Errors
///
/// Returns error if instrument type doesn't match snapshot type.
///
/// # Examples
///
/// ```rust,ignore
/// // Extract T₀ parameters
/// let params_t0 = extract_model_params(&instrument);
///
/// // Create instrument with T₀ params for attribution
/// let instrument_t0_params = with_model_params(&instrument, &params_t0)?;
/// ```
pub fn with_model_params(
    instrument: &Arc<dyn Instrument>,
    params: &ModelParamsSnapshot,
) -> Result<Arc<dyn Instrument>> {
    match params {
        ModelParamsSnapshot::StructuredCredit {
            prepayment_spec,
            default_spec,
            recovery_spec,
        } => {
            if let Some(structured) = instrument.as_any().downcast_ref::<StructuredCredit>() {
                let mut modified = structured.clone();
                modified.prepayment_spec = prepayment_spec.clone();
                modified.default_spec = default_spec.clone();
                modified.recovery_spec = recovery_spec.clone();
                Ok(Arc::new(modified) as Arc<dyn Instrument>)
            } else {
                Err(Error::Validation(
                    "Instrument type mismatch: expected StructuredCredit".to_string(),
                ))
            }
        }

        ModelParamsSnapshot::Convertible { conversion_spec } => {
            if let Some(convertible) = instrument.as_any().downcast_ref::<ConvertibleBond>() {
                let mut modified = convertible.clone();
                modified.conversion = conversion_spec.clone();
                Ok(Arc::new(modified) as Arc<dyn Instrument>)
            } else {
                Err(Error::Validation(
                    "Instrument type mismatch: expected ConvertibleBond".to_string(),
                ))
            }
        }

        ModelParamsSnapshot::None => {
            // No model params to modify, return original
            Ok(Arc::clone(instrument))
        }
    }
}

/// Measure prepayment parameter shift between two snapshots.
///
/// Returns shift in basis points for use with Prepayment01 metric.
///
/// # Arguments
///
/// * `snapshot_t0` - Parameters at T₀
/// * `snapshot_t1` - Parameters at T₁
///
/// # Returns
///
/// Shift in basis points, or 0.0 if not applicable.
pub fn measure_prepayment_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::StructuredCredit {
                prepayment_spec: prep_t0,
                ..
            },
            ModelParamsSnapshot::StructuredCredit {
                prepayment_spec: prep_t1,
                ..
            },
        ) => {
            use crate::cashflow::builder::specs::PrepaymentCurve;

            match (&prep_t0.curve, &prep_t1.curve) {
                (
                    Some(PrepaymentCurve::Psa {
                        speed_multiplier: mult_t0,
                    }),
                    Some(PrepaymentCurve::Psa {
                        speed_multiplier: mult_t1,
                    }),
                ) => {
                    // PSA multiplier change (convert to CPR change approximation)
                    // PSA 100% ≈ 6% CPR terminal, so multiply difference by 6%
                    (mult_t1 - mult_t0) * 600.0 // Convert to basis points
                }
                (None, None)
                | (Some(PrepaymentCurve::Constant), Some(PrepaymentCurve::Constant)) => {
                    // Direct CPR difference in basis points
                    (prep_t1.cpr - prep_t0.cpr) * 10000.0
                }
                _ => 0.0, // Mixed or unsupported model types
            }
        }
        _ => 0.0,
    }
}

/// Measure default rate parameter shift between two snapshots.
///
/// Returns shift in basis points for use with Default01 metric.
pub fn measure_default_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::StructuredCredit {
                default_spec: def_t0,
                ..
            },
            ModelParamsSnapshot::StructuredCredit {
                default_spec: def_t1,
                ..
            },
        ) => {
            // CDR difference in basis points (works for both constant and SDA curves)
            (def_t1.cdr - def_t0.cdr) * 10000.0
        }
        _ => 0.0,
    }
}

/// Measure recovery rate parameter shift between two snapshots.
///
/// Returns shift in percentage points (not basis points) for use with Recovery01 metric.
pub fn measure_recovery_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::StructuredCredit {
                recovery_spec: rec_t0,
                ..
            },
            ModelParamsSnapshot::StructuredCredit {
                recovery_spec: rec_t1,
                ..
            },
        ) => {
            // Direct recovery rate difference in percentage points
            (rec_t1.rate - rec_t0.rate) * 100.0
        }
        _ => 0.0,
    }
}

/// Measure conversion ratio shift between two snapshots.
///
/// Returns shift in percentage points for use with Conversion01 metric.
pub fn measure_conversion_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::Convertible {
                conversion_spec: conv_t0,
            },
            ModelParamsSnapshot::Convertible {
                conversion_spec: conv_t1,
            },
        ) => {
            match (conv_t0.ratio, conv_t1.ratio) {
                (Some(ratio_t0), Some(ratio_t1)) => {
                    // Conversion ratio change as percentage
                    ((ratio_t1 - ratio_t0) / ratio_t0) * 100.0
                }
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_none_for_unsupported_instrument() {
        // For instruments without model params, should return None
        // Testing will be done with actual instruments in integration tests
    }

    #[test]
    fn test_measure_prepayment_shift_psa() {
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
        // PSA increased by 0.5, which is 0.5 * 600bp = 300bp
        assert_eq!(shift, 300.0);
    }

    #[test]
    fn test_measure_default_shift_cdr() {
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
        // CDR increased by 1% = 100bp
        assert!((shift - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_measure_recovery_shift() {
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
}
