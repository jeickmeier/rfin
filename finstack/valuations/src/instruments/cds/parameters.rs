//! Shared credit reference parameters used across CDS-related instruments.
//!
//! `CreditParams` captures the reference entity and recovery assumption used
//! for default settlement. Keep this lightweight and serde-stable; prefer
//! constructing with `new` or one of the standard presets.

use finstack_core::F;

/// ISDA-style recovery rate presets (named constants; do not hard-code literals).
pub const RECOVERY_SENIOR_UNSECURED: F = 0.40;
pub const RECOVERY_SUBORDINATED: F = 0.20;
/// Convenience non-ISDA preset for high-yield style assumptions (common heuristic).
pub const RECOVERY_HIGH_YIELD_DEFAULT: F = 0.30;

/// Credit parameters for CDS and credit options.
#[derive(Clone, Debug)]
pub struct CreditParams {
    /// Reference entity name
    pub reference_entity: String,
    /// Recovery rate assumption
    pub recovery_rate: F,
    /// Credit/hazard curve identifier
    pub credit_id: &'static str,
}

impl CreditParams {
    /// Create credit parameters with an explicit recovery rate.
    ///
    /// - `reference_entity`: issuer or name of the protection reference
    /// - `recovery_rate`: fraction in [0,1] (e.g., 0.40 for 40%)
    /// - `credit_id`: identifier of the hazard/credit curve
    pub fn new(
        reference_entity: impl Into<String>,
        recovery_rate: F,
        credit_id: &'static str,
    ) -> Self {
        Self {
            reference_entity: reference_entity.into(),
            recovery_rate,
            credit_id,
        }
    }

    /// ISDA standard senior unsecured parameters (40% recovery).
    pub fn senior_unsecured(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, RECOVERY_SENIOR_UNSECURED, credit_id)
    }

    /// ISDA standard subordinated parameters (20% recovery).
    pub fn subordinated(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, RECOVERY_SUBORDINATED, credit_id)
    }

    /// High yield parameters (30% recovery).
    pub fn high_yield(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, RECOVERY_HIGH_YIELD_DEFAULT, credit_id)
    }
}
