//! Type-safe money amounts.
//!
//! Combines a numeric `amount` with a [`Currency`] code.
//! Prevents mixing currencies; mismatches return `Error::CurrencyMismatch`.
//!
//! Example: `let total = rfin_core::Money::usd(19.99);`

use super::currency::Currency;
use crate::error::Error;
use core::fmt;
use core::ops::{Div, Mul};
use num_traits::Num;

#[cfg(feature = "decimal128")]
use rust_decimal::Decimal;

/// Numeric type accepted by [`Money`].
///
/// Must implement `Num + Copy + PartialEq + Display`.
/// Auto-implemented for standard ints/floats and `rust_decimal::Decimal` (feature `decimal128`).
///
/// Example: `let cash: rfin_core::Money<f64> = rfin_core::Money::usd(1.0);`
pub trait MoneyAmount: Num + Copy + PartialEq + fmt::Display {}

impl<T> MoneyAmount for T where T: Num + Copy + PartialEq + fmt::Display {}

/// Monetary amount tagged with a [`Currency`].
///
/// Guarantees arithmetic uses a single currency; mismatches yield `Error::CurrencyMismatch`.
/// `F` is the numeric type (`f64` by default).
///
/// Example: `let price = rfin_core::Money::from_parts(100.0, rfin_core::Currency::USD);`
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Money<F: MoneyAmount = f64> {
    amount: F,
    currency: Currency,
}

impl<F: MoneyAmount> Money<F> {
    /// Creates a new `Money` value from an amount and currency.
    ///
    /// This is a const function, allowing usage in const contexts.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::{Money, Currency};
    ///
    /// const PRICE: Money = Money::from_parts(99.99, Currency::USD);
    /// let dynamic_price = Money::from_parts(49.99, Currency::EUR);
    /// ```
    #[inline]
    pub const fn from_parts(amount: F, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// Returns a reference to the amount of this money value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// let money = Money::usd(42.50);
    /// assert_eq!(*money.amount(), 42.50);
    /// ```
    #[inline]
    pub const fn amount(&self) -> &F {
        &self.amount
    }

    /// Returns the currency of this money value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::{Money, Currency};
    ///
    /// let money = Money::eur(100.0);
    /// assert_eq!(money.currency(), Currency::EUR);
    /// ```
    #[inline]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Consumes the `Money` and returns the inner amount.
    ///
    /// This is useful when you need to extract the numeric value
    /// and no longer need the currency information.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// let money = Money::usd(99.99);
    /// let amount: f64 = money.into_amount();
    /// assert_eq!(amount, 99.99);
    /// ```
    #[inline]
    pub fn into_amount(self) -> F {
        self.amount
    }

    /// Decomposes the `Money` into a tuple of (amount, currency).
    ///
    /// This consumes the `Money` value and returns both components.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::{Money, Currency};
    ///
    /// let money = Money::usd(75.25);
    /// let (amount, currency) = money.into_parts();
    /// assert_eq!(amount, 75.25);
    /// assert_eq!(currency, Currency::USD);
    /// ```
    #[inline]
    pub fn into_parts(self) -> (F, Currency) {
        (self.amount, self.currency)
    }

    /// Adds two `Money` values, returning an error if currencies don't match.
    ///
    /// This method performs checked addition, ensuring that only money values
    /// with the same currency can be added together. This prevents accidental
    /// mixing of currencies in financial calculations.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CurrencyMismatch`] if the currencies differ.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::{Money, Currency, Error};
    ///
    /// // Same currency - succeeds
    /// let a = Money::usd(100.0);
    /// let b = Money::usd(50.0);
    /// let sum = a.checked_add(b).unwrap();
    /// assert_eq!(*sum.amount(), 150.0);
    ///
    /// // Different currencies - fails
    /// let usd = Money::usd(100.0);
    /// let eur = Money::eur(50.0);
    /// assert!(matches!(
    ///     usd.checked_add(eur),
    ///     Err(Error::CurrencyMismatch { .. })
    /// ));
    /// ```
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        if self.currency != rhs.currency {
            return Err(Error::CurrencyMismatch {
                expected: self.currency,
                actual: rhs.currency,
            });
        }
        Ok(Self {
            amount: self.amount + rhs.amount,
            currency: self.currency,
        })
    }

    /// Subtracts two `Money` values, returning an error if currencies don't match.
    ///
    /// This method performs checked subtraction, ensuring that only money values
    /// with the same currency can be subtracted. The second value is subtracted
    /// from the first.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CurrencyMismatch`] if the currencies differ.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::{Money, Currency, Error};
    ///
    /// // Same currency - succeeds
    /// let total = Money::usd(100.0);
    /// let payment = Money::usd(30.0);
    /// let remaining = total.checked_sub(payment).unwrap();
    /// assert_eq!(*remaining.amount(), 70.0);
    ///
    /// // Different currencies - fails
    /// let usd = Money::usd(100.0);
    /// let eur = Money::eur(30.0);
    /// assert!(matches!(
    ///     usd.checked_sub(eur),
    ///     Err(Error::CurrencyMismatch { .. })
    /// ));
    /// ```
    ///
    /// # Note on Negative Values
    ///
    /// This method allows the result to be negative, which can represent
    /// debts or deficits in financial calculations:
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// let balance = Money::usd(50.0);
    /// let charge = Money::usd(75.0);
    /// let overdraft = balance.checked_sub(charge).unwrap();
    /// assert_eq!(*overdraft.amount(), -25.0);
    /// ```
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        if self.currency != rhs.currency {
            return Err(Error::CurrencyMismatch {
                expected: self.currency,
                actual: rhs.currency,
            });
        }
        Ok(Self {
            amount: self.amount - rhs.amount,
            currency: self.currency,
        })
    }
}

/// Convenience constructors for commonly used currencies.
///
/// These methods provide shortcuts for creating `Money` values in
/// frequently used currencies without explicitly specifying the currency.
impl<F: MoneyAmount> Money<F> {
    /// Creates a new `Money` value in US Dollars (USD).
    ///
    /// This is a convenience method equivalent to:
    /// `Money::from_parts(amount, Currency::USD)`
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// let price = Money::usd(19.99);
    /// let cents = Money::<i64>::usd(1999);  // Using cents as integers
    /// ```
    #[inline]
    pub const fn usd(amount: F) -> Self {
        Self { amount, currency: Currency::USD }
    }

    /// Creates a new `Money` value in Euros (EUR).
    ///
    /// This is a convenience method equivalent to:
    /// `Money::from_parts(amount, Currency::EUR)`
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// let price = Money::eur(49.99);
    /// let cents = Money::<i64>::eur(4999);  // Using cents as integers
    /// ```
    #[inline]
    pub const fn eur(amount: F) -> Self {
        Self { amount, currency: Currency::EUR }
    }
}

/// Formats `Money` for display as "amount currency".
///
/// The format shows the numeric amount followed by the three-letter
/// currency code, separated by a space.
///
/// # Examples
///
/// ```
/// use rfin_core::Money;
///
/// let money = Money::usd(1234.56);
/// assert_eq!(format!("{}", money), "1234.56 USD");
///
/// let euros = Money::eur(99.0);
/// assert_eq!(format!("{}", euros), "99 EUR");
///
/// // Works with different numeric types
/// let cents = Money::<i64>::usd(9999);
/// assert_eq!(format!("{}", cents), "9999 USD");
/// ```
impl<F: MoneyAmount> fmt::Display for Money<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

// Arithmetic operations with scalar values (currency preserved)

/// Multiplication of `Money` by a scalar value.
///
/// Multiplying money by a scalar is useful for:
/// - Calculating multiple quantities (e.g., price × quantity)
/// - Applying percentage increases (e.g., amount × 1.1 for 10% increase)
/// - Scaling financial values
///
/// The currency is preserved in the result.
impl<F: MoneyAmount> Mul<F> for Money<F> {
    type Output = Self;

    /// Multiplies the money amount by a scalar value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// let unit_price = Money::usd(25.00);
    /// let total = unit_price * 4.0;  // 4 items
    /// assert_eq!(*total.amount(), 100.00);
    ///
    /// // Applying a percentage increase
    /// let base = Money::eur(100.00);
    /// let with_tax = base * 1.21;  // 21% tax
    /// assert_eq!(*with_tax.amount(), 121.00);
    /// ```
    #[inline]
    fn mul(self, rhs: F) -> Self::Output {
        Self {
            amount: self.amount * rhs,
            currency: self.currency,
        }
    }
}

/// Division of `Money` by a scalar value.
///
/// Dividing money by a scalar is useful for:
/// - Splitting amounts equally (e.g., bill ÷ number of people)
/// - Calculating unit prices (e.g., total ÷ quantity)
/// - Applying percentage discounts (e.g., amount ÷ 1.1 to remove 10% markup)
///
/// The currency is preserved in the result.
impl<F: MoneyAmount> Div<F> for Money<F> {
    type Output = Self;

    /// Divides the money amount by a scalar value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rfin_core::Money;
    ///
    /// // Splitting a bill
    /// let total_bill = Money::usd(120.00);
    /// let per_person = total_bill / 4.0;
    /// assert_eq!(*per_person.amount(), 30.00);
    ///
    /// // Calculating unit price
    /// let bulk_price = Money::eur(250.00);
    /// let unit_price = bulk_price / 10.0;
    /// assert_eq!(*unit_price.amount(), 25.00);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `rhs` is zero (same behavior as the underlying numeric type).
    #[inline]
    fn div(self, rhs: F) -> Self::Output {
        Self {
            amount: self.amount / rhs,
            currency: self.currency,
        }
    }
}

// Type aliases for common Money types

/// Money with `f64` amount - the default and recommended type for most calculations.
///
/// # Characteristics
/// - **Precision**: ~15-17 significant decimal digits
/// - **Range**: ±1.8 × 10^308
/// - **Use cases**: General financial calculations, prices, exchange rates
///
/// # Example
/// ```
/// use rfin_core::MoneyF64;
/// let price: MoneyF64 = MoneyF64::usd(99.99);
/// ```
pub type MoneyF64 = Money<f64>;

/// Money with `f32` amount - reduced precision but smaller memory footprint.
///
/// # Characteristics
/// - **Precision**: ~6-9 significant decimal digits
/// - **Range**: ±3.4 × 10^38
/// - **Size**: 8 bytes total (vs 16 bytes for MoneyF64)
/// - **Use cases**: Large datasets where precision isn't critical
///
/// # Example
/// ```
/// use rfin_core::MoneyF32;
/// let price: MoneyF32 = MoneyF32::usd(99.99);
/// ```
pub type MoneyF32 = Money<f32>;

/// Money with `i64` amount - for exact integer calculations.
///
/// # Characteristics
/// - **Precision**: Exact integer arithmetic
/// - **Range**: ±9,223,372,036,854,775,807
/// - **Use cases**: Representing amounts in smallest currency units (e.g., cents)
///
/// # Example
/// ```
/// use rfin_core::MoneyI64;
/// // $99.99 represented as 9999 cents
/// let price_cents: MoneyI64 = MoneyI64::usd(9999);
/// ```
pub type MoneyI64 = Money<i64>;

/// Money with `i32` amount - for exact integer calculations with smaller range.
///
/// # Characteristics
/// - **Precision**: Exact integer arithmetic
/// - **Range**: ±2,147,483,647
/// - **Size**: 8 bytes total (vs 16 bytes for MoneyI64)
/// - **Use cases**: Representing amounts in cents when range is sufficient
///
/// # Example
/// ```
/// use rfin_core::MoneyI32;
/// // $99.99 represented as 9999 cents
/// let price_cents: MoneyI32 = MoneyI32::usd(9999);
/// ```
pub type MoneyI32 = Money<i32>;

/// Money backed by `rust_decimal::Decimal` for arbitrary precision decimal arithmetic.
///
/// Requires the `decimal128` feature to be enabled.
///
/// # Characteristics
/// - **Precision**: 28-29 significant decimal digits
/// - **Range**: ±7.9 × 10^28
/// - **Use cases**: Financial calculations requiring exact decimal representation
///
/// # Example
/// ```ignore
/// use rfin_core::MoneyDecimal;
/// use rust_decimal_macros::dec;
/// 
/// let price: MoneyDecimal = MoneyDecimal::from_parts(dec!(99.99), Currency::USD);
/// ```
#[cfg(feature = "decimal128")]
pub type MoneyDecimal = Money<Decimal>;

/// Convenient alias for the default money type (`f64`).
///
/// This is the recommended type for most use cases, providing a good balance
/// of precision, range, and performance.
///
/// # Example
/// ```
/// use rfin_core::DefaultMoney;
/// let price: DefaultMoney = DefaultMoney::usd(99.99);
/// ```
pub type DefaultMoney = MoneyF64;

// Prevent clippy warning about items after test module.
#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use core::mem;

    #[cfg(feature = "std")]
    use std::format;

    #[test]
    fn test_money_creation() {
        let usd_money = Money::from_parts(100.0, Currency::USD);
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
        let usd_100 = Money::usd(100.0);
        let usd_50 = Money::usd(50.0);
        let total = usd_100.checked_add(usd_50).unwrap();

        assert_eq!(*total.amount(), 150.0);
        assert_eq!(total.currency(), Currency::USD);
    }

    #[test]
    fn test_money_subtraction_same_currency() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let usd_30 = Money::from_parts(30.0, Currency::USD);
        let result = usd_100.checked_sub(usd_30).unwrap();

        assert_eq!(*result.amount(), 70.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_money_multiplication() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let result = usd_100 * 2.5;

        assert_eq!(*result.amount(), 250.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_money_division() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let result = usd_100 / 4.0;

        assert_eq!(*result.amount(), 25.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_money_addition_different_currency_error() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let eur_50 = Money::from_parts(50.0, Currency::EUR);
        let result = usd_100.checked_add(eur_50);

        assert!(result.is_err());
        matches!(result.unwrap_err(), Error::CurrencyMismatch { .. });
    }

    #[test]
    fn test_money_subtraction_different_currency_error() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let eur_30 = Money::from_parts(30.0, Currency::EUR);
        let result = usd_100.checked_sub(eur_30);

        assert!(result.is_err());
        matches!(result.unwrap_err(), Error::CurrencyMismatch { .. });
    }

    #[test]
    fn test_money_size() {
        assert!(mem::size_of::<Money<f64>>() <= 16);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_money_display() {
        let usd_money = Money::from_parts(42.50, Currency::USD);
        assert_eq!(format!("{}", usd_money), "42.5 USD");

        let eur_money = Money::from_parts(100.0, Currency::EUR);
        assert_eq!(format!("{}", eur_money), "100 EUR");
    }

    #[test]
    fn test_money_into_parts() {
        let money = Money::from_parts(123.45, Currency::GBP);
        let (amount, currency) = money.into_parts();
        assert_eq!(amount, 123.45);
        assert_eq!(currency, Currency::GBP);
    }

    #[test]
    fn test_money_into_amount() {
        let money = Money::from_parts(99.99, Currency::JPY);
        let amount = money.into_amount();
        assert_eq!(amount, 99.99);
    }

    #[test]
    fn test_checked_add_same_currency() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let usd_50 = Money::from_parts(50.0, Currency::USD);
        let result = usd_100.checked_add(usd_50).unwrap();

        assert_eq!(*result.amount(), 150.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_checked_add_different_currency() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let eur_50 = Money::from_parts(50.0, Currency::EUR);
        let result = usd_100.checked_add(eur_50);

        assert!(result.is_err());
        matches!(result.unwrap_err(), Error::CurrencyMismatch { .. });
    }

    #[test]
    fn test_checked_sub_same_currency() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let usd_30 = Money::from_parts(30.0, Currency::USD);
        let result = usd_100.checked_sub(usd_30).unwrap();

        assert_eq!(*result.amount(), 70.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_checked_sub_different_currency() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let eur_30 = Money::from_parts(30.0, Currency::EUR);
        let result = usd_100.checked_sub(eur_30);

        assert!(result.is_err());
        matches!(result.unwrap_err(), Error::CurrencyMismatch { .. });
    }

    #[test]
    fn test_money_with_different_numeric_types() {
        let f32_money = Money::<f32>::from_parts(100.5, Currency::USD);
        assert_eq!(*f32_money.amount(), 100.5f32);

        let i64_money = Money::<i64>::from_parts(1000, Currency::EUR);
        assert_eq!(*i64_money.amount(), 1000i64);
    }

    #[test]
    fn test_type_aliases() {
        let _f64_money: MoneyF64 = Money::from_parts(100.0, Currency::USD);
        let _f32_money: MoneyF32 = Money::from_parts(100.0f32, Currency::USD);
        let _i64_money: MoneyI64 = Money::from_parts(100i64, Currency::USD);
        let _i32_money: MoneyI32 = Money::from_parts(100i32, Currency::USD);
        let _default_money: DefaultMoney = Money::from_parts(100.0, Currency::USD);
    }

    #[cfg(feature = "decimal128")]
    #[test]
    fn test_money_decimal() {
        use rust_decimal_macros::dec;
        let decimal_money: MoneyDecimal = Money::from_parts(dec!(123.45), Currency::USD);
        assert_eq!(*decimal_money.amount(), dec!(123.45));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_money_serde() {
        let money = Money::from_parts(123.45, Currency::USD);
        let serialized = serde_json::to_string(&money).unwrap();
        let deserialized: Money<f64> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(money, deserialized);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_money_performance() {
        let usd_100 = Money::from_parts(100.0, Currency::USD);
        let usd_50 = Money::from_parts(50.0, Currency::USD);

        let start = std::time::Instant::now();
        let mut total = usd_100;
        for _ in 0..1000 {
            total = total.checked_add(usd_50).unwrap();
        }
        let duration = start.elapsed();

        assert!(duration.as_nanos() < 1_000_000);
        assert_eq!(*total.amount(), 100.0 + 50.0 * 1000.0);
    }
}

//--- Trait conversions ------------------------------------------------------

/// Converts a tuple of (amount, currency) into a `Money` value.
///
/// This provides a convenient way to create `Money` from tuples,
/// especially useful when working with functions that return tuples.
///
/// # Examples
///
/// ```
/// use rfin_core::{Money, Currency};
///
/// // Direct conversion
/// let money: Money = (100.0, Currency::USD).into();
/// assert_eq!(*money.amount(), 100.0);
/// assert_eq!(money.currency(), Currency::USD);
///
/// // From function returning tuple
/// fn calculate_price() -> (f64, Currency) {
///     (99.99, Currency::EUR)
/// }
/// let price: Money = calculate_price().into();
/// ```
impl<F: MoneyAmount> From<(F, Currency)> for Money<F> {
    #[inline]
    fn from(value: (F, Currency)) -> Self {
        Money {
            amount: value.0,
            currency: value.1,
        }
    }
} 