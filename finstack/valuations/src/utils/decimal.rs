//! Decimal conversion utilities.
//!
//! Provides a canonical `f64 -> Decimal` helper that propagates errors on
//! non-finite or unrepresentable values instead of silently returning zero.
//! All production code that converts raw `f64` user input to `Decimal` should
//! use this helper; trusted literals in tests and examples can use
//! `Decimal::from_f64_retain(...).expect(...)` or the `dec!` macro.

use finstack_core::{InputError, NonFiniteKind};
use rust_decimal::Decimal;

/// Convert an `f64` to [`Decimal`], returning an error for non-finite values.
///
/// This prevents silent masking of `NaN`/`Infinity` values as zero, which would
/// result in zero rates, strikes, or spreads that materially misprice instruments.
///
/// # Errors
///
/// Returns [`InputError::NonFiniteValue`] for `NaN`, `+inf`, or `-inf`.
/// Returns [`InputError::ConversionOverflow`] when the finite value cannot be
/// represented as `Decimal` (extremely large `f64` magnitude).
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::utils::decimal::f64_to_decimal;
/// use finstack_core::InputError;
///
/// assert!(f64_to_decimal(0.05, "strike").is_ok());
/// assert!(f64_to_decimal(f64::NAN, "strike").is_err());
/// assert!(f64_to_decimal(f64::INFINITY, "strike").is_err());
/// ```
pub fn f64_to_decimal(value: f64, _field: &str) -> finstack_core::Result<Decimal> {
    if value.is_nan() {
        return Err(InputError::NonFiniteValue {
            kind: NonFiniteKind::NaN,
        }
        .into());
    }
    if value.is_infinite() {
        let kind = if value.is_sign_positive() {
            NonFiniteKind::PosInfinity
        } else {
            NonFiniteKind::NegInfinity
        };
        return Err(InputError::NonFiniteValue { kind }.into());
    }
    Decimal::try_from(value).map_err(|_| finstack_core::Error::from(InputError::ConversionOverflow))
}
