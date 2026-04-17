//! Asset correlation shock adapter for structured credit instruments.
//!
//! This module provides adapters for shocking asset correlation parameters
//! in structured credit instruments, enabling scenario analysis of:
//!
//! - **Asset correlation**: Correlation between underlying asset defaults
//! - **Prepay-default correlation**: Correlation between prepayment and default
//! - **Recovery-default correlation**: Correlation between recovery and defaults
//!
//! # Usage
//!
//! ```rust,no_run
//! use finstack_scenarios::adapters::asset_corr::*;
//! use finstack_valuations::instruments::fixed_income::structured_credit::StructuredCredit;
//!
//! # fn main() -> finstack_scenarios::Result<()> {
//! # let mut instruments: Vec<StructuredCredit> = Vec::new();
//! // Shock all asset correlations by +5%
//! let (_count, _warnings) = apply_asset_correlation_shock(&mut instruments, 0.05)?;
//!
//! // Shock prepay-default correlation by -10%
//! let (_count, _warnings) = apply_prepay_default_correlation_shock(&mut instruments, -0.10)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Clamping
//!
//! Correlation values are clamped to valid ranges after shocks:
//! - Asset correlation: [0, 0.99]
//! - Prepay-default correlation: [-0.99, 0.99]
//! - Recovery correlation: [-0.99, 0.99]

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_valuations::instruments::fixed_income::structured_credit::CorrelationStructure;
use finstack_valuations::instruments::fixed_income::structured_credit::StructuredCredit;

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
            // RecoveryCorrelationPts / PrepayFactorLoadingPts are rejected at
            // OperationSpec::validate() time; see spec.rs for rationale.
            // Any engine path that bypasses validate() would fall through to
            // `_ => Ok(None)` and be reported as "unhandled operation" rather
            // than silently no-op'd with an opaque discriminant warning.
            _ => Ok(None),
        }
    }
}

/// Apply a parallel shock to asset correlation across structured credit instruments.
///
/// # Arguments
/// - `instruments`: Mutable slice of structured credit instruments
/// - `shock_points`: Additive shock in absolute correlation points (e.g., 0.05 for +5%)
///
/// # Returns
/// Result containing modification count and any warnings.
///
/// # Clamping
/// Asset correlation is clamped to [0, 0.99] after the shock. The clamp check
/// happens at the *underlying field* level (asset_correlation, or
/// intra/inter-sector for sectored structures) rather than at the aggregated
/// average, so the warning channel is not polluted with false positives for
/// sectored structures.
pub fn apply_asset_correlation_shock(
    instruments: &mut [StructuredCredit],
    shock_points: f64,
) -> Result<(usize, Vec<String>)> {
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

    Ok((modified, warnings))
}

/// Apply a parallel shock to prepay-default correlation.
///
/// # Arguments
/// - `instruments`: Mutable slice of structured credit instruments
/// - `shock_points`: Additive shock in absolute correlation points
///
/// # Returns
/// Result containing modification count and any warnings.
///
/// # Clamping
/// Prepay-default correlation is clamped to [-0.99, 0.99] after the shock.
pub fn apply_prepay_default_correlation_shock(
    instruments: &mut [StructuredCredit],
    shock_points: f64,
) -> Result<(usize, Vec<String>)> {
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

    Ok((modified, warnings))
}

/// Apply correlation shock by instrument selector (attribute-based).
///
/// # Arguments
/// - `instruments`: Mutable slice of structured credit instruments
/// - `selector`: Closure that returns true for instruments to modify
/// - `asset_corr_shock`: Shock to asset correlation (None to skip)
/// - `prepay_default_shock`: Shock to prepay-default correlation (None to skip)
///
/// # Returns
/// Result containing modification count and any warnings.
pub fn apply_selective_correlation_shock<F>(
    instruments: &mut [StructuredCredit],
    selector: F,
    asset_corr_shock: Option<f64>,
    prepay_default_shock: Option<f64>,
) -> Result<(usize, Vec<String>)>
where
    F: Fn(&StructuredCredit) -> bool,
{
    let mut modified = 0;
    let mut warnings = Vec::new();

    for inst in instruments.iter_mut() {
        if !selector(inst) {
            continue;
        }

        if let Some(ref corr_structure) = inst.credit_model.correlation_structure {
            let mut new_structure = corr_structure.clone();

            if let Some(shock) = asset_corr_shock {
                let (bumped, clamp_info) = new_structure.bump_asset_with_clamp_info(shock);
                if let Some(info) = clamp_info {
                    warnings.push(format!("Asset correlation bump for '{}': {info}", inst.id));
                }
                new_structure = bumped;
            }

            if let Some(shock) = prepay_default_shock {
                let (bumped, clamp_info) = new_structure.bump_prepay_default_with_clamp_info(shock);
                if let Some(info) = clamp_info {
                    warnings.push(format!(
                        "Prepay-default correlation bump for '{}': {info}",
                        inst.id
                    ));
                }
                new_structure = bumped;
            }

            inst.credit_model.correlation_structure = Some(new_structure);
            modified += 1;
        }
    }

    Ok((modified, warnings))
}

// === Internal helpers ===
// Note: bump_asset_correlation and bump_prepay_default_correlation
// now use methods on CorrelationStructure directly

/// Set correlation structure to a specific configuration.
///
/// # Arguments
///
/// - `instruments`: Structured-credit instruments to update.
/// - `structure`: Correlation structure to clone onto each instrument.
///
/// # Returns
///
/// The number of instruments updated.
pub fn set_correlation_structure(
    instruments: &mut [StructuredCredit],
    structure: CorrelationStructure,
) -> Result<usize> {
    let mut count = 0;
    for inst in instruments.iter_mut() {
        inst.credit_model.correlation_structure = Some(structure.clone());
        count += 1;
    }
    Ok(count)
}

/// Enable stochastic pricing for instruments with default configurations.
///
/// Instruments that are already stochastic are left unchanged.
///
/// # Arguments
///
/// - `instruments`: Structured-credit instruments to inspect and update.
///
/// # Returns
///
/// The number of instruments newly switched to stochastic pricing.
pub fn enable_stochastic_pricing(instruments: &mut [StructuredCredit]) -> Result<usize> {
    let mut count = 0;
    for inst in instruments.iter_mut() {
        if !inst.is_stochastic() {
            inst.enable_stochastic_defaults();
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_instrument() -> StructuredCredit {
        // Create a minimal structured credit instrument for testing using the example method
        let mut inst = StructuredCredit::example();

        // Set correlation structure
        inst.credit_model.correlation_structure = Some(CorrelationStructure::flat(0.20, -0.30));
        inst
    }

    #[test]
    fn test_asset_correlation_shock() {
        let mut instruments = vec![sample_instrument()];

        let (modified, _) =
            apply_asset_correlation_shock(&mut instruments, 0.05).expect("should succeed");

        assert_eq!(modified, 1);

        // New correlation should be ~25%
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

        // Large shock that would push past 99%
        let (_, warnings) =
            apply_asset_correlation_shock(&mut instruments, 0.90).expect("should succeed");

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

        // Shock prepay-default correlation by +0.10 (from -30% to -20%)
        let (modified, _) =
            apply_prepay_default_correlation_shock(&mut instruments, 0.10).expect("should succeed");

        assert_eq!(modified, 1);

        let new_corr = instruments[0]
            .credit_model
            .correlation_structure
            .as_ref()
            .expect("should have correlation")
            .prepay_default_correlation();

        assert!((new_corr - (-0.20)).abs() < 0.01);
    }

    #[test]
    fn test_selective_shock() {
        let mut instruments = vec![sample_instrument(), sample_instrument()];

        // Only shock first instrument
        let count = std::cell::Cell::new(0);
        let (modified, _) = apply_selective_correlation_shock(
            &mut instruments,
            |_| {
                let c = count.get();
                count.set(c + 1);
                c == 0
            },
            Some(0.05),
            None,
        )
        .expect("should succeed");

        assert_eq!(modified, 1);
    }

    #[test]
    fn test_enable_stochastic_pricing() {
        // Start with a non-stochastic instrument (no correlation_structure set)
        let mut instruments = vec![StructuredCredit::example()];

        // Verify it starts as non-stochastic
        assert!(!instruments[0].is_stochastic());

        let count = enable_stochastic_pricing(&mut instruments).expect("should succeed");

        assert_eq!(count, 1);
        assert!(instruments[0].is_stochastic());
    }
}
