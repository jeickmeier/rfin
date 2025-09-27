//! Fixed Income Index Total Return Swap instrument definitions and helpers.

use super::types::{FinancingLegSpec, IndexUnderlyingParams, TrsScheduleSpec, TrsSide};
use crate::{
    cashflow::traits::{CashflowProvider, DatedFlows},
    instruments::common::traits::Attributes,
};
use finstack_core::{
    dates::Date, market_data::MarketContext, money::Money, types::InstrumentId, Result, F,
};

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

// Attributable implementation is provided by the impl_instrument! macro

// Use the macro to implement Instrument with pricing
crate::impl_instrument!(
    FIIndexTotalReturnSwap,
    "FIIndexTotalReturnSwap",
    pv = |s, curves, as_of| {
        use crate::instruments::trs::pricing::engine::TrsEngine;
        use crate::instruments::trs::pricing::fixed_income_index;

        // Calculate total return leg PV
        let total_return_pv = fixed_income_index::pv_total_return_leg(s, curves, as_of)?;

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

impl crate::instruments::common::HasDiscountCurve for FIIndexTotalReturnSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.financing.disc_id
    }
}
