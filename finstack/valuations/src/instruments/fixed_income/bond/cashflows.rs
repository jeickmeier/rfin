//! Cashflow construction for bonds (deterministic schedules only).
//!
//! This module implements the [`CashflowProvider`] trait for [`Bond`], converting
//! the bond's internal cashflow schedule into a simplified holder-view stream
//! of `(Date, Money)` pairs used by pricing and risk engines.
//!
//! # Holder-View Convention
//!
//! All cashflows returned by `CashflowProvider::build_dated_flows` follow a **holder-view** convention:
//! - **Positive amounts** represent contractual inflows to a long holder
//!   (coupons, amortization, redemption).
//! - **Initial draw / funding legs** are excluded (handled at trade level).
//!
//! # Examples
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::fixed_income::bond::Bond;
//! use finstack_valuations::cashflow::CashflowProvider;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::dates::Date;
//!
//! # let bond = Bond::example();
//! # let market = MarketContext::new();
//! # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
//! let flows = bond.build_dated_flows(&market, as_of)?;
//! // flows is Vec<(Date, Money)> with positive holder receipts
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - [`Bond`] for the main bond struct
//! - [`CashflowProvider`] for the trait interface

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::CashflowProvider;

use super::types::Bond;

impl CashflowProvider for Bond {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        _as_of: Date,
    ) -> Result<crate::cashflow::builder::CashFlowSchedule> {
        // Get the full schedule from either custom_cashflows or builder
        let mut schedule = if let Some(ref custom) = self.custom_cashflows {
            custom.clone()
        } else {
            self.get_full_schedule(curves)?
        };

        // Filter flows to holder view, preserving CashFlow objects with CFKind
        let filtered_flows: Vec<crate::cashflow::primitives::CashFlow> = schedule
            .flows
            .iter()
            .filter_map(|cf| {
                match cf.kind {
                    // Include coupons and interest flows
                    CFKind::Fixed | CFKind::FloatReset | CFKind::Stub => Some(*cf),
                    // Include amortization
                    CFKind::Amortization => Some(*cf),
                    // Include positive notional (redemption)
                    CFKind::Notional if cf.amount.amount() > 0.0 => Some(*cf),
                    // Exclude others
                    _ => None,
                }
            })
            .collect();

        schedule.flows = filtered_flows;
        // Sort by date
        schedule.flows.sort_by_key(|cf| cf.date);

        Ok(schedule)
    }
}
