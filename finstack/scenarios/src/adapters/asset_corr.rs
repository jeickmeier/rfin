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
//! ```ignore
//! use finstack_scenarios::adapters::asset_corr::*;
//!
//! // Shock all asset correlations by +5%
//! let (count, warnings) = apply_asset_correlation_shock(&mut instruments, 0.05)?;
//!
//! // Shock prepay-default correlation by -10%
//! let (count, warnings) = apply_prepay_default_correlation_shock(&mut instruments, -0.10)?;
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
use finstack_valuations::instruments::structured_credit::pricing::stochastic::CorrelationStructure;
use finstack_valuations::instruments::structured_credit::StructuredCredit;

/// Adapter for asset correlation operations.
pub struct AssetCorrAdapter;

impl ScenarioAdapter for AssetCorrAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        _ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::AssetCorrelationPts { .. }
            | OperationSpec::PrepayDefaultCorrelationPts { .. }
            | OperationSpec::RecoveryCorrelationPts { .. }
            | OperationSpec::PrepayFactorLoadingPts { .. } => {
                // Currently returning warning as engine only holds generic instruments
                Ok(Some(vec![ScenarioEffect::Warning(
                    "Correlation operation: requires StructuredCredit instruments (not supported in generic context)".to_string(),
                )]))
            }
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
/// Asset correlation is clamped to [0, 0.99] after the shock.
pub fn apply_asset_correlation_shock(
    instruments: &mut [StructuredCredit],
    shock_points: f64,
) -> Result<(usize, Vec<String>)> {
    let mut modified = 0;
    let mut warnings = Vec::new();

    for inst in instruments.iter_mut() {
        if let Some(ref corr_structure) = inst.correlation_structure {
            let original = corr_structure.asset_correlation();
            let new_corr_structure = corr_structure.bump_asset(shock_points);
            let new = new_corr_structure.asset_correlation();

            // Check for clamping
            let expected = original + shock_points;
            if (new - expected).abs() > 0.001 {
                warnings.push(format!(
                    "Asset correlation clamped for '{}': requested {:.2}%, got {:.2}%",
                    inst.id.as_str(),
                    expected * 100.0,
                    new * 100.0
                ));
            }

            inst.correlation_structure = Some(new_corr_structure);
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
        if let Some(ref corr_structure) = inst.correlation_structure {
            let original = corr_structure.prepay_default_correlation();
            let new_corr_structure = corr_structure.bump_prepay_default(shock_points);
            let new = new_corr_structure.prepay_default_correlation();

            // Check for clamping
            let expected = original + shock_points;
            if (new - expected).abs() > 0.001 {
                warnings.push(format!(
                    "Prepay-default correlation clamped for '{}': requested {:.2}%, got {:.2}%",
                    inst.id.as_str(),
                    expected * 100.0,
                    new * 100.0
                ));
            }

            inst.correlation_structure = Some(new_corr_structure);
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

        if let Some(ref corr_structure) = inst.correlation_structure {
            let mut new_structure = corr_structure.clone();

            // Apply asset correlation shock
            if let Some(shock) = asset_corr_shock {
                let original = new_structure.asset_correlation();
                new_structure = new_structure.bump_asset(shock);
                let new = new_structure.asset_correlation();

                let expected = original + shock;
                if (new - expected).abs() > 0.001 {
                    warnings.push(format!(
                        "Asset correlation clamped for '{}': {:.2}% -> {:.2}% (requested {:.2}%)",
                        inst.id.as_str(),
                        original * 100.0,
                        new * 100.0,
                        expected * 100.0
                    ));
                }
            }

            // Apply prepay-default correlation shock
            if let Some(shock) = prepay_default_shock {
                let original = new_structure.prepay_default_correlation();
                new_structure = new_structure.bump_prepay_default(shock);
                let new = new_structure.prepay_default_correlation();

                let expected = original + shock;
                if (new - expected).abs() > 0.001 {
                    warnings.push(format!(
                        "Prepay-default correlation clamped for '{}': {:.2}% -> {:.2}% (requested {:.2}%)",
                        inst.id.as_str(),
                        original * 100.0,
                        new * 100.0,
                        expected * 100.0
                    ));
                }
            }

            inst.correlation_structure = Some(new_structure);
            modified += 1;
        }
    }

    Ok((modified, warnings))
}

// === Internal helpers ===
// Note: bump_asset_correlation and bump_prepay_default_correlation
// now use methods on CorrelationStructure directly

/// Set correlation structure to a specific configuration.
pub fn set_correlation_structure(
    instruments: &mut [StructuredCredit],
    structure: CorrelationStructure,
) -> Result<usize> {
    let mut count = 0;
    for inst in instruments.iter_mut() {
        inst.correlation_structure = Some(structure.clone());
        count += 1;
    }
    Ok(count)
}

/// Enable stochastic pricing for instruments with default configurations.
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
mod tests {
    use super::*;

    fn sample_instrument() -> StructuredCredit {
        // Create a minimal structured credit instrument for testing using the example method
        let mut inst = StructuredCredit::example();

        // Set correlation structure
        inst.correlation_structure = Some(CorrelationStructure::flat(0.20, -0.30));
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
