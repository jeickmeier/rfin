//! Money type and operations.
//!
//! This module provides the [`Money`] type for representing monetary values
//! with currency information, ensuring type safety for financial calculations.

use super::currency::Currency;
use crate::error::{Error, InputError};
use core::fmt;
use core::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "decimal128")]
use rust_decimal::Decimal;

/// A monetary value with an associated currency.
///
/// The generic parameter `F` represents the numeric type used for the amount,
/// defaulting to `f64` for convenience.
///
/// # Examples
///
/// ```
/// use rfin_core::primitives::{Money, Currency};
///
/// let usd_100 = Money::new(100.0, Currency::USD);
/// let usd_50 = Money::new(50.0, Currency::USD);
/// let total = usd_100 + usd_50;
///
/// assert_eq!(*total.amount(), 150.0);
/// assert_eq!(total.currency(), Currency::USD);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Money<F = f64> {
    amount: F,
    currency: Currency,
}

impl<F> Money<F> {
    /// Creates a new Money value with the specified amount and currency.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::primitives::{Money, Currency};
    ///
    /// let usd_money = Money::new(42.50, Currency::USD);
    /// let eur_money = Money::new(100, Currency::EUR);
    /// ```
    #[inline]
    pub const fn new(amount: F, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Returns the amount of this money value.
    #[inline]
    pub const fn amount(&self) -> &F {
        &self.amount
    }

    /// Returns the currency of this money value.
    #[inline]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Consumes the Money and returns the amount.
    #[inline]
    pub fn into_amount(self) -> F {
        self.amount
    }

    /// Returns a tuple of (amount, currency).
    #[inline]
    pub fn into_parts(self) -> (F, Currency) {
        (self.amount, self.currency)
    }

    /// Adds two Money values with error handling.
    ///
    /// Returns an error if the currencies don't match.
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error>
    where
        F: Add<Output = F>,
    {
        if self.currency != rhs.currency {
            return Err(InputError::CurrencyMismatch {
                expected: self.currency,
                actual: rhs.currency,
            }
            .into());
        }
        Ok(Self {
            amount: self.amount + rhs.amount,
            currency: self.currency,
        })
    }

    /// Subtracts two Money values with error handling.
    ///
    /// Returns an error if the currencies don't match.
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error>
    where
        F: Sub<Output = F>,
    {
        if self.currency != rhs.currency {
            return Err(InputError::CurrencyMismatch {
                expected: self.currency,
                actual: rhs.currency,
            }
            .into());
        }
        Ok(Self {
            amount: self.amount - rhs.amount,
            currency: self.currency,
        })
    }
}

/// Convenience constructor for USD money.
impl<F> Money<F> {
    /// Creates a new Money value in USD.
    #[inline]
    pub const fn usd(amount: F) -> Self {
        Self::new(amount, Currency::USD)
    }

    /// Creates a new Money value in EUR.
    #[inline]
    pub const fn eur(amount: F) -> Self {
        Self::new(amount, Currency::EUR)
    }
}

impl<F: fmt::Display> fmt::Display for Money<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

// Arithmetic operations with currency guards

/// Addition of two Money values.
///
/// This operation requires both Money values to have the same currency.
/// Mixing currencies will result in a panic in debug builds.
impl<F: Add<Output = F>> Add for Money<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(
            self.currency, rhs.currency,
            "Cannot add money with different currencies: {} and {}",
            self.currency, rhs.currency
        );
        Self {
            amount: self.amount + rhs.amount,
            currency: self.currency,
        }
    }
}

/// Subtraction of two Money values.
///
/// This operation requires both Money values to have the same currency.
/// Mixing currencies will result in a panic in debug builds.
impl<F: Sub<Output = F>> Sub for Money<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(
            self.currency, rhs.currency,
            "Cannot subtract money with different currencies: {} and {}",
            self.currency, rhs.currency
        );
        Self {
            amount: self.amount - rhs.amount,
            currency: self.currency,
        }
    }
}

/// Multiplication of Money by a scalar value.
///
/// This allows scaling a monetary amount while preserving the currency.
impl<F: Mul<F, Output = F> + Copy> Mul<F> for Money<F> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: F) -> Self::Output {
        Self {
            amount: self.amount * rhs,
            currency: self.currency,
        }
    }
}

/// Division of Money by a scalar value.
///
/// This allows scaling down a monetary amount while preserving the currency.
impl<F: Div<F, Output = F> + Copy> Div<F> for Money<F> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: F) -> Self::Output {
        Self {
            amount: self.amount / rhs,
            currency: self.currency,
        }
    }
}

// Type aliases for common Money types

/// Money with f64 amount (default)
pub type MoneyF64 = Money<f64>;

/// Money with f32 amount
pub type MoneyF32 = Money<f32>;

/// Money with i64 amount (for integer-based calculations)
pub type MoneyI64 = Money<i64>;

/// Money with i32 amount
pub type MoneyI32 = Money<i32>;

#[cfg(feature = "decimal128")]
/// Money with Decimal amount (requires decimal128 feature)
pub type MoneyDecimal = Money<Decimal>;

// Default type alias for backwards compatibility
/// Default Money type using f64
pub type DefaultMoney = MoneyF64;

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    #[cfg(feature = "std")]
    use std::format;

    #[test]
    fn test_money_creation() {
        let usd_money = Money::new(100.0, Currency::USD);
        assert_eq!(*usd_money.amount(), 100.0);
        assert_eq!(usd_money.currency(), Currency::USD);
    }

    #[test]
    fn test_money_convenience_constructors() {
        let usd_money = Money::usd(50.0);
        assert_eq!(*usd_money.amount(), 50.0);
        assert_eq!(usd_money.currency(), Currency::USD);

        let eur_money = Money::eur(75.0);
        assert_eq!(*eur_money.amount(), 75.0);
        assert_eq!(eur_money.currency(), Currency::EUR);
    }

    #[test]
    fn test_money_addition_same_currency() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let usd_50 = Money::new(50.0, Currency::USD);
        let total = usd_100 + usd_50;

        assert_eq!(*total.amount(), 150.0);
        assert_eq!(total.currency(), Currency::USD);
    }

    #[test]
    fn test_money_subtraction_same_currency() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let usd_30 = Money::new(30.0, Currency::USD);
        let result = usd_100 - usd_30;

        assert_eq!(*result.amount(), 70.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_money_multiplication() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let result = usd_100 * 2.5;

        assert_eq!(*result.amount(), 250.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_money_division() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let result = usd_100 / 4.0;

        assert_eq!(*result.amount(), 25.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    #[should_panic(expected = "Cannot add money with different currencies")]
    fn test_money_addition_different_currency_panics() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let eur_50 = Money::new(50.0, Currency::EUR);
        let _result = usd_100 + eur_50; // Should panic
    }

    #[test]
    #[should_panic(expected = "Cannot subtract money with different currencies")]
    fn test_money_subtraction_different_currency_panics() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let eur_30 = Money::new(30.0, Currency::EUR);
        let _result = usd_100 - eur_30; // Should panic
    }

    #[test]
    fn test_money_size() {
        // Money should be relatively compact
        assert!(mem::size_of::<Money<f64>>() <= 16);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_money_display() {
        let usd_money = Money::new(42.50, Currency::USD);
        assert_eq!(format!("{}", usd_money), "42.5 USD");

        let eur_money = Money::new(100.0, Currency::EUR);
        assert_eq!(format!("{}", eur_money), "100 EUR");
    }

    #[test]
    fn test_money_into_parts() {
        let money = Money::new(123.45, Currency::GBP);
        let (amount, currency) = money.into_parts();
        assert_eq!(amount, 123.45);
        assert_eq!(currency, Currency::GBP);
    }

    #[test]
    fn test_money_into_amount() {
        let money = Money::new(99.99, Currency::JPY);
        let amount = money.into_amount();
        assert_eq!(amount, 99.99);
    }

    #[test]
    fn test_checked_add_same_currency() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let usd_50 = Money::new(50.0, Currency::USD);
        let result = usd_100.checked_add(usd_50).unwrap();

        assert_eq!(*result.amount(), 150.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_checked_add_different_currency() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let eur_50 = Money::new(50.0, Currency::EUR);
        let result = usd_100.checked_add(eur_50);

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            Error::Input(InputError::CurrencyMismatch { .. })
        );
    }

    #[test]
    fn test_checked_sub_same_currency() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let usd_30 = Money::new(30.0, Currency::USD);
        let result = usd_100.checked_sub(usd_30).unwrap();

        assert_eq!(*result.amount(), 70.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_checked_sub_different_currency() {
        let usd_100 = Money::new(100.0, Currency::USD);
        let eur_30 = Money::new(30.0, Currency::EUR);
        let result = usd_100.checked_sub(eur_30);

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            Error::Input(InputError::CurrencyMismatch { .. })
        );
    }

    #[test]
    fn test_money_with_different_numeric_types() {
        let f32_money = Money::<f32>::new(100.5, Currency::USD);
        assert_eq!(*f32_money.amount(), 100.5f32);

        let i64_money = Money::<i64>::new(1000, Currency::EUR);
        assert_eq!(*i64_money.amount(), 1000i64);
    }

    #[test]
    fn test_type_aliases() {
        let _f64_money: MoneyF64 = Money::new(100.0, Currency::USD);
        let _f32_money: MoneyF32 = Money::new(100.0f32, Currency::USD);
        let _i64_money: MoneyI64 = Money::new(100i64, Currency::USD);
        let _i32_money: MoneyI32 = Money::new(100i32, Currency::USD);
        let _default_money: DefaultMoney = Money::new(100.0, Currency::USD);
    }

    #[cfg(feature = "decimal128")]
    #[test]
    fn test_money_decimal() {
        use rust_decimal_macros::dec;
        let decimal_money: MoneyDecimal = Money::new(dec!(123.45), Currency::USD);
        assert_eq!(*decimal_money.amount(), dec!(123.45));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_money_serde() {
        let money = Money::new(123.45, Currency::USD);
        let serialized = serde_json::to_string(&money).unwrap();
        let deserialized: Money<f64> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(money, deserialized);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_money_performance() {
        // Simple performance test to ensure addition is fast
        let usd_100 = Money::new(100.0, Currency::USD);
        let usd_50 = Money::new(50.0, Currency::USD);

        // Perform many additions
        let start = std::time::Instant::now();
        let mut total = usd_100;
        for _ in 0..1000 {
            total = total + usd_50;
        }
        let duration = start.elapsed();

        // Each addition should be very fast (this is a basic smoke test)
        assert!(duration.as_nanos() < 1_000_000); // Less than 1ms for 1000 operations
        assert_eq!(*total.amount(), 100.0 + 50.0 * 1000.0);
    }
}
