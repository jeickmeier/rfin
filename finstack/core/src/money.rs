//! Type-safe money amounts – **simplified**.
//!
//! This version collapses the previous generic `Money<F>` hierarchy into a
//! single concrete type using `f64` for the numeric representation.
//! The struct guarantees that all arithmetic keeps the same [`Currency`].
//!
//! ```
//! use finstack_core::{Money, Currency};
//! let price = Money::new(19.99, Currency::USD);
//! let tax   = Money::new( 5.0, Currency::EUR); // different currency ➜ error on addition
//! assert!((price + tax).is_err());
//! ```

#![allow(clippy::items_after_test_module)]

use crate::currency::Currency;
use crate::error::Error;
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

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
    #[must_use]
    #[inline]
    pub const fn new(amount: f64, currency: Currency) -> Self {
        Self { amount, currency }
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
        ensure_same_currency(&self, &rhs)?;
        Ok(Self::new(self.amount + rhs.amount, self.currency))
    }

    /// Subtract two amounts, returning an `Error::CurrencyMismatch` if the currencies differ.
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self::new(self.amount - rhs.amount, self.currency))
    }
}

// -------------------------------------------------------------------------
// Formatting
// -------------------------------------------------------------------------
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {:.2}", self.currency, self.amount)
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

impl Add for Money {
    type Output = Result<Self, Error>;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self::new(self.amount + rhs.amount, self.currency))
    }
}

impl Sub for Money {
    type Output = Result<Self, Error>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self::new(self.amount - rhs.amount, self.currency))
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
/// use finstack_core::{money, Currency, Money};
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
// Unchecked arithmetic (default) – currency must match (debug_assert)
// -------------------------------------------------------------------------

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        ensure_same_currency(self, &rhs).unwrap();
        self.amount += rhs.amount;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        ensure_same_currency(self, &rhs).unwrap();
        self.amount -= rhs.amount;
    }
}

impl MulAssign<f64> for Money {
    fn mul_assign(&mut self, rhs: f64) {
        self.amount *= rhs;
    }
}

impl DivAssign<f64> for Money {
    fn div_assign(&mut self, rhs: f64) {
        self.amount /= rhs;
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
