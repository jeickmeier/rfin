//! Interest rate convexity and cross-gamma calculators for interest rate swaps.
//!
//! # IR Convexity (Parallel Gamma)
//!
//! Second derivative of PV with respect to parallel rate curve shifts:
//! ```text
//! IR Convexity = d²PV / dr² ≈ (PV(+h) + PV(-h) - 2×PV_base) / h²
//! ```
//!
//! # Cross-Gamma (Discount vs Forward)
//!
//! Mixed second derivative measuring how DV01 changes when the other curve moves:
//! ```text
//! CrossGamma = d²PV / (dr_disc × dr_fwd)
//!            ≈ (PV(d+,f+) - PV(d+,f-) - PV(d-,f+) + PV(d-,f-)) / (4 × h_d × h_f)
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

/// Parallel IR convexity (second-order rate sensitivity).
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

/// Cross-gamma between discount and forward curves.
///
/// Uses central mixed finite differences:
/// `d²PV/(dr_disc × dr_fwd) ≈ (PV(d+,f+) - PV(d+,f-) - PV(d-,f+) + PV(d-,f-)) / (4hk)`
///
/// Returns 0.0 when the swap is single-curve (discount == forward) since the
/// mixed derivative is not meaningful in that case.
pub struct CrossGammaCalculator;

impl MetricCalculator for CrossGammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;
        let bump_bp = RATE_BUMP_BP;

        if irs.fixed.discount_curve_id == irs.float.forward_curve_id {
            return Ok(0.0);
        }

        let disc_id = &irs.fixed.discount_curve_id;
        let fwd_id = &irs.float.forward_curve_id;

        let disc_exists = context.curves.get_discount(disc_id.as_str()).is_ok();
        let fwd_exists = context.curves.get_forward(fwd_id.as_str()).is_ok();
        if !disc_exists || !fwd_exists {
            return Ok(0.0);
        }

        let bump_disc_up = vec![MarketBump::Curve {
            id: disc_id.clone(),
            spec: BumpSpec::parallel_bp(bump_bp),
        }];
        let bump_disc_down = vec![MarketBump::Curve {
            id: disc_id.clone(),
            spec: BumpSpec::parallel_bp(-bump_bp),
        }];
        let bump_fwd_up = MarketBump::Curve {
            id: fwd_id.clone(),
            spec: BumpSpec::parallel_bp(bump_bp),
        };
        let bump_fwd_down = MarketBump::Curve {
            id: fwd_id.clone(),
            spec: BumpSpec::parallel_bp(-bump_bp),
        };

        let ctx_disc_up = context.curves.bump(bump_disc_up)?;
        let ctx_disc_down = context.curves.bump(bump_disc_down)?;

        let pv_du_fu = irs.value_raw(&ctx_disc_up.bump(vec![bump_fwd_up.clone()])?, as_of)?;
        let pv_du_fd = irs.value_raw(&ctx_disc_up.bump(vec![bump_fwd_down.clone()])?, as_of)?;
        let pv_dd_fu = irs.value_raw(&ctx_disc_down.bump(vec![bump_fwd_up])?, as_of)?;
        let pv_dd_fd = irs.value_raw(&ctx_disc_down.bump(vec![bump_fwd_down])?, as_of)?;

        let h = bump_bp * 1e-4;
        let cross_gamma = (pv_du_fu - pv_du_fd - pv_dd_fu + pv_dd_fd) / (4.0 * h * h);

        Ok(cross_gamma)
    }
}
