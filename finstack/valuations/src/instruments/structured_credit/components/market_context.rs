//! Market and credit context structures for behavioral models.
//!
//! This module provides the context structures used by prepayment, default,
//! and recovery models to factor in market conditions and credit characteristics.

use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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

