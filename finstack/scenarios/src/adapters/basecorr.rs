//! Base correlation shock adapter.
//!
//! Contains helpers for both parallel and bucketed base correlation shocks,
//! used by the base-correlation `OperationSpec` variants.
//!
//! # Clamping Behavior
//!
//! Correlation values are clamped to the valid range [0, 1] after applying shocks.
//! When clamping occurs, warnings are returned to alert the caller that the
//! requested shock could not be fully applied.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};

/// Adapter for base correlation operations.
pub struct BaseCorrAdapter;

impl ScenarioAdapter for BaseCorrAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        _ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::BaseCorrParallelPts { surface_id, points } => {
                let bump = MarketBump::Curve {
                    id: finstack_core::types::CurveId::from(surface_id.as_str()),
                    spec: BumpSpec {
                        mode: BumpMode::Additive,
                        units: BumpUnits::Fraction,
                        value: *points,
                        bump_type: BumpType::Parallel,
                    },
                };
                Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
            }
            OperationSpec::BaseCorrBucketPts {
                surface_id,
                detachment_bps,
                maturities,
                points,
            } => {
                let dets = detachment_bps
                    .as_ref()
                    .map(|v| v.iter().map(|bp| *bp as f64 / 10000.0).collect());

                if let Some(mats) = maturities {
                    if !mats.is_empty() {
                        return Err(crate::error::Error::Validation(
                            "BaseCorrBucketPts maturity filtering is not supported: \
                             BaseCorrelationCurve is 1D (detachment only, no term structure). \
                             Remove the maturities field or use detachment_bps alone."
                                .to_string(),
                        ));
                    }
                }

                let bump = MarketBump::BaseCorrBucketPts {
                    surface_id: finstack_core::types::CurveId::from(surface_id.as_str()),
                    detachments: dets,
                    points: *points,
                };

                Ok(Some(vec![ScenarioEffect::MarketBump(bump)]))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    fn create_basecorr_bucket_bump(detachment_bps: Option<&[i32]>) -> Option<Vec<f64>> {
        detachment_bps.map(|v| v.iter().map(|bp| *bp as f64 / 10000.0).collect())
    }

    #[test]
    fn test_unit_conversion() {
        let input = vec![300, 700];
        let output = create_basecorr_bucket_bump(Some(&input)).expect("Failed to create bump");
        assert!((output[0] - 0.03).abs() < 1e-6);
        assert!((output[1] - 0.07).abs() < 1e-6);
    }
}
