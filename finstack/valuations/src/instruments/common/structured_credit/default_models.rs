//! Generic default and recovery modeling framework for structured credit.
//!
//! This module provides flexible models for CDR (Constant Default Rate),
//! MDR (Monthly Default Rate), recovery rates, and loss severity across
//! different asset classes.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::prepayment::calculate_seasoning_months;

/// Trait for default behavior modeling
pub trait DefaultBehavior: dyn_clone::DynClone + Send + Sync {
    /// Calculate the default rate for a given period
    fn default_rate(
        &self,
        as_of: Date,
        origination_date: Date,
        seasoning_months: u32,
        credit_factors: &CreditFactors,
    ) -> f64;

    /// Calculate the cumulative default rate
    fn cumulative_default_rate(
        &self,
        as_of: Date,
        origination_date: Date,
        credit_factors: &CreditFactors,
    ) -> f64 {
        let months = calculate_seasoning_months(origination_date, as_of);
        let mut cumulative = 0.0;
        let mut remaining = 1.0;

        for month in 0..=months {
            let mdr = self.default_rate(as_of, origination_date, month, credit_factors);
            cumulative += remaining * mdr;
            remaining *= 1.0 - mdr;
        }

        cumulative
    }
}

dyn_clone::clone_trait_object!(DefaultBehavior);

/// Trait for recovery modeling
pub trait RecoveryBehavior: dyn_clone::DynClone + Send + Sync {
    /// Calculate expected recovery rate
    fn recovery_rate(
        &self,
        default_date: Date,
        resolution_lag_months: u32,
        collateral_value: Option<Money>,
        outstanding_balance: Money,
        market_factors: &MarketFactors,
    ) -> f64;

    /// Calculate loss severity (1 - recovery rate)
    fn loss_severity(
        &self,
        default_date: Date,
        resolution_lag_months: u32,
        collateral_value: Option<Money>,
        outstanding_balance: Money,
        market_factors: &MarketFactors,
    ) -> f64 {
        1.0 - self.recovery_rate(
            default_date,
            resolution_lag_months,
            collateral_value,
            outstanding_balance,
            market_factors,
        )
    }
}

dyn_clone::clone_trait_object!(RecoveryBehavior);

/// Credit factors affecting default probability
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditFactors {
    /// Current FICO/credit score
    pub credit_score: Option<u32>,
    /// Debt-to-income ratio
    pub dti: Option<f64>,
    /// Loan-to-value ratio
    pub ltv: Option<f64>,
    /// Payment delinquency status (days)
    pub delinquency_days: u32,
    /// Unemployment rate
    pub unemployment_rate: Option<f64>,
    /// Additional custom factors
    pub custom_factors: HashMap<String, f64>,
}

/// Market factors affecting recovery
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MarketFactors {
    /// Property/collateral price index
    pub price_index: f64,
    /// Market liquidation discount
    pub liquidation_discount: f64,
    /// Legal/foreclosure costs (optional, defaults to 0 if None)
    pub foreclosure_costs: Option<Money>,
    /// Time to resolution affects holding costs
    pub resolution_months: u32,
}

/// Constant Default Rate (CDR) model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CDRModel {
    /// Annual default rate
    pub annual_rate: f64,
}

impl CDRModel {
    pub fn new(annual_rate: f64) -> Self {
        Self { annual_rate }
    }

    /// Convert CDR to MDR (Monthly Default Rate)
    pub fn to_mdr(&self) -> f64 {
        1.0 - (1.0 - self.annual_rate).powf(1.0 / 12.0)
    }
}

impl DefaultBehavior for CDRModel {
    fn default_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        _seasoning_months: u32,
        _credit_factors: &CreditFactors,
    ) -> f64 {
        self.to_mdr()
    }
}

/// Standard Default Assumption (SDA) model for mortgages
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SDAModel {
    /// SDA speed multiplier (100% = standard SDA)
    pub speed: f64,
    /// Peak default month
    pub peak_month: u32,
    /// Peak annual default rate
    pub peak_cdr: f64,
    /// Terminal default rate after peak
    pub terminal_cdr: f64,
}

impl Default for SDAModel {
    fn default() -> Self {
        Self {
            speed: 1.0,
            peak_month: 30,
            peak_cdr: 0.006,      // 0.6% annual at peak
            terminal_cdr: 0.0003, // 0.03% terminal
        }
    }
}

impl DefaultBehavior for SDAModel {
    fn default_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _credit_factors: &CreditFactors,
    ) -> f64 {
        let cdr = if seasoning_months <= self.peak_month {
            // Ramp up to peak
            (seasoning_months as f64 / self.peak_month as f64) * self.peak_cdr
        } else if seasoning_months <= 60 {
            // Decline from peak to terminal
            let months_past_peak = (seasoning_months - self.peak_month) as f64;
            let decline_period = 30.0;
            self.peak_cdr
                - (months_past_peak / decline_period) * (self.peak_cdr - self.terminal_cdr)
        } else {
            // Terminal rate
            self.terminal_cdr
        } * self.speed;

        // Convert to MDR
        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
    }
}

/// Vector default model with custom default rates by period
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VectorDefaultModel {
    /// CDR rates by month
    pub cdr_vector: Vec<f64>,
    /// Terminal CDR after vector
    pub terminal_cdr: f64,
}

impl VectorDefaultModel {
    pub fn new(cdr_vector: Vec<f64>, terminal_cdr: f64) -> Self {
        Self {
            cdr_vector,
            terminal_cdr,
        }
    }
}

impl DefaultBehavior for VectorDefaultModel {
    fn default_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        _credit_factors: &CreditFactors,
    ) -> f64 {
        let cdr = if (seasoning_months as usize) < self.cdr_vector.len() {
            self.cdr_vector[seasoning_months as usize]
        } else {
            self.terminal_cdr
        };

        // Convert to MDR
        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
    }
}

/// Mortgage default model with credit sensitivity
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MortgageDefaultModel {
    /// Base default curve
    pub base_cdr: f64,
    /// FICO score sensitivity (multiplier per 50 points below 700)
    pub fico_sensitivity: f64,
    /// LTV sensitivity (multiplier per 10% above 80%)
    pub ltv_sensitivity: f64,
    /// Unemployment sensitivity
    pub unemployment_sensitivity: f64,
    /// Seasoning curve peak
    pub peak_default_month: u32,
}

impl Default for MortgageDefaultModel {
    fn default() -> Self {
        Self {
            base_cdr: 0.002,
            fico_sensitivity: 2.0,
            ltv_sensitivity: 1.5,
            unemployment_sensitivity: 3.0,
            peak_default_month: 24,
        }
    }
}

impl DefaultBehavior for MortgageDefaultModel {
    fn default_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        credit_factors: &CreditFactors,
    ) -> f64 {
        let mut cdr = self.base_cdr;

        // Seasoning adjustment
        if seasoning_months < self.peak_default_month {
            cdr *= seasoning_months as f64 / self.peak_default_month as f64;
        } else if seasoning_months > 60 {
            cdr *= 0.5; // Burnout factor
        }

        // FICO adjustment
        if let Some(fico) = credit_factors.credit_score {
            if fico < 700 {
                let fico_diff = (700 - fico) as f64;
                cdr *= 1.0 + (fico_diff / 50.0) * self.fico_sensitivity;
            }
        }

        // LTV adjustment
        if let Some(ltv) = credit_factors.ltv {
            if ltv > 0.80 {
                let ltv_excess = (ltv - 0.80) * 100.0;
                cdr *= 1.0 + (ltv_excess / 10.0) * self.ltv_sensitivity;
            }
        }

        // Unemployment adjustment
        if let Some(unemployment) = credit_factors.unemployment_rate {
            if unemployment > 0.04 {
                cdr *= 1.0 + (unemployment - 0.04) * self.unemployment_sensitivity;
            }
        }

        // Convert to MDR
        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
    }
}

/// Auto loan default model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AutoDefaultModel {
    /// Base annual default rate
    pub base_cdr: f64,
    /// Default multiplier for used vehicles
    pub used_multiplier: f64,
    /// Peak default month
    pub peak_month: u32,
    /// Credit tier adjustments
    pub credit_tier_multipliers: HashMap<String, f64>,
}

impl Default for AutoDefaultModel {
    fn default() -> Self {
        let mut credit_tiers = HashMap::new();
        credit_tiers.insert("Prime".to_string(), 0.5);
        credit_tiers.insert("NearPrime".to_string(), 1.0);
        credit_tiers.insert("Subprime".to_string(), 2.5);
        credit_tiers.insert("DeepSubprime".to_string(), 4.0);

        Self {
            base_cdr: 0.02, // 2% base CDR
            used_multiplier: 1.3,
            peak_month: 18,
            credit_tier_multipliers: credit_tiers,
        }
    }
}

impl DefaultBehavior for AutoDefaultModel {
    fn default_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        seasoning_months: u32,
        credit_factors: &CreditFactors,
    ) -> f64 {
        let mut cdr = self.base_cdr;

        // Seasoning curve
        if seasoning_months < self.peak_month {
            cdr *= seasoning_months as f64 / self.peak_month as f64;
        } else if seasoning_months > 36 {
            cdr *= 0.3; // Significant burnout after 3 years
        }

        // Credit tier adjustment
        if let Some(tier) = credit_factors.custom_factors.get("credit_tier") {
            let tier_name = if *tier < 650.0 {
                "DeepSubprime"
            } else if *tier < 700.0 {
                "Subprime"
            } else if *tier < 740.0 {
                "NearPrime"
            } else {
                "Prime"
            };

            if let Some(multiplier) = self.credit_tier_multipliers.get(tier_name) {
                cdr *= multiplier;
            }
        }

        // Convert to MDR
        1.0 - (1.0 - cdr).powf(1.0 / 12.0)
    }
}

/// Credit card charge-off model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditCardChargeOffModel {
    /// Base monthly charge-off rate
    pub base_charge_off_rate: f64,
    /// Delinquency roll rates
    pub roll_rates: HashMap<u32, f64>,
    /// Unemployment sensitivity
    pub unemployment_beta: f64,
}

impl Default for CreditCardChargeOffModel {
    fn default() -> Self {
        let mut roll_rates = HashMap::new();
        roll_rates.insert(30, 0.10); // 30 DPD to 60 DPD
        roll_rates.insert(60, 0.30); // 60 DPD to 90 DPD
        roll_rates.insert(90, 0.60); // 90 DPD to 120 DPD
        roll_rates.insert(120, 0.85); // 120 DPD to charge-off

        Self {
            base_charge_off_rate: 0.004, // 0.4% monthly
            roll_rates,
            unemployment_beta: 0.8,
        }
    }
}

impl DefaultBehavior for CreditCardChargeOffModel {
    fn default_rate(
        &self,
        _as_of: Date,
        _origination_date: Date,
        _seasoning_months: u32,
        credit_factors: &CreditFactors,
    ) -> f64 {
        let mut rate = self.base_charge_off_rate;

        // Apply delinquency roll rate
        if credit_factors.delinquency_days > 0 {
            let bucket = (credit_factors.delinquency_days / 30) * 30;
            if let Some(roll_rate) = self.roll_rates.get(&bucket) {
                rate = self.base_charge_off_rate * (1.0 + roll_rate * 10.0);
            }
        }

        // Unemployment adjustment
        if let Some(unemployment) = credit_factors.unemployment_rate {
            let baseline_unemployment = 0.04;
            let unemployment_impact = (unemployment - baseline_unemployment).max(0.0);
            rate *= 1.0 + unemployment_impact * self.unemployment_beta;
        }

        rate
    }
}

/// Constant recovery rate model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConstantRecoveryModel {
    /// Fixed recovery rate
    pub recovery_rate: f64,
}

impl ConstantRecoveryModel {
    pub fn new(recovery_rate: f64) -> Self {
        Self { recovery_rate }
    }
}

impl RecoveryBehavior for ConstantRecoveryModel {
    fn recovery_rate(
        &self,
        _default_date: Date,
        _resolution_lag_months: u32,
        _collateral_value: Option<Money>,
        _outstanding_balance: Money,
        _market_factors: &MarketFactors,
    ) -> f64 {
        self.recovery_rate
    }
}

/// Collateral-based recovery model
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CollateralRecoveryModel {
    /// Base recovery rate without collateral
    pub base_recovery: f64,
    /// Advance rate on collateral value
    pub advance_rate: f64,
    /// Time decay factor (per month)
    pub time_decay: f64,
}

impl Default for CollateralRecoveryModel {
    fn default() -> Self {
        Self {
            base_recovery: 0.10,
            advance_rate: 0.85,
            time_decay: 0.01,
        }
    }
}

impl RecoveryBehavior for CollateralRecoveryModel {
    fn recovery_rate(
        &self,
        _default_date: Date,
        resolution_lag_months: u32,
        collateral_value: Option<Money>,
        outstanding_balance: Money,
        market_factors: &MarketFactors,
    ) -> f64 {
        if let Some(collateral) = collateral_value {
            // Apply market adjustments
            let adjusted_collateral = collateral.amount()
                * market_factors.price_index
                * (1.0 - market_factors.liquidation_discount);

            // Subtract foreclosure costs
            let foreclosure_cost = market_factors
                .foreclosure_costs
                .map(|m| m.amount())
                .unwrap_or(0.0);
            let net_collateral = (adjusted_collateral - foreclosure_cost).max(0.0);

            // Apply time decay
            let decay_factor = 1.0 - (resolution_lag_months as f64 * self.time_decay);
            let final_collateral = net_collateral * decay_factor.max(0.5);

            // Calculate recovery as percentage of outstanding
            let recovery = (final_collateral / outstanding_balance.amount()).min(1.0);
            recovery * self.advance_rate + self.base_recovery * (1.0 - self.advance_rate)
        } else {
            self.base_recovery
        }
    }
}

/// Factory for creating default and recovery models
pub struct DefaultModelFactory;

impl DefaultModelFactory {
    /// Create default model for asset type
    pub fn create_default_model(asset_type: &str) -> Box<dyn DefaultBehavior> {
        match asset_type.to_lowercase().as_str() {
            "mortgage" | "rmbs" => Box::new(MortgageDefaultModel::default()),
            "auto" | "abs_auto" => Box::new(AutoDefaultModel::default()),
            "card" | "credit_card" => Box::new(CreditCardChargeOffModel::default()),
            _ => Box::new(CDRModel::new(0.02)), // 2% CDR default
        }
    }

    /// Create recovery model for asset type
    pub fn create_recovery_model(asset_type: &str) -> Box<dyn RecoveryBehavior> {
        match asset_type.to_lowercase().as_str() {
            "mortgage" | "rmbs" => Box::new(CollateralRecoveryModel {
                base_recovery: 0.60,
                advance_rate: 0.90,
                time_decay: 0.005,
            }),
            "auto" | "abs_auto" | "consumer" => Box::new(CollateralRecoveryModel {
                base_recovery: 0.45,
                advance_rate: 0.75,
                time_decay: 0.02,
            }),
            "card" | "credit_card" => Box::new(ConstantRecoveryModel::new(0.05)),
            "corporate" | "clo" | "commercial" => Box::new(ConstantRecoveryModel::new(0.40)),
            _ => Box::new(ConstantRecoveryModel::new(0.30)),
        }
    }
}

// Utility functions

/// Convert MDR to CDR
pub fn mdr_to_cdr(mdr: f64) -> f64 {
    1.0 - (1.0 - mdr).powi(12)
}

/// Convert CDR to MDR
pub fn cdr_to_mdr(cdr: f64) -> f64 {
    1.0 - (1.0 - cdr).powf(1.0 / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn test_cdr_mdr_conversion() {
        let cdr = CDRModel::new(0.06);
        let mdr = cdr.to_mdr();

        // Test roundtrip
        let cdr_back = mdr_to_cdr(mdr);
        assert!((cdr_back - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_sda_model() {
        let sda = SDAModel::default();
        let factors = CreditFactors::default();

        let orig = date!(2020 - 01 - 01);
        let as_of = date!(2022 - 07 - 01); // 30 months

        let mdr = sda.default_rate(as_of, orig, 30, &factors);
        let cdr = mdr_to_cdr(mdr);

        // At peak should be 0.6% CDR
        assert!((cdr - 0.006).abs() < 0.001);
    }

    #[test]
    fn test_collateral_recovery() {
        let model = CollateralRecoveryModel::default();
        let market = MarketFactors {
            price_index: 0.90, // 10% price decline
            foreclosure_costs: Some(Money::new(5000.0, finstack_core::currency::Currency::USD)),
            ..Default::default()
        };

        let collateral = Some(Money::new(
            200_000.0,
            finstack_core::currency::Currency::USD,
        ));
        let outstanding = Money::new(250_000.0, finstack_core::currency::Currency::USD);

        let recovery =
            model.recovery_rate(date!(2023 - 01 - 01), 6, collateral, outstanding, &market);

        // Should recover substantial portion with collateral
        assert!(recovery > 0.5);
        assert!(recovery < 0.9);
    }
}
