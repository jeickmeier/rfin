//! Gamma calculator for commodity options.
//!
//! Computes gamma (second derivative of price with respect to the forward/futures price)
//! using finite differences. This is forward-based gamma, consistent with Black-76:
//!
//! - If `quoted_forward` is set: bumps the instrument's quoted forward override
//! - Else if a `PriceCurve` exists: bumps the PriceCurve (parallel percent bump)
//! - Only as fallback: bumps `spot_price_id` (if present) to propagate via cost-of-carry

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::instruments::common::traits::InstrumentNpvExt;
use crate::metrics::bump_sizes;
use crate::metrics::{bump_scalar_price, MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Forward-based gamma calculator for commodity options.
///
/// Computes gamma with respect to the forward/futures price, consistent with Black-76.
/// The bump target priority is:
/// 1. `quoted_forward` (instrument override)
/// 2. `PriceCurve` (market data)
/// 3. `spot_price_id` (cost-of-carry fallback)
pub struct GammaCalculator;

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

impl GammaCalculator {
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
            "Cannot compute gamma: no quoted_forward, PriceCurve, or spot_price_id available"
                .to_string(),
        ))
    }
}

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CommodityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let bump_pct = bump_sizes::SPOT;

        // Determine what drives the forward price and how to bump it
        let driver = Self::determine_driver(option, context)?;

        // Get base forward price for bump size calculation
        let forward_price = option.forward_price(&context.curves, as_of)?;
        let bump_size = forward_price * bump_pct;

        // Base PV
        let pv_base = option.npv(&context.curves, as_of)?.amount();

        let (pv_up, pv_down) = match driver {
            ForwardDriver::QuotedForward(fwd) => {
                // Clone option and bump the quoted_forward field
                let mut option_up = option.clone();
                option_up.quoted_forward = Some(fwd * (1.0 + bump_pct));
                let pv_up = option_up.npv(&context.curves, as_of)?.amount();

                let mut option_down = option.clone();
                option_down.quoted_forward = Some(fwd * (1.0 - bump_pct));
                let pv_down = option_down.npv(&context.curves, as_of)?.amount();

                (pv_up, pv_down)
            }
            ForwardDriver::PriceCurve => {
                // Bump PriceCurve (parallel percent bump)
                let price_curve_id = CurveId::new(option.forward_curve_id.as_str());

                let bump_up = MarketBump::Curve {
                    id: price_curve_id.clone(),
                    spec: BumpSpec {
                        bump_type: BumpType::Parallel,
                        mode: BumpMode::Additive,
                        units: BumpUnits::Percent,
                        value: bump_pct * 100.0, // Convert to percent units
                    },
                };
                let curves_up = context.curves.bump([bump_up])?;
                let pv_up = option.npv(&curves_up, as_of)?.amount();

                let bump_down = MarketBump::Curve {
                    id: price_curve_id,
                    spec: BumpSpec {
                        bump_type: BumpType::Parallel,
                        mode: BumpMode::Additive,
                        units: BumpUnits::Percent,
                        value: -bump_pct * 100.0,
                    },
                };
                let curves_down = context.curves.bump([bump_down])?;
                let pv_down = option.npv(&curves_down, as_of)?.amount();

                (pv_up, pv_down)
            }
            ForwardDriver::SpotScalar(ref spot_id) => {
                // Bump spot scalar (cost-of-carry fallback)
                let curves_up = bump_scalar_price(&context.curves, spot_id, bump_pct)?;
                let pv_up = option.npv(&curves_up, as_of)?.amount();

                let curves_down = bump_scalar_price(&context.curves, spot_id, -bump_pct)?;
                let pv_down = option.npv(&curves_down, as_of)?.amount();

                (pv_up, pv_down)
            }
        };

        // Gamma = (PV_up - 2*PV_base + PV_down) / bump_size^2
        Ok((pv_up - 2.0 * pv_base + pv_down) / (bump_size * bump_size))
    }
}
