//! Deposit instrument types and pricing impl.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::F;
use finstack_core::types::{CurveId, InstrumentId};

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::traits::Attributes;

/// Simple deposit instrument with optional quoted rate.
///
/// Represents a single-period deposit where principal is exchanged
/// at start and principal plus interest at maturity.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct Deposit {
    /// Unique identifier for the deposit.
    pub id: InstrumentId,
    /// Principal amount of the deposit.
    pub notional: Money,
    /// Start date of the deposit period.
    pub start: Date,
    /// End date of the deposit period.
    pub end: Date,
    /// Day count convention for interest accrual.
    pub day_count: DayCount,
    
    /// Optional quoted simple rate r (annualised) for the deposit.
    #[builder(optional)]
    pub quote_rate: Option<F>,
    /// Discount curve id used for valuation and par extraction.
    pub disc_id: CurveId,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl_instrument_schedule_pv!(
    Deposit, "Deposit",
    disc_field: disc_id,
    dc_field: day_count
);

impl CashflowProvider for Deposit {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // True single-period deposit: two flows with simple interest
        let yf = self
            .day_count
            .year_fraction(
                self.start,
                self.end,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let r = self.quote_rate.unwrap_or(0.0);
        let redemption = self.notional * (1.0 + r * yf);
        Ok(vec![
            (self.start, self.notional * -1.0),
            (self.end, redemption),
        ])
    }
}
