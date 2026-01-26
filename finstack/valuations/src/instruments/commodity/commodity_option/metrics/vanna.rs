//! Vanna calculator for commodity options.
//!
//! Vanna is the cross-gamma between underlying and volatility:
//! Vanna = ∂²V / (∂S × ∂σ)
//!
//! When spot_price_id is present, bumps the spot scalar.
//! Otherwise, bumps the PriceCurve referenced by forward_curve_id.

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::metrics::bump_sizes;
use crate::metrics::{
    bump_scalar_price, bump_surface_vol_absolute, MetricCalculator, MetricContext,
};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Vanna calculator for commodity options.
///
/// Computes cross-gamma between underlying price and volatility using
/// finite differences. Works with either spot price scalar or PriceCurve.
pub struct VannaCalculator;

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CommodityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let spot_bump_pct = bump_sizes::SPOT;
        let vol_bump = bump_sizes::VOLATILITY;

        // Determine spot price for bump size calculation
        let spot_price = if let Some(ref spot_id) = option.spot_price_id {
            let scalar = context.curves.price(spot_id)?;
            match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
            }
        } else {
            option.forward_price(&context.curves, as_of)?
        };

        let spot_bump_size = spot_price * spot_bump_pct;
        let vol_surface_id = option.vol_surface_id.as_str();

        if let Some(ref spot_id) = option.spot_price_id {
            // Bump spot up and vol up
            let curves_spot_up = bump_scalar_price(&context.curves, spot_id, spot_bump_pct)?;
            let curves_spot_up_vol_up =
                bump_surface_vol_absolute(&curves_spot_up, vol_surface_id, vol_bump)?;
            let pv_up_up = option.npv(&curves_spot_up_vol_up, as_of)?.amount();

            // Bump spot up and vol down
            let curves_spot_up_vol_down =
                bump_surface_vol_absolute(&curves_spot_up, vol_surface_id, -vol_bump)?;
            let pv_up_down = option.npv(&curves_spot_up_vol_down, as_of)?.amount();

            // Bump spot down and vol up
            let curves_spot_down = bump_scalar_price(&context.curves, spot_id, -spot_bump_pct)?;
            let curves_spot_down_vol_up =
                bump_surface_vol_absolute(&curves_spot_down, vol_surface_id, vol_bump)?;
            let pv_down_up = option.npv(&curves_spot_down_vol_up, as_of)?.amount();

            // Bump spot down and vol down
            let curves_spot_down_vol_down =
                bump_surface_vol_absolute(&curves_spot_down, vol_surface_id, -vol_bump)?;
            let pv_down_down = option.npv(&curves_spot_down_vol_down, as_of)?.amount();

            // Vanna = (PV_up_up - PV_up_down - PV_down_up + PV_down_down) / (4 * spot_bump * vol_bump)
            Ok((pv_up_up - pv_up_down - pv_down_up + pv_down_down)
                / (4.0 * spot_bump_size * vol_bump))
        } else {
            // Bump PriceCurve instead of spot
            let price_curve_id = CurveId::new(option.forward_curve_id.as_str());

            let bump_price_up = MarketBump::Curve {
                id: price_curve_id.clone(),
                spec: BumpSpec {
                    bump_type: BumpType::Parallel,
                    mode: BumpMode::Additive,
                    units: BumpUnits::Percent,
                    value: spot_bump_pct * 100.0,
                },
            };
            let bump_price_down = MarketBump::Curve {
                id: price_curve_id,
                spec: BumpSpec {
                    bump_type: BumpType::Parallel,
                    mode: BumpMode::Additive,
                    units: BumpUnits::Percent,
                    value: -spot_bump_pct * 100.0,
                },
            };

            // Price up + vol up
            let curves_price_up = context.curves.bump([bump_price_up.clone()])?;
            let curves_up_vol_up =
                bump_surface_vol_absolute(&curves_price_up, vol_surface_id, vol_bump)?;
            let pv_up_up = option.npv(&curves_up_vol_up, as_of)?.amount();

            // Price up + vol down
            let curves_up_vol_down =
                bump_surface_vol_absolute(&curves_price_up, vol_surface_id, -vol_bump)?;
            let pv_up_down = option.npv(&curves_up_vol_down, as_of)?.amount();

            // Price down + vol up
            let curves_price_down = context.curves.bump([bump_price_down.clone()])?;
            let curves_down_vol_up =
                bump_surface_vol_absolute(&curves_price_down, vol_surface_id, vol_bump)?;
            let pv_down_up = option.npv(&curves_down_vol_up, as_of)?.amount();

            // Price down + vol down
            let curves_down_vol_down =
                bump_surface_vol_absolute(&curves_price_down, vol_surface_id, -vol_bump)?;
            let pv_down_down = option.npv(&curves_down_vol_down, as_of)?.amount();

            // Vanna = (PV_up_up - PV_up_down - PV_down_up + PV_down_down) / (4 * price_bump * vol_bump)
            Ok((pv_up_up - pv_up_down - pv_down_up + pv_down_down)
                / (4.0 * spot_bump_size * vol_bump))
        }
    }
}
