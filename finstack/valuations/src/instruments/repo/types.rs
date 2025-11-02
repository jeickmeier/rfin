//! Core types for Repurchase Agreement (Repo) instruments.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::types::CurveId;

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

impl std::fmt::Display for RepoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoType::Term => write!(f, "term"),
            RepoType::Open => write!(f, "open"),
            RepoType::Overnight => write!(f, "overnight"),
        }
    }
}

impl std::str::FromStr for RepoType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "term" => Ok(RepoType::Term),
            "open" => Ok(RepoType::Open),
            "overnight" => Ok(RepoType::Overnight),
            other => Err(format!("Unknown repo type: {}", other)),
        }
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
        rate_adjustment_bp: Option<f64>,
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
    pub quantity: f64,
    /// Market value identifier in MarketContext (e.g., "BOND_ABC_PRICE")
    pub market_value_id: String,
}

impl CollateralSpec {
    /// Create a new collateral specification.
    pub fn new(
        instrument_id: impl Into<String>,
        quantity: f64,
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
        quantity: f64,
        market_value_id: impl Into<String>,
        rate_adjustment_bp: Option<f64>,
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

        // Derive currency from price scalar; error on unitless to enforce currency safety
        let currency = match price_scalar {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.currency(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(_) => {
                return Err(Error::Input(finstack_core::error::InputError::Invalid));
            }
        };

        Ok(Money::new(unit_value * self.quantity, currency))
    }
}

/// Repurchase Agreement instrument.
#[derive(Debug, Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Repo {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Cash amount being lent/borrowed
    pub cash_amount: Money,
    /// Collateral specification
    pub collateral: CollateralSpec,
    /// Repo rate (annual, as decimal)
    pub repo_rate: f64,
    /// Start date of the repo
    pub start_date: Date,
    /// Maturity date of the repo
    pub maturity: Date,
    /// Haircut percentage (as decimal, e.g., 0.02 = 2%)
    pub haircut: f64,
    /// Type of repo
    pub repo_type: RepoType,
    /// Whether this is a tri-party repo
    pub triparty: bool,
    /// Day count convention for interest calculations
    pub day_count: DayCount,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments
    pub calendar_id: Option<String>,
    /// Discount curve identifier for valuation
    pub discount_curve_id: CurveId,
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
        repo_rate: f64,
        start_date: Date,
        discount_curve_id: impl Into<CurveId>,
    ) -> Result<Self> {
        let maturity = start_date.add_business_days(1, &finstack_core::dates::calendar::TARGET2)?;
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
            .calendar_id_opt(Some("target2".to_string()))
            .discount_curve_id(discount_curve_id.into())
            .attributes(Attributes::default())
            .build()
    }

    /// Create a term repo with specified maturity.
    pub fn term(
        id: impl Into<String>,
        cash_amount: Money,
        collateral: CollateralSpec,
        repo_rate: f64,
        start_date: Date,
        maturity: Date,
        discount_curve_id: impl Into<CurveId>,
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
            .calendar_id_opt(Some("target2".to_string()))
            .discount_curve_id(discount_curve_id.into())
            .attributes(Attributes::default())
            .build()
            .expect("term repo default construction should not fail")
    }

    /// Create an open repo with an initial maturity (can be rolled/terminated later).
    pub fn open(
        id: impl Into<String>,
        cash_amount: Money,
        collateral: CollateralSpec,
        repo_rate: f64,
        start_date: Date,
        initial_maturity: Date,
        discount_curve_id: impl Into<CurveId>,
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
            .calendar_id_opt(Some("target2".to_string()))
            .discount_curve_id(discount_curve_id.into())
            .attributes(Attributes::default())
            .build()
            .expect("open repo default construction should not fail")
    }

    /// Calculate the effective repo rate considering special collateral adjustments.
    pub fn effective_rate(&self) -> f64 {
        match &self.collateral.collateral_type {
            CollateralType::General => self.repo_rate,
            CollateralType::Special {
                rate_adjustment_bp, ..
            } => {
                if let Some(adjustment_bp) = rate_adjustment_bp {
                    const ONE_BP: f64 = 1e-4;
                    self.repo_rate + (adjustment_bp * ONE_BP) // Convert bp to decimal
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

    /// Compute present value of the repo using curves in the market context.
    ///
    /// NPV = PV(total_repayment) - initial_cash_outflow
    /// where total_repayment = principal + interest, and discounting is
    /// performed off the configured discount curve.
    pub fn pv(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        let disc_curve = context.get_discount_ref(self.discount_curve_id.as_str())?;

        // Total repayment at maturity (principal + interest)
        let total_repayment = self.total_repayment()?;

        // Discount from as_of for correct theta
        let disc_dc = disc_curve.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc_curve.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc_curve.df(t_as_of);

        let t_maturity = disc_dc
            .year_fraction(
                disc_curve.base_date(),
                self.maturity,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let t_start = disc_dc
            .year_fraction(
                disc_curve.base_date(),
                self.start_date,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        let df_maturity_abs = disc_curve.df(t_maturity);
        let df_start_abs = disc_curve.df(t_start);

        let df_maturity = if df_as_of != 0.0 {
            df_maturity_abs / df_as_of
        } else {
            1.0
        };
        let df_start = if df_as_of != 0.0 {
            df_start_abs / df_as_of
        } else {
            1.0
        };

        // Present value of inflow at maturity minus PV of initial cash outflow at start
        let pv_in = total_repayment * df_maturity;
        let pv_out = self.cash_amount * df_start;
        pv_in.checked_sub(pv_out)
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

impl Instrument for Repo {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::Repo
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

    fn value(&self, context: &MarketContext, as_of: Date) -> Result<Money> {
        // Use the instrument's own pricing method
        self.pv(context, as_of)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        let base_value = self.value(context, as_of)?;

        // Use existing utility function to build metrics
        crate::instruments::common::helpers::build_with_metrics_dyn(
            self as &dyn Instrument,
            context,
            as_of,
            base_value,
            metrics,
        )
    }
}

// Do not add explicit Instrument impl; provided by blanket impl.

// Attributable is provided via blanket impl for all Instrument types

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

impl crate::instruments::common::pricing::HasDiscountCurve for Repo {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::MarketContext;
    use finstack_core::money::Money;
    use time::Month;

    fn date(y: i32, m: u8, d: u8) -> Date {
        Date::from_calendar_date(y, Month::try_from(m).unwrap(), d).unwrap()
    }

    #[test]
    fn required_collateral_includes_haircut() {
        let collateral = CollateralSpec::new("BOND", 100.0, "BOND_PX");
        let repo = Repo::term(
            "R",
            Money::new(1_000_000.0, Currency::USD),
            collateral,
            0.05,
            date(2025, 1, 1),
            date(2025, 2, 1),
            "USD-OIS",
        );
        let req = repo.required_collateral_value();
        assert!((req.amount() - 1_020_000.0).abs() < 1e-6);
    }

    #[test]
    fn interest_amount_respects_daycount_and_rate() {
        let collateral = CollateralSpec::new("BOND", 100.0, "BOND_PX");
        let repo = Repo::term(
            "R",
            Money::new(1_000_000.0, Currency::USD),
            collateral,
            0.12, // 12%
            date(2025, 1, 1),
            date(2025, 4, 1), // ~0.25 years Act/360 ≈ 0.25
            "USD-OIS",
        );
        let interest = repo.interest_amount().unwrap();
        // Rough check near 1_000_000 * 0.12 * 0.25 = 30_000
        assert!((interest.amount() - 30_000.0).abs() < 100.0);
    }

    #[test]
    fn collateral_value_requires_currency_price() {
        let collateral = CollateralSpec::new("BOND", 100.0, "BOND_PX");
        let repo = Repo::term(
            "R",
            Money::new(1_000_000.0, Currency::USD),
            collateral,
            0.05,
            date(2025, 1, 1),
            date(2025, 2, 1),
            "USD-OIS",
        );
        let ctx = MarketContext::new().insert_price(
            "BOND_PX",
            finstack_core::market_data::scalars::MarketScalar::Unitless(1.0),
        );
        // Should error due to currency safety enforcement
        assert!(repo.collateral.market_value(&ctx).is_err());
    }
}
