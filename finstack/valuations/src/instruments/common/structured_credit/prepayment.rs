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
pub trait PrepaymentBehavior: Send + Sync {
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

    /// Clone the behavior into a box
    fn clone_box(&self) -> Box<dyn PrepaymentBehavior>;
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

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
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
            ramp_months: 30,
            terminal_cpr: 0.06,
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

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
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

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
    }
}

/// Mortgage-specific prepayment model with refinancing sensitivity
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MortgagePrepaymentModel {
    /// Base prepayment curve (e.g., PSA)
    pub base_speed: f64,
    /// Refinancing sensitivity (refi multiplier per 100bps of incentive)
    pub refi_sensitivity: f64,
    /// Burnout factor (reduces prepayments for seasoned loans)
    pub burnout_factor: f64,
    /// Seasonality adjustments by month (Jan=index 0)
    pub seasonality: [f64; 12],
    /// HPA (home price appreciation) sensitivity
    pub hpa_sensitivity: f64,
}

impl Default for MortgagePrepaymentModel {
    fn default() -> Self {
        Self {
            base_speed: 1.0,
            refi_sensitivity: 4.0, // 4x multiplier per 100bps incentive
            burnout_factor: 0.3,
            seasonality: [
                0.94, 0.76, 0.74, 0.95, 0.98, 0.92, // Jan-Jun
                1.10, 1.18, 1.22, 1.23, 0.98, 1.00, // Jul-Dec
            ],
            hpa_sensitivity: 0.1,
        }
    }
}

impl PrepaymentBehavior for MortgagePrepaymentModel {
    fn prepayment_rate(
        &self,
        as_of: Date,
        origination_date: Date,
        seasoning_months: u32,
        market_conditions: &MarketConditions,
    ) -> f64 {
        // Start with PSA base
        let psa = PSAModel::new(self.base_speed);
        let base_smm =
            psa.prepayment_rate(as_of, origination_date, seasoning_months, market_conditions);

        // Apply refinancing incentive
        let mut multiplier = 1.0;
        if let Some(orig_rate) = market_conditions.original_rate {
            let refi_incentive = (orig_rate - market_conditions.refi_rate) * 100.0; // in bps
            if refi_incentive > 0.0 {
                multiplier *= 1.0 + (refi_incentive / 100.0) * self.refi_sensitivity;
            }
        }

        // Apply burnout for seasoned loans
        if seasoning_months > 60 {
            let burnout =
                1.0 - self.burnout_factor * ((seasoning_months - 60) as f64 / 120.0).min(1.0);
            multiplier *= burnout;
        }

        // Apply seasonality
        let month = as_of.month() as usize - 1;
        multiplier *= self.seasonality[month];

        // Apply HPA effect if available
        if let Some(hpa) = market_conditions.hpa {
            multiplier *= 1.0 + hpa * self.hpa_sensitivity;
        }

        base_smm * multiplier
    }

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
    }
}

/// Auto loan prepayment model with absolute prepayment speeds
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AutoPrepaymentModel {
    /// ABS (Absolute Prepayment Speed) - % of original balance
    pub abs_speed: f64,
    /// Seasoning ramp (months to reach full speed)
    pub ramp_months: u32,
    /// Loss severity adjustment
    pub loss_severity: f64,
}

impl Default for AutoPrepaymentModel {
    fn default() -> Self {
        Self {
            abs_speed: 0.015, // 1.5% ABS
            ramp_months: 12,
            loss_severity: 0.35,
        }
    }
}

impl PrepaymentBehavior for AutoPrepaymentModel {
    fn prepayment_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        // Ramp up to full ABS speed
        let ramp_factor = if seasoning_months < self.ramp_months {
            seasoning_months as f64 / self.ramp_months as f64
        } else {
            1.0
        };

        self.abs_speed * ramp_factor
    }

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
    }
}

/// Credit card payment model with payment rates
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditCardPaymentModel {
    /// Monthly payment rate (% of balance)
    pub payment_rate: f64,
    /// Charge-off rate
    pub charge_off_rate: f64,
    /// Seasonal adjustment
    pub use_seasonality: bool,
}

impl Default for CreditCardPaymentModel {
    fn default() -> Self {
        Self {
            payment_rate: 0.15,     // 15% monthly payment rate
            charge_off_rate: 0.005, // 0.5% monthly
            use_seasonality: true,
        }
    }
}

impl PrepaymentBehavior for CreditCardPaymentModel {
    fn prepayment_rate(
        &self,
        as_of: Date,
        _origination_date: Date,
        _seasoning_months: u32,
        market_conditions: &MarketConditions,
    ) -> f64 {
        let mut rate = self.payment_rate;

        // Apply seasonality (higher payments in Jan/Feb, Dec)
        if self.use_seasonality {
            let seasonal_factors = [
                1.15, 1.10, 1.0, 0.95, 0.95, 0.95, // Jan-Jun
                0.95, 0.95, 1.0, 1.05, 1.05, 1.10, // Jul-Dec
            ];
            let month = as_of.month() as usize - 1;
            rate *= seasonal_factors[month];
        }

        // Apply custom seasonal factor if provided
        if let Some(factor) = market_conditions.seasonal_factor {
            rate *= factor;
        }

        rate
    }

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
    }
}

/// Commercial real estate prepayment model with lockout and yield maintenance
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CommercialPrepaymentModel {
    /// Lockout period in months (no prepayments allowed)
    pub lockout_months: u32,
    /// Yield maintenance period after lockout
    pub yield_maintenance_months: u32,
    /// Defeasance period option
    pub defeasance_months: Option<u32>,
    /// Open period CPR (after all restrictions)
    pub open_cpr: f64,
    /// Balloon payment month
    pub balloon_month: Option<u32>,
}

impl Default for CommercialPrepaymentModel {
    fn default() -> Self {
        Self {
            lockout_months: 24,
            yield_maintenance_months: 36,
            defeasance_months: None,
            open_cpr: 0.10,           // 10% CPR in open period
            balloon_month: Some(120), // 10-year balloon
        }
    }
}

impl PrepaymentBehavior for CommercialPrepaymentModel {
    fn prepayment_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        // Check if in lockout period
        if seasoning_months < self.lockout_months {
            return 0.0;
        }

        // Check if in yield maintenance period
        let ym_end = self.lockout_months + self.yield_maintenance_months;
        if seasoning_months < ym_end {
            // Minimal voluntary prepayments during yield maintenance
            return 0.001; // 0.1% CPR
        }

        // Check if in defeasance period
        if let Some(defeasance) = self.defeasance_months {
            let defeasance_end = ym_end + defeasance;
            if seasoning_months < defeasance_end {
                return 0.002; // 0.2% CPR during defeasance
            }
        }

        // Check if approaching balloon
        if let Some(balloon) = self.balloon_month {
            if seasoning_months >= balloon - 3 {
                // Increase prepayments near balloon
                return 0.25; // 25% CPR near maturity
            }
        }

        // Open period - use standard CPR
        1.0 - (1.0 - self.open_cpr).powf(1.0 / 12.0)
    }

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
    }
}

/// Student loan prepayment model with grace period and consolidation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StudentLoanPrepaymentModel {
    /// Grace period in months
    pub grace_period_months: u32,
    /// In-school deferment months
    pub deferment_months: u32,
    /// Base CPR after entering repayment
    pub repayment_cpr: f64,
    /// Consolidation hazard rate
    pub consolidation_rate: f64,
}

impl Default for StudentLoanPrepaymentModel {
    fn default() -> Self {
        Self {
            grace_period_months: 6,
            deferment_months: 48,
            repayment_cpr: 0.03,      // 3% CPR
            consolidation_rate: 0.05, // 5% annual consolidation
        }
    }
}

impl PrepaymentBehavior for StudentLoanPrepaymentModel {
    fn prepayment_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _market_conditions: &MarketConditions,
    ) -> f64 {
        // In deferment period - only consolidations
        if seasoning_months < self.deferment_months {
            return self.consolidation_rate / 12.0;
        }

        // Grace period - reduced prepayments
        let grace_end = self.deferment_months + self.grace_period_months;
        if seasoning_months < grace_end {
            return self.consolidation_rate / 12.0 * 0.5;
        }

        // Full repayment period
        let base_smm = 1.0 - (1.0 - self.repayment_cpr).powf(1.0 / 12.0);
        let consol_smm = self.consolidation_rate / 12.0;

        // Combined prepayment rate
        base_smm + consol_smm - (base_smm * consol_smm)
    }

    fn clone_box(&self) -> Box<dyn PrepaymentBehavior> {
        Box::new(self.clone())
    }
}

/// Factory for creating prepayment models based on asset type
pub struct PrepaymentModelFactory;

impl PrepaymentModelFactory {
    /// Create a default prepayment model for an asset type
    pub fn create_default(asset_type: &str) -> Box<dyn PrepaymentBehavior> {
        match asset_type.to_lowercase().as_str() {
            "mortgage" | "rmbs" => Box::new(MortgagePrepaymentModel::default()),
            "auto" | "abs_auto" => Box::new(AutoPrepaymentModel::default()),
            "card" | "credit_card" | "cc" => Box::new(CreditCardPaymentModel::default()),
            "commercial" | "cmbs" | "cre" => Box::new(CommercialPrepaymentModel::default()),
            "student" | "student_loan" => Box::new(StudentLoanPrepaymentModel::default()),
            _ => Box::new(CPRModel::new(0.05)), // Default 5% CPR
        }
    }

    /// Create a PSA model with specified speed
    pub fn create_psa(speed: f64) -> Box<dyn PrepaymentBehavior> {
        Box::new(PSAModel::new(speed))
    }

    /// Create a constant CPR model
    pub fn create_cpr(annual_rate: f64) -> Box<dyn PrepaymentBehavior> {
        Box::new(CPRModel::new(annual_rate))
    }

    /// Create a vector model from CPR schedule
    pub fn create_vector(cpr_vector: Vec<f64>, terminal_cpr: f64) -> Box<dyn PrepaymentBehavior> {
        Box::new(VectorModel::new(cpr_vector, terminal_cpr))
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
    use time::macros::date;

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
    fn test_mortgage_prepayment() {
        let model = MortgagePrepaymentModel::default();
        let conditions = MarketConditions {
            original_rate: Some(0.05),
            refi_rate: 0.03, // 200bps incentive
            ..Default::default()
        };

        let orig = date!(2020 - 01 - 01);
        let as_of = date!(2023 - 07 - 01); // 42 months seasoning, July

        let smm = model.prepayment_rate(as_of, orig, 42, &conditions);

        // Should have base rate * refi multiplier * seasonality
        // With 200bps incentive and July seasonality
        assert!(smm > 0.005); // Should be elevated due to refi incentive
    }

    #[test]
    fn test_commercial_lockout() {
        let model = CommercialPrepaymentModel::default();
        let conditions = MarketConditions::default();
        let orig = date!(2020 - 01 - 01);

        // During lockout (month 12)
        let as_of_lockout = date!(2021 - 01 - 01);
        let smm_lockout = model.prepayment_rate(as_of_lockout, orig, 12, &conditions);
        assert_eq!(smm_lockout, 0.0);

        // During yield maintenance (month 36)
        let as_of_ym = date!(2023 - 01 - 01);
        let smm_ym = model.prepayment_rate(as_of_ym, orig, 36, &conditions);
        assert_eq!(smm_ym, 0.001); // Returns exactly 0.1% CPR during yield maintenance

        // Open period (month 72)
        let as_of_open = date!(2026 - 01 - 01);
        let smm_open = model.prepayment_rate(as_of_open, orig, 72, &conditions);
        assert!(smm_open > 0.007); // Should be close to 10% CPR as SMM
    }

    #[test]
    fn test_factory_creation() {
        let _mortgage = PrepaymentModelFactory::create_default("mortgage");
        let _auto = PrepaymentModelFactory::create_default("auto");
        let _psa = PrepaymentModelFactory::create_psa(2.0);
        // Factory successfully creates models
    }
}
