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
//! let amt = Money::new(100.0, Currency::USD);
//! assert_eq!(amt.currency(), Currency::USD);
//! assert_eq!(format!("{}", amt), "USD 100.00");
//! ```

use crate::config::{FinstackConfig, RoundingMode};
use crate::currency::Currency;
use crate::dates::Date;
use crate::error::{Error, InputError, NonFiniteKind};
use core::fmt;
use core::ops::{AddAssign, Div, DivAssign, Mul, MulAssign, SubAssign};

use super::rounding::{
    amount_from_repr, repr_add, repr_div_f64, repr_mul_f64, repr_sub, round_f64,
    try_amount_from_repr, AmountRepr,
};

/// Helper function to format an integer string (optionally prefixed by `-`) with thousands separators.
fn format_with_separators(int_str: &str) -> String {
    let (is_neg, digits) = match int_str.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, int_str),
    };

    let bytes = digits.as_bytes();
    let mut rev: Vec<u8> = Vec::with_capacity(bytes.len() + bytes.len() / 3 + 1);
    for (i, &b) in bytes.iter().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            rev.push(b',');
        }
        rev.push(b);
    }
    rev.reverse();

    let mut out = String::with_capacity(rev.len() + usize::from(is_neg));
    if is_neg {
        out.push('-');
    }
    for b in rev {
        // We only ever push ASCII digits and commas.
        out.push(b as char);
    }
    out
}

/// Currency-tagged monetary amount with safe arithmetic.
///
/// Values are stored using a fixed-point representation derived from ISO 4217
/// decimal places.
///
/// When you need configurable rounding during ingestion, use
/// [`Money::new_with_config`].
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
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Uses Bankers rounding (IEEE 754 round-half-to-even). For other rounding
    /// modes, use [`format_with_config`].
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
    /// assert_eq!(amount.format(2, true), "USD 1042315.67");
    /// assert_eq!(amount.format(2, false), "1042315.67");
    /// assert_eq!(amount.format(0, true), "USD 1042316");
    /// ```
    pub fn format(&self, decimals: usize, show_currency: bool) -> String {
        use super::rounding::round_decimal;
        let rounded = round_decimal(
            self.amount,
            decimals as i32,
            crate::config::RoundingMode::Bankers,
        );
        let value = format!("{val:.prec$}", val = rounded, prec = decimals);
        if show_currency {
            format!("{} {}", self.currency(), value)
        } else {
            value
        }
    }

    /// Format with thousands separators and currency.
    ///
    /// Uses Bankers rounding. For custom rounding, use [`format_with_config`].
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let amount = Money::new(1_042_315.67, Currency::USD);
    /// let formatted = amount.format_with_separators(2);
    /// assert_eq!(formatted, "USD 1,042,315.67");
    /// ```
    pub fn format_with_separators(&self, decimals: usize) -> String {
        use super::rounding::round_decimal;
        let rounded = round_decimal(
            self.amount,
            decimals as i32,
            crate::config::RoundingMode::Bankers,
        );
        let s = format!("{val:.prec$}", val = rounded, prec = decimals);
        let (int_part, frac_part) = match s.split_once('.') {
            Some((i, f)) => (i, Some(f)),
            None => (s.as_str(), None),
        };
        let int_fmt = format_with_separators(int_part);
        let value = if let Some(frac) = frac_part {
            format!("{int_fmt}.{frac}")
        } else {
            int_fmt
        };
        format!("{} {}", self.currency(), value)
    }

    /// Create a new [`Money`] value using ISO-4217 minor units and bankers rounding.
    ///
    /// # Panics
    ///
    /// Panics if `amount` is not finite (NaN or infinity). Use [`Money::try_new`]
    /// for a fallible constructor.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let amt = Money::new(10.0, Currency::USD);
    /// assert_eq!(format!("{}", amt), "USD 10.00");
    /// ```
    #[inline]
    pub fn new(amount: f64, currency: Currency) -> Self {
        Self::new_impl(amount, currency, None, "Money::new")
    }

    /// Create a new [`Money`] value using an explicit configuration for rounding.
    ///
    /// # Panics
    ///
    /// Panics if `amount` is not finite (NaN or infinity). Use
    /// [`Money::try_new_with_config`] for a fallible constructor.
    pub fn new_with_config(amount: f64, currency: Currency, cfg: &FinstackConfig) -> Self {
        Self::new_impl(amount, currency, Some(cfg), "Money::new_with_config")
    }

    /// Fallible constructor using ISO-4217 minor units and bankers rounding.
    pub fn try_new(amount: f64, currency: Currency) -> Result<Self, Error> {
        Self::try_new_impl(amount, currency, None)
    }

    /// Fallible constructor using an explicit configuration for rounding.
    pub fn try_new_with_config(
        amount: f64,
        currency: Currency,
        cfg: &FinstackConfig,
    ) -> Result<Self, Error> {
        Self::try_new_impl(amount, currency, Some(cfg))
    }

    #[inline]
    fn ingest_rounding_params(
        currency: Currency,
        cfg: Option<&FinstackConfig>,
    ) -> (u32, RoundingMode) {
        match cfg {
            Some(cfg) => (cfg.ingest_scale(currency), cfg.rounding.mode),
            None => (u32::from(currency.decimals()), RoundingMode::Bankers),
        }
    }

    #[inline]
    fn new_finite(amount: f64, currency: Currency, cfg: Option<&FinstackConfig>) -> Self {
        let rounded = if let Some(cfg) = cfg {
            let (dp, mode) = Self::ingest_rounding_params(currency, Some(cfg));
            round_f64(amount, dp as i32, mode)
        } else {
            AmountRepr::from_f64_retain(amount)
                .unwrap_or_else(|| unreachable!("finite amount should convert to Decimal"))
        };
        Self {
            amount: rounded,
            currency,
        }
    }

    #[inline]
    fn try_new_impl(
        amount: f64,
        currency: Currency,
        cfg: Option<&FinstackConfig>,
    ) -> Result<Self, Error> {
        if !amount.is_finite() {
            let kind = if amount.is_nan() {
                NonFiniteKind::NaN
            } else if amount.is_sign_positive() {
                NonFiniteKind::PosInfinity
            } else {
                NonFiniteKind::NegInfinity
            };
            return Err(Error::Input(InputError::NonFiniteValue { kind }));
        }
        Ok(Self::new_finite(amount, currency, cfg))
    }

    #[inline]
    fn new_impl(
        amount: f64,
        currency: Currency,
        cfg: Option<&FinstackConfig>,
        caller: &'static str,
    ) -> Self {
        assert!(
            amount.is_finite(),
            "{caller} requires finite amount (got {:?})",
            amount
        );
        Self::new_finite(amount, currency, cfg)
    }

    #[inline]
    fn amount_and_currency(self) -> (f64, Currency) {
        (amount_from_repr(self.amount), self.currency)
    }

    #[inline]
    fn try_amount_and_currency(self) -> Result<(f64, Currency), Error> {
        Ok((try_amount_from_repr(self.amount)?, self.currency))
    }

    /// Amount accessor (by value).
    #[inline]
    pub fn amount(&self) -> f64 {
        (*self).into_amount()
    }

    /// Currency accessor.
    #[inline]
    pub const fn currency(&self) -> Currency {
        self.currency
    }

    /// Consume `self` and return just the numeric amount.
    #[inline]
    #[must_use]
    pub fn into_amount(self) -> f64 {
        self.into_parts().0
    }

    /// Consume `self` into `(amount, currency)`.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (f64, Currency) {
        self.amount_and_currency()
    }

    // ---------------------------------------------------------------------
    // Fallible accessors
    // ---------------------------------------------------------------------

    /// Fallible amount accessor.
    ///
    /// Returns `Err(ConversionOverflow)` if the internal Decimal representation
    /// cannot be converted to f64. Use this at API boundaries when explicit
    /// error handling is preferred over panics.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// # fn main() -> finstack_core::Result<()> {
    ///
    /// let amt = Money::new(1_000_000.0, Currency::USD);
    /// assert_eq!(amt.try_amount()?, 1_000_000.0);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn try_amount(&self) -> Result<f64, Error> {
        (*self).try_into_amount()
    }

    /// Fallible consuming amount accessor.
    ///
    /// Returns `Err(ConversionOverflow)` if the internal Decimal representation
    /// cannot be converted to f64.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// # fn main() -> finstack_core::Result<()> {
    ///
    /// let amt = Money::new(1_000_000.0, Currency::USD);
    /// assert_eq!(amt.try_into_amount()?, 1_000_000.0);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn try_into_amount(self) -> Result<f64, Error> {
        self.try_into_parts().map(|(amount, _)| amount)
    }

    /// Fallible consuming parts accessor.
    ///
    /// Returns `Err(ConversionOverflow)` if the internal Decimal representation
    /// cannot be converted to f64.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// # fn main() -> finstack_core::Result<()> {
    ///
    /// let amt = Money::new(1_000_000.0, Currency::USD);
    /// let (value, ccy) = amt.try_into_parts()?;
    /// assert_eq!(value, 1_000_000.0);
    /// assert_eq!(ccy, Currency::USD);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn try_into_parts(self) -> Result<(f64, Currency), Error> {
        self.try_amount_and_currency()
    }

    // ---------------------------------------------------------------------
    // Checked arithmetic
    // ---------------------------------------------------------------------

    /// Add two amounts, returning an error when currencies do not match.
    ///
    /// This method is semantically identical to the `+` operator, but is preferred
    /// in application code because it makes the `Result` return type explicit.
    /// The `Add` trait impl for `Money` unusually returns `Result<Money, Error>`
    /// rather than `Money`, which can surprise readers unfamiliar with the API.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let lhs = Money::new(50.0, Currency::USD);
    /// let rhs = Money::new(25.0, Currency::USD);
    ///
    /// // Preferred: explicit about Result return
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
    /// This method is semantically identical to the `-` operator, but is preferred
    /// in application code because it makes the `Result` return type explicit.
    /// The `Sub` trait impl for `Money` unusually returns `Result<Money, Error>`
    /// rather than `Money`, which can surprise readers unfamiliar with the API.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let lhs = Money::new(50.0, Currency::USD);
    /// let rhs = Money::new(25.0, Currency::USD);
    ///
    /// // Preferred: explicit about Result return
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

    /// Convert this [`Money`] into another currency using an `FxProvider`.
    ///
    /// # Parameters
    /// - `to`: target [`Currency`](crate::currency::Currency)
    /// - `on`: valuation date used for the FX lookup
    /// - `provider`: FX source implementing `FxProvider`
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
        if !rate.is_finite() || rate <= 0.0 {
            return Err(crate::error::InputError::InvalidFxRate {
                from: self.currency,
                to,
                rate,
            }
            .into());
        }
        let new_amount = super::rounding::try_repr_mul_f64(self.amount, rate)?;
        let rounded = super::rounding::round_decimal(
            new_amount,
            to.decimals() as i32,
            crate::config::RoundingMode::Bankers,
        );
        Ok(Self {
            amount: rounded,
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
// Unchecked arithmetic – currency must match or panic
// -------------------------------------------------------------------------
// NOTE: AddAssign and SubAssign require matching currencies. Currency
// mismatch will always panic regardless of build type. For explicit error
// handling, use `checked_add` and `checked_sub` which return `Result<Money, Error>`.

impl AddAssign for Money {
    /// Adds another [`Money`] value to this one in place.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` has a different currency. For fallible
    /// arithmetic, use [`Money::checked_add`] which returns `Result`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let mut total = Money::new(100.0, Currency::USD);
    /// total += Money::new(50.0, Currency::USD);
    /// assert_eq!(total.amount(), 150.0);
    /// ```
    #[track_caller]
    fn add_assign(&mut self, rhs: Self) {
        // Always fail loudly on currency mismatch; silent no-ops are correctness bugs.
        assert!(
            self.currency == rhs.currency,
            "Currency mismatch in Money::add_assign: lhs={}, rhs={}",
            self.currency,
            rhs.currency
        );
        self.amount = repr_add(self.amount, rhs.amount);
    }
}

impl SubAssign for Money {
    /// Subtracts another [`Money`] value from this one in place.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` has a different currency. For fallible
    /// arithmetic, use [`Money::checked_sub`] which returns `Result`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    ///
    /// let mut total = Money::new(100.0, Currency::USD);
    /// total -= Money::new(30.0, Currency::USD);
    /// assert_eq!(total.amount(), 70.0);
    /// ```
    #[track_caller]
    fn sub_assign(&mut self, rhs: Self) {
        // Always fail loudly on currency mismatch; silent no-ops are correctness bugs.
        assert!(
            self.currency == rhs.currency,
            "Currency mismatch in Money::sub_assign: lhs={}, rhs={}",
            self.currency,
            rhs.currency
        );
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
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
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
        let c = a
            .checked_add(b)
            .expect("Currency match should succeed in test");
        assert_eq!(c.amount(), 75.0);
    }

    #[test]
    fn currency_mismatch_error() {
        let usd = Money::new(10.0, Currency::USD);
        let eur = Money::new(10.0, Currency::EUR);
        assert!(usd.checked_add(eur).is_err());
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

    #[test]
    fn format_with_separators_handles_negative_values() {
        let m = Money::new(-1234.56, Currency::USD);
        let formatted = m.format_with_separators(2);
        assert!(
            formatted.starts_with("USD -1,234.56"),
            "formatted output should keep sign on integer part only: {}",
            formatted
        );
    }

    #[test]
    #[should_panic(expected = "finite amount")]
    fn new_rejects_non_finite_amounts() {
        let _ = Money::new(f64::NAN, Currency::USD);
    }

    #[test]
    #[should_panic(expected = "Money division requires finite, non-zero, representable scalar")]
    fn division_by_zero_panics() {
        let _ = Money::new(10.0, Currency::USD) / 0.0;
    }

    #[test]
    #[should_panic(expected = "Money multiplication requires finite, representable scalar")]
    fn multiply_by_nan_panics() {
        let _ = Money::new(10.0, Currency::USD) * f64::NAN;
    }

    struct NaNProvider;
    impl crate::money::fx::FxProvider for NaNProvider {
        fn rate(
            &self,
            _from: Currency,
            _to: Currency,
            _on: Date,
            _policy: crate::money::fx::FxConversionPolicy,
        ) -> crate::Result<f64> {
            Ok(f64::NAN)
        }
    }

    #[test]
    fn convert_rejects_non_finite_rate() {
        let usd = Money::new(5.0, Currency::USD);
        let date =
            Date::from_calendar_date(2024, time::Month::January, 1).expect("Valid test date");
        let res = usd.convert(
            Currency::EUR,
            date,
            &NaNProvider,
            crate::money::fx::FxConversionPolicy::CashflowDate,
        );
        assert!(res.is_err());
    }

    // -------------------------------------------------------------------------
    // Fallible accessor tests (try_amount, try_into_amount, try_into_parts)
    // -------------------------------------------------------------------------

    #[test]
    fn try_amount_returns_ok_for_normal_values() {
        let m = Money::new(12345.67, Currency::USD);
        let result = m.try_amount().expect("Conversion should succeed");
        assert!((result - 12345.67).abs() < 1e-10);
    }

    #[test]
    fn try_into_amount_returns_ok_for_normal_values() {
        let m = Money::new(999.99, Currency::EUR);
        let result = m.try_into_amount().expect("Conversion should succeed");
        assert!((result - 999.99).abs() < 1e-10);
    }

    #[test]
    fn try_into_parts_returns_ok_for_normal_values() {
        let m = Money::new(500.0, Currency::GBP);
        let (amount, currency) = m.try_into_parts().expect("Conversion should succeed");
        assert!((amount - 500.0).abs() < 1e-10);
        assert_eq!(currency, Currency::GBP);
    }

    #[test]
    fn amount_does_not_silently_return_zero_for_large_values() {
        // This test documents the fix: large values must NOT silently become 0.
        // Prior to the fix, conversion failure would return 0.0 silently.
        let large_amount = Money::new(1_000_000_000_000.0, Currency::USD);
        let amount = large_amount.amount();
        assert!(
            amount > 0.0,
            "Large monetary amount must not silently become zero"
        );
        assert!(
            (amount - 1_000_000_000_000.0).abs() < 1e3,
            "Amount should preserve the large value"
        );
    }

    #[test]
    fn try_amount_preserves_negative_values() {
        let m = Money::new(-1_000_000.0, Currency::JPY);
        let amount = m.try_amount().expect("Conversion should succeed");
        assert!(amount < 0.0, "Negative values must remain negative");
    }

    // -------------------------------------------------------------------------
    // Fallible constructor tests (Money::try_new / Money::try_new_with_config)
    // -------------------------------------------------------------------------

    #[test]
    fn try_new_succeeds_for_finite_values() {
        let m = Money::try_new(123.45, Currency::USD).expect("Finite value should succeed");
        assert!((m.amount() - 123.45).abs() < 1e-10);
        assert_eq!(m.currency(), Currency::USD);
    }

    #[test]
    fn try_new_returns_error_for_nan() {
        let result = Money::try_new(f64::NAN, Currency::USD);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::error::Error::Input(
                crate::error::InputError::NonFiniteValue { kind }
            )) if kind == crate::error::NonFiniteKind::NaN
        ));
    }

    #[test]
    fn try_new_returns_error_for_positive_infinity() {
        let result = Money::try_new(f64::INFINITY, Currency::EUR);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::error::Error::Input(
                crate::error::InputError::NonFiniteValue { kind }
            )) if kind == crate::error::NonFiniteKind::PosInfinity
        ));
    }

    #[test]
    fn try_new_returns_error_for_negative_infinity() {
        let result = Money::try_new(f64::NEG_INFINITY, Currency::GBP);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(crate::error::Error::Input(
                crate::error::InputError::NonFiniteValue { kind }
            )) if kind == crate::error::NonFiniteKind::NegInfinity
        ));
    }

    #[test]
    fn try_new_with_config_succeeds_for_finite_values() {
        let mut cfg = FinstackConfig::default();
        cfg.rounding.ingest_scale.overrides.insert(Currency::USD, 3);
        let m =
            Money::try_new_with_config(1.2345, Currency::USD, &cfg).expect("Finite should succeed");
        assert!((m.amount() - 1.234).abs() < 1e-9);
    }

    #[test]
    fn try_new_preserves_internal_precision_by_default() {
        let m = Money::try_new(10.005, Currency::USD).expect("Finite should succeed");
        assert!((m.amount() - 10.005).abs() < 1e-12);
    }

    #[test]
    fn try_new_with_config_returns_error_for_non_finite() {
        let cfg = FinstackConfig::default();
        let result = Money::try_new_with_config(f64::NAN, Currency::USD, &cfg);
        assert!(result.is_err());
    }

    #[test]
    fn try_new_handles_zero() {
        let m = Money::try_new(0.0, Currency::USD).expect("Zero should succeed");
        assert_eq!(m.amount(), 0.0);
    }

    #[test]
    fn try_new_handles_negative_zero() {
        let m = Money::try_new(-0.0, Currency::USD).expect("Negative zero should succeed");
        // -0.0 == 0.0 in floating point
        assert_eq!(m.amount(), 0.0);
    }

    #[test]
    fn try_new_handles_very_small_values() {
        let small = 1e-15;
        let m = Money::try_new(small, Currency::USD).expect("Small value should succeed");
        // Construction preserves the raw finite amount; formatting/rounding is a separate concern.
        assert_eq!(m.amount(), small);
    }

    #[test]
    fn try_new_handles_large_finite_values() {
        let large = 1e15;
        let m = Money::try_new(large, Currency::USD).expect("Large finite value should succeed");
        assert!(m.amount() > 0.0);
    }

    #[test]
    #[should_panic(expected = "Currency mismatch")]
    fn add_assign_panics_on_currency_mismatch() {
        let mut usd = Money::new(100.0, Currency::USD);
        let eur = Money::new(50.0, Currency::EUR);
        usd += eur;
    }

    #[test]
    #[should_panic(expected = "Currency mismatch")]
    fn sub_assign_panics_on_currency_mismatch() {
        let mut usd = Money::new(100.0, Currency::USD);
        let eur = Money::new(50.0, Currency::EUR);
        usd -= eur;
    }

    #[test]
    fn add_assign_succeeds_for_matching_currencies() {
        let mut total = Money::new(100.0, Currency::USD);
        total += Money::new(50.0, Currency::USD);
        assert_eq!(total.amount(), 150.0);
    }

    #[test]
    fn sub_assign_succeeds_for_matching_currencies() {
        let mut total = Money::new(100.0, Currency::USD);
        total -= Money::new(30.0, Currency::USD);
        assert_eq!(total.amount(), 70.0);
    }
}
