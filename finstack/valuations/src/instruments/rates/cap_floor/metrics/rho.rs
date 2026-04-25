//! Rho calculator for interest rate options.
//!
//! Computes rho via a discount-curve bump-and-reprice using the shared
//! MarketContext bump API. Returns sensitivity per 1bp.
//!
//! Units & sign:
//! - Rho = PV(rate + 1bp) − PV(base)
//! - Positive Rho means the instrument gains value when rates go up

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::rates::cap_floor::CapFloor;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

/// Rho calculator (per 1bp)
pub(crate) struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CapFloor = context.instrument_as()?;

        // Base PV
        let base = context.base_value.amount();

        let bumped_ctx = context.curves.bump([MarketBump::Curve {
            id: option.discount_curve_id.to_owned(),
            spec: BumpSpec::parallel_bp(1.0), // +1bp
        }])?;

        // Reprice with bumped discount curve (vol held constant)
        let bumped = option.value(&bumped_ctx, context.as_of)?;

        // Rho per 1bp
        let rho = bumped.amount() - base;
        Ok(rho)
    }
}
