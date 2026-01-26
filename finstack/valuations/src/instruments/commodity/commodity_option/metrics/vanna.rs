//! Vanna calculator for commodity options.
//!
//! Vanna is the cross-gamma between forward price and volatility:
//! Vanna = ∂²V / (∂F × ∂σ)
//!
//! This is forward-based vanna, consistent with Black-76:
//! - If `quoted_forward` is set: bumps the instrument's quoted forward override
//! - Else if a `PriceCurve` exists: bumps the PriceCurve (parallel percent bump)
//! - Only as fallback: bumps `spot_price_id` (if present) to propagate via cost-of-carry

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::metrics::bump_sizes;
use crate::metrics::{
    bump_scalar_price, bump_surface_vol_absolute, MetricCalculator, MetricContext,
};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Forward-based vanna calculator for commodity options.
///
/// Computes cross-gamma between forward price and volatility using
/// central finite differences: `Vanna = ∂²V / (∂F × ∂σ)`.
///
/// The bump target priority is:
/// 1. `quoted_forward` (instrument override)
/// 2. `PriceCurve` (market data)
/// 3. `spot_price_id` (cost-of-carry fallback)
pub struct VannaCalculator;

/// Determines the forward price driver for bumping.
#[derive(Debug)]
enum ForwardDriver {
    /// Bump the instrument's `quoted_forward` field
    QuotedForward(f64),
    /// Bump the PriceCurve in market data
    PriceCurve,
    /// Bump spot scalar (cost-of-carry fallback)
    SpotScalar(String),
}

impl VannaCalculator {
    /// Determine what to bump based on the forward price retrieval priority.
    fn determine_driver(
        option: &CommodityOption,
        context: &MetricContext,
    ) -> Result<ForwardDriver> {
        // 1. If quoted_forward is set, bump that
        if let Some(fwd) = option.quoted_forward {
            return Ok(ForwardDriver::QuotedForward(fwd));
        }

        // 2. Try to find a PriceCurve
        if context
            .curves
            .get_price_curve(option.forward_curve_id.as_str())
            .is_ok()
        {
            return Ok(ForwardDriver::PriceCurve);
        }

        // 3. Fall back to spot scalar (cost-of-carry)
        if let Some(ref spot_id) = option.spot_price_id {
            return Ok(ForwardDriver::SpotScalar(spot_id.clone()));
        }

        // No valid driver found
        Err(finstack_core::Error::Validation(
            "Cannot compute vanna: no quoted_forward, PriceCurve, or spot_price_id available"
                .to_string(),
        ))
    }

    /// Compute PV with both forward and vol bumps applied.
    fn pv_with_bumps(
        option: &CommodityOption,
        context: &MetricContext,
        driver: &ForwardDriver,
        fwd_bump_pct: f64,
        vol_bump: f64,
    ) -> Result<f64> {
        let as_of = context.as_of;
        let vol_surface_id = option.vol_surface_id.as_str();

        match driver {
            ForwardDriver::QuotedForward(fwd) => {
                // Clone option and bump the quoted_forward field
                let mut option_bumped = option.clone();
                option_bumped.quoted_forward = Some(fwd * (1.0 + fwd_bump_pct));

                // Also bump vol surface
                let curves_bumped =
                    bump_surface_vol_absolute(&context.curves, vol_surface_id, vol_bump)?;
                option_bumped.npv(&curves_bumped, as_of).map(|m| m.amount())
            }
            ForwardDriver::PriceCurve => {
                // Bump PriceCurve
                let price_curve_id = CurveId::new(option.forward_curve_id.as_str());
                let bump_price = MarketBump::Curve {
                    id: price_curve_id,
                    spec: BumpSpec {
                        bump_type: BumpType::Parallel,
                        mode: BumpMode::Additive,
                        units: BumpUnits::Percent,
                        value: fwd_bump_pct * 100.0,
                    },
                };
                let curves_price_bumped = context.curves.bump([bump_price])?;

                // Then bump vol surface
                let curves_bumped =
                    bump_surface_vol_absolute(&curves_price_bumped, vol_surface_id, vol_bump)?;
                option.npv(&curves_bumped, as_of).map(|m| m.amount())
            }
            ForwardDriver::SpotScalar(ref spot_id) => {
                // Bump spot scalar
                let curves_spot_bumped = bump_scalar_price(&context.curves, spot_id, fwd_bump_pct)?;

                // Then bump vol surface
                let curves_bumped =
                    bump_surface_vol_absolute(&curves_spot_bumped, vol_surface_id, vol_bump)?;
                option.npv(&curves_bumped, as_of).map(|m| m.amount())
            }
        }
    }
}

impl MetricCalculator for VannaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CommodityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let fwd_bump_pct = bump_sizes::SPOT;
        let vol_bump = bump_sizes::VOLATILITY;

        // Determine what drives the forward price and how to bump it
        let driver = Self::determine_driver(option, context)?;

        // Get base forward price for bump size calculation
        let forward_price = option.forward_price(&context.curves, as_of)?;
        let fwd_bump_size = forward_price * fwd_bump_pct;

        // Central mixed finite difference:
        // Vanna = [V(F+h,σ+k) - V(F+h,σ-k) - V(F-h,σ+k) + V(F-h,σ-k)] / (4 * h * k)
        let pv_up_up = Self::pv_with_bumps(option, context, &driver, fwd_bump_pct, vol_bump)?;
        let pv_up_down = Self::pv_with_bumps(option, context, &driver, fwd_bump_pct, -vol_bump)?;
        let pv_down_up = Self::pv_with_bumps(option, context, &driver, -fwd_bump_pct, vol_bump)?;
        let pv_down_down = Self::pv_with_bumps(option, context, &driver, -fwd_bump_pct, -vol_bump)?;

        // Vanna = (PV_up_up - PV_up_down - PV_down_up + PV_down_down) / (4 * fwd_bump * vol_bump)
        Ok((pv_up_up - pv_up_down - pv_down_up + pv_down_down) / (4.0 * fwd_bump_size * vol_bump))
    }
}
