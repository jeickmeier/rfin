//! Specification types for the cashflow builder.
//!
//! This module contains type definitions for coupon, fee, and scheduling specifications
//! that configure the `CashflowBuilder`. These are the primary input types that users
//! interact with when building cashflow schedules.
//!
//! ## Responsibilities
//!
//! - Type definitions for fixed and floating coupon specifications
//! - Fee specification types (fixed and periodic)
//! - Schedule parameter types (frequency, day count, business day conventions)
//! - Coupon type enums (Cash, PIK, Split)
//! - Helper constructors for common market conventions (USD, EUR, GBP, etc.)

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CouponType {
    /// Cash variant.
    Cash,
    /// PIK variant.
    PIK,
    /// Split variant.
    Split {
        /// Cash pct.
        cash_pct: f64,
        /// Pik pct.
        pik_pct: f64,
    },
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FixedCouponSpec {
    /// coupon type.
    pub coupon_type: CouponType,
    /// rate.
    pub rate: f64,
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

/// Default gearing for floating rates.
fn default_gearing() -> f64 {
    1.0
}

/// Default reset lag for floating rates (T-2 standard).
fn default_reset_lag() -> i32 {
    2
}

/// Canonical floating rate specification for all instruments.
///
/// Used by bonds, swaps, credit facilities, and structured products.
/// All instruments should compose this type rather than defining their own
/// floating rate specifications.
///
/// # Rate Calculation
///
/// The all-in rate is computed as:
/// 1. Look up forward rate from `index_id` curve for the accrual period
/// 2. Apply `floor_bp` to index rate (if specified) - applied BEFORE adding spread
/// 3. Add `spread_bp` to get base rate
/// 4. Multiply by `gearing` (typically 1.0)
/// 5. Apply `cap_bp` to final rate (if specified) - applied AFTER spread and gearing
///
/// Formula: `cap(gearing * (floor(index) + spread))`
///
/// # Example
///
/// ```rust
/// use finstack_core::dates::{DayCount, Frequency, BusinessDayConvention};
/// use finstack_valuations::cashflow::builder::FloatingRateSpec;
///
/// // 3M SOFR + 200bps with 0% floor
/// let spec = FloatingRateSpec {
///     index_id: "USD-SOFR-3M".into(),
///     spread_bp: 200.0,
///     gearing: 1.0,
///     floor_bp: Some(0.0),
///     cap_bp: None,
///     reset_freq: Frequency::quarterly(),
///     reset_lag_days: 2,
///     dc: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     calendar_id: None,
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloatingRateSpec {
    /// Forward curve identifier (e.g., "USD-SOFR-3M", "EUR-EURIBOR-6M").
    pub index_id: CurveId,

    /// Spread/margin over index in basis points.
    pub spread_bp: f64,

    /// Gearing/leverage multiplier applied to the all-in rate (default: 1.0).
    ///
    /// Example: gearing = 2.0 means the rate is doubled.
    #[cfg_attr(feature = "serde", serde(default = "default_gearing"))]
    pub gearing: f64,

    /// Floor on index rate in basis points (applied before adding spread).
    ///
    /// Example: floor_bp = Some(0.0) ensures index rate >= 0%.
    #[cfg_attr(feature = "serde", serde(default))]
    pub floor_bp: Option<f64>,

    /// Cap on all-in rate in basis points (applied after spread and gearing).
    ///
    /// Example: cap_bp = Some(1000.0) ensures all-in rate <= 10%.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cap_bp: Option<f64>,

    /// Reset frequency for rate fixings.
    pub reset_freq: Frequency,

    /// Reset lag in business days (e.g., 2 for T-2 SOFR convention).
    #[cfg_attr(feature = "serde", serde(default = "default_reset_lag"))]
    pub reset_lag_days: i32,

    /// Day count convention for accrual calculations.
    pub dc: DayCount,

    /// Business day convention for date adjustments.
    pub bdc: BusinessDayConvention,

    /// Optional calendar for business day adjustments.
    #[cfg_attr(feature = "serde", serde(default))]
    pub calendar_id: Option<String>,
}

/// Floating coupon specification (composes FloatingRateSpec).
///
/// Used by the cashflow builder for instruments with floating rate coupons.
/// Embeds the canonical `FloatingRateSpec` for rate projection and adds
/// coupon-specific settings like payment frequency and PIK behavior.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloatingCouponSpec {
    /// Floating rate specification (contains index, spread, floor, cap, etc).
    pub rate_spec: FloatingRateSpec,

    /// Coupon type (Cash/PIK/Split).
    pub coupon_type: CouponType,

    /// Payment frequency (may differ from reset frequency in rate_spec).
    pub freq: Frequency,

    /// Stub rule for payment schedule generation.
    pub stub: StubKind,
}

/// Fee specification.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FeeSpec {
    /// Fixed variant.
    Fixed {
        /// Date.
        date: Date,
        /// Amount.
        amount: Money,
    },
    /// Periodic Bps variant.
    PeriodicBps {
        /// Base.
        base: FeeBase,
        /// Bps.
        bps: f64,
        /// Freq.
        freq: Frequency,
        /// Dc.
        dc: DayCount,
        /// Bdc.
        bdc: BusinessDayConvention,
        /// Calendar id.
        calendar_id: Option<String>,
        /// Stub.
        stub: StubKind,
    },
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit − outstanding, 0).
    Undrawn {
        /// Facility limit.
        facility_limit: Money,
    },
}

/// Fee tier for utilization-based fee structures.
///
/// Tiers are evaluated in order: the first tier where utilization >= threshold applies.
/// Tiers should be sorted by threshold (ascending).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeTier {
    /// Utilization threshold (0.0 to 1.0). Fee applies when utilization >= this threshold.
    pub threshold: f64,
    /// Fee rate in basis points for this tier.
    pub bps: f64,
}

/// Evaluate fee tiers to find the applicable rate for a given utilization.
///
/// Returns the fee rate from the highest tier where utilization >= threshold.
/// If no tiers match or tiers are empty, returns 0.0.
///
/// # Arguments
///
/// * `tiers` - Slice of fee tiers, should be sorted by threshold ascending
/// * `utilization` - Current utilization rate (0.0 to 1.0)
///
/// # Returns
///
/// The fee rate in basis points for the applicable tier, or 0.0 if no tier matches
pub fn evaluate_fee_tiers(tiers: &[FeeTier], utilization: f64) -> f64 {
    tiers
        .iter()
        .rev()
        .find(|tier| utilization >= tier.threshold)
        .map(|tier| tier.bps)
        .unwrap_or(0.0)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// Schedule Params structure.
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
/// Float Coupon Params structure.
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

#[derive(Debug, Clone)]
/// Fixed Window structure.
pub struct FixedWindow {
    /// rate.
    pub rate: f64,
    /// schedule.
    pub schedule: ScheduleParams,
}

#[derive(Debug, Clone)]
/// Float Window structure.
pub struct FloatWindow {
    /// params.
    pub params: FloatCouponParams,
    /// schedule.
    pub schedule: ScheduleParams,
}
