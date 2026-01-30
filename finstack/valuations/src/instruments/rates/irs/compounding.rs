//! Compounding conventions for floating leg calculations in interest rate swaps.
//!
//! Defines how floating rate coupons are calculated based on the
//! underlying reference rate (LIBOR, SOFR, SONIA, etc.).
//!
//! # Implementation Notes
//!
//! ## Compounded-in-Arrears (Full Daily Compounding)
//!
//! For overnight-indexed swaps (OIS) with `CompoundedInArrears` compounding,
//! the implementation uses **full daily compounding** per ISDA 2021:
//!
//! ```text
//! Coupon = N × [∏(1 + r_i × dcf_i) - 1] + spread × accrual
//! ```
//!
//! where the product is taken over daily observations in the accrual period.
//!
//! ## Fast Path for Unseasoned Single-Curve OIS
//!
//! When all of the following conditions are met, the discount curve identity
//! is used as an optimization:
//!
//! - Swap is unseasoned (`as_of <= accrual_start`)
//! - No lookback or observation shift (`lookback_days = 0`, `observation_shift = None`)
//! - Forward curve ID matches discount curve ID (single-curve)
//!
//! In this case:
//! ```text
//! ∏(1 + r_i × dcf_i) = DF(start) / DF(end)
//! ```
//!
//! This identity is exact and avoids iterating over daily observations.
//!
//! ## Lookback and Observation Shift
//!
//! The `lookback_days` and `observation_shift` parameters are fully supported:
//!
//! - **Lookback**: Shifts observation dates back from the accrual period. For example,
//!   with `lookback_days = 2`, observations for the period Jan 1-Apr 1 would be
//!   taken from Dec 28-Mar 28 (2 business days earlier).
//!
//! - **Observation Shift**: Additional adjustment to observation dates. The total
//!   shift is computed as `-lookback_days + observation_shift`.
//!
//! When lookback/shift is non-zero, the fast path is disabled and full daily
//! compounding is performed with shifted observation dates.
//!
//! ## Seasoned Swaps
//!
//! For seasoned swaps where `as_of` falls within an accrual period, historical
//! fixings are required for observation dates before `as_of`. Provide fixings
//! via `ScalarTimeSeries` with id `FIXING:{forward_curve_id}`.
//!
//! # References
//!
//! - **ISDA 2021 Definitions**: Compounded RFR conventions
//! - **ARRC** (Alternative Reference Rates Committee): SOFR conventions
//! - **BoE** (Bank of England): SONIA conventions

/// Method for calculating floating leg coupon payments.
///
/// Different reference rates require different compounding conventions:
/// - **Term rates (SOFR 3M, EURIBOR, historical LIBOR)**: Simple interest
/// - **Overnight rates (SOFR, SONIA, €STR, TONA)**: Compounded in arrears
///
/// # Market Standards
///
/// ## Simple (LIBOR-style)
/// - **Formula**: `Coupon = Notional × (Forward_Rate + Spread) × DCF`
/// - **Use for**: USD LIBOR, EUR EURIBOR, GBP LIBOR (historical)
/// - **Standard**: ISDA 2006 Definitions
///
/// ## Compounded In Arrears (RFR-style)
/// - **Formula**: `Coupon = Notional × [∏(1 + r_i × dcf_i) - 1]`
/// - **Use for**: USD SOFR, GBP SONIA, EUR €STR, JPY TONA
/// - **Standard**: ISDA 2021 Definitions
/// - **Lookback**: Typically 2-5 business days before period end
/// - **Observation shift**: Optional shift for operational convenience
///
/// # Examples
///
/// ```
/// use finstack_valuations::instruments::rates::irs::FloatingLegCompounding;
///
/// // LIBOR-style swap (simple compounding)
/// let simple = FloatingLegCompounding::Simple;
/// assert_eq!(simple, FloatingLegCompounding::default());
///
/// // SOFR swap with 2-day lookback
/// let sofr = FloatingLegCompounding::CompoundedInArrears {
///     lookback_days: 2,
///     observation_shift: None,
/// };
/// assert_eq!(sofr, FloatingLegCompounding::sofr());
///
/// // SONIA swap with 5-day lookback
/// let sonia = FloatingLegCompounding::sonia();
/// if let FloatingLegCompounding::CompoundedInArrears { lookback_days, .. } = sonia {
///     assert_eq!(lookback_days, 5);
/// }
/// ```
///
/// # References
///
/// - **ISDA 2021 Definitions**: Compounded RFR conventions
/// - **ARRC** (Alternative Reference Rates Committee): SOFR conventions
/// - **BoE** (Bank of England): SONIA conventions
/// - **ECB**: €STR conventions
///
/// In the IRS instrument implementation, the RFR-style variant
/// (`CompoundedInArrears`) is also used to classify swaps as OIS for
/// discount-only float-leg pricing; see `InterestRateSwap::is_single_curve_ois` for details.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum FloatingLegCompounding {
    /// Simple interest compounding (term-rate style).
    ///
    /// Coupon = Notional × (Forward_Rate + Spread) × Day_Count_Fraction
    ///
    /// Use for:
    /// - Term SOFR / EURIBOR-style swaps with fixed-tenor indices
    /// - Legacy USD/EUR/GBP LIBOR swaps (for back-testing only)
    ///
    /// This is the current default behavior for vanilla IRS and matches
    /// ISDA 2006 term-rate conventions.
    Simple,

    /// Compounded in arrears (overnight RFR rates).
    ///
    /// Coupon = Notional × [∏(1 + r_i × dcf_i) - 1] where the product
    /// is taken over daily observations in the accrual period.
    ///
    /// Use for:
    /// - USD SOFR (Secured Overnight Financing Rate)
    /// - GBP SONIA (Sterling Overnight Index Average)
    /// - EUR €STR (Euro Short-Term Rate)
    /// - JPY TONA (Tokyo Overnight Average Rate)
    ///
    /// # Fields
    ///
    /// - `lookback_days`: Days to shift observation end date before period end (typically 2-5)
    /// - `observation_shift`: Optional additional shift for operational convenience
    ///
    /// # Market Conventions
    ///
    /// - **SOFR**: 2-day lookback (ARRC recommended)
    /// - **SONIA**: 5-day lookback (BoE recommended)
    /// - **€STR**: 2-day shift (ECB convention)
    /// - **TONA**: 2-day lag (JSCC convention)
    CompoundedInArrears {
        /// Number of business days to shift observation end (lookback).
        /// Typically 2-5 days depending on market convention.
        lookback_days: i32,

        /// Optional observation shift (in business days).
        /// Some markets use observation shift instead of lookback.
        observation_shift: Option<i32>,
    },
}

impl Default for FloatingLegCompounding {
    /// Default to simple compounding (LIBOR-style, most conservative).
    fn default() -> Self {
        Self::Simple
    }
}

/// Market-standard compounding presets for common RFR swaps.
impl FloatingLegCompounding {
    /// USD SOFR standard convention (2-day lookback per ARRC).
    pub fn sofr() -> Self {
        Self::CompoundedInArrears {
            lookback_days: 2,
            observation_shift: None,
        }
    }

    /// USD Fed Funds / EFFR-style overnight convention (no lookback).
    ///
    /// Bloomberg `FEDL01 Index` OIS conventions typically do **not** apply the SOFR-style
    /// observation lookback. We model that as `lookback_days = 0`.
    pub fn fedfunds() -> Self {
        Self::CompoundedInArrears {
            lookback_days: 0,
            observation_shift: None,
        }
    }

    /// GBP SONIA standard convention (5-day lookback per BoE).
    pub fn sonia() -> Self {
        Self::CompoundedInArrears {
            lookback_days: 5,
            observation_shift: None,
        }
    }

    /// EUR €STR standard convention (2-day shift per ECB).
    pub fn estr() -> Self {
        Self::CompoundedInArrears {
            lookback_days: 2,
            observation_shift: None,
        }
    }

    /// JPY TONA standard convention (2-day lag per JSCC).
    pub fn tona() -> Self {
        Self::CompoundedInArrears {
            lookback_days: 2,
            observation_shift: None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_simple() {
        assert_eq!(
            FloatingLegCompounding::default(),
            FloatingLegCompounding::Simple
        );
    }

    #[test]
    fn test_market_presets() {
        // Verify standard market conventions
        assert_eq!(
            FloatingLegCompounding::sofr(),
            FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 2,
                observation_shift: None,
            }
        );

        assert_eq!(
            FloatingLegCompounding::sonia(),
            FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 5,
                observation_shift: None,
            }
        );
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_serde_roundtrip() {
        let methods = vec![
            FloatingLegCompounding::Simple,
            FloatingLegCompounding::sofr(),
            FloatingLegCompounding::sonia(),
        ];

        for method in methods {
            let json =
                serde_json::to_string(&method).expect("Serialization should succeed in test");
            let deserialized: FloatingLegCompounding =
                serde_json::from_str(&json).expect("Deserialization should succeed in test");
            assert_eq!(method, deserialized);
        }
    }
}
