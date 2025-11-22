//! Internal rounding helpers backing `Money` arithmetic.
//!
//! Amounts are stored as `Decimal` values (`AmountRepr`) to provide accounting-grade
//! precision and deterministic arithmetic. The routines here provide Decimal arithmetic
//! on that representation and expose helpers that honour [`RoundingMode`](crate::config::RoundingMode).
//!
//! The rounding functions are used internally by `Money` operations and
//! are not part of the public API. For rounding examples, see `Money` documentation.

use crate::config::RoundingMode;
use rust_decimal::Decimal;

/// Internal numeric representation for `Money` amounts.
/// Uses Decimal for accounting-grade precision and deterministic arithmetic.
pub(crate) type AmountRepr = Decimal;

#[inline]
pub(crate) fn amount_from_repr(x: AmountRepr) -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    x.to_f64().unwrap_or(0.0)
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
