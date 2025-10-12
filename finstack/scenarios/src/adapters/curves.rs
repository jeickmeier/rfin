//! Curve shock adapters (discount, forecast, hazard, inflation).

use crate::error::{Error, Result};
use crate::spec::CurveKind;
use finstack_core::market_data::bumps::{BumpSpec, Bumpable};
use finstack_core::market_data::MarketContext;

/// Apply parallel bp shock to a curve.
pub fn apply_curve_parallel_shock(
    market: &mut MarketContext,
    curve_kind: CurveKind,
    curve_id: &str,
    bp: f64,
) -> Result<()> {
    let bump_spec = BumpSpec::parallel_bp(bp);

    match curve_kind {
        CurveKind::Discount => {
            let curve = market.get_discount_ref(curve_id).map_err(|_| {
                Error::MarketDataNotFound {
                    id: curve_id.to_string(),
                }
            })?;
            let bumped = curve.apply_bump(bump_spec).ok_or_else(|| {
                Error::UnsupportedOperation {
                    operation: format!("parallel bp={}", bp),
                    target: format!("discount curve {}", curve_id),
                }
            })?;
            market.insert_discount_mut(std::sync::Arc::new(bumped));
        }
        CurveKind::Forecast => {
            let curve = market.get_forward_ref(curve_id).map_err(|_| {
                Error::MarketDataNotFound {
                    id: curve_id.to_string(),
                }
            })?;
            let bumped = curve.apply_bump(bump_spec).ok_or_else(|| {
                Error::UnsupportedOperation {
                    operation: format!("parallel bp={}", bp),
                    target: format!("forward curve {}", curve_id),
                }
            })?;
            market.insert_forward_mut(std::sync::Arc::new(bumped));
        }
        CurveKind::Hazard => {
            let curve = market.get_hazard_ref(curve_id).map_err(|_| {
                Error::MarketDataNotFound {
                    id: curve_id.to_string(),
                }
            })?;
            let bumped = curve.apply_bump(bump_spec).ok_or_else(|| {
                Error::UnsupportedOperation {
                    operation: format!("parallel bp={}", bp),
                    target: format!("hazard curve {}", curve_id),
                }
            })?;
            market.insert_hazard_mut(std::sync::Arc::new(bumped));
        }
        CurveKind::Inflation => {
            let curve = market.get_inflation_ref(curve_id).map_err(|_| {
                Error::MarketDataNotFound {
                    id: curve_id.to_string(),
                }
            })?;
            // Inflation curves use percent bumps, convert bp to pct
            let pct_bump = BumpSpec::inflation_shift_pct(bp / 100.0);
            let bumped = curve.apply_bump(pct_bump).ok_or_else(|| {
                Error::UnsupportedOperation {
                    operation: format!("inflation bump pct={}", bp / 100.0),
                    target: format!("inflation curve {}", curve_id),
                }
            })?;
            market.insert_inflation_mut(std::sync::Arc::new(bumped));
        }
    }

    Ok(())
}

/// Apply node-specific bp shocks to a curve.
///
/// Phase A: simplified implementation applies bumps sequentially to the same curve.
pub fn apply_curve_node_shock(
    market: &mut MarketContext,
    curve_kind: CurveKind,
    curve_id: &str,
    nodes: &[(String, f64)],
) -> Result<()> {
    // Phase A: apply each node shock as a separate parallel bump
    // Real implementation would need tenor-based node lookup and selective bumping
    for (tenor, bp) in nodes {
        // For now, apply as parallel bumps (simplified)
        // TODO: implement proper node-specific bumping with tenor matching
        let _ = tenor; // silence unused warning
        apply_curve_parallel_shock(market, curve_kind, curve_id, *bp)?;
    }

    Ok(())
}

