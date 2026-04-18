//! Schedule parameter types for cashflow generation.

use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use rust_decimal::Decimal;
/// Canonical schedule-generation parameters for coupons and periodic fees.
///
/// This type controls how accrual boundaries and payment dates are generated.
/// The fields describe schedule construction conventions, not discounting or
/// valuation conventions.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema, Debug, Clone)]
pub struct ScheduleParams {
    /// Accrual and payment frequency used to generate the schedule boundaries.
    pub freq: Tenor,
    /// Day-count convention used to convert each generated accrual period into a
    /// year fraction.
    pub dc: DayCount,
    /// Business-day convention applied when rolling accrual-end and payment
    /// dates onto valid business days.
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Holiday calendar identifier used together with `bdc`.
    ///
    /// Use `"weekends_only"` when only Saturday/Sunday adjustment is needed.
    pub calendar_id: String,
    /// Stub-handling rule used when the start/end dates do not fit an exact
    /// whole number of periods.
    #[serde(default = "crate::serde_defaults::stub_short_front")]
    pub stub: StubKind,
    /// Whether end-of-month rolling should be preserved when generating the
    /// schedule.
    pub end_of_month: bool,
    /// Payment lag in business days after the adjusted accrual end date.
    pub payment_lag_days: i32,
}

const WK: &str = crate::builder::calendar::WEEKENDS_ONLY_ID;

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
}

/// Fixed-rate coupon window with a shared schedule.
#[derive(Debug, Clone)]
pub struct FixedWindow {
    /// Annual coupon rate as a decimal, for example `0.05` for 5%.
    pub rate: Decimal,
    /// Schedule-generation parameters for this fixed-rate window.
    pub schedule: ScheduleParams,
}
