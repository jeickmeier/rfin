//! Deposit instrument types and trait implementations.
//!
//! Defines the `Deposit` instrument with explicit trait implementations
//! mirroring the modern instrument style used elsewhere in valuations
//! (cf. basis swap). Pricing logic is delegated to the deposit pricing
//! engine in `pricing::engine`.

use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::{Attributable, Attributes, Instrument};
use std::any::Any;

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

impl Attributable for Deposit {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Instrument for Deposit {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn instrument_type(&self) -> &'static str {
        "Deposit"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn attributes(&self) -> &Attributes {
        <Self as Attributable>::attributes(self)
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        <Self as Attributable>::attributes_mut(self)
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }
}

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
