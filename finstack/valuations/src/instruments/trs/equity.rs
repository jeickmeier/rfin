//! Equity Total Return Swap instrument definitions and helpers.

use super::types::{FinancingLegSpec, TrsScheduleSpec, TrsSide};
use crate::instruments::traits::{Attributable, Instrument};
use crate::{
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::{traits::Attributes, underlying::EquityUnderlyingParams},
};
use finstack_core::{
    dates::Date, market_data::MarketContext, money::Money, types::InstrumentId, Result, F,
};
use std::any::Any;

/// Equity Total Return Swap instrument.
///
/// A TRS where the total return leg is based on an equity index or single stock.
/// The holder receives the total return (price appreciation + dividends) of the underlying
/// equity in exchange for paying a floating rate plus spread on the notional amount.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::trs::EquityTotalReturnSwap;
/// use finstack_core::{money::Money, currency::Currency, dates::Date};
/// use time::Month;
///
/// let trs = EquityTotalReturnSwap::builder()
///     .id("EQ_TRS_001".into())
///     .notional(Money::new(1_000_000.0, Currency::USD))
///     .build();
/// ```
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EquityTotalReturnSwap {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Notional amount for the swap.
    pub notional: Money,
    /// Underlying equity parameters (spot ID, dividend yield, contract size).
    pub underlying: EquityUnderlyingParams,
    /// Financing leg specification (curves, spread, day count).
    pub financing: FinancingLegSpec,
    /// Schedule specification (payment dates and frequency).
    pub schedule: TrsScheduleSpec,
    /// Trade side (receive/pay total return).
    pub side: TrsSide,
    /// Initial index level (if known, otherwise fetched from market).
    pub initial_level: Option<F>,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl EquityTotalReturnSwap {}

impl Attributable for EquityTotalReturnSwap {
    fn attributes(&self) -> &crate::instruments::traits::Attributes {
        // For now, return a static empty attributes
        // In a real implementation, this would be a field in the struct
        static EMPTY: once_cell::sync::Lazy<crate::instruments::traits::Attributes> =
            once_cell::sync::Lazy::new(crate::instruments::traits::Attributes::default);
        &EMPTY
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes {
        // This would normally return a mutable reference to an attributes field
        // For now, we'll panic as this is not properly implemented
        unimplemented!("Mutable attributes not yet implemented for EquityTotalReturnSwap")
    }
}

impl Instrument for EquityTotalReturnSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn instrument_type(&self) -> &'static str {
        "EquityTotalReturnSwap"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn attributes(&self) -> &crate::instruments::traits::Attributes {
        <Self as Attributable>::attributes(self)
    }
    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes {
        <Self as Attributable>::attributes_mut(self)
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }
}

impl CashflowProvider for EquityTotalReturnSwap {
    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // For TRS, we'll return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = self.schedule.period_schedule();

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            // In practice, the amount would be determined at fixing
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}
