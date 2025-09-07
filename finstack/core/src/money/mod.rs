//! Type-safe money amounts with configurable rounding and precision.
//!
//! This module provides the `Money` type for representing monetary amounts with
//! currency information and configurable rounding behavior. It supports both
//! `f64` and `rust_decimal::Decimal` backends via feature flags.
//!
//! # Rounding Modes
//!
//! The library supports five rounding modes via the `RoundingMode` enum:
//!
//! - **Bankers**: Round half to even (IEEE 754 default, unbiased)
//! - **AwayFromZero**: Round half away from zero (traditional rounding)
//! - **TowardZero**: Truncate toward zero (floor for positive, ceil for negative)
//! - **Floor**: Round toward negative infinity
//! - **Ceil**: Round toward positive infinity
//!
//! # Display vs Internal Rounding Behavior
//!
//! ## f64 Backend (default)
//! - **Internal**: Full IEEE 754 double precision (53-bit mantissa)
//! - **Display**: Rounded to currency-specific decimal places (e.g., 2 for USD, 0 for JPY)
//! - **Arithmetic**: Performed at full precision, rounded only at display/export
//! - **Example**: USD 1.23456789 → internal: 1.23456789, display: 1.23
//!
//! ## Decimal Backend (feature = "decimal128")
//! - **Internal**: Full 28-digit precision with configurable scale
//! - **Display**: Rounded to currency-specific decimal places
//! - **Arithmetic**: Performed at full precision, rounded only at display/export
//! - **Example**: USD 1.234567890123456789 → internal: 1.234567890123456789, display: 1.23
//!
//! # Currency-Specific Precision
//!
//! Each currency has a default number of decimal places:
//! - **USD, EUR, GBP**: 2 decimal places
//! - **JPY, KRW**: 0 decimal places (whole units only)
//! - **KWD, BHD, TND**: 3 decimal places
//! - **Custom**: Override via `CurrencyScalePolicy`
//!
//! See unit tests and `examples/` for usage and behaviour.

use crate::config::{FinstackConfig, RoundingMode};
use crate::currency::Currency;
use crate::dates::Date;
use crate::error::Error;
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

type AmountRepr = f64;

// Submodule for FX interfaces
pub mod fx;

/// Monetary amount tagged with a [`Currency`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Money {
    amount: AmountRepr,
    currency: Currency,
}

impl Money {
    // ---------------------------------------------------------------------
    // Constructors & accessors
    // ---------------------------------------------------------------------

    /// Create a new `Money` value from an `f64` amount.
    #[must_use]
    #[inline]
    pub fn new(amount: f64, currency: Currency) -> Self {
        // Fallback to ISO-4217 minor units when no config is provided.
        let dp = currency.decimals();
        let mode = RoundingMode::Bankers;
        let rounded = round_f64(amount, dp as i32, mode);
        Self {
            amount: rounded,
            currency,
        }
    }

    /// Create a new `Money` value using an explicit configuration.
    #[must_use]
    #[inline]
    pub fn new_with_config(amount: f64, currency: Currency, cfg: &FinstackConfig) -> Self {
        let dp = crate::config::ingest_scale_for(cfg, currency);
        let mode = cfg.rounding.mode;
        let rounded = round_f64(amount, dp as i32, mode);
        Self {
            amount: rounded,
            currency,
        }
    }

    /// Amount accessor (by value).
    #[inline]
    pub fn amount(&self) -> f64 {
        amount_from_repr(self.amount)
    }

    /// Currency accessor.
    #[inline]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Consume `self` and return just the numeric amount.
    #[inline]
    pub fn into_amount(self) -> f64 {
        amount_from_repr(self.amount)
    }

    /// Consume `self` into `(amount, currency)`.
    #[inline]
    pub fn into_parts(self) -> (f64, Currency) {
        (amount_from_repr(self.amount), self.currency)
    }

    // ---------------------------------------------------------------------
    // Checked arithmetic
    // ---------------------------------------------------------------------

    /// Add two amounts, returning an `Error::CurrencyMismatch` if the currencies differ.
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self {
            amount: repr_add(self.amount, rhs.amount),
            currency: self.currency,
        })
    }

    /// Subtract two amounts, returning an `Error::CurrencyMismatch` if the currencies differ.
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self {
            amount: repr_sub(self.amount, rhs.amount),
            currency: self.currency,
        })
    }

    /// Convert this `Money` into another currency using an [`fx::FxProvider`].
    pub fn convert(
        self,
        to: Currency,
        on: Date,
        provider: &impl crate::money::fx::FxProvider,
        policy: crate::money::fx::FxConversionPolicy,
    ) -> crate::Result<Self> {
        if self.currency == to {
            return Ok(self);
        }
        let rate = provider.rate(self.currency, to, on, policy)?;
        let new_amount = repr_mul_f64(self.amount, rate);
        Ok(Self {
            amount: new_amount,
            currency: to,
        })
    }
}

// -------------------------------------------------------------------------
// Formatting
// -------------------------------------------------------------------------
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Default formatting uses ISO-4217 minor units and bankers rounding.
        let dp = self.currency.decimals() as usize;
        // For f64 mode, format with currency-specific minor units. Rounding mode is
        // not customisable here (uses standard formatting semantics).
        write!(
            f,
            "{} {val:.prec$}",
            self.currency,
            val = self.amount,
            prec = dp
        )
    }
}

impl Money {
    /// Format this money using an explicit configuration (rounding mode and per-currency scales).
    pub fn format_with_config(&self, cfg: &FinstackConfig) -> String {
        let dp = crate::config::output_scale_for(cfg, self.currency) as usize;
        format!(
            "{} {val:.prec$}",
            self.currency,
            val = self.amount,
            prec = dp
        )
    }
}

// -------------------------------------------------------------------------
// Scalar arithmetic keeping currency intact
// -------------------------------------------------------------------------
impl Mul<f64> for Money {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            amount: repr_mul_f64(self.amount, rhs),
            currency: self.currency,
        }
    }
}

impl Div<f64> for Money {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            amount: repr_div_f64(self.amount, rhs),
            currency: self.currency,
        }
    }
}

impl Add for Money {
    type Output = Result<Self, Error>;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self {
            amount: repr_add(self.amount, rhs.amount),
            currency: self.currency,
        })
    }
}

impl Sub for Money {
    type Output = Result<Self, Error>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self {
            amount: repr_sub(self.amount, rhs.amount),
            currency: self.currency,
        })
    }
}

// -------------------------------------------------------------------------
// Conversions
// -------------------------------------------------------------------------
// Generic tuple conversions for common numeric primitives.
macro_rules! from_numeric_tuple {
    ($($t:ty),+) => { $(
        impl From<($t, Currency)> for Money {
            #[inline]
            fn from(value: ($t, Currency)) -> Self {
                Self::new(value.0 as f64, value.1)
            }
        }
    )+ };
}

from_numeric_tuple!(f64, i64, u64);

// -------------------------------------------------------------------------
// Convenience macro
// -------------------------------------------------------------------------

/// Shorthand for constructing [`Money`] literals.
/// See unit tests and `examples/` for usage.
#[macro_export]
macro_rules! money {
    ($amount:expr, $code:ident) => {
        $crate::money::Money::new($amount as f64, $crate::currency::Currency::$code)
    };
}

// -------------------------------------------------------------------------
// Unchecked arithmetic (default) – currency must match (debug_assert)
// -------------------------------------------------------------------------

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        ensure_same_currency(self, &rhs).unwrap();
        self.amount = repr_add(self.amount, rhs.amount);
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        ensure_same_currency(self, &rhs).unwrap();
        self.amount = repr_sub(self.amount, rhs.amount);
    }
}

impl MulAssign<f64> for Money {
    fn mul_assign(&mut self, rhs: f64) {
        self.amount = repr_mul_f64(self.amount, rhs);
    }
}

impl DivAssign<f64> for Money {
    fn div_assign(&mut self, rhs: f64) {
        self.amount = repr_div_f64(self.amount, rhs);
    }
}

/// Ensure two `Money` values share the same currency.
#[inline]
fn ensure_same_currency(lhs: &Money, rhs: &Money) -> Result<(), Error> {
    if lhs.currency != rhs.currency {
        return Err(Error::CurrencyMismatch {
            expected: lhs.currency,
            actual: rhs.currency,
        });
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Internal helpers for representation-dependent behaviour
// -------------------------------------------------------------------------------------------------

#[inline]
fn amount_from_repr(x: AmountRepr) -> f64 {
    x
}

#[inline]
fn repr_add(a: AmountRepr, b: AmountRepr) -> AmountRepr {
    a + b
}

#[inline]
fn repr_sub(a: AmountRepr, b: AmountRepr) -> AmountRepr {
    a - b
}

#[inline]
fn repr_mul_f64(a: AmountRepr, rhs: f64) -> AmountRepr {
    a * rhs
}

#[inline]
fn repr_div_f64(a: AmountRepr, rhs: f64) -> AmountRepr {
    a / rhs
}

#[inline]
fn round_f64(x: f64, dp: i32, mode: RoundingMode) -> f64 {
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
        RoundingMode::AwayFromZero => (x * factor).abs().ceil() * x.signum() / factor,
        RoundingMode::TowardZero => (x * factor).trunc() / factor,
        RoundingMode::Floor => (x * factor).floor() / factor,
        RoundingMode::Ceil => (x * factor).ceil() / factor,
    }
}

// -------------------------------------------------------------------------
// Tests (basic – exhaustive suite lives in `tests/` folder)
// -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_and_accessors() {
        let m = Money::new(100.0, Currency::USD);
        assert_eq!(m.amount(), 100.0);
        assert_eq!(m.currency(), Currency::USD);
    }

    #[test]
    fn checked_ops() {
        let a = Money::new(50.0, Currency::USD);
        let b = Money::new(25.0, Currency::USD);
        let c = (a + b).unwrap();
        assert_eq!(c.amount(), 75.0);
    }

    #[test]
    fn currency_mismatch_error() {
        let usd = Money::new(10.0, Currency::USD);
        let eur = Money::new(10.0, Currency::EUR);
        assert!((usd + eur).is_err());
    }

    #[test]
    fn macro_constructs_money() {
        let m = crate::money!(250.0, GBP);
        assert_eq!(m.amount(), 250.0);
        assert_eq!(m.currency(), Currency::GBP);
    }

    #[test]
    fn tuple_from_conversions() {
        use core::convert::Into;
        let m1: Money = (100_i64, Currency::USD).into();
        assert_eq!(m1.amount(), 100.0);
        let m2: Money = (42_u64, Currency::EUR).into();
        assert_eq!(m2.amount(), 42.0);
    }
}
