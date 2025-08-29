//! Deposit instrument implementation.

pub mod metrics;

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;

use crate::traits::{CashflowProvider, DatedFlows, Attributes};
use crate::cashflow::builder::{cf, FixedCouponSpec, CouponType};
use finstack_core::dates::{BusinessDayConvention, StubKind, Frequency};
use crate::metrics::MetricId;

/// Simple deposit instrument with optional quoted rate.
/// 
/// Represents a single-period deposit where principal is exchanged
/// at start and principal plus interest at maturity.
#[derive(Clone, Debug)]
pub struct Deposit {
    /// Unique identifier for the deposit.
    pub id: String,
    /// Principal amount of the deposit.
    pub notional: Money,
    /// Start date of the deposit period.
    pub start: Date,
    /// End date of the deposit period.
    pub end: Date,
    /// Day count convention for interest accrual.
    pub day_count: DayCount,
    /// Optional quoted simple rate r (annualised) for the deposit.
    pub quote_rate: Option<F>,
    /// Discount curve id used for valuation and par extraction.
    pub disc_id: &'static str,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

// Apply the instrument! macro to generate ALL boilerplate:
// - Priceable trait implementation
// - Attributable trait implementation  
// - Builder pattern with all setters
// - Conversions to/from unified Instrument enum
instrument! {
    Deposit {
        metrics: [
            MetricId::Yf,
            MetricId::DfStart,
            MetricId::DfEnd,
            MetricId::DepositParRate
        ],
        required: [
            id: String,
            notional: Money,
            start: Date,
            end: Date,
            day_count: DayCount,
            disc_id: &'static str
        ],
        optional: [
            quote_rate: F
        ]
    }
}

impl CashflowProvider for Deposit {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Build a single-period schedule using a custom day-step equal to the total span
        let days = (self.end - self.start).whole_days();
        let rate = self.quote_rate.unwrap_or(0.0);

        let mut b = cf();
        b.principal(self.notional, self.start, self.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate,
                freq: if days <= 1 { Frequency::daily() } 
                      else if days == 7 { Frequency::weekly() } 
                      else if days == 14 { Frequency::biweekly() } 
                      else { Frequency::monthly() },
                dc: self.day_count,
                bdc: BusinessDayConvention::Unadjusted,
                calendar_id: None,
                stub: StubKind::None,
            });
        let sched = b.build()?;

        // Map to two-flow holder schedule: principal out at start; redemption at end including interest
        // Sum all amounts on end date except the initial notional outflow
        let mut redemption = Money::new(0.0, self.notional.currency());
        for cf in &sched.flows {
            if cf.date == self.end {
                // Include both coupon and final notional
                redemption = (redemption + cf.amount)?;
            }
        }
        Ok(vec![(self.start, self.notional * -1.0), (self.end, redemption)])
    }
}