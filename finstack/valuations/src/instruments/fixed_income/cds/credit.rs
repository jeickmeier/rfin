//! Shared credit reference parameters used across CDS-related instruments.

use finstack_core::F;

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
    /// Create credit parameters
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

    /// ISDA standard senior unsecured parameters (40% recovery)
    pub fn senior_unsecured(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.4, credit_id)
    }

    /// ISDA standard subordinated parameters (20% recovery)
    pub fn subordinated(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.2, credit_id)
    }

    /// Standard investment grade parameters (40% recovery) - alias for senior_unsecured
    pub fn investment_grade(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::senior_unsecured(reference_entity, credit_id)
    }

    /// High yield parameters (30% recovery)
    pub fn high_yield(reference_entity: impl Into<String>, credit_id: &'static str) -> Self {
        Self::new(reference_entity, 0.3, credit_id)
    }
}
