//! Rho calculator for interest rate options.
//!
//! Computes rho via a discount-curve bump-and-reprice using the shared
//! MarketContext bump API. Returns sensitivity per 1% (i.e., 100bp).

use crate::instruments::cap_floor::InterestRateOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::Result;

/// Rho calculator (per 1%)
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &InterestRateOption = context.instrument_as()?;

        // Base PV
        let base = context.base_value.amount();

        // Build bumps map: parallel 1bp on the discount curve only, then scale to 1%
        let mut bumps = hashbrown::HashMap::new();
        bumps.insert(
            option.disc_id.to_owned(),
            BumpSpec::parallel_bp(1.0), // +1bp
        );

        let bumped_ctx = context.curves.bump(bumps)?;

        // Reprice with bumped discount curve (vol held constant)
        let bumped = option.npv(&bumped_ctx, context.as_of)?;

        let dv01 = bumped.amount() - base; // per 1bp
        Ok(dv01 * 100.0) // per 1%
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
