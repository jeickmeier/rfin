//! Schedule parameter types for cashflow generation.

use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::types::CurveId;
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

/// Floating-rate window parameters used by segmented coupon programs.
///
/// These fields describe the floating index, margin, and optional floor/cap
/// policy for a window. Schedule-generation settings live on the surrounding
/// [`ScheduleParams`].
///
/// Most callers should start from a [`FloatingRateSpec`](super::FloatingRateSpec)
/// and convert via `FloatCouponParams::from(&spec)`, which copies the rate-only
/// fields shared between the two types. This keeps `FloatingRateSpec` the
/// canonical serde-level representation while `FloatCouponParams` is just a
/// builder convenience for the in-memory segmented-program API.
#[derive(Debug, Clone)]
pub struct FloatCouponParams {
    /// Forward-curve identifier for the projected floating index, such as
    /// `"USD-SOFR-3M"`.
    pub index_id: CurveId,
    /// Margin over the index in basis points, stored as `Decimal` for exact
    /// quote preservation.
    pub margin_bp: Decimal,
    /// Gearing or leverage multiplier applied to the floating rate formula.
    pub gearing: Decimal,
    /// Reset lag in business days from the accrual start to the fixing date.
    pub reset_lag_days: i32,
    /// Whether gearing is applied to `(index + spread)` or only to the index
    /// leg before the spread is added back.
    pub gearing_includes_spread: bool,
    /// Optional floor on the index component in basis points.
    pub floor_bp: Option<Decimal>,
    /// Optional cap on the all-in coupon rate in basis points.
    pub cap_bp: Option<Decimal>,
    /// Optional floor on the all-in coupon rate in basis points.
    pub all_in_floor_bp: Option<Decimal>,
    /// Optional cap on the index component in basis points.
    pub index_cap_bp: Option<Decimal>,
    /// Optional fixing calendar distinct from the payment/accrual calendar.
    ///
    /// When `None`, the schedule calendar is also used for fixing adjustment.
    pub fixing_calendar_id: Option<String>,
    /// Overnight compounding convention for overnight indices such as SOFR,
    /// ESTR, or SONIA.
    pub overnight_compounding: Option<super::OvernightCompoundingMethod>,
    /// Day-count basis for the overnight compounding denominator.
    ///
    /// Defaults to `None` (= Act/360). Set to `Some(DayCount::Act365F)`
    /// for SONIA.
    pub overnight_basis: Option<finstack_core::dates::DayCount>,
    /// Policy applied when the forward curve cannot be resolved or projected.
    pub fallback: super::FloatingRateFallback,
}

impl From<&super::FloatingRateSpec> for FloatCouponParams {
    /// Copy rate-level fields from the canonical `FloatingRateSpec` into the
    /// builder-facing `FloatCouponParams`. Schedule-gen fields on the spec
    /// (`reset_freq`, `dc`, `bdc`, `calendar_id`, `end_of_month`,
    /// `payment_lag_days`) are intentionally not copied — they live on the
    /// sibling [`ScheduleParams`].
    fn from(spec: &super::FloatingRateSpec) -> Self {
        Self {
            index_id: spec.index_id.clone(),
            margin_bp: spec.spread_bp,
            gearing: spec.gearing,
            reset_lag_days: spec.reset_lag_days,
            gearing_includes_spread: spec.gearing_includes_spread,
            floor_bp: spec.floor_bp,
            cap_bp: spec.cap_bp,
            all_in_floor_bp: spec.all_in_floor_bp,
            index_cap_bp: spec.index_cap_bp,
            fixing_calendar_id: spec.fixing_calendar_id.clone(),
            overnight_compounding: spec.overnight_compounding,
            overnight_basis: spec.overnight_basis,
            fallback: spec.fallback.clone(),
        }
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

/// Floating-rate coupon window with a shared schedule.
#[derive(Debug, Clone)]
pub struct FloatWindow {
    /// Floating-index, spread, and floor/cap parameters for this window.
    pub params: FloatCouponParams,
    /// Schedule-generation parameters for this floating-rate window.
    pub schedule: ScheduleParams,
}
