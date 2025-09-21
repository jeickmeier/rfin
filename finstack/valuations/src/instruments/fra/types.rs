//! Forward Rate Agreement (FRA) instrument types and trait implementations.
//!
//! Defines the `ForwardRateAgreement` instrument in the modern instrument style
//! used across valuations. Core PV logic is delegated to the pricing engine in
//! `pricing::engine`, and metrics are provided in the `metrics` submodule.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::traits::{Attributable, Attributes, Instrument};
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;
use std::any::Any;

/// Forward Rate Agreement instrument.
///
/// A FRA is a forward contract on an interest rate. The holder receives
/// the difference between the realized rate and the fixed rate, paid at
/// the start of the interest period (FRA convention).
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct ForwardRateAgreement {
    /// Unique identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
    /// Rate fixing date (start of interest period)
    pub fixing_date: Date,
    /// Interest period start date
    pub start_date: Date,
    /// Interest period end date
    pub end_date: Date,
    /// Fixed rate (decimal, e.g., 0.05 for 5%)
    pub fixed_rate: F,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Reset lag in business days (fixing to value date)
    pub reset_lag: i32,
    /// Discount curve identifier
    pub disc_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Pay/receive flag (true = receive fixed, pay floating)
    pub pay_fixed: bool,
    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl Attributable for ForwardRateAgreement {
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Instrument for ForwardRateAgreement {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn instrument_type(&self) -> &'static str {
        "FRA"
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

impl CashflowProvider for ForwardRateAgreement {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Settlement at start of accrual period; if already settled, no flows
        if self.start_date <= as_of {
            return Ok(vec![]);
        }

        let pv = crate::instruments::fra::pricing::engine::FraEngine::pv(self, curves)?;
        Ok(vec![(self.start_date, pv)])
    }
}
