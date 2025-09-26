//! Core types for structured credit instruments.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Frequency};
use finstack_core::dates::utils::add_months;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;
use std::any::Any;

use super::coverage_tests::CoverageTests;
use super::pool::AssetPool;
use super::tranches::TrancheStructure;
use super::waterfall::StructuredCreditWaterfall;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type of structured credit deal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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
    NR, // Not Rated
}

impl CreditRating {
    /// Check if rating is investment grade (BBB and above)
    pub fn is_investment_grade(&self) -> bool {
        matches!(self, Self::AAA | Self::AA | Self::A | Self::BBB)
    }

    /// Get rating factor for diversity score calculations
    pub fn rating_factor(&self) -> F {
        match self {
            Self::AAA => 1.0,
            Self::AA => 2.0,
            Self::A => 4.0,
            Self::BBB => 7.0,
            Self::BB => 13.0,
            Self::B => 27.0,
            Self::CCC | Self::CC | Self::C => 54.0,
            Self::D => 100.0,
            Self::NR => 50.0, // Conservative assumption
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
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum AssetType {
    /// Corporate loan (leverage existing Loan type)
    Loan {
        loan_type: LoanType,
        industry: Option<String>,
    },
    /// Corporate bond (leverage existing Bond type)
    Bond {
        bond_type: BondType,
        industry: Option<String>,
    },
    /// Mortgage
    Mortgage {
        property_type: PropertyType,
        ltv: Option<F>, // Loan-to-value ratio
    },
    /// Auto loan
    AutoLoan {
        vehicle_type: VehicleType,
        ltv: Option<F>,
    },
    /// Credit card receivables
    CreditCard { portfolio_type: CardPortfolioType },
    /// Student loan
    StudentLoan { loan_type: StudentLoanType },
    /// Equipment financing
    Equipment { equipment_type: String },
    /// Generic asset
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
    /// First lien term loan
    FirstLien,
    /// Second lien term loan
    SecondLien,
    /// Revolving credit facility
    Revolver,
    /// Bridge loan
    Bridge,
    /// Mezzanine debt
    Mezzanine,
}

/// Bond type classification
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BondType {
    /// High yield corporate bond
    HighYield,
    /// Investment grade corporate bond
    InvestmentGrade,
    /// Distressed debt
    Distressed,
    /// Emerging markets
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
    FFELP, // Federal Family Education Loan Program
    Consolidation,
}

/// Payment mode for waterfall distribution
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PaymentMode {
    /// Normal pro-rata payments to all tranches
    ProRata,
    /// Sequential payment (turbo) due to trigger breach
    Sequential {
        triggered_by: String,
        trigger_date: Date,
    },
    /// Hybrid mode with custom rules
    Hybrid { description: String },
}

/// Coverage test type
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CoverageTestType {
    /// Overcollateralization test
    OC,
    /// Interest coverage test
    IC,
    /// Par value test
    ParValue,
    /// Custom test
    Custom(String),
}

/// Trigger consequence when coverage tests fail
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TriggerConsequence {
    /// Divert cashflow to senior tranches
    DivertCashFlow,
    /// Trap excess spread in reserve account
    TrapExcessSpread,
    /// Accelerate principal payments (turbo)
    AccelerateAmortization,
    /// Stop reinvestment in new assets
    StopReinvestment,
    /// Apply manager fee reduction
    ReduceManagerFee,
    /// Custom action
    Custom(String),
}

/// Main structured credit instrument (CLO/ABS)
#[derive(Debug, Clone, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructuredCredit {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification
    pub deal_type: DealType,

    /// Asset pool
    pub pool: AssetPool,

    /// Tranche structure
    pub tranches: TrancheStructure,

    /// Waterfall distribution rules
    pub waterfall: StructuredCreditWaterfall,

    /// Coverage tests and monitoring
    pub coverage_tests: CoverageTests,

    /// Key dates
    pub closing_date: Date,
    pub first_payment_date: Date,
    pub reinvestment_end_date: Option<Date>,
    pub legal_maturity: Date,

    /// Payment frequency for the structure
    pub payment_frequency: Frequency,

    /// Manager/servicer information
    pub manager_id: Option<String>,
    pub servicer_id: Option<String>,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl StructuredCredit {
    /// Create a new structured credit instrument
    pub fn new(
        id: impl Into<String>,
        deal_type: DealType,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: InstrumentId::new(id_str),
            deal_type,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::April, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::quarterly(),
            manager_id: None,
            servicer_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
        }
    }

    // Derive-based builder available via StructuredCredit::builder()

    /// Calculate current loss percentage of the pool
    pub fn current_loss_percentage(&self) -> F {
        let total_balance = self.pool.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        (self.pool.cumulative_defaults.amount() - self.pool.cumulative_recoveries.amount())
            / total_balance
            * 100.0
    }

    /// Get cashflows for a specific tranche
    pub fn tranche_cashflows(
        &self,
        _tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // This would be implemented to extract tranche-specific flows
        // from the overall structure cashflows
        let _all_flows = self.build_schedule(context, as_of)?;

        // Placeholder: return empty flows for now
        Ok(Vec::new())
    }

    /// Calculate expected life of the structure
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<F> {
        // Simplified calculation based on pool WAL
        Ok(self.pool.weighted_avg_life(as_of))
    }
}

// Removed bespoke StructuredCreditBuilder in favor of derive-based builder.

/// Convenience type alias for CLO instruments
pub type Clo = StructuredCredit;

/// Convenience type alias for ABS instruments  
pub type Abs = StructuredCredit;

// Trait implementations for StructuredCredit

impl CashflowProvider for StructuredCredit {
    fn build_schedule(
        &self,
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // 1. Get pool cashflows by aggregating individual asset cashflows
        let mut pool_flows = Vec::new();

        for asset in &self.pool.assets {
            // Get cashflows based on asset type
            match &asset.asset_type {
                AssetType::Loan { .. } => {
                    // Use existing loan cashflow generation if available
                    // For now, simplified approach
                    let quarterly_interest = asset.balance.amount() * asset.rate / 4.0;
                    let interest_payment = Money::new(quarterly_interest, asset.balance.currency());

                    // Generate quarterly payments (simplified)
                    let mut payment_date = as_of;
                    for _ in 0..20 {
                        // 5 years of quarterly payments using core add_months
                        payment_date = add_months(payment_date, 3);
                        if payment_date <= asset.maturity {
                            pool_flows.push((payment_date, interest_payment));
                        }
                    }

                    // Principal at maturity (simplified)
                    if asset.maturity > as_of {
                        pool_flows.push((asset.maturity, asset.balance));
                    }
                }
                AssetType::Bond { .. } => {
                    // Similar logic for bonds
                    let quarterly_interest = asset.balance.amount() * asset.rate / 4.0;
                    let interest_payment = Money::new(quarterly_interest, asset.balance.currency());
                    pool_flows.push((asset.maturity, interest_payment));
                    pool_flows.push((asset.maturity, asset.balance));
                }
                _ => {
                    // Generic asset - simplified cashflow
                    pool_flows.push((asset.maturity, asset.balance));
                }
            }
        }

        // 2. Sort pool flows by date
        pool_flows.sort_by_key(|(date, _)| *date);

        // 3. Apply pool behavior (prepayments/defaults) - simplified
        // In a full implementation, this would use the loan simulation framework

        // 4. Run through waterfall to get tranche-specific flows
        // For now, return aggregated pool flows
        // In full implementation, this would distribute through waterfall

        Ok(pool_flows)
    }
}

// Structured credit pricing is included in the Instrument trait implementation below

// Attributable is provided via blanket impl for all Instrument types

impl Instrument for StructuredCredit {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn instrument_type(&self) -> &'static str {
        match self.deal_type {
            DealType::CLO => "CLO",
            DealType::CBO => "CBO",
            DealType::ABS => "ABS",
            DealType::RMBS => "RMBS",
            DealType::CMBS => "CMBS",
            DealType::Auto => "AutoABS",
            DealType::Card => "CardABS",
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    // === Pricing Methods ===

    fn value(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        // Get discount curve
        let disc = context
            .get_discount_ref(
            self.disc_id.as_str(),
        )?;

        // Get all cashflows
        let flows = self.build_schedule(context, as_of)?;

        // Discount to present value
        use crate::instruments::common::discountable::Discountable;
        flows.npv(disc, as_of, finstack_core::dates::DayCount::Act360)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        _metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(context, as_of)?;

        // Create basic valuation result
        // In full implementation, would calculate requested metrics
        Ok(ValuationResult::stamped(
            self.id.as_str(),
            as_of,
            base_value,
        ))
    }
}

// Do not add explicit Instrument impl; provided by blanket impl.
