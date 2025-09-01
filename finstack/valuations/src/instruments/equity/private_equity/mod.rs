//! Private equity investment instruments with equity waterfall engine.
//!
//! This module provides a private equity fund investment instrument that supports
//! standard waterfall allocation logic including return of capital, preferred IRR
//! hurdles, catch-up provisions, promote splits, and clawback mechanisms.
//!
//! # Key Features
//!
//! - **Waterfall Processing**: Deterministic allocation across tranches per PE conventions
//! - **American vs European**: Support both deal-by-deal and fund-level allocation styles  
//! - **IRR Hurdles**: Robust root-finding for preferred return calculations
//! - **Auditable Ledgers**: Complete allocation history with DataFrame export
//! - **Currency Safety**: Single-currency enforcement with explicit FX conversion
//! - **Serde Stable**: Stable serialization shapes across bindings
//!
//! # Quick Example
//!
//! ```rust
//! use finstack_valuations::instruments::equity::private_equity::*;
//! use finstack_core::prelude::*;
//! use time::Month;
//!
//! let spec = WaterfallSpec::builder()
//!     .style(WaterfallStyle::European)
//!     .return_of_capital()
//!     .preferred_irr(0.08) // 8% hurdle
//!     .catchup(1.0) // 100% to GP until target split
//!     .promote_tier(0.0, 0.8, 0.2) // 80/20 split
//!     .build().unwrap();
//!
//! let events = vec![
//!     FundEvent::contribution(Date::from_calendar_date(2020, Month::January, 1).unwrap(),
//!                            Money::new(1000000.0, Currency::USD)),
//!     FundEvent::distribution(Date::from_calendar_date(2025, Month::January, 1).unwrap(),
//!                            Money::new(1500000.0, Currency::USD)),
//! ];
//!
//! let investment = PrivateEquityInvestment::new("PE_FUND_A", Currency::USD, spec, events);
//! let ledger = investment.run_waterfall().unwrap();
//! let (columns, rows) = ledger.to_tabular_data();
//! println!("Allocation ledger has {} rows with {} columns", rows.len(), columns.len());
//! ```

pub mod metrics;
pub mod waterfall;

pub use metrics::*;
pub use waterfall::*;

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::traits::Attributes;
use crate::metrics::MetricRegistry;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;

/// Private equity fund investment instrument.
///
/// Represents an LP's investment in a private equity fund with configurable
/// waterfall allocation rules. The instrument can price the investment's NPV
/// and compute standard PE performance metrics.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PrivateEquityInvestment {
    /// Unique identifier for the investment
    pub id: String,
    /// Currency of the fund (all events must match)
    pub currency: Currency,
    /// Waterfall specification defining allocation rules
    pub spec: WaterfallSpec,
    /// Fund cash flow events (contributions and distributions)
    pub events: Vec<FundEvent>,
    /// Optional discount curve for NPV calculations
    pub disc_id: Option<&'static str>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl PrivateEquityInvestment {
    /// Create a new private equity investment.
    pub fn new(
        id: impl Into<String>,
        currency: Currency,
        spec: WaterfallSpec,
        events: Vec<FundEvent>,
    ) -> Self {
        Self {
            id: id.into(),
            currency,
            spec,
            events,
            disc_id: None,
            attributes: Attributes::new(),
        }
    }

    /// Set the discount curve for NPV calculations.
    pub fn with_discount_curve(mut self, disc_id: &'static str) -> Self {
        self.disc_id = Some(disc_id);
        self
    }

    /// Run the waterfall allocation and return the complete ledger.
    pub fn run_waterfall(&self) -> finstack_core::Result<AllocationLedger> {
        // Validate single currency
        for event in &self.events {
            if event.amount.currency() != self.currency {
                return Err(finstack_core::Error::CurrencyMismatch {
                    expected: self.currency,
                    actual: event.amount.currency(),
                });
            }
        }

        let engine = EquityWaterfallEngine::new(&self.spec);
        engine.run(&self.events)
    }

    /// Get LP-only cashflows for NPV calculation.
    pub fn lp_cashflows(&self) -> finstack_core::Result<Vec<(Date, Money)>> {
        let ledger = self.run_waterfall()?;
        Ok(ledger.lp_cashflows())
    }

    // Removed legacy standard metrics helper.
}

// Manual Priceable implementation since we have optional disc_id
crate::impl_attributable!(PrivateEquityInvestment);
crate::impl_instrument_like!(PrivateEquityInvestment, "PrivateEquityInvestment");

impl crate::instruments::traits::Priceable for PrivateEquityInvestment {
    fn value(&self, curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        // If discount curve is specified, calculate NPV of LP flows
        if let Some(disc_id) = self.disc_id {
            use crate::instruments::fixed_income::discountable::Discountable;
            let flows = self.lp_cashflows()?;
            let disc = curves.discount(disc_id)?;
            flows.npv(&*disc, disc.base_date(), self.spec.irr_basis)
        } else {
            // Return residual LP value from waterfall
            let ledger = self.run_waterfall()?;
            let residual_value = ledger
                .rows
                .last()
                .map(|r| r.lp_unreturned)
                .unwrap_or_else(|| Money::new(0.0, self.currency));
            Ok(residual_value)
        }
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::build_with_metrics(self.clone(), curves, as_of, base_value, metrics)
    }
}

impl CashflowProvider for PrivateEquityInvestment {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        self.lp_cashflows()
    }
}

/// Register private equity metrics in the registry.
pub fn register_private_equity_metrics(registry: &mut MetricRegistry) {
    metrics::register_private_equity_metrics(registry);
}
