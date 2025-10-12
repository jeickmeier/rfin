//! Behavioral model specifications for structured credit instruments.
//!
//! This module provides serializable enum types with direct calculation methods
//! for prepayment, default, and recovery modeling. These specs are the single
//! source of truth for behavioral assumptions and serialize cleanly to JSON.

use super::market_context::{CreditFactors, MarketConditions, MarketFactors};
use crate::instruments::structured_credit::config::{
    PSA_RAMP_MONTHS, PSA_TERMINAL_CPR, SDA_PEAK_MONTH, SDA_PEAK_CDR, SDA_TERMINAL_CDR,
};

// ============================================================================
// Prepayment Model Specification
// ============================================================================

/// Serializable prepayment model specification.
///
/// This enum represents different prepayment modeling approaches and provides
/// direct calculation methods without requiring trait objects.
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
    /// Calculate prepayment rate (SMM) for this specification.
    ///
    /// This method evaluates the prepayment model directly, making it
    /// efficient and serialization-friendly.
    ///
    /// # Returns
    ///
    /// Single Monthly Mortality (SMM) rate - the monthly prepayment rate.
    pub fn prepayment_rate(
        &self,
        _as_of: finstack_core::dates::Date,
        _origination_date: finstack_core::dates::Date,
        seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        match self {
            PrepaymentModelSpec::Psa { multiplier } => {
                // PSA calculation per config constants
                let base_cpr = if seasoning_months <= PSA_RAMP_MONTHS {
                    (seasoning_months as f64 / PSA_RAMP_MONTHS as f64) * PSA_TERMINAL_CPR
                } else {
                    PSA_TERMINAL_CPR
                };
                let cpr = base_cpr * multiplier;
                // Convert CPR to SMM
                1.0 - (1.0 - cpr).powf(1.0 / 12.0)
            }
            PrepaymentModelSpec::ConstantCpr { cpr } => {
                super::rates::cpr_to_smm(*cpr)
            }
            PrepaymentModelSpec::ConstantSmm { smm } => *smm,
            PrepaymentModelSpec::AssetDefault { asset_type } => {
                // Asset-specific defaults
                match asset_type.to_lowercase().as_str() {
                    "mortgage" | "rmbs" => {
                        // 100% PSA default
                        let base_cpr = if seasoning_months <= PSA_RAMP_MONTHS {
                            (seasoning_months as f64 / PSA_RAMP_MONTHS as f64) * PSA_TERMINAL_CPR
                        } else {
                            PSA_TERMINAL_CPR
                        };
                        let cpr = base_cpr * 1.0; // 100% PSA
                        1.0 - (1.0 - cpr).powf(1.0 / 12.0)
                    }
                    "auto" | "abs_auto" => super::rates::cpr_to_smm(0.18),
                    "card" | "credit_card" | "cc" => super::rates::cpr_to_smm(0.15),
                    "commercial" | "cmbs" | "cre" => super::rates::cpr_to_smm(0.10),
                    "student" | "student_loan" => super::rates::cpr_to_smm(0.03),
                    _ => super::rates::cpr_to_smm(0.05),
                }
            }
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
// Default Model Specification
// ============================================================================

/// Serializable default model specification.
///
/// This enum represents different default modeling approaches for structured
/// credit instruments.
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
    /// Calculate default rate (MDR) for this specification.
    ///
    /// This method evaluates the default model directly, making it
    /// efficient and serialization-friendly.
    ///
    /// # Returns
    ///
    /// Monthly Default Rate (MDR) - the monthly default rate.
    pub fn default_rate(
        &self,
        _as_of: finstack_core::dates::Date,
        _origination_date: finstack_core::dates::Date,
        seasoning_months: u32,
        _credit_factors: &CreditFactors,
    ) -> f64 {
        match self {
            DefaultModelSpec::Sda { multiplier } => {
                // SDA calculation per config constants
                let cdr = if seasoning_months <= SDA_PEAK_MONTH {
                    // Ramp up to peak
                    (seasoning_months as f64 / SDA_PEAK_MONTH as f64) * SDA_PEAK_CDR
                } else if seasoning_months <= (SDA_PEAK_MONTH + 30) {
                    // Decline from peak to terminal over 30 months
                    let months_past_peak = (seasoning_months - SDA_PEAK_MONTH) as f64;
                    let decline_period = 30.0;
                    SDA_PEAK_CDR - (months_past_peak / decline_period) * (SDA_PEAK_CDR - SDA_TERMINAL_CDR)
                } else {
                    // Terminal rate
                    SDA_TERMINAL_CDR
                } * multiplier;
                
                // Convert CDR to MDR
                1.0 - (1.0 - cdr).powf(1.0 / 12.0)
            }
            DefaultModelSpec::ConstantCdr { cdr } => {
                super::rates::cdr_to_mdr(*cdr)
            }
            DefaultModelSpec::ConstantMdr { mdr } => *mdr,
            DefaultModelSpec::AssetDefault { asset_type } => {
                // Asset-specific defaults
                match asset_type.to_lowercase().as_str() {
                    "mortgage" | "rmbs" => {
                        // Simplified mortgage default (0.2% CDR)
                        let cdr = 0.002_f64;
                        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
                    }
                    "auto" | "abs_auto" | "consumer" => {
                        // Auto base 2% CDR
                        let cdr = 0.02_f64;
                        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
                    }
                    "card" | "credit_card" => {
                        // Credit card base 0.4% monthly
                        0.004_f64
                    }
                    "corporate" | "clo" | "commercial" => {
                        // Corporate base 2% CDR
                        let cdr = 0.02_f64;
                        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
                    }
                    _ => {
                        // Generic 2% CDR
                        let cdr = 0.02_f64;
                        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
                    }
                }
            }
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
// Recovery Model Specification
// ============================================================================

/// Serializable recovery model specification.
///
/// This enum represents different recovery modeling approaches for defaulted
/// assets in structured credit pools.
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
    /// Calculate recovery rate for this specification.
    ///
    /// This method evaluates the recovery model directly, making it
    /// efficient and serialization-friendly.
    ///
    /// # Returns
    ///
    /// Recovery rate as a fraction (0.0 to 1.0).
    pub fn recovery_rate(
        &self,
        _default_date: finstack_core::dates::Date,
        resolution_lag_months: u32,
        collateral_value: Option<finstack_core::money::Money>,
        outstanding_balance: finstack_core::money::Money,
        market_factors: &MarketFactors,
    ) -> f64 {
        match self {
            RecoveryModelSpec::Constant { rate } => *rate,
            RecoveryModelSpec::AssetDefault { asset_type } => {
                // Asset-specific recovery logic
                match asset_type.to_lowercase().as_str() {
                    "mortgage" | "rmbs" | "collateral" => {
                        // Collateral-based recovery with market adjustments
                        if let Some(collateral) = collateral_value {
                            let adjusted_collateral = collateral.amount()
                                * market_factors.price_index
                                * (1.0 - market_factors.liquidation_discount);
                            
                            let foreclosure_cost = market_factors
                                .foreclosure_costs
                                .map(|m| m.amount())
                                .unwrap_or(0.0);
                            let net_collateral = (adjusted_collateral - foreclosure_cost).max(0.0);
                            
                            let decay_factor = 1.0 - (resolution_lag_months as f64 * 0.005);
                            let final_collateral = net_collateral * decay_factor.max(0.5);
                            
                            let recovery = (final_collateral / outstanding_balance.amount()).min(1.0);
                            recovery * 0.85 + 0.10 * (1.0 - 0.85)
                        } else {
                            0.60 // Base mortgage recovery
                        }
                    }
                    "auto" | "abs_auto" | "consumer" => 0.45,
                    "card" | "credit_card" | "unsecured" => 0.05,
                    "corporate" | "clo" | "commercial" => 0.40,
                    _ => 0.30,
                }
            }
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
    use finstack_core::dates::Date;
    use finstack_core::money::Money;
    use finstack_core::currency::Currency;

    #[test]
    fn test_prepayment_spec_calculation() {
        let spec = PrepaymentModelSpec::Psa { multiplier: 1.5 };
        
        // Calculate prepayment rate at month 30
        let rate = spec.prepayment_rate(
            Date::from_calendar_date(2025, time::Month::July, 1).unwrap(),
            Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
            30,
            &MarketConditions::default(),
        );
        
        // 150% PSA at month 30 = 9% CPR ≈ 0.77% SMM
        assert!(rate > 0.0);
        assert!(rate < 0.01); // Less than 1% monthly
    }

    #[test]
    fn test_default_spec_calculation() {
        let spec = DefaultModelSpec::Sda { multiplier: 2.0 };
        
        // Calculate default rate at peak month
        let rate = spec.default_rate(
            Date::from_calendar_date(2025, time::Month::July, 1).unwrap(),
            Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
            30,
            &CreditFactors::default(),
        );
        
        // Should be positive
        assert!(rate > 0.0);
    }

    #[test]
    fn test_recovery_spec_calculation() {
        let spec = RecoveryModelSpec::Constant { rate: 0.6 };
        
        // Calculate recovery rate
        let rate = spec.recovery_rate(
            Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            6,
            None,
            Money::new(100_000.0, Currency::USD),
            &MarketFactors::default(),
        );
        
        assert!((rate - 0.6).abs() < 1e-10);
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

    #[test]
    fn test_all_prepayment_variants() {
        let specs = vec![
            PrepaymentModelSpec::Psa { multiplier: 1.0 },
            PrepaymentModelSpec::ConstantCpr { cpr: 0.15 },
            PrepaymentModelSpec::ConstantSmm { smm: 0.012 },
            PrepaymentModelSpec::AssetDefault { asset_type: "auto".to_string() },
        ];
        
        let market = MarketConditions::default();
        for spec in specs {
            let rate = spec.prepayment_rate(
                Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
                Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                12,
                &market,
            );
            assert!((0.0..=1.0).contains(&rate), "Rate should be valid: {}", rate);
        }
    }

    #[test]
    fn test_all_default_variants() {
        let specs = vec![
            DefaultModelSpec::Sda { multiplier: 1.0 },
            DefaultModelSpec::ConstantCdr { cdr: 0.02 },
            DefaultModelSpec::ConstantMdr { mdr: 0.002 },
            DefaultModelSpec::AssetDefault { asset_type: "corporate".to_string() },
        ];
        
        let factors = CreditFactors::default();
        for spec in specs {
            let rate = spec.default_rate(
                Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
                Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
                12,
                &factors,
            );
            assert!((0.0..=1.0).contains(&rate), "Rate should be valid: {}", rate);
        }
    }

    #[test]
    fn test_all_recovery_variants() {
        let specs = vec![
            RecoveryModelSpec::Constant { rate: 0.4 },
            RecoveryModelSpec::AssetDefault { asset_type: "corporate".to_string() },
        ];
        
        let market = MarketFactors::default();
        let balance = Money::new(100_000.0, Currency::USD);
        
        for spec in specs {
            let rate = spec.recovery_rate(
                Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
                6,
                None,
                balance,
                &market,
            );
            assert!((0.0..=1.0).contains(&rate), "Rate should be valid: {}", rate);
        }
    }
}

