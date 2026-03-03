//! Interest rate convexity (gamma) calculator for interest rate swaps.
//!
//! Calculates the second derivative of the swap PV with respect to parallel
//! rate curve shifts using central finite differences.
//!
//! # Mathematical Definition
//!
//! ```text
//! IR Convexity = d²PV / dr² ≈ (PV(+h) + PV(-h) - 2×PV_base) / h²
//! ```
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives*. Chapter 7.
//! - Tuckman, B., & Serrat, A. (2011). *Fixed Income Securities*. Chapter 5.

use crate::instruments::common_impl::traits::{CurveDependencies, Instrument, RatesCurveKind};
use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::Result;

const RATE_BUMP_BP: f64 = 1.0;

pub struct IrConvexityCalculator;

impl MetricCalculator for IrConvexityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;
        let bump_bp = RATE_BUMP_BP;

        let base_pv = irs.value_raw(context.curves.as_ref(), as_of)?;

        let deps = irs.curve_dependencies()?;
        let mut bumps_up: Vec<MarketBump> = Vec::new();
        let mut bumps_down: Vec<MarketBump> = Vec::new();

        for (curve_id, kind) in deps.all_with_kind() {
            let exists = match kind {
                RatesCurveKind::Discount => context.curves.get_discount(curve_id.as_str()).is_ok(),
                RatesCurveKind::Forward => context.curves.get_forward(curve_id.as_str()).is_ok(),
                RatesCurveKind::Credit => false,
            };
            if !exists {
                continue;
            }
            bumps_up.push(MarketBump::Curve {
                id: curve_id.clone(),
                spec: BumpSpec::parallel_bp(bump_bp),
            });
            bumps_down.push(MarketBump::Curve {
                id: curve_id.clone(),
                spec: BumpSpec::parallel_bp(-bump_bp),
            });
        }

        if bumps_up.is_empty() {
            return Ok(0.0);
        }

        let curves_up = context.curves.bump(bumps_up)?;
        let pv_up = irs.value_raw(&curves_up, as_of)?;

        let curves_down = context.curves.bump(bumps_down)?;
        let pv_down = irs.value_raw(&curves_down, as_of)?;

        let h = bump_bp * 1e-4;
        let convexity = (pv_up + pv_down - 2.0 * base_pv) / (h * h);

        Ok(convexity)
    }
}
