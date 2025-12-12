//! Internal rounding helpers backing `Money` arithmetic.
//!
//! Amounts are stored as `Decimal` values (`AmountRepr`) to provide accounting-grade
//! precision and deterministic arithmetic. The routines here provide Decimal arithmetic
//! on that representation and expose helpers that honour [`RoundingMode`](crate::config::RoundingMode).
//!
//! The rounding functions are used internally by `Money` operations and
//! are not part of the public API. For rounding examples, see `Money` documentation.

use crate::config::RoundingMode;
use crate::error::{Error, InputError};
use rust_decimal::Decimal;

/// Internal numeric representation for `Money` amounts.
/// Uses Decimal for accounting-grade precision and deterministic arithmetic.
pub(crate) type AmountRepr = Decimal;

/// Convert a Decimal representation to f64.
///
/// # Panics
///
/// Panics if the Decimal value cannot be represented as f64. This follows the
/// "fail closed" principle - silent conversion to zero is catastrophic in
/// financial systems and must be caught immediately.
///
/// Use [`try_amount_from_repr`] for fallible conversion when error handling is needed.
#[inline]
pub(crate) fn amount_from_repr(x: AmountRepr) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    x.to_f64().expect(
        "Decimal to f64 conversion failed: value outside representable range. \
         This indicates a bug or invalid data - monetary amounts should be within f64 bounds.",
    )
}

/// Fallible conversion from Decimal representation to f64.
///
/// Returns `Err(ConversionOverflow)` if the Decimal value cannot be represented as f64.
/// Use this when you need explicit error handling at API boundaries.
#[inline]
pub(crate) fn try_amount_from_repr(x: AmountRepr) -> Result<f64, Error> {
    use rust_decimal::prelude::ToPrimitive;
    x.to_f64()
        .ok_or_else(|| InputError::ConversionOverflow.into())
}

#[inline]
pub(crate) fn repr_add(a: AmountRepr, b: AmountRepr) -> AmountRepr {
    a + b
}

#[inline]
pub(crate) fn repr_sub(a: AmountRepr, b: AmountRepr) -> AmountRepr {
    a - b
}

#[inline]
pub(crate) fn repr_mul_f64(a: AmountRepr, rhs: f64) -> AmountRepr {
    assert!(
        rhs.is_finite(),
        "Money multiplication requires finite scalar (got {:?})",
        rhs
    );
    a * Decimal::from_f64_retain(rhs)
        .expect("finite scalar should convert to Decimal without loss of finiteness")
}

#[inline]
pub(crate) fn repr_div_f64(a: AmountRepr, rhs: f64) -> AmountRepr {
    assert!(
        rhs.is_finite(),
        "Money division requires finite scalar (got {:?})",
        rhs
    );
    assert!(rhs != 0.0, "Money division by zero is not allowed");
    let rhs_decimal = Decimal::from_f64_retain(rhs)
        .expect("finite non-zero scalar should convert to Decimal without loss of finiteness");
    a / rhs_decimal
}

/// Round `x` to `dp` decimal places using the supplied [`RoundingMode`].
/// Converts f64 input to Decimal for proper rounding.
#[inline]
pub(crate) fn round_f64(x: f64, dp: i32, mode: RoundingMode) -> Decimal {
    assert!(
        x.is_finite(),
        "Money rounding requires finite scalar (got {:?})",
        x
    );
    let decimal = Decimal::from_f64_retain(x)
        .expect("finite scalar should convert to Decimal without loss of finiteness");
    round_decimal(decimal, dp, mode)
}

/// Round a Decimal to `dp` decimal places using the supplied [`RoundingMode`].
#[inline]
pub(crate) fn round_decimal(x: Decimal, dp: i32, mode: RoundingMode) -> Decimal {
    use rust_decimal::RoundingStrategy;

    if dp < 0 {
        return x;
    }

    let strategy = match mode {
        RoundingMode::Bankers => RoundingStrategy::MidpointNearestEven,
        RoundingMode::AwayFromZero => RoundingStrategy::MidpointAwayFromZero,
        RoundingMode::TowardZero => RoundingStrategy::ToZero,
        RoundingMode::Floor => RoundingStrategy::ToNegativeInfinity,
        RoundingMode::Ceil => RoundingStrategy::ToPositiveInfinity,
    };

    x.round_dp_with_strategy(dp as u32, strategy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn amount_from_repr_converts_normal_values() {
        // Normal monetary amounts should convert without issues
        let cases = [
            ("0.0", 0.0),
            ("100.0", 100.0),
            ("-50.25", -50.25),
            ("1000000.00", 1_000_000.0),
            ("0.0001", 0.0001),
        ];
        for (decimal_str, expected) in cases {
            let decimal = Decimal::from_str(decimal_str).expect("Valid decimal string");
            let result = amount_from_repr(decimal);
            assert!(
                (result - expected).abs() < 1e-10,
                "Expected {} but got {}",
                expected,
                result
            );
        }
    }

    #[test]
    fn try_amount_from_repr_converts_normal_values() {
        let decimal = Decimal::from_str("12345.67").expect("Valid decimal string");
        let result = try_amount_from_repr(decimal).expect("Conversion should succeed");
        assert!((result - 12345.67).abs() < 1e-10);
    }

    #[test]
    fn amount_from_repr_handles_large_values_within_f64_range() {
        // Decimal max is ~7.9e28, which is within f64 range
        // Test with a large value that should still convert
        let large = Decimal::from_str("1000000000000000.0").expect("Valid decimal"); // 1 quadrillion
        let result = amount_from_repr(large);
        assert!(result > 0.0, "Large value should not silently become 0");
        assert!((result - 1e15).abs() < 1e5);
    }

    #[test]
    fn amount_from_repr_preserves_sign() {
        let negative = Decimal::from_str("-999999999.99").expect("Valid decimal");
        let result = amount_from_repr(negative);
        assert!(result < 0.0, "Negative value must remain negative");
        assert!((result - (-999_999_999.99)).abs() < 1e-2);
    }

    #[test]
    fn try_amount_returns_ok_for_representable_decimal() {
        // rust_decimal's max is within f64 range, so this should succeed
        let decimal = Decimal::MAX;
        let result = try_amount_from_repr(decimal);
        // Even MAX should be representable (though with precision loss)
        // The key is it doesn't return 0 or fail silently
        assert!(
            result.is_ok(),
            "Decimal::MAX should be convertible to f64"
        );
        let val = result.expect("Conversion should succeed");
        assert!(val > 0.0, "Converted value must not be zero");
    }
}
