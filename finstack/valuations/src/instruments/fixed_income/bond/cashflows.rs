//! Cashflow construction for bonds (deterministic schedules only).

use finstack_core::dates::{BusinessDayConvention, Date, StubKind};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::builder::{cf, FixedCouponSpec};
use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::{CashflowProvider, DatedFlows};

use super::types::Bond;

impl CashflowProvider for Bond {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> Result<DatedFlows> {
        if let Some(ref custom) = self.custom_cashflows {
            let flows: Vec<(Date, Money)> = custom
                .flows
                .iter()
                .filter_map(|cf| match cf.kind {
                    CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                    CFKind::Amortization => Some((cf.date, Money::new(-cf.amount.amount(), cf.amount.currency()))),
                    CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                    _ => None,
                })
                .collect();
            return Ok(flows);
        }

        let mut b = cf();
        b.principal(self.notional, self.issue, self.maturity);
        if let Some(am) = &self.amortization {
            b.amortization(am.clone());
        }
        b.fixed_cf(FixedCouponSpec {
            coupon_type: crate::cashflow::builder::CouponType::Cash,
            rate: self.coupon,
            freq: self.freq,
            dc: self.dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        });
        let sched = b.build()?;

        let flows: Vec<(Date, Money)> = sched
            .flows
            .iter()
            .filter_map(|cf| match cf.kind {
                CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                CFKind::Amortization => Some((cf.date, Money::new(-cf.amount.amount(), cf.amount.currency()))),
                CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                _ => None,
            })
            .collect();

        Ok(flows)
    }
}


