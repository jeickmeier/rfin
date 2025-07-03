//! Type-safe money amounts – **simplified**.
//!
//! This version collapses the previous generic `Money<F>` hierarchy into a
//! single concrete type using `f64` for the numeric representation.
//! The struct guarantees that all arithmetic keeps the same [`Currency`].
//!
//! ```
//! use rfin_core::{Money, Currency};
//! let price = Money::usd(19.99);
//! let tax   = Money::eur( 5.0); // different currency ➜ error on addition
//! assert!((price + tax).is_err());
//! ```

#![allow(clippy::items_after_test_module)]

use crate::currency::Currency;
use crate::error::Error;
use core::fmt;
use core::ops::{Div, Mul};

// Internal macro to assert that two Money values share the same currency.
// In release builds it performs the runtime check and returns the appropriate
// `Error`; in debug builds it additionally triggers `debug_assert_eq!` to fail
// fast during testing.
macro_rules! assert_currency_eq {
    ($lhs:expr, $rhs:expr) => {
        // Fast‐fail in release via returned Error; debug builds no longer panic
        if $lhs.currency != $rhs.currency {
            return Err(Error::CurrencyMismatch {
                expected: $lhs.currency,
                actual: $rhs.currency,
            });
        }
    };
}

/// Monetary amount tagged with a [`Currency`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Money {
    amount: f64,
    currency: Currency,
}

impl Money {
    // ---------------------------------------------------------------------
    // Constructors & accessors
    // ---------------------------------------------------------------------

    /// Create a new `Money` value.
    #[inline]
    pub const fn new(amount: f64, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Shorthand for `(amount, Currency::USD)`.
    #[inline]
    pub const fn usd(amount: f64) -> Self {
        Self::new(amount, Currency::USD)
    }

    /// Shorthand for `(amount, Currency::EUR)`.
    #[inline]
    pub const fn eur(amount: f64) -> Self {
        Self::new(amount, Currency::EUR)
    }

    /// Amount accessor (by value).
    #[inline]
    pub const fn amount(&self) -> f64 {
        self.amount
    }

    /// Currency accessor.
    #[inline]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Consume `self` and return just the numeric amount.
    #[inline]
    pub fn into_amount(self) -> f64 {
        self.amount
    }

    /// Consume `self` into `(amount, currency)`.
    #[inline]
    pub fn into_parts(self) -> (f64, Currency) {
        (self.amount, self.currency)
    }

    // ---------------------------------------------------------------------
    // Checked arithmetic
    // ---------------------------------------------------------------------

    /// Add two amounts, returning an `Error::CurrencyMismatch` if the currencies differ.
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        assert_currency_eq!(self, rhs);
        Ok(Self::new(self.amount + rhs.amount, self.currency))
    }

    /// Subtract two amounts, returning an `Error::CurrencyMismatch` if the currencies differ.
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        assert_currency_eq!(self, rhs);
        Ok(Self::new(self.amount - rhs.amount, self.currency))
    }
}

// -------------------------------------------------------------------------
// Formatting
// -------------------------------------------------------------------------
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

// -------------------------------------------------------------------------
// Scalar arithmetic keeping currency intact
// -------------------------------------------------------------------------
impl Mul<f64> for Money {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        Self::new(self.amount * rhs, self.currency)
    }
}

impl Div<f64> for Money {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self::Output {
        Self::new(self.amount / rhs, self.currency)
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
///
/// ```
/// use rfin_core::{money, Currency, Money};
/// let price: Money = money!(99.99, USD);
/// assert_eq!(price, Money::new(99.99, Currency::USD));
/// ```
#[macro_export]
macro_rules! money {
    ($amount:expr, $code:ident) => {
        $crate::Money::new($amount as f64, $crate::Currency::$code)
    };
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
        let a = Money::usd(50.0);
        let b = Money::usd(25.0);
        let c = (a + b).unwrap();
        assert_eq!(c.amount(), 75.0);
    }

    #[test]
    fn currency_mismatch_error() {
        let usd = Money::usd(10.0);
        let eur = Money::eur(10.0);
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

// -------------------------------------------------------------------------
// Unchecked arithmetic (default) – currency must match (debug_assert)
// -------------------------------------------------------------------------

use core::ops::{Add, Sub};

impl Add for Money {
    type Output = Result<Self, Error>;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        if self.currency != rhs.currency {
            return Err(Error::CurrencyMismatch {
                expected: self.currency,
                actual: rhs.currency,
            });
        }
        Ok(Self::new(self.amount + rhs.amount, self.currency))
    }
}

impl Sub for Money {
    type Output = Result<Self, Error>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        if self.currency != rhs.currency {
            return Err(Error::CurrencyMismatch {
                expected: self.currency,
                actual: rhs.currency,
            });
        }
        Ok(Self::new(self.amount - rhs.amount, self.currency))
    }
}
