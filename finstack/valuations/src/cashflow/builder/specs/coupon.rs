//! Coupon specification types for fixed and floating rate coupons.

use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_core::error::InputError;
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
