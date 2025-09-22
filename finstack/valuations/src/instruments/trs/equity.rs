//! Equity Total Return Swap instrument definitions and helpers.

use super::types::{FinancingLegSpec, TrsScheduleSpec, TrsSide};
use crate::{
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::{traits::Attributes, underlying::EquityUnderlyingParams},
};
use finstack_core::{
    dates::Date, market_data::MarketContext, money::Money, types::InstrumentId, Result, F,
};

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

// Attributable implementation is provided by the impl_instrument! macro

// Use the macro to implement Instrument with pricing
crate::impl_instrument!(
    EquityTotalReturnSwap,
    "EquityTotalReturnSwap",
    pv = |s, curves, as_of| {
        use crate::instruments::trs::pricing::engine::TrsEngine;
        use crate::instruments::trs::pricing::equity;

        // Calculate total return leg PV
        let total_return_pv = equity::pv_total_return_leg(s, curves, as_of)?;

        // Calculate financing leg PV
        let financing_pv =
            TrsEngine::pv_financing_leg(&s.financing, &s.schedule, s.notional, curves, as_of)?;

        // Net PV depends on side
        let net_pv = match s.side {
            super::TrsSide::ReceiveTotalReturn => (total_return_pv - financing_pv)?,
            super::TrsSide::PayTotalReturn => (financing_pv - total_return_pv)?,
        };

        Ok(net_pv)
    }
);

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
