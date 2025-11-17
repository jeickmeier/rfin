//! Cashflow construction for bonds (deterministic schedules only).
//!
//! This module implements the [`CashflowProvider`] trait for [`Bond`], converting
//! the bond's internal cashflow schedule into a simplified holder-view stream
//! of `(Date, Money)` pairs used by pricing and risk engines.
//!
//! # Holder-View Convention
//!
//! All cashflows returned by `build_schedule` follow a **holder-view** convention:
//! - **Positive amounts** represent contractual inflows to a long holder
//!   (coupons, amortization, redemption).
//! - **Initial draw / funding legs** are excluded (handled at trade level).
//!
//! # Examples
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::bond::Bond;
//! use finstack_core::market_data::MarketContext;
//! use finstack_core::dates::Date;
//!
//! # let bond = Bond::example();
//! # let market = MarketContext::new();
//! # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
//! let flows = bond.build_schedule(&market, as_of)?;
//! // flows is Vec<(Date, Money)> with positive holder receipts
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - [`Bond`] for the main bond struct
//! - [`CashflowProvider`] for the trait interface

use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::{CashflowProvider, DatedFlows};

use super::types::Bond;

impl CashflowProvider for Bond {
    fn build_schedule(&self, curves: &MarketContext, _as_of: Date) -> Result<DatedFlows> {
        // Get the full schedule from either custom_cashflows or builder
        let schedule = if let Some(ref custom) = self.custom_cashflows {
            custom.clone()
        } else {
            self.get_full_schedule(curves)?
        };

        // Pre-allocate flows vector with capacity based on schedule size
        // Most cashflows will be included (coupons + amortization + final notional)
        let mut flows: Vec<(Date, Money)> = Vec::with_capacity(schedule.flows.len());

        // Map CashFlowSchedule to holder view (Date, Money) pairs
        //
        // Holder view convention:
        // - All contractual inflows to a long holder (coupons, amortization, redemption)
        //   are POSITIVE amounts.
        // - Cash outflows (e.g., purchase price, funding, shorts) are NEGATIVE and are
        //   handled outside this schedule (e.g., via trade price).
        for cf in &schedule.flows {
            match cf.kind {
                // Include coupons and interest flows as-is (holder receives them)
                CFKind::Fixed | CFKind::FloatReset | CFKind::Stub => {
                    flows.push((cf.date, cf.amount));
                }
                // Amortization principal repayment: schedule already stores amortization
                // as a POSITIVE reduction of outstanding principal. For a long holder
                // this is an inflow, so we keep the sign as-is.
                CFKind::Amortization => {
                    flows.push((cf.date, cf.amount));
                }
                // Notional: only redemption (positive), exclude initial draw (negative)
                CFKind::Notional if cf.amount.amount() > 0.0 => {
                    flows.push((cf.date, cf.amount));
                }
                // Exclude other kinds (initial draw, PIK capitalization, etc.)
                _ => {}
            }
        }

        // Sort by date for deterministic ordering
        flows.sort_by_key(|(d, _)| *d);

        Ok(flows)
    }
}
