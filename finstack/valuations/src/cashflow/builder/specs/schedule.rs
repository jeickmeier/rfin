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

const WK: &str = crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID;

impl ScheduleParams {
    fn preset(freq: Tenor, dc: DayCount, bdc: BusinessDayConvention, calendar_id: &str) -> Self {
        Self {
            freq,
            dc,
            bdc,
            calendar_id: calendar_id.to_string(),
            stub: StubKind::ShortFront,
            end_of_month: false,
            payment_lag_days: 0,
        }
    }

    fn preset_with_lag(
        freq: Tenor,
        dc: DayCount,
        bdc: BusinessDayConvention,
        calendar_id: &str,
        payment_lag_days: i32,
    ) -> Self {
        Self {
            payment_lag_days,
            ..Self::preset(freq, dc, bdc, calendar_id)
        }
    }

    // ── Generic presets (calendar-agnostic) ──────────────────────────────────

    /// Quarterly payments with Act/360 day count and Modified Following BDC.
    pub fn quarterly_act360() -> Self {
        Self::preset(
            Tenor::quarterly(),
            DayCount::Act360,
            BusinessDayConvention::ModifiedFollowing,
            WK,
        )
    }

    /// Semi-annual payments with 30/360 day count and Modified Following BDC.
    pub fn semiannual_30360() -> Self {
        Self::preset(
            Tenor::semi_annual(),
            DayCount::Thirty360,
            BusinessDayConvention::ModifiedFollowing,
            WK,
        )
    }

    /// Annual payments with Act/Act day count and Following BDC.
    pub fn annual_actact() -> Self {
        Self::preset(
            Tenor::annual(),
            DayCount::ActAct,
            BusinessDayConvention::Following,
            WK,
        )
    }

    // ── USD ──────────────────────────────────────────────────────────────────

    /// USD SOFR swap (quarterly, Act/360, Modified Following, USNY, T+2 payment lag).
    ///
    /// Follows ARRC SOFR conventions and ISDA 2021 definitions.
    pub fn usd_sofr_swap() -> Self {
        Self::preset_with_lag(
            Tenor::quarterly(),
            DayCount::Act360,
            BusinessDayConvention::ModifiedFollowing,
            "usny",
            2,
        )
    }

    /// USD corporate bond (semi-annual, 30/360, Following, USNY).
    pub fn usd_corporate_bond() -> Self {
        Self::preset(
            Tenor::semi_annual(),
            DayCount::Thirty360,
            BusinessDayConvention::Following,
            "usny",
        )
    }

    /// USD Treasury bond (semi-annual, Act/Act, Following, USNY).
    pub fn usd_treasury() -> Self {
        Self::preset(
            Tenor::semi_annual(),
            DayCount::ActAct,
            BusinessDayConvention::Following,
            "usny",
        )
    }

    /// USD market standard (quarterly, Act/360, Modified Following, USNY).
    ///
    /// Generic USD preset; for specific products use [`usd_sofr_swap`](Self::usd_sofr_swap),
    /// [`usd_corporate_bond`](Self::usd_corporate_bond), or [`usd_treasury`](Self::usd_treasury).
    pub fn usd_standard() -> Self {
        Self::preset(
            Tenor::quarterly(),
            DayCount::Act360,
            BusinessDayConvention::ModifiedFollowing,
            "usny",
        )
    }

    // ── EUR ──────────────────────────────────────────────────────────────────

    /// EUR ESTR swap (annual, Act/360, Modified Following, TARGET2, T+2 payment lag).
    ///
    /// Follows ECB €STR conventions.
    pub fn eur_estr_swap() -> Self {
        Self::preset_with_lag(
            Tenor::annual(),
            DayCount::Act360,
            BusinessDayConvention::ModifiedFollowing,
            "target2",
            2,
        )
    }

    /// EUR government bond (annual, Act/Act, Following, TARGET2).
    pub fn eur_gov_bond() -> Self {
        Self::preset(
            Tenor::annual(),
            DayCount::ActAct,
            BusinessDayConvention::Following,
            "target2",
        )
    }

    /// EUR market standard (semi-annual, 30/360, Modified Following, TARGET2).
    ///
    /// Generic EUR preset suitable for EUR corporate bonds. For swaps use
    /// [`eur_estr_swap`](Self::eur_estr_swap).
    pub fn eur_standard() -> Self {
        Self::preset(
            Tenor::semi_annual(),
            DayCount::Thirty360,
            BusinessDayConvention::ModifiedFollowing,
            "target2",
        )
    }

    // ── GBP ──────────────────────────────────────────────────────────────────

    /// GBP SONIA swap (annual, Act/365F, Modified Following, GBLO, no payment lag).
    ///
    /// Follows BoE SONIA conventions.
    pub fn gbp_sonia_swap() -> Self {
        Self::preset(
            Tenor::annual(),
            DayCount::Act365F,
            BusinessDayConvention::ModifiedFollowing,
            "gblo",
        )
    }

    /// GBP market standard (semi-annual, Act/365F, Modified Following, GBLO).
    ///
    /// Suitable for UK Gilts and GBP corporate bonds.
    pub fn gbp_standard() -> Self {
        Self::preset(
            Tenor::semi_annual(),
            DayCount::Act365F,
            BusinessDayConvention::ModifiedFollowing,
            "gblo",
        )
    }

    // ── JPY ──────────────────────────────────────────────────────────────────

    /// JPY TONA swap (annual, Act/365F, Modified Following, JPTO, T+2 payment lag).
    ///
    /// Follows BoJ TONA conventions.
    pub fn jpy_tona_swap() -> Self {
        Self::preset_with_lag(
            Tenor::annual(),
            DayCount::Act365F,
            BusinessDayConvention::ModifiedFollowing,
            "jpto",
            2,
        )
    }

    /// JPY market standard (semi-annual, Act/365F, Modified Following, JPTO).
    pub fn jpy_standard() -> Self {
        Self::preset(
            Tenor::semi_annual(),
            DayCount::Act365F,
            BusinessDayConvention::ModifiedFollowing,
            "jpto",
        )
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
    /// Policy when forward curve lookup fails during emission.
    pub fallback: super::FloatingRateFallback,
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
