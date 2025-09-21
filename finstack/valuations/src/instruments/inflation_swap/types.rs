//! Zero-coupon Inflation Swap types and pricing implementation.

use crate::instruments::common::traits::Attributes;
use finstack_core::market_data::scalars::inflation_index::InflationLag;
use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::F;

/// Direction from the perspective of paying fixed real vs receiving inflation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceiveInflation {
    /// Pay fixed (real) leg, receive inflation leg
    PayFixed,
    /// Receive fixed (real) leg, pay inflation leg
    ReceiveFixed,
}

/// Inflation swap definition (boilerplate)
///
/// Minimal fields to represent a zero-coupon inflation swap. We keep this
/// intentionally compact until full pricing is implemented.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct InflationSwap {
    /// Unique instrument identifier
    pub id: String,
    /// Notional in quote currency
    pub notional: Money,
    /// Start date of indexation
    pub start: Date,
    /// Maturity date
    pub maturity: Date,
    /// Fixed real rate (as decimal)
    pub fixed_rate: F,
    /// Inflation index identifier (e.g., US-CPI-U)
    pub inflation_id: &'static str,
    /// Discount curve identifier (quote currency)
    pub disc_id: &'static str,
    /// Day count for any accrual-style metrics if needed
    pub dc: DayCount,
    /// Trade side
    pub side: PayReceiveInflation,
    /// Optional contract-level lag override (if set, overrides index lag)
    #[builder(optional)]
    pub lag_override: Option<InflationLag>,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl InflationSwap {}

impl InflationSwap {
    /// Calculate PV of the fixed leg (real rate leg)
    pub fn pv_fixed_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let pricer = crate::instruments::inflation_swap::pricing::InflationSwapPricer::new();
        pricer.pv_fixed_leg(self, curves, as_of)
    }

    /// Calculate PV of the inflation leg
    pub fn pv_inflation_leg(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let pricer = crate::instruments::inflation_swap::pricing::InflationSwapPricer::new();
        pricer.pv_inflation_leg(self, curves, as_of)
    }
}

impl_instrument!(
    InflationSwap,
    "InflationSwap",
    pv = |s, curves, as_of| {
        // Calculate PV of both legs
        let pv_fixed = s.pv_fixed_leg(curves, as_of)?;
        let pv_inflation = s.pv_inflation_leg(curves, as_of)?;

        // Net PV based on trade direction
        match s.side {
            PayReceiveInflation::ReceiveFixed => pv_fixed - pv_inflation,
            PayReceiveInflation::PayFixed => pv_inflation - pv_fixed,
        }
    },
);
