//! Gamma calculator for commodity options.
//!
//! Computes gamma (second derivative of price with respect to underlying) using
//! finite differences. When a spot_price_id is present, bumps the spot scalar.
//! Otherwise, bumps the PriceCurve referenced by forward_curve_id.

use crate::instruments::commodity::commodity_option::CommodityOption;
use crate::metrics::bump_sizes;
use crate::metrics::{bump_scalar_price, MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::types::CurveId;
use finstack_core::Result;

/// Gamma calculator for commodity options.
///
/// Supports two scenarios:
/// - If `spot_price_id` is set: bumps the spot price scalar
/// - If no spot: bumps the `PriceCurve` (parallel percent bump)
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CommodityOption = context.instrument_as()?;
        let as_of = context.as_of;
        let bump_pct = bump_sizes::SPOT;

        // Base PV
        let pv_base = option.npv(&context.curves, as_of)?.amount();

        // Determine spot price for bump size calculation
        let spot_price = if let Some(ref spot_id) = option.spot_price_id {
            let scalar = context.curves.price(spot_id)?;
            match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
            }
        } else {
            // Use forward price from PriceCurve as proxy for spot
            option.forward_price(&context.curves, as_of)?
        };

        let bump_size = spot_price * bump_pct;

        if let Some(ref spot_id) = option.spot_price_id {
            // Bump spot scalar
            let curves_up = bump_scalar_price(&context.curves, spot_id, bump_pct)?;
            let pv_up = option.npv(&curves_up, as_of)?.amount();

            let curves_down = bump_scalar_price(&context.curves, spot_id, -bump_pct)?;
            let pv_down = option.npv(&curves_down, as_of)?.amount();

            // Gamma = (PV_up - 2*PV_base + PV_down) / bump_size^2
            Ok((pv_up - 2.0 * pv_base + pv_down) / (bump_size * bump_size))
        } else {
            // Bump PriceCurve (parallel percent)
            let price_curve_id = CurveId::new(option.forward_curve_id.as_str());

            // Up bump (percent expressed as decimal: 0.01 = 1%)
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

            // Down bump
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

            // Gamma = (PV_up - 2*PV_base + PV_down) / bump_size^2
            Ok((pv_up - 2.0 * pv_base + pv_down) / (bump_size * bump_size))
        }
    }
}
