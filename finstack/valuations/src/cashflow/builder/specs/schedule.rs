//! Schedule parameter types for cashflow generation.

use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::types::CurveId;
use rust_decimal::Decimal;

/// Schedule Params structure.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ScheduleParams {
    /// freq.
    pub freq: Tenor,
    /// dc.
    pub dc: DayCount,
    /// bdc.
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Calendar id (use "weekends_only" for weekends-only adjustments).
    pub calendar_id: String,
    /// stub.
    #[serde(default = "crate::serde_defaults::stub_short_front")]
    pub stub: StubKind,
    /// End-of-month rolling.
    pub end_of_month: bool,
    /// Payment lag in business days after accrual end.
    pub payment_lag_days: i32,
}

impl ScheduleParams {
    /// Quarterly payments with Act/360 day count and Following BDC
    pub fn quarterly_act360() -> Self {
        Self {
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID.to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    /// Semi-annual payments with 30/360 day count and Modified Following BDC
    pub fn semiannual_30360() -> Self {
        Self {
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID.to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    /// Annual payments with Act/Act day count and Following BDC
    pub fn annual_actact() -> Self {
        Self {
            freq: Tenor::annual(),
            dc: DayCount::ActAct,
            bdc: BusinessDayConvention::Following,
            calendar_id: crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID.to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    /// USD market standard (quarterly, Act/360, Modified Following, USD calendar)
    pub fn usd_standard() -> Self {
        Self {
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            // Use a real calendar identifier (currency codes are not calendar IDs).
            calendar_id: "usny".to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    /// EUR market standard (semi-annual, 30/360, Modified Following, EUR calendar)
    pub fn eur_standard() -> Self {
        Self {
            freq: Tenor::semi_annual(),
            dc: DayCount::Thirty360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: "target2".to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    /// GBP market standard (semi-annual, Act/365, Modified Following, GBP calendar)
    pub fn gbp_standard() -> Self {
        Self {
            freq: Tenor::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: "gblo".to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    /// JPY market standard (semi-annual, Act/365, Modified Following, JPY calendar)
    pub fn jpy_standard() -> Self {
        Self {
            freq: Tenor::semi_annual(),
            dc: DayCount::Act365F,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: "jpto".to_string(),
            stub: StubKind::None,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }
}

/// Float Coupon Params structure.
#[derive(Debug, Clone)]
pub struct FloatCouponParams {
    /// index id.
    pub index_id: CurveId,
    /// Margin over index in basis points. Uses Decimal for exact representation.
    pub margin_bp: Decimal,
    /// Gearing/leverage multiplier. Uses Decimal for exact representation.
    pub gearing: Decimal,
    /// reset lag days.
    pub reset_lag_days: i32,
    /// Whether gearing includes the spread (default: true).
    pub gearing_includes_spread: bool,
    /// Floor on index rate in basis points.
    pub floor_bp: Option<Decimal>,
    /// Cap on all-in rate in basis points.
    pub cap_bp: Option<Decimal>,
    /// Floor on all-in rate in basis points.
    pub all_in_floor_bp: Option<Decimal>,
    /// Cap on index rate in basis points.
    pub index_cap_bp: Option<Decimal>,
    /// Optional fixing calendar (distinct from payment calendar).
    pub fixing_calendar_id: Option<String>,
    /// Overnight compounding method for overnight indices (SOFR, ESTR, SONIA).
    pub overnight_compounding: Option<super::OvernightCompoundingMethod>,
}

/// Fixed Window structure.
#[derive(Debug, Clone)]
pub struct FixedWindow {
    /// Coupon rate as a decimal (e.g., 0.05 for 5%). Uses Decimal for exact representation.
    pub rate: Decimal,
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
