//! Generic prepayment modeling framework for structured credit products.
//!
//! This module provides a flexible, extensible framework for modeling prepayments
//! across different asset classes including mortgages, auto loans, credit cards,
//! commercial loans, and more.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Generic trait for prepayment behavior that can be implemented for any asset class
pub trait PrepaymentBehavior: dyn_clone::DynClone + Send + Sync {
    /// Calculate the prepayment rate for a given period
    fn prepayment_rate(
        &self,
        as_of: Date,
        origination_date: Date,
        seasoning_months: u32,
        market_conditions: &MarketConditions,
    ) -> f64;

    /// Calculate actual prepayment amount given a balance
    fn prepayment_amount(
        &self,
        balance: Money,
        as_of: Date,
        origination_date: Date,
        market_conditions: &MarketConditions,
    ) -> Result<Money> {
        let seasoning = calculate_seasoning_months(origination_date, as_of);
        let rate = self.prepayment_rate(as_of, origination_date, seasoning, market_conditions);
        Ok(Money::new(balance.amount() * rate, balance.currency()))
    }

    /// Downcast to Any for serialization support
    fn as_any(&self) -> &dyn std::any::Any;
}

dyn_clone::clone_trait_object!(PrepaymentBehavior);

/// Create a default prepayment model for an asset type
pub fn prepayment_model_for(asset_type: &str) -> Box<dyn PrepaymentBehavior> {
    // Simplified: Use generic model with asset-specific parameters
    match asset_type.to_lowercase().as_str() {
        "mortgage" | "rmbs" => Box::new(PSAModel::default()),
        "auto" | "abs_auto" => Box::new(CPRModel::new(0.18)), // ~1.5% monthly
        "card" | "credit_card" | "cc" => Box::new(CPRModel::new(0.15)), // Payment rate
        "commercial" | "cmbs" | "cre" => Box::new(CPRModel::new(0.10)), // 10% CPR
        "student" | "student_loan" => Box::new(CPRModel::new(0.03)), // 3% CPR
        _ => Box::new(CPRModel::new(0.05)),                   // Default 5% CPR
    }
}

/// Create a PSA model with specified speed
pub fn psa_model(speed: f64) -> Box<dyn PrepaymentBehavior> {
    Box::new(PSAModel::new(speed))
}

/// Create a constant CPR model
pub fn cpr_model(annual_rate: f64) -> Box<dyn PrepaymentBehavior> {
    Box::new(CPRModel::new(annual_rate))
}

/// Create a vector model from CPR schedule
pub fn vector_model(cpr_vector: Vec<f64>, terminal_cpr: f64) -> Box<dyn PrepaymentBehavior> {
    Box::new(VectorModel::new(cpr_vector, terminal_cpr))
}

/// Market conditions that affect prepayment behavior
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MarketConditions {
    /// Current refinancing rate
    pub refi_rate: f64,
    /// Rate at origination for refinancing incentive calculation
    pub original_rate: Option<f64>,
    /// Home price appreciation (for mortgages)
    pub hpa: Option<f64>,
    /// Unemployment rate
    pub unemployment: Option<f64>,
    /// Seasonal adjustment factor
    pub seasonal_factor: Option<f64>,
    /// Custom market factors
    pub custom_factors: HashMap<String, f64>,
}

impl Default for MarketConditions {
    fn default() -> Self {
        Self {
            refi_rate: 0.04,
            original_rate: None,
            hpa: None,
            unemployment: None,
            seasonal_factor: Some(1.0),
            custom_factors: HashMap::new(),
        }
    }
}

/// Standard CPR (Constant Prepayment Rate) model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CPRModel {
    /// Annual prepayment rate
    pub annual_rate: f64,
}

impl CPRModel {
    pub fn new(annual_rate: f64) -> Self {
        Self { annual_rate }
    }

    /// Convert CPR to SMM (Single Monthly Mortality)
    pub fn to_smm(&self) -> f64 {
        1.0 - (1.0 - self.annual_rate).powf(1.0 / 12.0)
    }
}

impl PrepaymentBehavior for CPRModel {
    fn prepayment_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        _seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        self.to_smm()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// PSA (Public Securities Association) standard prepayment model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PSAModel {
    /// PSA speed multiplier (100% = standard PSA)
    pub speed: f64,
    /// Ramp-up period in months (standard is 30)
    pub ramp_months: u32,
    /// Terminal CPR rate (standard is 6%)
    pub terminal_cpr: f64,
}

impl Default for PSAModel {
    fn default() -> Self {
        Self {
            speed: 1.0, // 100% PSA
            ramp_months: super::constants::PSA_RAMP_MONTHS,
            terminal_cpr: super::constants::PSA_TERMINAL_CPR,
        }
    }
}

impl PSAModel {
    pub fn new(speed: f64) -> Self {
        Self {
            speed,
            ..Default::default()
        }
    }

    /// Get the PSA multiplier
    pub fn multiplier(&self) -> f64 {
        self.speed
    }

    /// Calculate CPR for a given month under PSA model
    pub fn cpr_at_month(&self, month: u32) -> f64 {
        let base_cpr = if month <= self.ramp_months {
            (month as f64 / self.ramp_months as f64) * self.terminal_cpr
        } else {
            self.terminal_cpr
        };

        base_cpr * self.speed
    }
}

impl PrepaymentBehavior for PSAModel {
    fn prepayment_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        let cpr = self.cpr_at_month(seasoning_months);
        // Convert CPR to SMM
        1.0 - (1.0 - cpr).powf(1.0 / 12.0)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Vector prepayment model with custom speeds by seasoning
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VectorModel {
    /// CPR rates by month (seasoning)
    pub cpr_vector: Vec<f64>,
    /// What to use after vector ends
    pub terminal_cpr: f64,
}

impl VectorModel {
    pub fn new(cpr_vector: Vec<f64>, terminal_cpr: f64) -> Self {
        Self {
            cpr_vector,
            terminal_cpr,
        }
    }
}

impl PrepaymentBehavior for VectorModel {
    fn prepayment_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        let cpr = if (seasoning_months as usize) < self.cpr_vector.len() {
            self.cpr_vector[seasoning_months as usize]
        } else {
            self.terminal_cpr
        };

        // Convert CPR to SMM
        1.0 - (1.0 - cpr).powf(1.0 / 12.0)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Utility functions

/// Calculate seasoning in months between two dates
pub fn calculate_seasoning_months(origination: Date, as_of: Date) -> u32 {
    let months = (as_of.year() - origination.year()) * 12
        + (as_of.month() as i32 - origination.month() as i32);
    months.max(0) as u32
}

/// Convert SMM to CPR
pub fn smm_to_cpr(smm: f64) -> f64 {
    1.0 - (1.0 - smm).powi(12)
}

/// Convert CPR to SMM
pub fn cpr_to_smm(cpr: f64) -> f64 {
    1.0 - (1.0 - cpr).powf(1.0 / 12.0)
}

/// Convert PSA speed to CPR at a given month
pub fn psa_to_cpr(psa_speed: f64, month: u32) -> f64 {
    let model = PSAModel::new(psa_speed);
    model.cpr_at_month(month)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpr_to_smm_conversion() {
        let cpr = CPRModel::new(0.06);
        let smm = cpr.to_smm();

        // 6% CPR should be approximately 0.5143% SMM
        assert!((smm - 0.005143).abs() < 0.0001);

        // Test roundtrip
        let cpr_back = smm_to_cpr(smm);
        assert!((cpr_back - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_psa_model() {
        let psa = PSAModel::new(1.5); // 150% PSA

        // At month 15, should be 15/30 * 6% * 1.5 = 4.5% CPR
        let cpr_15 = psa.cpr_at_month(15);
        assert!((cpr_15 - 0.045).abs() < 0.0001);

        // At month 30 and beyond, should be 6% * 1.5 = 9% CPR
        let cpr_30 = psa.cpr_at_month(30);
        assert!((cpr_30 - 0.09).abs() < 0.0001);

        let cpr_60 = psa.cpr_at_month(60);
        assert!((cpr_60 - 0.09).abs() < 0.0001);
    }

    #[test]
    fn test_model_creation() {
        let _mortgage = prepayment_model_for("mortgage");
        let _auto = prepayment_model_for("auto");
        let _psa = psa_model(2.0);
        // Successfully creates models
    }
}
