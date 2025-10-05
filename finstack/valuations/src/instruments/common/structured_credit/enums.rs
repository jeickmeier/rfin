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

/// Asset type classification for pool composition (flattened hierarchy)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum AssetType {
    // ========== LOAN TYPES ==========
    /// First lien corporate loan
    FirstLienLoan { industry: Option<String> },
    /// Second lien corporate loan
    SecondLienLoan { industry: Option<String> },
    /// Revolving credit facility
    RevolverLoan { industry: Option<String> },
    /// Bridge loan
    BridgeLoan { industry: Option<String> },
    /// Mezzanine loan
    MezzanineLoan { industry: Option<String> },

    // ========== BOND TYPES ==========
    /// High yield bond
    HighYieldBond { industry: Option<String> },
    /// Investment grade bond
    InvestmentGradeBond { industry: Option<String> },
    /// Distressed bond
    DistressedBond { industry: Option<String> },
    /// Emerging markets bond
    EmergingMarketsBond { industry: Option<String> },

    // ========== MORTGAGE TYPES ==========
    /// Single family residential mortgage
    SingleFamilyMortgage { ltv: Option<f64> },
    /// Multifamily residential mortgage
    MultifamilyMortgage { ltv: Option<f64> },
    /// Commercial real estate mortgage
    CommercialMortgage { ltv: Option<f64> },
    /// Industrial property mortgage
    IndustrialMortgage { ltv: Option<f64> },
    /// Retail property mortgage
    RetailMortgage { ltv: Option<f64> },
    /// Office property mortgage
    OfficeMortgage { ltv: Option<f64> },
    /// Hotel property mortgage
    HotelMortgage { ltv: Option<f64> },
    /// Other property type mortgage
    OtherMortgage {
        property_type: String,
        ltv: Option<f64>,
    },

    // ========== AUTO LOAN TYPES ==========
    /// New vehicle auto loan
    NewAutoLoan { ltv: Option<f64> },
    /// Used vehicle auto loan
    UsedAutoLoan { ltv: Option<f64> },
    /// Vehicle lease
    LeaseAutoLoan { ltv: Option<f64> },
    /// Fleet vehicle loan
    FleetAutoLoan { ltv: Option<f64> },

    // ========== CREDIT CARD TYPES ==========
    /// Prime credit card receivables
    PrimeCreditCard,
    /// Subprime credit card receivables
    SubPrimeCreditCard,
    /// Super prime credit card receivables
    SuperPrimeCreditCard,
    /// Commercial credit card receivables
    CommercialCreditCard,

    // ========== STUDENT LOAN TYPES ==========
    /// Federal student loan
    FederalStudentLoan,
    /// Private student loan
    PrivateStudentLoan,
    /// FFELP student loan
    FFELPStudentLoan,
    /// Consolidation student loan
    ConsolidationStudentLoan,

    // ========== OTHER TYPES ==========
    /// Equipment financing
    Equipment { equipment_type: String },
    /// Generic asset placeholder
    Generic {
        description: String,
        asset_class: String,
    },
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
    Hybrid { description: String },
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
