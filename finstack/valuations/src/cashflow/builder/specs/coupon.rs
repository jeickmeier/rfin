//! Coupon specification types for fixed and floating rate coupons.

use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_core::types::CurveId;
use finstack_core::InputError;
use rust_decimal::Decimal;

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
        cash_pct: Decimal,
        /// Pik pct.
        pik_pct: Decimal,
    },
}

impl CouponType {
    /// Returns (cash_fraction, pik_fraction) as Decimal values.
    pub(crate) fn split_parts(self) -> finstack_core::Result<(Decimal, Decimal)> {
        match self {
            CouponType::Cash => Ok((Decimal::ONE, Decimal::ZERO)),
            CouponType::PIK => Ok((Decimal::ZERO, Decimal::ONE)),
            CouponType::Split { cash_pct, pik_pct } => {
                // Validate within [0,1]
                if cash_pct < Decimal::ZERO
                    || cash_pct > Decimal::ONE
                    || pik_pct < Decimal::ZERO
                    || pik_pct > Decimal::ONE
                {
                    return Err(InputError::Invalid.into());
                }
                // Sum must be ~ 1.0; normalize within tolerance
                let sum = cash_pct + pik_pct;
                let tol = Decimal::new(1, 6); // 1e-6
                let diff = if sum >= Decimal::ONE {
                    sum - Decimal::ONE
                } else {
                    Decimal::ONE - sum
                };
                if diff <= tol {
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
    /// Coupon rate as a decimal (e.g., 0.05 for 5%). Uses Decimal for exact representation.
    pub rate: Decimal,
    /// freq.
    pub freq: Tenor,
    /// dc.
    pub dc: DayCount,
    /// bdc.
    pub bdc: BusinessDayConvention,
    /// Calendar id (use "weekends_only" for weekends-only adjustments).
    pub calendar_id: String,
    /// stub.
    pub stub: StubKind,
    /// End-of-month rolling.
    pub end_of_month: bool,
    /// Payment lag in business days after accrual end.
    pub payment_lag_days: i32,
}

/// Compounding method for overnight rate indices (SOFR, ESTR, SONIA).
///
/// Controls how daily overnight fixings are aggregated into a period rate
/// for floating rate coupons. The choice of compounding method affects both
/// the accrued amount and the payment timing/certainty.
///
/// # Market Conventions
///
/// | Index | Standard Method | Lookback | Reference |
/// |-------|----------------|----------|-----------|
/// | USD SOFR | CompoundedInArrears | 2 BD | ISDA 2021 |
/// | EUR €STR | CompoundedWithObservationShift | 2 BD | ECB |
/// | GBP SONIA | CompoundedWithObservationShift | 5 BD | BoE |
/// | JPY TONA | CompoundedInArrears | 2 BD | BoJ |
///
/// # Reference
///
/// - ISDA (2021). "IBOR Fallbacks Supplement." Section 7.
/// - ARRC (2020). "SOFR: A User's Guide." Federal Reserve Bank of New York.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OvernightCompoundingMethod {
    /// Simple average of daily rates (non-standard, for reference only).
    SimpleAverage,

    /// Compounded in arrears with daily compounding (ISDA 2021 standard).
    ///
    /// ```text
    /// Rate = [∏(1 + r_i × d_i/360) - 1] × 360/D
    /// ```
    #[default]
    CompoundedInArrears,

    /// Compounded in arrears with lookback (shift observation period).
    ///
    /// Uses rates from `lookback_days` business days before each accrual date.
    CompoundedWithLookback {
        /// Number of business days to look back for rate observations.
        lookback_days: u32,
    },

    /// Compounded in arrears with lockout (freeze rate near end of period).
    ///
    /// Uses the rate from `lockout_days` business days before period end for all
    /// remaining days in the period.
    CompoundedWithLockout {
        /// Number of business days before period end to freeze the rate.
        lockout_days: u32,
    },

    /// Compounded in arrears with observation shift.
    ///
    /// Both observation dates AND weights are shifted back by `shift_days`
    /// business days. This is the ISDA 2021 recommended convention for SOFR
    /// and the standard for GBP SONIA and EUR €STR.
    CompoundedWithObservationShift {
        /// Number of business days to shift observations.
        shift_days: u32,
    },
}

/// Default gearing for floating rates.
fn default_gearing() -> Decimal {
    Decimal::ONE
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
/// # Negative Rate Handling
///
/// Negative index rates are supported and will flow through calculations
/// unless constrained by floors. For markets with negative rates (EUR, JPY, CHF):
///
/// - Set `floor_bp: Some(0.0)` to floor the index at zero
/// - Set `all_in_floor_bp: Some(0.0)` to floor the total coupon at zero
/// - Omit floors to allow negative coupons (rare but valid in some structures)
///
/// The implementation does not reject negative rates; the policy is controlled
/// by the floor configuration.
///
/// # Example
///
/// ```rust
/// use finstack_core::dates::{DayCount, Tenor, BusinessDayConvention};
/// use finstack_valuations::cashflow::builder::FloatingRateSpec;
/// use rust_decimal_macros::dec;
///
/// // 3M SOFR + 200bps with 0% floor
/// let spec = FloatingRateSpec {
///     index_id: "USD-SOFR-3M".into(),
///     spread_bp: dec!(200.0),
///     gearing: dec!(1.0),
///     gearing_includes_spread: true,
///     floor_bp: Some(dec!(0.0)),
///     all_in_floor_bp: None,
///     cap_bp: None,
///     index_cap_bp: None,
///     reset_freq: Tenor::quarterly(),
///     reset_lag_days: 2,
///     dc: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     calendar_id: "weekends_only".to_string(),
///     fixing_calendar_id: None,
///     end_of_month: false,
///     payment_lag_days: 0,
/// };
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloatingRateSpec {
    /// Forward curve identifier (e.g., "USD-SOFR-3M", "EUR-EURIBOR-6M").
    pub index_id: CurveId,

    /// Spread/margin over index in basis points. Uses Decimal for exact representation.
    pub spread_bp: Decimal,

    /// Gearing/leverage multiplier applied to the all-in rate (default: 1.0).
    ///
    /// Example: gearing = 2.0 means the rate is doubled.
    #[cfg_attr(feature = "serde", serde(default = "default_gearing"))]
    pub gearing: Decimal,

    /// Whether gearing includes the spread (default: true).
    ///
    /// - `true`: `rate = (index + spread) * gearing`
    /// - `false`: `rate = (index * gearing) + spread` (Affine model)
    #[cfg_attr(feature = "serde", serde(default = "default_gearing_includes_spread"))]
    pub gearing_includes_spread: bool,

    /// Floor on index rate in basis points (applied to index component).
    ///
    /// Example: floor_bp = Some(0.0) ensures index rate >= 0%.
    #[cfg_attr(feature = "serde", serde(default))]
    pub floor_bp: Option<Decimal>,

    /// Floor on all-in rate in basis points (Min Coupon).
    ///
    /// Applied to the final calculated rate after gearing and spread.
    #[cfg_attr(feature = "serde", serde(default))]
    pub all_in_floor_bp: Option<Decimal>,

    /// Cap on all-in rate in basis points (applied after spread and gearing).
    ///
    /// Example: cap_bp = Some(1000.0) ensures all-in rate <= 10%.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cap_bp: Option<Decimal>,

    /// Cap on index rate in basis points (applied to index component).
    #[cfg_attr(feature = "serde", serde(default))]
    pub index_cap_bp: Option<Decimal>,

    /// Reset frequency for rate fixings.
    pub reset_freq: Tenor,

    /// Reset lag in business days (e.g., 2 for T-2 SOFR convention).
    #[cfg_attr(feature = "serde", serde(default = "default_reset_lag"))]
    pub reset_lag_days: i32,

    /// Day count convention for accrual calculations.
    pub dc: DayCount,

    /// Business day convention for date adjustments.
    pub bdc: BusinessDayConvention,

    /// Calendar for business day adjustments (accrual/payment).
    pub calendar_id: String,

    /// Optional calendar for rate fixing (reset lag).
    ///
    /// If not provided, defaults to `calendar_id`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fixing_calendar_id: Option<String>,
    /// End-of-month rolling.
    pub end_of_month: bool,
    /// Payment lag in business days after accrual end.
    pub payment_lag_days: i32,

    /// Overnight compounding method for overnight rate indices (SOFR, ESTR, SONIA).
    ///
    /// When set to `Some(method)`, the rate for each accrual period is computed
    /// by compounding daily overnight fixings according to the specified method,
    /// rather than looking up a single forward rate for the period.
    ///
    /// Leave as `None` for term rates (e.g., 3M EURIBOR, 6M LIBOR).
    #[cfg_attr(feature = "serde", serde(default))]
    pub overnight_compounding: Option<OvernightCompoundingMethod>,
}

fn default_gearing_includes_spread() -> bool {
    true
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
    pub freq: Tenor,

    /// Stub rule for payment schedule generation.
    pub stub: StubKind,
}
