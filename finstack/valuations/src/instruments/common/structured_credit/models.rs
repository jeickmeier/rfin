//! Data model structures for structured credit instruments.
//!
//! This module provides the complex data structures that represent assets, tranches,
//! and other entities within structured credit deals, building on the core enumerations.

use finstack_core::dates::Date;
use finstack_core::money::Money;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::enums::{AssetType, CreditRating};

/// Tranche identifier type alias for clarity
pub type TrancheId = String;

/// Coupon/interest payment types
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CouponType {
    Fixed,
    Floating,
    StepUp,
    PIK,
    Deferrable,
}

/// Alternative seniority naming (for backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Seniority {
    Senior,
    Subordinated,
    Mezzanine,
    Junior,
}

/// Individual asset in a structured credit pool
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Asset {
    /// Unique asset identifier
    pub asset_id: String,
    /// Obligor/borrower identifier
    pub obligor_id: Option<String>,
    /// Asset type
    pub asset_type: AssetType,
    /// Original balance at origination
    pub original_balance: Money,
    /// Current outstanding balance
    pub current_balance: Money,
    /// Interest rate
    pub interest_rate: f64,
    /// Spread over base rate (basis points)
    pub spread_bps: Option<f64>,
    /// Maturity date
    pub maturity_date: Date,
    /// Credit rating
    pub rating: Option<CreditRating>,
    /// Industry classification
    pub industry: Option<String>,
    /// Country/region
    pub country: Option<String>,
    /// Default status
    pub is_defaulted: bool,
    /// Recovery rate if defaulted
    pub recovery_rate: Option<f64>,
}

/// Simplified tranche definition
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Tranche {
    /// Unique tranche identifier
    pub id: TrancheId,
    /// Tranche name
    pub name: String,
    /// Credit rating
    pub rating: Option<CreditRating>,
    /// Original balance
    pub original_balance: Money,
    /// Current outstanding balance
    pub current_balance: Money,
    /// Annual coupon rate
    pub coupon_rate: f64,
    /// Coupon type
    pub coupon_type: CouponType,
    /// Payment priority (1 = highest)
    pub payment_priority: u32,
    /// Legal maturity date
    pub legal_maturity: Date,
    /// Coverage test requirements
    pub coverage_tests: Option<TrancheCoverageTests>,
}

/// Coverage test configuration for a tranche
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TrancheCoverageTests {
    /// OC trigger level
    pub oc_trigger: Option<f64>,
    /// IC trigger level
    pub ic_trigger: Option<f64>,
}
