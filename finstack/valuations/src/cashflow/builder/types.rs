//! Types for the cashflow builder.
//!
//! These types describe coupon, fee, and scheduling specifications used by
//! `CashflowBuilder` to produce deterministic schedules.

use finstack_core::dates::{Date, DayCount, Frequency, StubKind};
use finstack_core::dates::BusinessDayConvention;
use finstack_core::money::Money;

/// Coupon cashflow type for fixed/floating coupons.
///
/// - `Cash`: 100% paid in cash.
/// - `PIK`: 100% capitalized into principal.
/// - `Split { cash_pct, pik_pct }`: percentages applied to the coupon amount.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CouponType {
    Cash,
    PIK,
    Split { cash_pct: f64, pik_pct: f64 },
}

impl CouponType {
    #[inline]
    pub(crate) fn split_parts(self) -> (f64, f64) {
        match self {
            CouponType::Cash => (1.0, 0.0),
            CouponType::PIK => (0.0, 1.0),
            CouponType::Split { cash_pct, pik_pct } => (cash_pct, pik_pct),
        }
    }
}

/// Fixed coupon specification.
#[derive(Debug, Clone, Copy)]
pub struct FixedCouponSpec {
    pub coupon_type: CouponType,
    pub rate: f64,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
}

/// Floating coupon specification.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct FloatingCouponSpec {
    pub index_id: &'static str,
    pub margin_bp: f64,
    pub gearing: f64,
    pub coupon_type: CouponType,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
    pub reset_lag_days: i32,
}

/// Fee specification.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FeeSpec {
    Fixed { date: Date, amount: Money },
    PeriodicBps { base: FeeBase, bps: f64, freq: Frequency, dc: DayCount, bdc: BusinessDayConvention, calendar_id: Option<&'static str>, stub: StubKind },
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone)]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit − outstanding, 0).
    Undrawn { facility_limit: Money },
}

#[derive(Debug, Clone, Copy)]
pub struct ScheduleParams {
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<&'static str>,
    pub stub: StubKind,
}

#[derive(Debug, Clone, Copy)]
pub struct FloatCouponParams {
    pub index_id: &'static str,
    pub margin_bp: f64,
    pub gearing: f64,
    pub reset_lag_days: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct FixedWindow {
    pub rate: f64,
    pub schedule: ScheduleParams,
}

#[derive(Debug, Clone, Copy)]
pub struct FloatWindow {
    pub params: FloatCouponParams,
    pub schedule: ScheduleParams,
}


