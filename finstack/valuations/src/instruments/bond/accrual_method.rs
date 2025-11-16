//! Accrual method specification for bond interest calculations.
//!
//! Defines how accrued interest is calculated between coupon dates.
//! Different bond types and markets use different accrual conventions.

use finstack_core::dates::Frequency;
use finstack_core::types::CurveId;

/// Method for calculating accrued interest between coupon dates.
///
/// Specifies how to calculate the portion of the next coupon payment
/// that has accrued up to the valuation date. Different methods are
/// required for different bond types and market conventions.
///
/// # Market Standards
///
/// - **Linear (Default)**: Most common for simple interest bonds
///   - US Treasuries
///   - Most corporate bonds
///   - Formula: `Accrued = Coupon × (days_elapsed / days_in_period)`
///
/// - **Compounded**: Required for bonds with compounding coupons
///   - Some European government bonds
///   - Formula per ICMA Rule 251: `Accrued = Principal × [(1 + coupon_rate)^(elapsed/period) - 1]`
///
/// - **Indexed**: Required for inflation-linked bonds
///   - US TIPS
///   - UK Index-Linked Gilts
///   - Formula: Uses CPI index ratio interpolation
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::instruments::bond::AccrualMethod;
/// use finstack_core::dates::Frequency;
///
/// // Standard bond with linear accrual (default)
/// let linear = AccrualMethod::Linear;
///
/// // Bond with semi-annual compounding
/// let compounded = AccrualMethod::Compounded {
///     frequency: Frequency::semi_annual(),
/// };
///
/// // Inflation-linked bond (TIPS)
/// let indexed = AccrualMethod::Indexed {
///     index_curve_id: "US-CPI".into(),
/// };
/// ```
///
/// # References
///
/// - **ICMA Rule 251**: Accrued Interest Calculations for bonds with compounding
/// - **US Treasury**: TIPS pricing methodology for indexed bonds
/// - **Bloomberg BVAL**: Market-standard accrual conventions
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum AccrualMethod {
    /// Linear accrual (simple interest interpolation).
    ///
    /// Most common method for standard bonds. Accrued interest grows
    /// linearly between coupon dates.
    ///
    /// Formula:
    /// ```text
    /// Accrued = Coupon × (days_elapsed / days_in_period)
    /// ```
    ///
    /// Use for:
    /// - US Treasuries (non-inflation-linked)
    /// - Most corporate bonds
    /// - Municipal bonds
    Linear,

    /// Compounded accrual (actuarial method per ICMA Rule 251).
    ///
    /// Required for bonds where coupons compound within the period.
    /// Uses the actuarial method to calculate the accrued portion.
    ///
    /// Formula:
    /// ```text
    /// Accrued = Principal × [(1 + coupon_rate)^(elapsed/period) - 1]
    /// ```
    ///
    /// Use for:
    /// - Bonds with compounding coupons
    /// - Some European government bonds
    /// - Bonds explicitly specifying actuarial accrual
    ///
    /// # Fields
    ///
    /// - `frequency`: Compounding frequency (must match coupon frequency)
    Compounded {
        /// Compounding frequency (should match coupon frequency).
        frequency: Frequency,
    },

    /// Indexed accrual for inflation-linked bonds.
    ///
    /// Uses index ratio interpolation to calculate accrued interest.
    /// The index ratio adjusts both principal and coupon payments
    /// for inflation.
    ///
    /// Formula:
    /// ```text
    /// Index Ratio = Linear_Interp(Index_Start, Index_End, days_elapsed)
    /// Accrued = Coupon × Index_Ratio × (days_elapsed / days_in_period)
    /// ```
    ///
    /// Use for:
    /// - US TIPS (Treasury Inflation-Protected Securities)
    /// - UK Index-Linked Gilts
    /// - Other inflation-indexed bonds
    ///
    /// # Fields
    ///
    /// - `index_curve_id`: Reference to inflation index curve (e.g., CPI-U)
    Indexed {
        /// Inflation index curve identifier (e.g., "US-CPI", "UK-RPI").
        index_curve_id: CurveId,
    },
}

impl Default for AccrualMethod {
    /// Default to linear accrual (most common convention).
    fn default() -> Self {
        Self::Linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_linear() {
        assert_eq!(AccrualMethod::default(), AccrualMethod::Linear);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_serde_roundtrip() {
        let methods = vec![
            AccrualMethod::Linear,
            AccrualMethod::Compounded {
                frequency: Frequency::semi_annual(),
            },
            AccrualMethod::Indexed {
                index_curve_id: "US-CPI".into(),
            },
        ];

        for method in methods {
            let json = serde_json::to_string(&method).expect("Serialization should succeed in test");
            let deserialized: AccrualMethod = serde_json::from_str(&json)
                .expect("Deserialization should succeed in test");
            assert_eq!(method, deserialized);
        }
    }
}

