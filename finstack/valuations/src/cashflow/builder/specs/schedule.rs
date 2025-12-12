//! Schedule parameter types for cashflow generation.

use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::types::CurveId;

/// Schedule Params structure.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ScheduleParams {
    /// freq.
    pub freq: Frequency,
    /// dc.
    pub dc: DayCount,
    /// bdc.
    pub bdc: BusinessDayConvention,
    /// calendar id.
    pub calendar_id: Option<String>,
    /// stub.
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
            // Use a real calendar identifier (currency codes are not calendar IDs).
            calendar_id: Some("usny".to_string()),
            stub: StubKind::None,
        }
    }

    /// EUR market standard (semi-annual, 30/360, Modified Following, EUR calendar)
    pub fn eur_standard() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("target2".to_string()),
            stub: StubKind::None,
        }
    }

    /// GBP market standard (semi-annual, Act/365, Modified Following, GBP calendar)
    pub fn gbp_standard() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("gblo".to_string()),
            stub: StubKind::None,
        }
    }

    /// JPY market standard (semi-annual, Act/365, Modified Following, JPY calendar)
    pub fn jpy_standard() -> Self {
        Self {
            freq: Frequency::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("jpto".to_string()),
            stub: StubKind::None,
        }
    }
}

/// Float Coupon Params structure.
#[derive(Debug, Clone)]
pub struct FloatCouponParams {
    /// index id.
    pub index_id: CurveId,
    /// margin bp.
    pub margin_bp: f64,
    /// gearing.
    pub gearing: f64,
    /// reset lag days.
    pub reset_lag_days: i32,
}

/// Fixed Window structure.
#[derive(Debug, Clone)]
pub struct FixedWindow {
    /// rate.
    pub rate: f64,
    /// schedule.
    pub schedule: ScheduleParams,
}

/// Float Window structure.
#[derive(Debug, Clone)]
pub struct FloatWindow {
    /// params.
    pub params: FloatCouponParams,
    /// schedule.
    pub schedule: ScheduleParams,
}
