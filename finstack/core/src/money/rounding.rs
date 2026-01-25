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
/// Converts Decimal representation to f64.
///
/// # Safety
///
/// - **Debug builds**: Panics if conversion fails (catches bugs early)
/// - **Release builds**: Panics if conversion fails (silent fallback is unsafe)
///
/// Use [`try_amount_from_repr`] for explicit error handling at API boundaries.
#[inline]
pub(crate) fn amount_from_repr(x: AmountRepr) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    let result = x.to_f64();
    assert!(
        result.is_some(),
        "Decimal to f64 conversion failed: value outside representable range"
    );
    // Safe because the assert above guarantees `result` is Some.
    result.unwrap_or(0.0)
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
    let res = try_repr_mul_f64(a, rhs);
    assert!(
        res.is_ok(),
        "Money multiplication requires finite, representable scalar (got {:?})",
        rhs
    );
    // Safe because the assert above guarantees `res` is Ok.
    res.unwrap_or(Decimal::ZERO)
}

#[inline]
pub(crate) fn repr_div_f64(a: AmountRepr, rhs: f64) -> AmountRepr {
    let res = try_repr_div_f64(a, rhs);
    assert!(
        res.is_ok(),
        "Money division requires finite, non-zero, representable scalar (got {:?})",
        rhs
    );
    // Safe because the assert above guarantees `res` is Ok.
    res.unwrap_or(a)
}

/// Round `x` to `dp` decimal places using the supplied [`RoundingMode`].
/// Converts f64 input to Decimal for proper rounding.
#[inline]
pub(crate) fn round_f64(x: f64, dp: i32, mode: RoundingMode) -> Decimal {
    let res = try_round_f64(x, dp, mode);
    assert!(
        res.is_ok(),
        "Money rounding requires finite, representable scalar (got {:?})",
        x
    );
    // Safe because the assert above guarantees `res` is Ok.
    res.unwrap_or(Decimal::ZERO)
}

/// Fallible multiplication by an `f64` scalar (no silent substitution).
#[inline]
pub(crate) fn try_repr_mul_f64(a: AmountRepr, rhs: f64) -> Result<AmountRepr, Error> {
    if !rhs.is_finite() {
        let kind = if rhs.is_nan() {
            crate::error::NonFiniteKind::NaN
        } else if rhs.is_sign_positive() {
            crate::error::NonFiniteKind::PosInfinity
        } else {
            crate::error::NonFiniteKind::NegInfinity
        };
        return Err(InputError::NonFiniteValue { kind }.into());
    }
    let Some(rhs_decimal) = Decimal::from_f64_retain(rhs) else {
        return Err(InputError::ConversionOverflow.into());
    };
    Ok(a * rhs_decimal)
}

/// Fallible division by an `f64` scalar (no silent substitution).
#[inline]
pub(crate) fn try_repr_div_f64(a: AmountRepr, rhs: f64) -> Result<AmountRepr, Error> {
    if !rhs.is_finite() {
        let kind = if rhs.is_nan() {
            crate::error::NonFiniteKind::NaN
        } else if rhs.is_sign_positive() {
            crate::error::NonFiniteKind::PosInfinity
        } else {
            crate::error::NonFiniteKind::NegInfinity
        };
        return Err(InputError::NonFiniteValue { kind }.into());
    }
    if rhs == 0.0 {
        return Err(InputError::Invalid.into());
    }
    let Some(rhs_decimal) = Decimal::from_f64_retain(rhs) else {
        return Err(InputError::ConversionOverflow.into());
    };
    Ok(a / rhs_decimal)
}

/// Fallible rounding of an `f64` into a Decimal (no silent substitution).
#[inline]
pub(crate) fn try_round_f64(x: f64, dp: i32, mode: RoundingMode) -> Result<Decimal, Error> {
    if !x.is_finite() {
        let kind = if x.is_nan() {
            crate::error::NonFiniteKind::NaN
        } else if x.is_sign_positive() {
            crate::error::NonFiniteKind::PosInfinity
        } else {
            crate::error::NonFiniteKind::NegInfinity
        };
        return Err(InputError::NonFiniteValue { kind }.into());
    }
    let Some(decimal) = Decimal::from_f64_retain(x) else {
        return Err(InputError::ConversionOverflow.into());
    };
    Ok(round_decimal(decimal, dp, mode))
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
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
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
        assert!(result.is_ok(), "Decimal::MAX should be convertible to f64");
        let val = result.expect("Conversion should succeed");
        assert!(val > 0.0, "Converted value must not be zero");
    }

    // ========================================================================
    // repr_add and repr_sub tests
    // ========================================================================

    #[test]
    fn repr_add_basic() {
        let a = Decimal::from_str("100.50").expect("valid decimal");
        let b = Decimal::from_str("50.25").expect("valid decimal");
        let result = repr_add(a, b);
        assert_eq!(result, Decimal::from_str("150.75").expect("valid decimal"));
    }

    #[test]
    fn repr_add_negative() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let b = Decimal::from_str("-25.00").expect("valid decimal");
        let result = repr_add(a, b);
        assert_eq!(result, Decimal::from_str("75.00").expect("valid decimal"));
    }

    #[test]
    fn repr_sub_basic() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let b = Decimal::from_str("30.00").expect("valid decimal");
        let result = repr_sub(a, b);
        assert_eq!(result, Decimal::from_str("70.00").expect("valid decimal"));
    }

    #[test]
    fn repr_sub_negative_result() {
        let a = Decimal::from_str("25.00").expect("valid decimal");
        let b = Decimal::from_str("100.00").expect("valid decimal");
        let result = repr_sub(a, b);
        assert_eq!(result, Decimal::from_str("-75.00").expect("valid decimal"));
    }

    // ========================================================================
    // repr_mul_f64 tests
    // ========================================================================

    #[test]
    fn repr_mul_f64_positive() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let result = repr_mul_f64(a, 2.5);
        assert_eq!(result, Decimal::from_str("250.00").expect("valid decimal"));
    }

    #[test]
    fn repr_mul_f64_negative_scalar() {
        let a = Decimal::from_str("50.00").expect("valid decimal");
        let result = repr_mul_f64(a, -2.0);
        assert_eq!(result, Decimal::from_str("-100.00").expect("valid decimal"));
    }

    #[test]
    fn repr_mul_f64_fractional() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let result = repr_mul_f64(a, 0.1);
        let expected = Decimal::from_str("10.00").expect("valid decimal");
        assert!((result - expected).abs() < Decimal::from_str("0.01").expect("valid decimal"));
    }

    #[test]
    #[should_panic(expected = "Money multiplication requires finite, representable scalar")]
    fn repr_mul_f64_panics_on_nan() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        repr_mul_f64(a, f64::NAN);
    }

    #[test]
    #[should_panic(expected = "Money multiplication requires finite, representable scalar")]
    fn repr_mul_f64_panics_on_infinity() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        repr_mul_f64(a, f64::INFINITY);
    }

    // ========================================================================
    // repr_div_f64 tests
    // ========================================================================

    #[test]
    fn repr_div_f64_positive() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let result = repr_div_f64(a, 2.0);
        assert_eq!(result, Decimal::from_str("50.00").expect("valid decimal"));
    }

    #[test]
    fn repr_div_f64_negative_scalar() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let result = repr_div_f64(a, -4.0);
        assert_eq!(result, Decimal::from_str("-25.00").expect("valid decimal"));
    }

    #[test]
    fn repr_div_f64_fractional_divisor() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        let result = repr_div_f64(a, 0.5);
        assert_eq!(result, Decimal::from_str("200.00").expect("valid decimal"));
    }

    #[test]
    #[should_panic(expected = "Money division requires finite, non-zero, representable scalar")]
    fn repr_div_f64_panics_on_nan() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        repr_div_f64(a, f64::NAN);
    }

    #[test]
    #[should_panic(expected = "Money division requires finite, non-zero, representable scalar")]
    fn repr_div_f64_panics_on_infinity() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        repr_div_f64(a, f64::INFINITY);
    }

    #[test]
    #[should_panic(expected = "Money division requires finite, non-zero, representable scalar")]
    fn repr_div_f64_panics_on_zero() {
        let a = Decimal::from_str("100.00").expect("valid decimal");
        repr_div_f64(a, 0.0);
    }

    // ========================================================================
    // round_f64 tests
    // ========================================================================

    #[test]
    fn round_f64_bankers_positive() {
        let result = round_f64(1.5, 0, RoundingMode::Bankers);
        assert_eq!(result, Decimal::from_str("2").expect("valid decimal"));
    }

    #[test]
    fn round_f64_bankers_tie_to_even() {
        // 2.5 rounds to 2 (even), 3.5 rounds to 4 (even)
        let result1 = round_f64(2.5, 0, RoundingMode::Bankers);
        assert_eq!(result1, Decimal::from_str("2").expect("valid decimal"));

        let result2 = round_f64(3.5, 0, RoundingMode::Bankers);
        assert_eq!(result2, Decimal::from_str("4").expect("valid decimal"));
    }

    #[test]
    fn round_f64_away_from_zero() {
        let result1 = round_f64(1.5, 0, RoundingMode::AwayFromZero);
        assert_eq!(result1, Decimal::from_str("2").expect("valid decimal"));

        let result2 = round_f64(-1.5, 0, RoundingMode::AwayFromZero);
        assert_eq!(result2, Decimal::from_str("-2").expect("valid decimal"));
    }

    #[test]
    fn round_f64_toward_zero() {
        let result1 = round_f64(1.9, 0, RoundingMode::TowardZero);
        assert_eq!(result1, Decimal::from_str("1").expect("valid decimal"));

        let result2 = round_f64(-1.9, 0, RoundingMode::TowardZero);
        assert_eq!(result2, Decimal::from_str("-1").expect("valid decimal"));
    }

    #[test]
    fn round_f64_floor() {
        let result1 = round_f64(1.9, 0, RoundingMode::Floor);
        assert_eq!(result1, Decimal::from_str("1").expect("valid decimal"));

        let result2 = round_f64(-1.1, 0, RoundingMode::Floor);
        assert_eq!(result2, Decimal::from_str("-2").expect("valid decimal"));
    }

    #[test]
    fn round_f64_ceil() {
        let result1 = round_f64(1.1, 0, RoundingMode::Ceil);
        assert_eq!(result1, Decimal::from_str("2").expect("valid decimal"));

        let result2 = round_f64(-1.9, 0, RoundingMode::Ceil);
        assert_eq!(result2, Decimal::from_str("-1").expect("valid decimal"));
    }

    #[test]
    fn round_f64_with_decimal_places() {
        let result = round_f64(1.234567, 2, RoundingMode::Bankers);
        let expected = Decimal::from_str("1.23").expect("valid decimal");
        assert!((result - expected).abs() < Decimal::from_str("0.01").expect("valid decimal"));
    }

    #[test]
    #[should_panic(expected = "Money rounding requires finite, representable scalar")]
    fn round_f64_panics_on_nan() {
        round_f64(f64::NAN, 2, RoundingMode::Bankers);
    }

    // ========================================================================
    // round_decimal tests
    // ========================================================================

    #[test]
    fn round_decimal_bankers() {
        let val = Decimal::from_str("2.5").expect("valid decimal");
        let result = round_decimal(val, 0, RoundingMode::Bankers);
        assert_eq!(result, Decimal::from_str("2").expect("valid decimal"));
    }

    #[test]
    fn round_decimal_away_from_zero() {
        let val = Decimal::from_str("1.5").expect("valid decimal");
        let result = round_decimal(val, 0, RoundingMode::AwayFromZero);
        assert_eq!(result, Decimal::from_str("2").expect("valid decimal"));
    }

    #[test]
    fn round_decimal_toward_zero() {
        let val = Decimal::from_str("1.9").expect("valid decimal");
        let result = round_decimal(val, 0, RoundingMode::TowardZero);
        assert_eq!(result, Decimal::from_str("1").expect("valid decimal"));
    }

    #[test]
    fn round_decimal_floor() {
        let val = Decimal::from_str("1.9").expect("valid decimal");
        let result = round_decimal(val, 0, RoundingMode::Floor);
        assert_eq!(result, Decimal::from_str("1").expect("valid decimal"));
    }

    #[test]
    fn round_decimal_ceil() {
        let val = Decimal::from_str("1.1").expect("valid decimal");
        let result = round_decimal(val, 0, RoundingMode::Ceil);
        assert_eq!(result, Decimal::from_str("2").expect("valid decimal"));
    }

    #[test]
    fn round_decimal_negative_dp_returns_unchanged() {
        let val = Decimal::from_str("123.456").expect("valid decimal");
        let result = round_decimal(val, -1, RoundingMode::Bankers);
        assert_eq!(result, val);
    }

    #[test]
    fn round_decimal_with_decimal_places() {
        let val = Decimal::from_str("1.23456").expect("valid decimal");
        let result = round_decimal(val, 2, RoundingMode::Bankers);
        let expected = Decimal::from_str("1.23").expect("valid decimal");
        assert!((result - expected).abs() < Decimal::from_str("0.01").expect("valid decimal"));
    }
}
