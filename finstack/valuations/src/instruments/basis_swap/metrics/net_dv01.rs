use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{error::InputError, Error, Result, F};

/// Net DV01 calculator (primary leg DV01 minus reference leg DV01).
#[derive(Default)]
pub struct NetDv01Calculator;

impl MetricCalculator for NetDv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        const DEPS: &[MetricId] = &[MetricId::BasisDv01Primary, MetricId::BasisDv01Reference];
        DEPS
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let primary = context
            .computed
            .get(&MetricId::BasisDv01Primary)
            .copied()
            .ok_or(Error::Input(InputError::Invalid))?;
        let reference = context
            .computed
            .get(&MetricId::BasisDv01Reference)
            .copied()
            .ok_or(Error::Input(InputError::Invalid))?;
        Ok(primary - reference)
    }
}
