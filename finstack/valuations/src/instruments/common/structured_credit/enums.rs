//! Core enumeration types for structured credit instruments.
//!
//! This module provides all the enumeration types used to classify and categorize
//! various aspects of structured credit instruments including deal types, asset types,
//! credit ratings, and payment modes.

use finstack_core::dates::Date;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// CORE DEAL TYPES
// ============================================================================

/// Primary structured credit deal classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DealType {
    /// Collateralized Loan Obligation
    CLO,
    /// Collateralized Bond Obligation
    CBO,
    /// Generic Asset-Backed Security
    ABS,
    /// Residential Mortgage-Backed Security
    RMBS,
    /// Commercial Mortgage-Backed Security
    CMBS,
    /// Auto Loan ABS
    Auto,
    /// Credit Card ABS
    Card,
}

// ============================================================================
// CREDIT & RATINGS
// ============================================================================

/// Credit rating scale (agency-agnostic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    CC,
    C,
    D,
    NR,
}

impl CreditRating {
    /// Check if rating is investment grade (BBB and above)
    pub fn is_investment_grade(&self) -> bool {
        matches!(self, Self::AAA | Self::AA | Self::A | Self::BBB)
    }
}

/// Tranche seniority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TrancheSeniority {
    /// Most senior debt tranche
    Senior = 0,
    /// Mezzanine debt tranches
    Mezzanine = 1,
    /// Subordinated debt tranches
    Subordinated = 2,
    /// Equity/first loss piece
    Equity = 3,
}

// ============================================================================
// ASSET CLASSIFICATION
// ============================================================================

/// Asset type classification for pool composition
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum AssetType {
    /// Corporate loan
    Loan {
        loan_type: LoanType,
        industry: Option<String>,
    },
    /// Corporate bond
    Bond {
        bond_type: BondType,
        industry: Option<String>,
    },
    /// Mortgage exposure
    Mortgage {
        property_type: PropertyType,
        ltv: Option<f64>,
    },
    /// Auto loan
    AutoLoan {
        vehicle_type: VehicleType,
        ltv: Option<f64>,
    },
    /// Credit card receivables
    CreditCard { 
        portfolio_type: CardPortfolioType 
    },
    /// Student loan assets
    StudentLoan { 
        loan_type: StudentLoanType 
    },
    /// Equipment financing
    Equipment { 
        equipment_type: String 
    },
    /// Generic asset placeholder
    Generic {
        description: String,
        asset_class: String,
    },
}

/// Corporate loan subtypes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LoanType {
    FirstLien,
    SecondLien,
    Revolver,
    Bridge,
    Mezzanine,
}

/// Bond classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BondType {
    HighYield,
    InvestmentGrade,
    Distressed,
    EmergingMarkets,
}

/// Property types for mortgage-backed securities
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PropertyType {
    SingleFamily,
    Multifamily,
    Commercial,
    Industrial,
    Retail,
    Office,
    Hotel,
    Other(String),
}

/// Vehicle types for auto ABS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VehicleType {
    New,
    Used,
    Lease,
    Fleet,
}

/// Credit card portfolio types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CardPortfolioType {
    Prime,
    SubPrime,
    SuperPrime,
    Commercial,
}

/// Student loan types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StudentLoanType {
    Federal,
    Private,
    FFELP,
    Consolidation,
}

// ============================================================================
// PAYMENT & WATERFALL
// ============================================================================

/// Payment distribution modes
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "mode"))]
pub enum PaymentMode {
    /// Normal pro-rata payments to all tranches
    #[cfg_attr(feature = "serde", serde(alias = "pro_rata"))]
    ProRata,
    /// Sequential payment (turbo) due to trigger breach
    #[cfg_attr(feature = "serde", serde(alias = "sequential"))]
    Sequential {
        triggered_by: String,
        trigger_date: Date,
    },
    /// Hybrid mode with custom rules
    #[cfg_attr(feature = "serde", serde(alias = "hybrid"))]
    Hybrid { 
        description: String 
    },
}

/// Consequences when triggers are breached
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TriggerConsequence {
    DivertCashFlow,
    TrapExcessSpread,
    AccelerateAmortization,
    StopReinvestment,
    ReduceManagerFee,
    Custom(String),
}
