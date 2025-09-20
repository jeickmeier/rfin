//! Rho metric for `CdsOption`.

use crate::instruments::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

/// Rho calculator for credit options on CDS spreads.
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CdsOption = context.instrument_as()?;
        let t = {
            let option: &CdsOption = context.instrument_as()?;
            option
                .day_count
                .year_fraction(context.as_of, option.expiry, finstack_core::dates::DayCountCtx::default())?
        };
        if t <= 0.0 { return Ok(0.0); }

        // Black-76 property: dPrice/dr = -t * Price, report per 1% change in rates
        let base_price = context.base_value.amount();
        Ok(-0.01 * t * base_price)
    }

    fn dependencies(&self) -> &[MetricId] { &[] }
}


