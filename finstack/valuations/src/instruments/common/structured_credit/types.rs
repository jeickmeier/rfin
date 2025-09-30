//! Shared structured credit enums and types used across instruments.

use finstack_core::dates::Date;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type of structured credit deal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// Using lowercase avoids inserting underscores into acronym variants (e.g. `ABS` ->
// `abs` instead of the previous `a_b_s`).
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
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

/// Credit rating for tranches and assets
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
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

    /// Get rating factor for diversity score calculations
    pub fn rating_factor(&self) -> f64 {
        match self {
            Self::AAA => 1.0,
            Self::AA => 2.0,
            Self::A => 4.0,
            Self::BBB => 7.0,
            Self::BB => 13.0,
            Self::B => 27.0,
            Self::CCC | Self::CC | Self::C => 54.0,
            Self::D => 100.0,
            Self::NR => 50.0,
        }
    }
}

/// Tranche seniority in the capital structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

/// Asset type classification for pool assets
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
// These variants are tagged in JSON (e.g. {"Loan": {...}}), so preserve their
// Rust casing instead of converting to snake_case.
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
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
    CreditCard { portfolio_type: CardPortfolioType },
    /// Student loan assets
    StudentLoan { loan_type: StudentLoanType },
    /// Equipment financing
    Equipment { equipment_type: String },
    /// Generic asset placeholder
    Generic {
        description: String,
        asset_class: String,
    },
}

/// Loan type classification
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum LoanType {
    FirstLien,
    SecondLien,
    Revolver,
    Bridge,
    Mezzanine,
}

/// Bond type classification
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BondType {
    HighYield,
    InvestmentGrade,
    Distressed,
    EmergingMarkets,
}

/// Property type for mortgage assets
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

/// Vehicle type for auto loans
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum VehicleType {
    New,
    Used,
    Lease,
    Fleet,
}

/// Credit card portfolio type
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CardPortfolioType {
    Prime,
    SubPrime,
    SuperPrime,
    Commercial,
}

/// Student loan type
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StudentLoanType {
    Federal,
    Private,
    FFELP,
    Consolidation,
}

/// Payment mode for waterfall distribution
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
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

/// Coverage test type
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CoverageTestType {
    OC,
    IC,
    ParValue,
    Custom(String),
}

/// Trigger consequence when coverage tests fail
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TriggerConsequence {
    DivertCashFlow,
    TrapExcessSpread,
    AccelerateAmortization,
    StopReinvestment,
    ReduceManagerFee,
    Custom(String),
}
