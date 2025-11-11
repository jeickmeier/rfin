//! Cashflow construction for bonds (deterministic schedules only).

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

        // Map CashFlowSchedule to holder view (Date, Money) pairs
        let mut flows: Vec<(Date, Money)> = schedule
            .flows
            .iter()
            .filter_map(|cf| match cf.kind {
                // Include coupons and interest flows as-is (holder receives them)
                CFKind::Fixed | CFKind::FloatReset | CFKind::Stub => Some((cf.date, cf.amount)),
                // Amortization: flip sign (holder receives principal back, stored as positive in schedule)
                CFKind::Amortization => Some((
                    cf.date,
                    Money::new(-cf.amount.amount(), cf.amount.currency()),
                )),
                // Notional: only redemption (positive), exclude initial draw (negative)
                CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                // Exclude other kinds (initial draw, PIK capitalization, etc.)
                _ => None,
            })
            .collect();

        // Sort by date for deterministic ordering
        flows.sort_by_key(|(d, _)| *d);

        Ok(flows)
    }
}
