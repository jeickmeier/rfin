//! Internal rounding helpers backing `Money` arithmetic.
//!
//! Amounts are stored as scaled `f64` values (`AmountRepr`).  The routines here
//! provide fast arithmetic on that representation and expose a single
//! [`round_f64`] helper that honours [`RoundingMode`](crate::config::RoundingMode).
//!
//! 
//! The `round_f64` function is used internally by `Money` operations and 
//! is not part of the public API. For rounding examples, see `Money` documentation.

use crate::config::RoundingMode;

/// Internal numeric representation for `Money` amounts.
pub(crate) type AmountRepr = f64;

#[inline]
pub(crate) fn amount_from_repr(x: AmountRepr) -> f64 {
    x
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
    a * rhs
}

#[inline]
pub(crate) fn repr_div_f64(a: AmountRepr, rhs: f64) -> AmountRepr {
    a / rhs
}

/// Round `x` to `dp` decimal places using the supplied [`RoundingMode`].
#[inline]
pub(crate) fn round_f64(x: f64, dp: i32, mode: RoundingMode) -> f64 {
    let factor = 10f64.powi(dp);
    match mode {
        RoundingMode::Bankers => {
            // Emulate bankers: round half to even using Rust's round() then adjust ties.
            let y = x * factor;
            let r = y.round();
            let tie = (y.abs().fract() - 0.5).abs() <= 1e-15;
            if tie && (r as i64).abs() % 2 != 0 {
                return (r - y.signum()) / factor;
            }
            r / factor
        }
        RoundingMode::AwayFromZero => {
            let y = x * factor;
            if y >= 0.0 {
                (y + 0.5).floor() / factor
            } else {
                (y - 0.5).ceil() / factor
            }
        }
        RoundingMode::TowardZero => (x * factor).trunc() / factor,
        RoundingMode::Floor => (x * factor).floor() / factor,
        RoundingMode::Ceil => (x * factor).ceil() / factor,
    }
}
