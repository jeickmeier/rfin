use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{error::InputError, Error, Result};

/// Net DV01 calculator (primary leg DV01 minus reference leg DV01).
#[derive(Default, Debug)]
pub struct NetDv01Calculator;

impl MetricCalculator for NetDv01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        const DEPS: &[MetricId] = &[MetricId::Dv01Primary, MetricId::Dv01Reference];
        DEPS
    }

    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let primary = context
            .computed
            .get(&MetricId::Dv01Primary)
            .copied()
            .ok_or(Error::Input(InputError::Invalid))?;
        let reference = context
            .computed
            .get(&MetricId::Dv01Reference)
            .copied()
            .ok_or(Error::Input(InputError::Invalid))?;
        Ok(primary - reference)
    }
}
