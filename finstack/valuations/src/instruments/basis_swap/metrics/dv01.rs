use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Error, Result, F};

/// DV01 calculator for a leg (annuity * notional * 1bp)
pub struct Dv01Calculator {
    pub is_primary: bool,
}

impl Dv01Calculator {
    pub const fn primary() -> Self { Self { is_primary: true } }
    pub const fn reference() -> Self { Self { is_primary: false } }
}

impl MetricCalculator for Dv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        if self.is_primary {
            &[MetricId::BasisAnnuityPrimary]
        } else {
            &[MetricId::BasisAnnuityReference]
        }
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let annuity = if self.is_primary {
            context.computed.get(&MetricId::BasisAnnuityPrimary).copied().unwrap_or(0.0)
        } else {
            context.computed.get(&MetricId::BasisAnnuityReference).copied().unwrap_or(0.0)
        };

        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        Ok(annuity * swap.notional.amount() * 1e-4)
    }
}


