use crate::instruments::basis_swap::types::BasisSwap;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Error, Result};

/// Calculator for the DV01 (dollar value of 1 basis point) of a basis swap leg.
///
/// DV01 represents the change in present value for a 1 basis point change in rates.
/// It is calculated as the product of the annuity, notional amount, and 1 basis point.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::basis_swap::metrics::Dv01Calculator;
///
/// let primary_calc = Dv01Calculator::primary();
/// let reference_calc = Dv01Calculator::reference();
/// ```
pub struct Dv01Calculator {
    /// Whether this calculator is for the primary leg (true) or reference leg (false).
    pub is_primary: bool,
}

impl Dv01Calculator {
    /// Creates a calculator for the primary leg.
    pub const fn primary() -> Self {
        Self { is_primary: true }
    }

    /// Creates a calculator for the reference leg.
    pub const fn reference() -> Self {
        Self { is_primary: false }
    }
}

impl MetricCalculator for Dv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        if self.is_primary {
            &[MetricId::BasisAnnuityPrimary]
        } else {
            &[MetricId::BasisAnnuityReference]
        }
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let annuity = if self.is_primary {
            context
                .computed
                .get(&MetricId::BasisAnnuityPrimary)
                .copied()
                .unwrap_or(0.0)
        } else {
            context
                .computed
                .get(&MetricId::BasisAnnuityReference)
                .copied()
                .unwrap_or(0.0)
        };

        let instrument = context.instrument.clone();
        let swap = instrument
            .as_any()
            .downcast_ref::<BasisSwap>()
            .ok_or(Error::Input(finstack_core::error::InputError::Invalid))?;
        Ok(annuity * swap.notional.amount() * 1e-4)
    }
}
