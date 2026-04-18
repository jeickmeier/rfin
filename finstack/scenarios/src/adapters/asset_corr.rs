//! Asset correlation shock adapter for structured credit instruments.
//!
//! This module provides the [`AssetCorrAdapter`] which translates the
//! [`OperationSpec::AssetCorrelationPts`] /
//! [`OperationSpec::PrepayDefaultCorrelationPts`] variants into
//! [`ScenarioEffect`]s consumed by the engine. The actual correlation bump is
//! applied by the engine via [`CorrelationStructure`] helpers; this module
//! keeps only the adapter and a small set of `#[cfg(test)]` helpers that
//! verify clamping behaviour on sample instruments.
//!
//! # Clamping
//!
//! Correlation values are clamped to valid ranges after shocks:
//! - Asset correlation: [0, 0.99]
//! - Prepay-default correlation: [-0.99, 0.99]

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;

/// Adapter for asset correlation operations.
pub struct AssetCorrAdapter;

impl ScenarioAdapter for AssetCorrAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        _ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::AssetCorrelationPts { delta_pts } => {
                Ok(Some(vec![ScenarioEffect::AssetCorrelationShock {
                    delta_pts: *delta_pts,
                }]))
            }
            OperationSpec::PrepayDefaultCorrelationPts { delta_pts } => {
                Ok(Some(vec![ScenarioEffect::PrepayDefaultCorrelationShock {
                    delta_pts: *delta_pts,
                }]))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use finstack_valuations::instruments::fixed_income::structured_credit::{
        CorrelationStructure, StructuredCredit,
    };

    /// Test-only helper: apply a parallel shock to asset correlation.
    ///
    /// Clamps to `[0, 0.99]` and reports any clamp events via the returned
    /// warnings vector. Only used by the clamping/parity tests in this module.
    fn apply_asset_correlation_shock(
        instruments: &mut [StructuredCredit],
        shock_points: f64,
    ) -> (usize, Vec<String>) {
        let mut modified = 0;
        let mut warnings = Vec::new();

        for inst in instruments.iter_mut() {
            if let Some(ref corr_structure) = inst.credit_model.correlation_structure {
                let (new_corr_structure, clamp_info) =
                    corr_structure.bump_asset_with_clamp_info(shock_points);
                if let Some(info) = clamp_info {
                    warnings.push(format!("Asset correlation bump for '{}': {info}", inst.id));
                }
                inst.credit_model.correlation_structure = Some(new_corr_structure);
                modified += 1;
            }
        }

        (modified, warnings)
    }

    /// Test-only helper: apply a parallel shock to prepay-default correlation.
    ///
    /// Clamps to `[-0.99, 0.99]` and reports any clamp events via the returned
    /// warnings vector. Only used by the clamping/parity tests in this module.
    fn apply_prepay_default_correlation_shock(
        instruments: &mut [StructuredCredit],
        shock_points: f64,
    ) -> (usize, Vec<String>) {
        let mut modified = 0;
        let mut warnings = Vec::new();

        for inst in instruments.iter_mut() {
            if let Some(ref corr_structure) = inst.credit_model.correlation_structure {
                let (new_corr_structure, clamp_info) =
                    corr_structure.bump_prepay_default_with_clamp_info(shock_points);
                if let Some(info) = clamp_info {
                    warnings.push(format!(
                        "Prepay-default correlation bump for '{}': {info}",
                        inst.id
                    ));
                }
                inst.credit_model.correlation_structure = Some(new_corr_structure);
                modified += 1;
            }
        }

        (modified, warnings)
    }

    fn sample_instrument() -> StructuredCredit {
        let mut inst = StructuredCredit::example();
        inst.credit_model.correlation_structure = Some(CorrelationStructure::flat(0.20, -0.30));
        inst
    }

    #[test]
    fn test_asset_correlation_shock() {
        let mut instruments = vec![sample_instrument()];
        let (modified, _) = apply_asset_correlation_shock(&mut instruments, 0.05);

        assert_eq!(modified, 1);

        let new_corr = instruments[0]
            .credit_model
            .correlation_structure
            .as_ref()
            .expect("should have correlation")
            .asset_correlation();
        assert!((new_corr - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_asset_correlation_clamping() {
        let mut instruments = vec![sample_instrument()];
        let (_, warnings) = apply_asset_correlation_shock(&mut instruments, 0.90);

        assert!(!warnings.is_empty(), "Should have clamping warning");
        let new_corr = instruments[0]
            .credit_model
            .correlation_structure
            .as_ref()
            .expect("should have correlation")
            .asset_correlation();
        assert!(new_corr <= 0.99);
    }

    #[test]
    fn test_prepay_default_correlation_shock() {
        let mut instruments = vec![sample_instrument()];
        let (modified, _) = apply_prepay_default_correlation_shock(&mut instruments, 0.10);

        assert_eq!(modified, 1);
        let new_corr = instruments[0]
            .credit_model
            .correlation_structure
            .as_ref()
            .expect("should have correlation")
            .prepay_default_correlation();
        assert!((new_corr - (-0.20)).abs() < 0.01);
    }
}
