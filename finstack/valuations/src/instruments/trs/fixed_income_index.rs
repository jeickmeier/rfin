//! Fixed Income Index Total Return Swap instrument definitions and helpers.

use super::types::{FinancingLegSpec, IndexUnderlyingParams, TrsScheduleSpec, TrsSide};
use crate::instruments::common::traits::{Attributable, Instrument};
use crate::{
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::common::traits::Attributes,
};
use finstack_core::{
    dates::Date, market_data::MarketContext, money::Money, types::InstrumentId, Result, F,
};
use std::any::Any;

/// Fixed Income Index Total Return Swap instrument.
///
/// A TRS where the total return leg is based on a fixed income index (e.g., corporate bond index).
/// The holder receives the total return (carry + roll) of the underlying index in exchange
/// for paying a floating rate plus spread on the notional amount.
///
/// # Examples
/// ```rust
/// use finstack_valuations::instruments::trs::FIIndexTotalReturnSwap;
/// use finstack_core::{money::Money, currency::Currency, types::id::IndexId};
///
/// let trs = FIIndexTotalReturnSwap::builder()
///     .id("FI_TRS_001".into())
///     .notional(Money::new(1_000_000.0, Currency::USD))
///     .build();
/// ```
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FIIndexTotalReturnSwap {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Notional amount for the swap.
    pub notional: Money,
    /// Underlying index parameters (index ID, yield, duration, base currency).
    pub underlying: IndexUnderlyingParams,
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

impl FIIndexTotalReturnSwap {}

impl Attributable for FIIndexTotalReturnSwap {
    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        // For now, return a static empty attributes
        // In a real implementation, this would be a field in the struct
        static EMPTY: once_cell::sync::Lazy<crate::instruments::common::traits::Attributes> =
            once_cell::sync::Lazy::new(crate::instruments::common::traits::Attributes::default);
        &EMPTY
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        // This would normally return a mutable reference to an attributes field
        // For now, we'll panic as this is not properly implemented
        unimplemented!("Mutable attributes not yet implemented for FIIndexTotalReturnSwap")
    }
}

impl Instrument for FIIndexTotalReturnSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn instrument_type(&self) -> &'static str {
        "FIIndexTotalReturnSwap"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        <Self as Attributable>::attributes(self)
    }
    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        <Self as Attributable>::attributes_mut(self)
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }
}

impl CashflowProvider for FIIndexTotalReturnSwap {
    fn build_schedule(&self, _context: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // For TRS, we'll return the expected payment dates
        // Actual amounts depend on realized returns
        let period_schedule = self.schedule.period_schedule();

        let mut flows = Vec::new();
        for date in period_schedule.dates.iter().skip(1) {
            // Add a placeholder flow for each payment date
            flows.push((*date, Money::new(0.0, self.notional.currency())));
        }

        Ok(flows)
    }
}
