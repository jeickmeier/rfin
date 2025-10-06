//! Serializable wrappers for behavioral models.
//!
//! This module provides serializable enum types that can be converted to/from
//! trait objects, enabling full JSON serialization of structured credit instruments.

use std::sync::Arc;

use super::{
    default_models::{CDRModel, ConstantRecoveryModel, DefaultBehavior, RecoveryBehavior, SDAModel},
    prepayment::{PSAModel, PrepaymentBehavior},
};

// ============================================================================
// Prepayment Model Enum
// ============================================================================

/// Serializable prepayment model specification.
///
/// This enum can be serialized to/from JSON and converted to a trait object
/// for use in structured credit instruments.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum PrepaymentModelSpec {
    /// PSA (Public Securities Association) model with multiplier
    Psa {
        /// PSA multiplier (1.0 = 100% PSA)
        multiplier: f64,
    },
    /// Constant CPR (Conditional Prepayment Rate)
    ConstantCpr {
        /// Annual CPR rate (e.g., 0.06 for 6% CPR)
        cpr: f64,
    },
    /// Constant SMM (Single Monthly Mortality)
    ConstantSmm {
        /// Monthly SMM rate
        smm: f64,
    },
    /// Asset-type specific default model
    AssetDefault {
        /// Asset type: "auto", "student", "credit_card", "rmbs", "cmbs", "clo"
        asset_type: String,
    },
}

impl PrepaymentModelSpec {
    /// Convert to a trait object for use in instruments.
    pub fn to_arc(&self) -> Arc<dyn PrepaymentBehavior> {
        match self {
            PrepaymentModelSpec::Psa { multiplier } => {
                Arc::new(PSAModel::new(*multiplier))
            }
            PrepaymentModelSpec::ConstantCpr { cpr } => {
                Arc::from(super::prepayment::cpr_model(*cpr))
            }
            PrepaymentModelSpec::ConstantSmm { smm } => {
                // Convert SMM to CPR for storage
                let cpr = super::prepayment::smm_to_cpr(*smm);
                Arc::from(super::prepayment::cpr_model(cpr))
            }
            PrepaymentModelSpec::AssetDefault { asset_type } => {
                Arc::from(super::prepayment::prepayment_model_for(asset_type))
            }
        }
    }

    /// Create from a trait object (for serialization).
    ///
    /// Note: This is a best-effort conversion. Custom models will default to PSA(1.0).
    pub fn from_arc(model: &Arc<dyn PrepaymentBehavior>) -> Self {
        // Try to downcast to known types
        if let Some(psa) = model.as_any().downcast_ref::<PSAModel>() {
            PrepaymentModelSpec::Psa {
                multiplier: psa.multiplier(),
            }
        } else {
            // Default fallback
            PrepaymentModelSpec::Psa { multiplier: 1.0 }
        }
    }

    /// PSA model with 100% speed
    pub fn psa_100() -> Self {
        PrepaymentModelSpec::Psa { multiplier: 1.0 }
    }

    /// PSA model with 150% speed
    pub fn psa_150() -> Self {
        PrepaymentModelSpec::Psa { multiplier: 1.5 }
    }

    /// Constant 6% CPR
    pub fn cpr_6pct() -> Self {
        PrepaymentModelSpec::ConstantCpr { cpr: 0.06 }
    }
}

impl Default for PrepaymentModelSpec {
    fn default() -> Self {
        PrepaymentModelSpec::Psa { multiplier: 1.0 }
    }
}

// ============================================================================
// Default Model Enum
// ============================================================================

/// Serializable default model specification.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum DefaultModelSpec {
    /// SDA (Standard Default Assumption) model
    Sda {
        /// SDA multiplier (1.0 = 100% SDA)
        multiplier: f64,
    },
    /// Constant CDR (Conditional Default Rate)
    ConstantCdr {
        /// Annual CDR rate
        cdr: f64,
    },
    /// Constant MDR (Monthly Default Rate)
    ConstantMdr {
        /// Monthly MDR rate
        mdr: f64,
    },
    /// Asset-type specific default model
    AssetDefault {
        /// Asset type: "consumer", "corporate", "commercial", "rmbs", "cmbs", "clo"
        asset_type: String,
    },
}

impl DefaultModelSpec {
    /// Convert to a trait object for use in instruments.
    pub fn to_arc(&self) -> Arc<dyn DefaultBehavior> {
        match self {
            DefaultModelSpec::Sda { multiplier } => {
                Arc::new(SDAModel::new(*multiplier))
            }
            DefaultModelSpec::ConstantCdr { cdr } => {
                Arc::new(CDRModel::new(*cdr))
            }
            DefaultModelSpec::ConstantMdr { mdr } => {
                // Convert MDR to CDR
                let cdr = super::default_models::mdr_to_cdr(*mdr);
                Arc::new(CDRModel::new(cdr))
            }
            DefaultModelSpec::AssetDefault { asset_type } => {
                Arc::from(super::default_models::default_model_for(asset_type))
            }
        }
    }

    /// Create from a trait object (for serialization).
    pub fn from_arc(model: &Arc<dyn DefaultBehavior>) -> Self {
        // Try to downcast to known types
        if let Some(sda) = model.as_any().downcast_ref::<SDAModel>() {
            DefaultModelSpec::Sda {
                multiplier: sda.multiplier(),
            }
        } else if let Some(cdr) = model.as_any().downcast_ref::<CDRModel>() {
            DefaultModelSpec::ConstantCdr { cdr: cdr.cdr() }
        } else {
            // Default fallback
            DefaultModelSpec::Sda { multiplier: 1.0 }
        }
    }

    /// SDA model with 100% speed
    pub fn sda_100() -> Self {
        DefaultModelSpec::Sda { multiplier: 1.0 }
    }

    /// Constant 2% CDR
    pub fn cdr_2pct() -> Self {
        DefaultModelSpec::ConstantCdr { cdr: 0.02 }
    }
}

impl Default for DefaultModelSpec {
    fn default() -> Self {
        DefaultModelSpec::Sda { multiplier: 1.0 }
    }
}

// ============================================================================
// Recovery Model Enum
// ============================================================================

/// Serializable recovery model specification.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "snake_case"))]
pub enum RecoveryModelSpec {
    /// Constant recovery rate
    Constant {
        /// Recovery rate (0.0 to 1.0)
        rate: f64,
    },
    /// Asset-type specific recovery model
    AssetDefault {
        /// Asset type: "collateral", "mortgage", "commercial", "corporate", "unsecured"
        asset_type: String,
    },
}

impl RecoveryModelSpec {
    /// Convert to a trait object for use in instruments.
    pub fn to_arc(&self) -> Arc<dyn RecoveryBehavior> {
        match self {
            RecoveryModelSpec::Constant { rate } => {
                Arc::new(ConstantRecoveryModel::new(*rate))
            }
            RecoveryModelSpec::AssetDefault { asset_type } => {
                Arc::from(super::default_models::recovery_model_for(asset_type))
            }
        }
    }

    /// Create from a trait object (for serialization).
    pub fn from_arc(model: &Arc<dyn RecoveryBehavior>) -> Self {
        // Try to downcast to known types
        if let Some(constant) = model.as_any().downcast_ref::<ConstantRecoveryModel>() {
            RecoveryModelSpec::Constant {
                rate: constant.rate(),
            }
        } else {
            // Default fallback
            RecoveryModelSpec::Constant { rate: 0.4 }
        }
    }

    /// 40% recovery (typical for senior unsecured)
    pub fn recovery_40pct() -> Self {
        RecoveryModelSpec::Constant { rate: 0.4 }
    }

    /// 70% recovery (typical for secured/collateral)
    pub fn recovery_70pct() -> Self {
        RecoveryModelSpec::Constant { rate: 0.7 }
    }
}

impl Default for RecoveryModelSpec {
    fn default() -> Self {
        RecoveryModelSpec::Constant { rate: 0.4 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepayment_spec_roundtrip() {
        let spec = PrepaymentModelSpec::Psa { multiplier: 1.5 };
        let arc = spec.to_arc();
        let recovered = PrepaymentModelSpec::from_arc(&arc);
        
        match recovered {
            PrepaymentModelSpec::Psa { multiplier } => {
                assert!((multiplier - 1.5).abs() < 1e-10);
            }
            _ => panic!("Expected PSA model"),
        }
    }

    #[test]
    fn test_default_spec_roundtrip() {
        let spec = DefaultModelSpec::Sda { multiplier: 2.0 };
        let arc = spec.to_arc();
        let recovered = DefaultModelSpec::from_arc(&arc);
        
        match recovered {
            DefaultModelSpec::Sda { multiplier } => {
                assert!((multiplier - 2.0).abs() < 1e-10);
            }
            _ => panic!("Expected SDA model"),
        }
    }

    #[test]
    fn test_recovery_spec_roundtrip() {
        let spec = RecoveryModelSpec::Constant { rate: 0.6 };
        let arc = spec.to_arc();
        let recovered = RecoveryModelSpec::from_arc(&arc);
        
        match recovered {
            RecoveryModelSpec::Constant { rate } => {
                assert!((rate - 0.6).abs() < 1e-10);
            }
            _ => panic!("Expected Constant model"),
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_prepayment_spec_json() {
        let spec = PrepaymentModelSpec::Psa { multiplier: 1.5 };
        let json = serde_json::to_string(&spec).unwrap();
        let recovered: PrepaymentModelSpec = serde_json::from_str(&json).unwrap();
        
        match recovered {
            PrepaymentModelSpec::Psa { multiplier } => {
                assert!((multiplier - 1.5).abs() < 1e-10);
            }
            _ => panic!("Expected PSA model"),
        }
    }
}
