//! Unified instrument handling with common operations
//!
//! This module provides the enhanced Instrument enum with common operations
//! that work across all instrument types.

use crate::metrics::MetricId;
use crate::pricing::result::ValuationResult;
use crate::traits::{Attributable, CashflowProvider, Priceable, RiskMeasurable};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::prelude::*;
use finstack_core::F;

use super::{
    Bond, CreditDefaultSwap, CreditOption, Deposit, Equity, EquityOption, FxOption, FxSpot,
    InflationLinkedBond, InterestRateOption, InterestRateSwap, Loan, Swaption,
};

/// Unified instrument wrapper with common operations.
///
/// This enum provides a type-safe way to handle different instrument types
/// while exposing common operations that work across all instruments.
#[derive(Clone, Debug)]
pub enum Instrument {
    Bond(Bond),
    IRS(InterestRateSwap),
    Deposit(Deposit),
    Equity(Equity),
    FxSpot(FxSpot),
    Loan(Loan),
    CDS(CreditDefaultSwap),
    ILB(InflationLinkedBond),
    EquityOption(EquityOption),
    FxOption(FxOption),
    InterestRateOption(InterestRateOption),
    CreditOption(CreditOption),
    Swaption(Swaption),
}

impl Instrument {
    /// Returns the instrument type as a string identifier.
    pub fn instrument_type(&self) -> &'static str {
        match self {
            Self::Bond(_) => "Bond",
            Self::IRS(_) => "InterestRateSwap",
            Self::Deposit(_) => "Deposit",
            Self::Equity(_) => "Equity",
            Self::FxSpot(_) => "FxSpot",
            Self::Loan(_) => "Loan",
            Self::CDS(_) => "CreditDefaultSwap",
            Self::ILB(_) => "InflationLinkedBond",
            Self::EquityOption(_) => "EquityOption",
            Self::FxOption(_) => "FxOption",
            Self::InterestRateOption(_) => "InterestRateOption",
            Self::CreditOption(_) => "CreditOption",
            Self::Swaption(_) => "Swaption",
        }
    }

    /// Get the instrument's unique identifier.
    pub fn id(&self) -> &str {
        match self {
            Self::Bond(b) => &b.id,
            Self::IRS(i) => &i.id,
            Self::Deposit(d) => &d.id,
            Self::Equity(e) => &e.id,
            Self::FxSpot(f) => &f.id,
            Self::Loan(l) => &l.id,
            Self::CDS(c) => &c.id,
            Self::ILB(i) => &i.id,
            Self::EquityOption(e) => &e.id,
            Self::FxOption(f) => &f.id,
            Self::InterestRateOption(i) => &i.id,
            Self::CreditOption(c) => &c.id,
            Self::Swaption(s) => &s.id,
        }
    }

    /// Get the instrument's notional amount if applicable.
    pub fn notional(&self) -> Option<Money> {
        match self {
            Self::Bond(b) => Some(b.notional),
            Self::IRS(i) => Some(i.notional),
            Self::Deposit(d) => Some(d.notional),
            Self::Equity(_) => None,
            Self::FxSpot(f) => f.notional,
            Self::Loan(l) => Some(l.outstanding),
            Self::CDS(c) => Some(c.notional),
            Self::ILB(i) => Some(i.notional),
            Self::EquityOption(e) => Some(e.strike),
            Self::FxOption(f) => Some(f.notional),
            Self::InterestRateOption(i) => Some(i.notional),
            Self::CreditOption(c) => Some(c.notional),
            Self::Swaption(s) => Some(s.notional),
        }
    }

    /// Get the instrument's maturity date if applicable.
    pub fn maturity(&self) -> Option<Date> {
        match self {
            Self::Bond(b) => Some(b.maturity),
            Self::IRS(i) => Some(i.fixed.end),
            Self::Deposit(d) => Some(d.end),
            Self::Equity(_) => None,
            Self::FxSpot(f) => f.settlement,
            Self::Loan(l) => Some(l.maturity_date),
            Self::CDS(c) => Some(c.premium.end),
            Self::ILB(i) => Some(i.maturity),
            Self::EquityOption(e) => Some(e.expiry),
            Self::FxOption(f) => Some(f.expiry),
            Self::InterestRateOption(i) => Some(i.end_date),
            Self::CreditOption(c) => Some(c.expiry),
            Self::Swaption(s) => Some(s.expiry),
        }
    }

    /// Check if this is a derivative instrument.
    pub fn is_derivative(&self) -> bool {
        matches!(
            self,
            Self::IRS(_)
                | Self::CDS(_)
                | Self::EquityOption(_)
                | Self::FxOption(_)
                | Self::InterestRateOption(_)
                | Self::CreditOption(_)
                | Self::Swaption(_)
        )
    }

    /// Check if this is a fixed income instrument.
    pub fn is_fixed_income(&self) -> bool {
        matches!(
            self,
            Self::Bond(_) | Self::Deposit(_) | Self::Loan(_) | Self::ILB(_) | Self::IRS(_)
        )
    }

    /// Check if this is an option instrument.
    pub fn is_option(&self) -> bool {
        matches!(
            self,
            Self::EquityOption(_)
                | Self::FxOption(_)
                | Self::InterestRateOption(_)
                | Self::CreditOption(_)
                | Self::Swaption(_)
        )
    }

    /// Get the primary currency for this instrument.
    pub fn currency(&self) -> Currency {
        match self {
            Self::Bond(b) => b.notional.currency(),
            Self::IRS(i) => i.notional.currency(),
            Self::Deposit(d) => d.notional.currency(),
            Self::Equity(e) => e.currency,
            Self::FxSpot(f) => f.effective_notional().currency(),
            Self::Loan(l) => l.outstanding.currency(),
            Self::CDS(c) => c.notional.currency(),
            Self::ILB(i) => i.notional.currency(),
            Self::EquityOption(e) => e.strike.currency(),
            Self::FxOption(f) => f.notional.currency(),
            Self::InterestRateOption(i) => i.notional.currency(),
            Self::CreditOption(c) => c.notional.currency(),
            Self::Swaption(s) => s.notional.currency(),
        }
    }

    /// Build cashflows if the instrument supports it.
    pub fn build_cashflows(
        &self,
        curves: &CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<Option<Vec<(Date, Money)>>> {
        // Check if instrument implements CashflowProvider
        match self {
            Self::Bond(b) => Ok(Some(b.build_schedule(curves, as_of)?)),
            Self::IRS(i) => Ok(Some(i.build_schedule(curves, as_of)?)),
            Self::Deposit(d) => Ok(Some(d.build_schedule(curves, as_of)?)),
            Self::Loan(l) => Ok(Some(l.build_schedule(curves, as_of)?)),
            Self::ILB(i) => Ok(Some(i.build_schedule(curves, as_of)?)),
            _ => Ok(None), // Options and spot instruments typically don't have schedules
        }
    }

    /// Check if instrument has risk reporting capability.
    pub fn has_risk_reporting(&self) -> bool {
        matches!(self, Self::Bond(_) | Self::IRS(_))
    }

    /// Get risk report if supported.
    pub fn risk_report(
        &self,
        curves: &CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<Option<crate::traits::RiskReport>> {
        match self {
            Self::Bond(b) => Ok(Some(b.risk_report(curves, as_of, None)?)),
            Self::IRS(i) => Ok(Some(i.risk_report(curves, as_of, None)?)),
            _ => Ok(None),
        }
    }
}

// Implement Priceable for the enum by delegating to the underlying type
impl Priceable for Instrument {
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        match self {
            Self::Bond(b) => b.value(curves, as_of),
            Self::IRS(i) => i.value(curves, as_of),
            Self::Deposit(d) => d.value(curves, as_of),
            Self::Equity(e) => e.value(curves, as_of),
            Self::FxSpot(f) => f.value(curves, as_of),
            Self::Loan(l) => l.value(curves, as_of),
            Self::CDS(c) => c.value(curves, as_of),
            Self::ILB(i) => i.value(curves, as_of),
            Self::EquityOption(e) => e.value(curves, as_of),
            Self::FxOption(f) => f.value(curves, as_of),
            Self::InterestRateOption(i) => i.value(curves, as_of),
            Self::CreditOption(c) => c.value(curves, as_of),
            Self::Swaption(s) => s.value(curves, as_of),
        }
    }

    fn price_with_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        match self {
            Self::Bond(b) => b.price_with_metrics(curves, as_of, metrics),
            Self::IRS(i) => i.price_with_metrics(curves, as_of, metrics),
            Self::Deposit(d) => d.price_with_metrics(curves, as_of, metrics),
            Self::Equity(e) => e.price_with_metrics(curves, as_of, metrics),
            Self::FxSpot(f) => f.price_with_metrics(curves, as_of, metrics),
            Self::Loan(l) => l.price_with_metrics(curves, as_of, metrics),
            Self::CDS(c) => c.price_with_metrics(curves, as_of, metrics),
            Self::ILB(i) => i.price_with_metrics(curves, as_of, metrics),
            Self::EquityOption(e) => e.price_with_metrics(curves, as_of, metrics),
            Self::FxOption(f) => f.price_with_metrics(curves, as_of, metrics),
            Self::InterestRateOption(i) => i.price_with_metrics(curves, as_of, metrics),
            Self::CreditOption(c) => c.price_with_metrics(curves, as_of, metrics),
            Self::Swaption(s) => s.price_with_metrics(curves, as_of, metrics),
        }
    }

    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        match self {
            Self::Bond(b) => b.price(curves, as_of),
            Self::IRS(i) => i.price(curves, as_of),
            Self::Deposit(d) => d.price(curves, as_of),
            Self::Equity(e) => e.price(curves, as_of),
            Self::FxSpot(f) => f.price(curves, as_of),
            Self::Loan(l) => l.price(curves, as_of),
            Self::CDS(c) => c.price(curves, as_of),
            Self::ILB(i) => i.price(curves, as_of),
            Self::EquityOption(e) => e.price(curves, as_of),
            Self::FxOption(f) => f.price(curves, as_of),
            Self::InterestRateOption(i) => i.price(curves, as_of),
            Self::CreditOption(c) => c.price(curves, as_of),
            Self::Swaption(s) => s.price(curves, as_of),
        }
    }
}

// Implement Attributable for the enum
impl Attributable for Instrument {
    fn attributes(&self) -> &crate::traits::Attributes {
        match self {
            Self::Bond(b) => b.attributes(),
            Self::IRS(i) => i.attributes(),
            Self::CDS(c) => c.attributes(),
            Self::ILB(i) => i.attributes(),
            _ => {
                // For instruments without attributes, return a static empty set
                // In a real implementation, we'd add attributes field to all instruments
                use once_cell::sync::Lazy;
                static EMPTY: Lazy<crate::traits::Attributes> =
                    Lazy::new(crate::traits::Attributes::new);
                &EMPTY
            }
        }
    }

    fn attributes_mut(&mut self) -> &mut crate::traits::Attributes {
        match self {
            Self::Bond(b) => b.attributes_mut(),
            Self::IRS(i) => i.attributes_mut(),
            Self::CDS(c) => c.attributes_mut(),
            Self::ILB(i) => i.attributes_mut(),
            _ => {
                // This is a limitation - instruments without attributes can't be mutated
                // In production, all instruments should have attributes
                panic!("Instrument type does not support mutable attributes")
            }
        }
    }
}

impl Default for Instrument {
    fn default() -> Self {
        // Default to a simple deposit
        Self::Deposit(Deposit {
            id: String::new(),
            notional: Money::new(0.0, Currency::USD),
            start: Date::from_calendar_date(2020, time::Month::January, 1).unwrap(),
            end: Date::from_calendar_date(2020, time::Month::April, 1).unwrap(),
            day_count: DayCount::Act365F,
            quote_rate: None,
            disc_id: "USD-OIS",
            attributes: Default::default(),
        })
    }
}

/// Portfolio of instruments with aggregation capabilities.
pub struct InstrumentPortfolio {
    instruments: Vec<Instrument>,
    base_currency: Option<Currency>,
}

impl Default for InstrumentPortfolio {
    fn default() -> Self {
        Self::new()
    }
}

impl InstrumentPortfolio {
    pub fn new() -> Self {
        Self {
            instruments: Vec::new(),
            base_currency: None,
        }
    }

    pub fn with_base_currency(mut self, currency: Currency) -> Self {
        self.base_currency = Some(currency);
        self
    }

    pub fn add(&mut self, instrument: Instrument) {
        self.instruments.push(instrument);
    }

    pub fn add_many(&mut self, instruments: impl IntoIterator<Item = Instrument>) {
        self.instruments.extend(instruments);
    }

    /// Filter instruments by type.
    pub fn filter_by_type(&self, instrument_type: &str) -> Vec<&Instrument> {
        self.instruments
            .iter()
            .filter(|i| i.instrument_type() == instrument_type)
            .collect()
    }

    /// Filter instruments by selector pattern.
    pub fn filter_by_selector(&self, selector: &str) -> Vec<&Instrument> {
        self.instruments
            .iter()
            .filter(|i| i.matches_selector(selector))
            .collect()
    }

    /// Calculate total portfolio value.
    pub fn total_value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Vec<Money>> {
        self.instruments
            .iter()
            .map(|i| i.value(curves, as_of))
            .collect()
    }

    /// Group instruments by currency.
    pub fn group_by_currency(&self) -> hashbrown::HashMap<Currency, Vec<&Instrument>> {
        let mut groups = hashbrown::HashMap::new();
        for instrument in &self.instruments {
            groups
                .entry(instrument.currency())
                .or_insert_with(Vec::new)
                .push(instrument);
        }
        groups
    }

    /// Group instruments by type.
    pub fn group_by_type(&self) -> hashbrown::HashMap<&'static str, Vec<&Instrument>> {
        let mut groups = hashbrown::HashMap::new();
        for instrument in &self.instruments {
            groups
                .entry(instrument.instrument_type())
                .or_insert_with(Vec::new)
                .push(instrument);
        }
        groups
    }

    /// Calculate aggregate metrics across the portfolio.
    pub fn aggregate_metrics(
        &self,
        curves: &CurveSet,
        as_of: Date,
    ) -> finstack_core::Result<hashbrown::HashMap<String, F>> {
        let mut aggregates = hashbrown::HashMap::new();

        for instrument in &self.instruments {
            if let Ok(result) = instrument.price(curves, as_of) {
                for (metric, value) in result.measures {
                    *aggregates.entry(metric).or_insert(0.0) += value;
                }
            }
        }

        Ok(aggregates)
    }
}
