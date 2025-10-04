//! Types for the cashflow builder.
//!
//! These types describe coupon, fee, and scheduling specifications used by
//! `CashflowBuilder` to produce deterministic schedules.

use finstack_core::dates::BusinessDayConvention;
use finstack_core::dates::{Date, DayCount, Frequency, StubKind};
use finstack_core::error::InputError;
use finstack_core::money::Money;
use finstack_core::types::CurveId;

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
    pub(crate) fn split_parts(self) -> finstack_core::Result<(f64, f64)> {
        match self {
            CouponType::Cash => Ok((1.0, 0.0)),
            CouponType::PIK => Ok((0.0, 1.0)),
            CouponType::Split { cash_pct, pik_pct } => {
                // Validate finite and within [0,1]
                if !cash_pct.is_finite() || !pik_pct.is_finite() {
                    return Err(InputError::Invalid.into());
                }
                if !(0.0..=1.0).contains(&cash_pct) || !(0.0..=1.0).contains(&pik_pct) {
                    return Err(InputError::Invalid.into());
                }
                // Sum must be ~ 1.0; normalize within tolerance
                let sum = cash_pct + pik_pct;
                let tol = 1e-6;
                if (sum - 1.0).abs() <= tol {
                    let norm_cash = cash_pct / sum;
                    let norm_pik = pik_pct / sum;
                    Ok((norm_cash, norm_pik))
                } else {
                    Err(InputError::Invalid.into())
                }
            }
        }
    }
}

/// Fixed coupon specification.
#[derive(Debug, Clone)]
pub struct FixedCouponSpec {
    pub coupon_type: CouponType,
    pub rate: f64,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<String>,
    pub stub: StubKind,
}

/// Floating coupon specification.
#[derive(Debug, Clone)]
pub struct FloatingCouponSpec {
    pub index_id: CurveId,
    pub margin_bp: f64,
    pub gearing: f64,
    pub coupon_type: CouponType,
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<String>,
    pub stub: StubKind,
    pub reset_lag_days: i32,
}

/// Fee specification.
#[derive(Debug, Clone)]
pub enum FeeSpec {
    Fixed {
        date: Date,
        amount: Money,
    },
    PeriodicBps {
        base: FeeBase,
        bps: f64,
        freq: Frequency,
        dc: DayCount,
        bdc: BusinessDayConvention,
        calendar_id: Option<&'static str>,
        stub: StubKind,
    },
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone)]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit − outstanding, 0).
    Undrawn { facility_limit: Money },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ScheduleParams {
    pub freq: Frequency,
    pub dc: DayCount,
    pub bdc: BusinessDayConvention,
    pub calendar_id: Option<String>,
    pub stub: StubKind,
}

impl ScheduleParams {
    /// Quarterly payments with Act/360 day count and Following BDC
    pub fn quarterly_act360() -> Self {
        Self {
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        }
    }

    /// Semi-annual payments with 30/360 day count and Modified Following BDC
    pub fn semiannual_30360() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
        }
    }

    /// Annual payments with Act/Act day count and Following BDC
    pub fn annual_actact() -> Self {
        Self {
            freq: Frequency::annual(),
            dc: DayCount::ActAct,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        }
    }

    /// USD market standard (quarterly, Act/360, Modified Following, USD calendar)
    pub fn usd_standard() -> Self {
        Self {
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("USD".to_string()),
            stub: StubKind::None,
        }
    }

    /// EUR market standard (semi-annual, 30/360, Modified Following, EUR calendar)
    pub fn eur_standard() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("EUR".to_string()),
            stub: StubKind::None,
        }
    }

    /// GBP market standard (semi-annual, Act/365, Modified Following, GBP calendar)
    pub fn gbp_standard() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("GBP".to_string()),
            stub: StubKind::None,
        }
    }

    /// JPY market standard (semi-annual, Act/365, Modified Following, JPY calendar)
    pub fn jpy_standard() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("JPY".to_string()),
            stub: StubKind::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FloatCouponParams {
    pub index_id: CurveId,
    pub margin_bp: f64,
    pub gearing: f64,
    pub reset_lag_days: i32,
}

#[derive(Debug, Clone)]
pub struct FixedWindow {
    pub rate: f64,
    pub schedule: ScheduleParams,
}

#[derive(Debug, Clone)]
pub struct FloatWindow {
    pub params: FloatCouponParams,
    pub schedule: ScheduleParams,
}
