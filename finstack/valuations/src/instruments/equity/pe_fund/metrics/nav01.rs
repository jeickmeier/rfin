//! NAV01 calculator for PrivateMarketsFund.
//!
//! Computes NAV01 (residual NAV sensitivity) using finite differences.
//! NAV01 measures the change in PV for a 1% change in residual NAV.
//!
//! # Formula
//! ```text
//! NAV01 = (PV(events scaled * 1.01) - PV(events scaled * 0.99)) / (2 * bump_size)
//! ```
//! Where bump_size is 1% (0.01).
//!
//! # Note
//! Residual NAV is the output of the waterfall calculation (`lp_unreturned`).
//! To measure NAV sensitivity, we scale all distribution/proceeds events by ±1%
//! and observe the impact on PV. This captures how changes in fund performance
//! (which affect NAV) impact the LP valuation.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::pe_fund::PrivateMarketsFund;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::Result;

/// Standard NAV bump: 1% (0.01)
const NAV_BUMP_PCT: f64 = 0.01;

/// NAV01 calculator for PrivateMarketsFund.
pub(crate) struct Nav01Calculator;

impl MetricCalculator for Nav01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let fund: &PrivateMarketsFund = context.instrument_as()?;
        let as_of = context.as_of;

        // Scale all distribution/proceeds events up by 1% (affects NAV)
        let mut fund_up = fund.clone();
        for event in &mut fund_up.events {
            use crate::instruments::equity::pe_fund::waterfall::FundEventKind;
            match event.kind {
                FundEventKind::Distribution | FundEventKind::Proceeds => {
                    event.amount = finstack_core::money::Money::new(
                        event.amount.amount() * (1.0 + NAV_BUMP_PCT),
                        event.amount.currency(),
                    );
                }
                _ => {
                    // Contributions and other events unchanged
                }
            }
        }
        let pv_up = fund_up.value(context.curves.as_ref(), as_of)?.amount();

        // Scale all distribution/proceeds events down by 1%
        let mut fund_down = fund.clone();
        for event in &mut fund_down.events {
            use crate::instruments::equity::pe_fund::waterfall::FundEventKind;
            match event.kind {
                FundEventKind::Distribution | FundEventKind::Proceeds => {
                    event.amount = finstack_core::money::Money::new(
                        event.amount.amount() * (1.0 - NAV_BUMP_PCT),
                        event.amount.currency(),
                    );
                }
                _ => {
                    // Contributions and other events unchanged
                }
            }
        }
        let pv_down = fund_down.value(context.curves.as_ref(), as_of)?.amount();

        // NAV01 = (PV_up - PV_down) / (2 * bump_size)
        // Result is per 1% change in NAV (via event scaling)
        let nav01 = (pv_up - pv_down) / (2.0 * NAV_BUMP_PCT);

        Ok(nav01)
    }
}
