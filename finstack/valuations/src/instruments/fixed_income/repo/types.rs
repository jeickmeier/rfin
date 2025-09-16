//! Core types for Repurchase Agreement (Repo) instruments.

use crate::instruments::traits::{Attributable, Attributes, Instrument, Priceable};
use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::prelude::*;
use finstack_core::market_data::MarketContext;
use finstack_core::F;
use std::any::Any;

/// Type of repurchase agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RepoType {
    /// Term repo with fixed maturity date
    Term,
    /// Open repo that can be terminated with notice
    Open,
    /// Overnight repo maturing next business day
    Overnight,
}

impl Default for RepoType {
    fn default() -> Self {
        Self::Term
    }
}

/// Classification of collateral for repos.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CollateralType {
    /// General collateral (standard market rates)
    General,
    /// Special collateral (specific securities in high demand, may trade at lower rates)
    Special {
        /// Identifier of the specific security
        security_id: String,
        /// Optional special rate adjustment in basis points (negative = lower rate)
        rate_adjustment_bp: Option<F>,
    },
}

impl Default for CollateralType {
    fn default() -> Self {
        Self::General
    }
}

/// Specification of collateral backing a repo.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CollateralSpec {
    /// Type of collateral (general vs special)
    pub collateral_type: CollateralType,
    /// Identifier for the collateral instrument
    pub instrument_id: String,
    /// Quantity/face value of collateral
    pub quantity: F,
    /// Market value identifier in MarketContext (e.g., "BOND_ABC_PRICE")
    pub market_value_id: String,
}

impl CollateralSpec {
    /// Create a new collateral specification.
    pub fn new(
        instrument_id: impl Into<String>,
        quantity: F,
        market_value_id: impl Into<String>,
    ) -> Self {
        Self {
            collateral_type: CollateralType::default(),
            instrument_id: instrument_id.into(),
            quantity,
            market_value_id: market_value_id.into(),
        }
    }

    /// Create special collateral specification.
    pub fn special(
        security_id: impl Into<String>,
        instrument_id: impl Into<String>,
        quantity: F,
        market_value_id: impl Into<String>,
        rate_adjustment_bp: Option<F>,
    ) -> Self {
        Self {
            collateral_type: CollateralType::Special {
                security_id: security_id.into(),
                rate_adjustment_bp,
            },
            instrument_id: instrument_id.into(),
            quantity,
            market_value_id: market_value_id.into(),
        }
    }

    /// Calculate the market value of this collateral.
    pub fn market_value(&self, context: &MarketContext) -> Result<Money> {
        let price_scalar = context.price(&self.market_value_id)?;
        let unit_value = match price_scalar {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
        };
        
        // Get currency from price scalar
        let currency = match price_scalar {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.currency(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(_) => Currency::USD, // Default
        };
        
        Ok(Money::new(unit_value * self.quantity, currency))
    }
}

/// Repurchase Agreement instrument.
#[derive(Debug, Clone, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Repo {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Cash amount being lent/borrowed
    pub cash_amount: Money,
    /// Collateral specification
    pub collateral: CollateralSpec,
    /// Repo rate (annual, as decimal)
    pub repo_rate: F,
    /// Start date of the repo
    pub start_date: Date,
    /// Maturity date of the repo
    pub maturity: Date,
    /// Haircut percentage (as decimal, e.g., 0.02 = 2%)
    pub haircut: F,
    /// Type of repo
    pub repo_type: RepoType,
    /// Whether this is a tri-party repo
    pub triparty: bool,
    /// Day count convention for interest calculations
    pub day_count: DayCount,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<&'static str>,
    /// Discount curve identifier for valuation
    pub disc_id: &'static str,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl Repo {
    /// Create a new repo builder (provided by derive).
    /// Create a standard overnight repo.
    pub fn overnight(
        id: impl Into<String>,
        cash_amount: Money,
        collateral: CollateralSpec,
        repo_rate: F,
        start_date: Date,
        disc_id: &'static str,
    ) -> Result<Self> {
        let maturity = start_date.add_business_days(1, &finstack_core::dates::calendar::Target2)?;
        RepoBuilder::new()
            .id(id.into().into())
            .cash_amount(cash_amount)
            .collateral(collateral)
            .repo_rate(repo_rate)
            .start_date(start_date)
            .maturity(maturity)
            .haircut(0.02)
            .repo_type(RepoType::Overnight)
            .triparty(false)
            .day_count(DayCount::Act360)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(Some("target2"))
            .disc_id(disc_id)
            .attributes(Attributes::default())
            .build()
    }

    /// Create a term repo with specified maturity.
    pub fn term(
        id: impl Into<String>,
        cash_amount: Money,
        collateral: CollateralSpec,
        repo_rate: F,
        start_date: Date,
        maturity: Date,
        disc_id: &'static str,
    ) -> Self {
        RepoBuilder::new()
            .id(id.into().into())
            .cash_amount(cash_amount)
            .collateral(collateral)
            .repo_rate(repo_rate)
            .start_date(start_date)
            .maturity(maturity)
            .haircut(0.02)
            .repo_type(RepoType::Term)
            .triparty(false)
            .day_count(DayCount::Act360)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(Some("target2"))
            .disc_id(disc_id)
            .attributes(Attributes::default())
            .build()
            .expect("term repo default construction should not fail")
    }

    /// Create an open repo with an initial maturity (can be rolled/terminated later).
    pub fn open(
        id: impl Into<String>,
        cash_amount: Money,
        collateral: CollateralSpec,
        repo_rate: F,
        start_date: Date,
        initial_maturity: Date,
        disc_id: &'static str,
    ) -> Self {
        RepoBuilder::new()
            .id(id.into().into())
            .cash_amount(cash_amount)
            .collateral(collateral)
            .repo_rate(repo_rate)
            .start_date(start_date)
            .maturity(initial_maturity)
            .haircut(0.02)
            .repo_type(RepoType::Open)
            .triparty(false)
            .day_count(DayCount::Act360)
            .bdc(BusinessDayConvention::Following)
            .calendar_id_opt(Some("target2"))
            .disc_id(disc_id)
            .attributes(Attributes::default())
            .build()
            .expect("open repo default construction should not fail")
    }

    /// Calculate the effective repo rate considering special collateral adjustments.
    pub fn effective_rate(&self) -> F {
        match &self.collateral.collateral_type {
            CollateralType::General => self.repo_rate,
            CollateralType::Special { rate_adjustment_bp, .. } => {
                if let Some(adjustment_bp) = rate_adjustment_bp {
                    self.repo_rate + (adjustment_bp / 10_000.0) // Convert bp to decimal
                } else {
                    self.repo_rate
                }
            }
        }
    }

    /// Calculate required collateral value including haircut.
    pub fn required_collateral_value(&self) -> Money {
        self.cash_amount * (1.0 + self.haircut)
    }

    /// Check if the repo is adequately collateralized.
    pub fn is_adequately_collateralized(&self, context: &MarketContext) -> Result<bool> {
        let collateral_value = self.collateral.market_value(context)?;
        let required_value = self.required_collateral_value();
        
        // Ensure same currency for comparison
        if collateral_value.currency() != required_value.currency() {
            return Err(Error::CurrencyMismatch {
                expected: required_value.currency(),
                actual: collateral_value.currency(),
            });
        }
        
        Ok(collateral_value.amount() >= required_value.amount())
    }

    /// Calculate repo interest amount.
    pub fn interest_amount(&self) -> Result<Money> {
        let year_fraction = self.day_count.year_fraction(
            self.start_date,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        
        let effective_rate = self.effective_rate();
        let interest = self.cash_amount.amount() * effective_rate * year_fraction;
        
        Ok(Money::new(interest, self.cash_amount.currency()))
    }

    /// Calculate total repayment amount (principal + interest).
    pub fn total_repayment(&self) -> Result<Money> {
        let interest = self.interest_amount()?;
        self.cash_amount.checked_add(interest)
    }
}

impl Priceable for Repo {
    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Get discount curve
        let disc_curve = context
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                self.disc_id,
            )?;
        
        // Calculate total repayment at maturity
        let total_repayment = self.total_repayment()?;
        
        // Calculate time to maturity
        let time_to_maturity = self.day_count.year_fraction(
            as_of,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        
        // Discount back to valuation date
        let df = disc_curve.df(time_to_maturity);
        let pv = total_repayment * df;
        
        // For repo buyer (cash lender), subtract initial cash outflow
        let net_pv = pv.checked_sub(self.cash_amount)?;
        
        Ok(net_pv)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let base_value = <Self as Priceable>::value(self, context, as_of)?;
        
        // Use existing utility function to build metrics
        crate::instruments::utils::build_with_metrics_dyn(
            self as &dyn Instrument,
            context,
            as_of,
            base_value,
            metrics,
        )
    }
}

impl Instrument for Repo {
    fn id(&self) -> &str { self.id.as_str() }
    fn instrument_type(&self) -> &'static str { "Repo" }
    fn as_any(&self) -> &dyn Any { self }
    fn attributes(&self) -> &Attributes { &self.attributes }
    fn attributes_mut(&mut self) -> &mut Attributes { &mut self.attributes }
    fn clone_box(&self) -> Box<dyn Instrument> { Box::new(self.clone()) }
}

// Do not add explicit Instrument impl; provided by blanket impl.

impl Attributable for Repo {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl CashflowProvider for Repo {
    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        let mut flows = Vec::new();
        
        // Initial cash outflow (lending cash) - negative amount for outflow
        let cash_outflow = Money::new(-self.cash_amount.amount(), self.cash_amount.currency());
        flows.push((self.start_date, cash_outflow));
        
        // Final cash inflow (principal + interest)
        let total_repayment = self.total_repayment()?;
        flows.push((self.maturity, total_repayment));
        
        Ok(flows)
    }
}
