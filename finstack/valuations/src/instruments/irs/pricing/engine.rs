//! Core IRS pricing engine and helpers.
//!
//! Provides deterministic present value calculation for a vanilla
//! fixed-for-floating interest rate swap. The engine uses the instrument
//! day-counts for accrual and the discount curve's own date helpers for
//! discounting to ensure policy visibility and currency safety.
//!
//! PV = sign × (PV_fixed − PV_float) with sign determined by `PayReceive`.

use crate::instruments::irs::types::{InterestRateSwap, PayReceive};
use finstack_core::market_data::term_structures::{
    discount_curve::DiscountCurve, forward_curve::ForwardCurve,
};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

/// Common IRS pricing engine providing core calculation methods.
pub struct IrsEngine;

impl IrsEngine {
    /// Calculates the present value of an IRS by composing leg PVs.
    pub fn pv(irs: &InterestRateSwap, context: &MarketContext) -> Result<Money> {
        let disc = context.get_ref::<DiscountCurve>(irs.fixed.disc_id.as_ref())?;
        let fwd = context.get_ref::<ForwardCurve>(irs.float.fwd_id.as_ref())?;

        let pv_fixed = irs.pv_fixed_leg(disc)?;
        let pv_float = irs.pv_float_leg(disc, fwd)?;

        let npv = match irs.side {
            PayReceive::PayFixed => (pv_float - pv_fixed)?,
            PayReceive::ReceiveFixed => (pv_fixed - pv_float)?,
        };
        Ok(npv)
    }
}
