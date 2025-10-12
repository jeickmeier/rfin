//! ABS Speed calculator for auto loans

use crate::instruments::common::structured_credit::DealType;
use crate::instruments::structured_credit::{InstrumentSpecificFields, StructuredCredit};
use crate::metrics::MetricContext;

/// ABS Speed calculator - monthly absolute prepayment speed
pub struct AbsSpeedCalculator {
    default_abs_speed: f64,
}

impl AbsSpeedCalculator {
    /// Create a new ABS speed calculator with specified default speed (as percentage)
    pub fn new(default_abs_speed: f64) -> Self {
        Self { default_abs_speed }
    }
}

impl crate::metrics::MetricCalculator for AbsSpeedCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let sc = context
            .instrument
            .as_any()
            .downcast_ref::<StructuredCredit>()
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Only applicable to ABS deals
        if sc.deal_type != DealType::ABS {
            return Err(finstack_core::Error::from(
                finstack_core::error::InputError::Invalid
            ));
        }

        // Return ABS speed if set, otherwise default
        match &sc.specific {
            InstrumentSpecificFields::Abs { abs_speed, .. } => {
                Ok(abs_speed.unwrap_or(self.default_abs_speed))
            }
            _ => Ok(self.default_abs_speed),
        }
    }
}
