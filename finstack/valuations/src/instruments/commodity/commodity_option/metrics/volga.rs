//! Volga (vomma) calculator for commodity options.
//!
//! Volga is the second derivative of option price with respect to volatility:
//! Volga = ∂²V / ∂σ²
//!
//! This is independent of spot/forward price bumping, so uses the same
//! implementation regardless of whether spot_price_id is present.

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::metrics::bump_sizes;
use crate::metrics::{bump_surface_vol_absolute, MetricCalculator, MetricContext};
use finstack_core::Result;

/// Volga (vomma) calculator for commodity options.
///
/// Computes the second derivative with respect to volatility using finite differences.
/// This calculator is independent of the spot vs PriceCurve distinction since it
/// only bumps volatility.
pub struct VolgaCalculator;

impl MetricCalculator for VolgaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CommodityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let vol_bump = bump_sizes::VOLATILITY;

        let vol_surface_id = option.vol_surface_id.as_str();

        // Base PV
        let pv_base = option.npv(&context.curves, as_of)?.amount();

        // Vol up
        let curves_vol_up = bump_surface_vol_absolute(&context.curves, vol_surface_id, vol_bump)?;
        let pv_up = option.npv(&curves_vol_up, as_of)?.amount();

        // Vol down
        let curves_vol_down =
            bump_surface_vol_absolute(&context.curves, vol_surface_id, -vol_bump)?;
        let pv_down = option.npv(&curves_vol_down, as_of)?.amount();

        // Volga = (PV_up - 2*PV_base + PV_down) / vol_bump^2
        Ok((pv_up - 2.0 * pv_base + pv_down) / (vol_bump * vol_bump))
    }
}
