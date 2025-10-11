//! ABS Speed calculator for auto loans

use crate::instruments::common::structured_credit::DealType;
use crate::instruments::structured_credit::{InstrumentSpecificFields, StructuredCredit};
use crate::metrics::MetricContext;

/// ABS Speed calculator - monthly absolute prepayment speed
pub struct AbsSpeedCalculator;

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
                Ok(abs_speed.unwrap_or(1.5)) // 1.5% ABS default
            }
            _ => Ok(1.5), // Default fallback
        }
    }
}
