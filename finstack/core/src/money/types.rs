//! Money type, conversions, formatting, and arithmetic operations.
//!
//! [`Money`] stores amounts as scaled integers to avoid cumulative rounding
//! error while retaining ergonomic APIs for arithmetic and formatting.
//! Instances retain their [`Currency`] tag and refuse to mix currencies unless
//! explicitly converted via [`super::fx::FxProvider`].
//!
//! Note: Formatting is intentionally non-locale. Separators are ASCII and
//! currency code precedes the amount (e.g., "USD 1,234.56"). Use
//! [`Money::format_with_config`] or wrap at the UI layer if locale-aware
//! presentation is required; the numeric representation remains deterministic
//! and stable for pipelines.
//!
//! # Examples
//! ```rust
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//!
//! let amt = Money::new(12.349, Currency::USD);
//! assert_eq!(amt.currency(), Currency::USD);
//! assert_eq!(format!("{}", amt), "USD 12.35");
//! ```

use crate::config::{FinstackConfig, RoundingMode};
use crate::currency::Currency;
use crate::dates::Date;
use crate::error::Error;
use core::fmt;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use super::rounding::{
    amount_from_repr, repr_add, repr_div_f64, repr_mul_f64, repr_sub, round_f64, AmountRepr,
};

/// Helper function to format integers with thousands separators.
fn format_with_separators(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, *c);
    }

    if n < 0 {
        result.insert(0, '-');
    }

    result
}

/// Currency-tagged monetary amount with safe arithmetic.
///
/// Values are stored using a fixed-point representation derived from ISO 4217
/// decimal places. Use [`Money::new_with_config`] when you need configurable
/// rounding during ingestion.
///
/// # Examples
/// ```rust
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
///
/// let notional = Money::new(1_000_000.0, Currency::EUR);
/// assert_eq!(notional.currency(), Currency::EUR);
/// assert_eq!(notional.amount(), 1_000_000.0);
/// ```
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

    /// Format the amount with custom decimals and optional currency symbol.
    ///
    /// # Arguments
    ///
    /// * `decimals` - Number of decimal places to display
    /// * `show_currency` - Whether to include currency code
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let amount = Money::new(1_042_315.67, Currency::USD);
    /// assert_eq!(amount.format(2, true), "1042315.67 USD");
    /// assert_eq!(amount.format(2, false), "1042315.67");
    /// assert_eq!(amount.format(0, true), "1042316 USD");
    /// ```
    pub fn format(&self, decimals: usize, show_currency: bool) -> String {
        use super::rounding::round_decimal;
        let rounded = round_decimal(
            self.amount,
            decimals as i32,
            crate::config::RoundingMode::Bankers,
        );
        let value = format!("{:.prec$}", rounded, prec = decimals);
        if show_currency {
            format!("{} {}", value, self.currency())
        } else {
            value
        }
    }

    /// Format with thousands separators and currency.
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let amount = Money::new(1_042_315.67, Currency::USD);
    /// let formatted = amount.format_with_separators(2);
    /// // Exact format may vary by locale, but should include currency
    /// assert!(formatted.contains("USD"));
    /// ```
    pub fn format_with_separators(&self, decimals: usize) -> String {
        use super::rounding::round_decimal;
        let rounded = round_decimal(
            self.amount,
            decimals as i32,
            crate::config::RoundingMode::Bankers,
        );
        let amt = amount_from_repr(rounded);
        let int_part = amt.trunc() as i64;
        let frac_part = ((amt - amt.trunc()) * 10_f64.powi(decimals as i32)).round() as i64;

        // Format integer part with thousands separators
        let int_str = format_with_separators(int_part);

        if decimals > 0 {
            format!(
                "{}.{:0width$} {}",
                int_str,
                frac_part,
                self.currency(),
                width = decimals
            )
        } else {
            format!("{} {}", int_str, self.currency())
        }
    }

    /// Create a new [`Money`] value using ISO 4217 minor units.
    ///
    /// # Parameters
    /// - `amount`: monetary amount expressed as an `f64`
    /// - `currency`: target [`Currency`]
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let amt = Money::new(10.005, Currency::USD);
    /// assert_eq!(format!("{}", amt), "USD 10.01");
    /// ```
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

    /// Create a new [`Money`] value using an explicit configuration.
    ///
    /// # Parameters
    /// - `amount`: monetary amount expressed as an `f64`
    /// - `currency`: target [`Currency`]
    /// - `cfg`: rounding configuration to apply during ingestion
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::config::FinstackConfig;
    ///
    /// let mut cfg = FinstackConfig::default();
    /// cfg.rounding
    ///     .ingest_scale
    ///     .overrides
    ///     .insert(Currency::USD, 3);
    /// let amt = Money::new_with_config(1.2345, Currency::USD, &cfg);
    /// assert!((amt.amount() - 1.234).abs() < 1e-9);
    /// ```
    #[must_use]
    #[inline]
    pub fn new_with_config(amount: f64, currency: Currency, cfg: &FinstackConfig) -> Self {
        let dp = cfg.ingest_scale(currency);
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

    /// Add two amounts, returning an error when currencies do not match.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let lhs = Money::new(50.0, Currency::USD);
    /// let rhs = Money::new(25.0, Currency::USD);
    /// let sum = lhs.checked_add(rhs).expect("Currency match should succeed");
    /// assert_eq!(sum.amount(), 75.0);
    /// ```
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Result<Self, Error> {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self {
            amount: repr_add(self.amount, rhs.amount),
            currency: self.currency,
        })
    }

    /// Subtract two amounts, returning an error when currencies do not match.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let lhs = Money::new(50.0, Currency::USD);
    /// let rhs = Money::new(25.0, Currency::USD);
    /// let diff = lhs.checked_sub(rhs).expect("Currency match should succeed");
    /// assert_eq!(diff.amount(), 25.0);
    /// ```
    #[must_use = "returns new Money if currencies match"]
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, Error> {
        ensure_same_currency(&self, &rhs)?;
        Ok(Self {
            amount: repr_sub(self.amount, rhs.amount),
            currency: self.currency,
        })
    }

    /// Convert this [`Money`] into another currency using an [`fx::FxProvider`].
    ///
    /// # Parameters
    /// - `to`: target [`Currency`]
    /// - `on`: valuation date used for the FX lookup
    /// - `provider`: FX source implementing [`fx::FxProvider`]
    /// - `policy`: lookup policy hint passed to the provider
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::{FxConversionPolicy, FxProvider};
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// struct StaticFx;
    /// impl FxProvider for StaticFx {
    ///     fn rate(
    ///         &self,
    ///         _from: Currency,
    ///         _to: Currency,
    ///         _on: Date,
    ///         _policy: FxConversionPolicy,
    ///     ) -> finstack_core::Result<f64> {
    ///         Ok(1.2)
    ///     }
    /// }
    ///
    /// let eur = Money::new(100.0, Currency::EUR);
    /// let trade_date = Date::from_calendar_date(2024, Month::January, 2).expect("Valid date");
    /// let usd = eur.convert(
    ///     Currency::USD,
    ///     trade_date,
    ///     &StaticFx,
    ///     FxConversionPolicy::CashflowDate,
    /// ).expect("Currency conversion should succeed");
    /// assert_eq!(usd.amount(), 120.0);
    /// assert_eq!(usd.currency(), Currency::USD);
    /// ```
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
        // Format with currency-specific minor units using Decimal precision
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
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::config::FinstackConfig;
    ///
    /// let amt = Money::new(10.0, Currency::USD);
    /// let mut cfg = FinstackConfig::default();
    /// cfg.rounding
    ///     .output_scale
    ///     .overrides
    ///     .insert(Currency::USD, 4);
    /// assert_eq!(amt.format_with_config(&cfg), "USD 10.0000");
    /// ```
    pub fn format_with_config(&self, cfg: &FinstackConfig) -> String {
        use super::rounding::round_decimal;
        let dp = cfg.output_scale(self.currency) as usize;
        let rounded = round_decimal(self.amount, dp as i32, cfg.rounding.mode);
        format!("{} {val:.prec$}", self.currency, val = rounded, prec = dp)
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
        $crate::money::Money::new($amount, $crate::currency::Currency::$code)
    };
}

// -------------------------------------------------------------------------
// Unchecked arithmetic (default) – currency must match (debug_assert)
// -------------------------------------------------------------------------

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        ensure_same_currency(self, &rhs)
            .expect("Currency mismatch in AddAssign - currencies must match");
        self.amount = repr_add(self.amount, rhs.amount);
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        ensure_same_currency(self, &rhs)
            .expect("Currency mismatch in SubAssign - currencies must match");
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
        let c = (a + b).expect("Currency match should succeed in test");
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
