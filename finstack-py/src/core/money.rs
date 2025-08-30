#![allow(clippy::useless_conversion)]
//! Python bindings for Money type.

use super::currency::PyCurrency;
use finstack_core::error::Error;
use finstack_core::money::Money as CoreMoney;
use pyo3::prelude::*;

/// Monetary amount with currency.
///
/// A Money value represents a monetary amount in a specific currency.
/// It provides currency-safe arithmetic operations and ensures that
/// operations between different currencies are handled appropriately.
///
/// Money values support:
/// - Addition and subtraction (same currency required)
/// - Multiplication and division by scalars
/// - Currency conversion (future feature)
/// - Formatting and display
///
/// Examples:
///     >>> from rfin import Money, Currency
///     
///     # Create money values
///     >>> usd_100 = Money(100.0, Currency.usd())
///     >>> usd_50 = Money(50.0, Currency.usd())
///     >>> eur_75 = Money(75.0, Currency.eur())
///     
///     # Arithmetic with same currency
///     >>> usd_100 + usd_50
///     Money(150.0, USD)
///     >>> usd_100 - usd_50
///     Money(50.0, USD)
///     >>> usd_100 * 2
///     Money(200.0, USD)
///     >>> usd_100 / 4
///     Money(25.0, USD)
///     
///     # Currency mismatch raises error
///     >>> usd_100 + eur_75
///     Traceback (most recent call last):
///         ...
///     ValueError: Cannot add money with different currencies: expected USD, got EUR
///     
///     # String representation
///     >>> str(usd_100)
///     '100 USD'
///     >>> f"{usd_100}"
///     '100 USD'
#[pyclass(name = "Money")]
#[derive(Clone)]
pub struct PyMoney {
    inner: CoreMoney,
}

#[pymethods]
impl PyMoney {
    /// Create a new Money value with the specified amount and currency.
    ///
    /// Args:
    ///     amount (float): The monetary amount.
    ///     currency (Currency): The currency of the amount.
    ///
    /// Returns:
    ///     Money: A new Money instance.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_money = Money(100.0, Currency.usd())
    ///     >>> eur_money = Money(75.50, Currency.eur())
    ///     >>> gbp_money = Money(85.25, Currency.gbp())
    #[new]
    fn new(amount: f64, currency: &PyCurrency) -> Self {
        PyMoney {
            inner: CoreMoney::new(amount, currency.inner()),
        }
    }

    /// The monetary amount.
    ///
    /// Returns:
    ///     float: The amount of money.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(123.45, Currency.usd())
    ///     >>> money.amount
    ///     123.45
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// The currency of the money.
    ///
    /// Returns:
    ///     Currency: The currency of this money value.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(100.0, Currency.usd())
    ///     >>> money.currency
    ///     Currency('USD')
    ///     >>> money.currency.code
    ///     'USD'
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.currency())
    }

    /// Add two Money values.
    ///
    /// Both money values must have the same currency.
    ///
    /// Args:
    ///     other (Money): The money value to add.
    ///
    /// Returns:
    ///     Money: The sum of the two money values.
    ///
    /// Raises:
    ///     ValueError: If the currencies don't match.
    ///     RuntimeError: If the addition fails for other reasons.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> usd_50 = Money(50.0, Currency.usd())
    ///     >>> usd_100 + usd_50
    ///     Money(150.0, USD)
    ///     
    ///     # Currency mismatch raises error
    ///     >>> eur_75 = Money(75.0, Currency.eur())
    ///     >>> usd_100 + eur_75
    ///     Traceback (most recent call last):
    ///         ...
    ///     ValueError: Cannot add money with different currencies: expected USD, got EUR
    fn __add__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_add(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Cannot add money with different currencies: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Money addition failed: {}",
                err
            ))),
        }
    }

    /// Subtract two Money values.
    ///
    /// Both money values must have the same currency.
    ///
    /// Args:
    ///     other (Money): The money value to subtract.
    ///
    /// Returns:
    ///     Money: The difference of the two money values.
    ///
    /// Raises:
    ///     ValueError: If the currencies don't match.
    ///     RuntimeError: If the subtraction fails for other reasons.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> usd_30 = Money(30.0, Currency.usd())
    ///     >>> usd_100 - usd_30
    ///     Money(70.0, USD)
    ///     
    ///     # Can result in negative amounts
    ///     >>> usd_30 - usd_100
    ///     Money(-70.0, USD)
    ///     
    ///     # Currency mismatch raises error
    ///     >>> eur_75 = Money(75.0, Currency.eur())
    ///     >>> usd_100 - eur_75
    ///     Traceback (most recent call last):
    ///         ...
    ///     ValueError: Cannot subtract money with different currencies: expected USD, got EUR
    fn __sub__(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_sub(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Cannot subtract money with different currencies: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Money subtraction failed: {}",
                err
            ))),
        }
    }

    /// Multiply Money by a scalar.
    ///
    /// Args:
    ///     scalar (float): The scalar value to multiply by.
    ///
    /// Returns:
    ///     Money: The money value multiplied by the scalar.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> usd_100 * 2.5
    ///     Money(250.0, USD)
    ///     >>> usd_100 * 0.5
    ///     Money(50.0, USD)
    ///     >>> usd_100 * -1
    ///     Money(-100.0, USD)
    fn __mul__(&self, scalar: f64) -> PyMoney {
        PyMoney {
            inner: self.inner * scalar,
        }
    }

    /// Divide Money by a scalar.
    ///
    /// Args:
    ///     scalar (float): The scalar value to divide by.
    ///
    /// Returns:
    ///     Money: The money value divided by the scalar.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> usd_100 / 4
    ///     Money(25.0, USD)
    ///     >>> usd_100 / 3
    ///     Money(33.333333333333336, USD)
    fn __truediv__(&self, scalar: f64) -> PyMoney {
        PyMoney {
            inner: self.inner / scalar,
        }
    }

    /// Right multiplication (scalar * money).
    ///
    /// Supports expressions like `2 * Money(100, Currency.usd())`.
    ///
    /// Args:
    ///     scalar (float): The scalar value to multiply by.
    ///
    /// Returns:
    ///     Money: The money value multiplied by the scalar.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> 2.5 * usd_100
    ///     Money(250.0, USD)
    fn __rmul__(&self, scalar: f64) -> PyMoney {
        self.__mul__(scalar)
    }

    /// Return string representation of the money.
    ///
    /// Returns:
    ///     str: A string showing the amount and currency.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(100.0, Currency.usd())
    ///     >>> str(money)
    ///     '100 USD'
    ///     >>> f"{money}"
    ///     '100 USD'
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Return debug representation of the money.
    ///
    /// Returns:
    ///     str: A string like "Money(100.0, USD)".
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(100.0, Currency.usd())
    ///     >>> repr(money)
    ///     'Money(100.0, USD)'
    fn __repr__(&self) -> String {
        format!("Money({}, {})", self.inner.amount(), self.inner.currency())
    }

    /// Return formatted representation for f-strings.
    ///
    /// Args:
    ///     _format_spec (str): Format specification (currently ignored).
    ///
    /// Returns:
    ///     str: The formatted string representation.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(100.0, Currency.usd())
    ///     >>> f"{money}"
    ///     '100 USD'
    ///
    /// Note:
    ///     The format specification is currently ignored. Future versions
    ///     may support custom number formatting.
    fn __format__(&self, _format_spec: &str) -> String {
        // For now, ignore format_spec and just return the string representation
        // Future enhancement could parse format_spec for number formatting
        format!("{}", self.inner)
    }

    /// Check equality between two Money values.
    ///
    /// Args:
    ///     other (Money): Another Money instance.
    ///
    /// Returns:
    ///     bool: True if both amount and currency are equal.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100a = Money(100.0, Currency.usd())
    ///     >>> usd_100b = Money(100.0, Currency.usd())
    ///     >>> usd_200 = Money(200.0, Currency.usd())
    ///     >>> eur_100 = Money(100.0, Currency.eur())
    ///     >>> usd_100a == usd_100b
    ///     True
    ///     >>> usd_100a == usd_200
    ///     False
    ///     >>> usd_100a == eur_100
    ///     False
    fn __eq__(&self, other: &PyMoney) -> bool {
        self.inner == other.inner
    }

    /// Convert to (amount, currency) tuple.
    ///
    /// Returns:
    ///     Tuple[float, Currency]: A tuple containing the amount and currency.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(123.45, Currency.usd())
    ///     >>> amount, currency = money.to_parts()
    ///     >>> amount
    ///     123.45
    ///     >>> currency
    ///     Currency('USD')
    #[allow(clippy::wrong_self_convention)]
    fn to_parts(&self) -> (f64, PyCurrency) {
        let (amount, currency) = self.inner.into_parts();
        (amount, PyCurrency::from_inner(currency))
    }

    /// Convert to (amount, currency) tuple.
    ///
    /// Returns:
    ///     Tuple[float, Currency]: A tuple containing the amount and currency.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> money = Money(123.45, Currency.usd())
    ///     >>> amount, currency = money.into_parts()
    ///     >>> amount
    ///     123.45
    ///     >>> currency
    ///     Currency('USD')
    ///
    /// Note:
    ///     This method is deprecated. Use `to_parts()` instead.
    #[pyo3(name = "into_parts")]
    #[allow(clippy::wrong_self_convention)]
    fn into_parts_alias(&self) -> (f64, PyCurrency) {
        self.to_parts()
    }

    /// Add two Money values with explicit error handling.
    ///
    /// This method provides the same functionality as the `+` operator
    /// but with more explicit error handling.
    ///
    /// Args:
    ///     other (Money): The money value to add.
    ///
    /// Returns:
    ///     Money: The sum of the two money values.
    ///
    /// Raises:
    ///     ValueError: If the currencies don't match.
    ///     RuntimeError: If the addition fails for other reasons.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> usd_50 = Money(50.0, Currency.usd())
    ///     >>> result = usd_100.checked_add(usd_50)
    ///     >>> result
    ///     Money(150.0, USD)
    ///     
    ///     # Currency mismatch
    ///     >>> eur_75 = Money(75.0, Currency.eur())
    ///     >>> usd_100.checked_add(eur_75)
    ///     Traceback (most recent call last):
    ///         ...
    ///     ValueError: Currency mismatch: expected USD, got EUR
    fn checked_add(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_add(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Currency mismatch: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Addition failed: {}",
                err
            ))),
        }
    }

    /// Subtract two Money values with explicit error handling.
    ///
    /// This method provides the same functionality as the `-` operator
    /// but with more explicit error handling.
    ///
    /// Args:
    ///     other (Money): The money value to subtract.
    ///
    /// Returns:
    ///     Money: The difference of the two money values.
    ///
    /// Raises:
    ///     ValueError: If the currencies don't match.
    ///     RuntimeError: If the subtraction fails for other reasons.
    ///
    /// Examples:
    ///     >>> from rfin import Money, Currency
    ///     >>> usd_100 = Money(100.0, Currency.usd())
    ///     >>> usd_30 = Money(30.0, Currency.usd())
    ///     >>> result = usd_100.checked_sub(usd_30)
    ///     >>> result
    ///     Money(70.0, USD)
    ///     
    ///     # Currency mismatch
    ///     >>> eur_75 = Money(75.0, Currency.eur())
    ///     >>> usd_100.checked_sub(eur_75)
    ///     Traceback (most recent call last):
    ///         ...
    ///     ValueError: Currency mismatch: expected USD, got EUR
    fn checked_sub(&self, other: &PyMoney) -> PyResult<PyMoney> {
        match self.inner.checked_sub(other.inner) {
            Ok(result) => Ok(PyMoney { inner: result }),
            Err(Error::CurrencyMismatch { expected, actual }) => {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Currency mismatch: expected {}, got {}",
                    expected, actual
                )))
            }
            Err(err) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Subtraction failed: {}",
                err
            ))),
        }
    }

    /// Convert money to a different currency using an FX provider.
    ///
    /// This method converts the monetary amount from its current currency
    /// to a target currency using the provided FX rate provider.
    ///
    /// Args:
    ///     to_currency (Currency): The target currency to convert to.
    ///     date (Date): The date for the FX rate.
    ///     provider (SimpleFxProvider): The FX rate provider.
    ///     policy (FxConversionPolicy): The policy for determining which rate to use.
    ///
    /// Returns:
    ///     Money: A new Money instance in the target currency.
    ///
    /// Raises:
    ///     RuntimeError: If the conversion fails (e.g., rate not available).
    ///
    /// Examples:
    ///     >>> from finstack import Money, Currency, Date
    ///     >>> from finstack.market_data import SimpleFxProvider, FxConversionPolicy
    ///     >>>
    ///     >>> # Set up FX provider with rates
    ///     >>> provider = SimpleFxProvider()
    ///     >>> provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
    ///     >>> provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)
    ///     >>>
    ///     >>> # Convert USD to EUR
    ///     >>> usd_money = Money(100.0, Currency("USD"))
    ///     >>> date = Date(2025, 1, 15)
    ///     >>> eur_money = usd_money.convert(
    ///     ...     Currency("EUR"),
    ///     ...     date,
    ///     ...     provider,
    ///     ...     FxConversionPolicy.CashflowDate
    ///     ... )
    ///     >>> print(f"${usd_money.amount:.2f} USD = €{eur_money.amount:.2f} EUR")
    ///     $100.00 USD = €85.00 EUR
    ///     
    ///     >>> # Convert back to USD
    ///     >>> usd_back = eur_money.convert(
    ///     ...     Currency("USD"),
    ///     ...     date,
    ///     ...     provider,
    ///     ...     FxConversionPolicy.CashflowDate
    ///     ... )
    ///     >>> print(f"€{eur_money.amount:.2f} EUR = ${usd_back.amount:.2f} USD")
    ///     €85.00 EUR = $100.30 USD
    fn convert(
        &self,
        to_currency: &PyCurrency,
        date: &crate::core::dates::PyDate,
        provider: &crate::core::market_data::fx::PySimpleFxProvider,
        policy: crate::core::market_data::fx::PyFxConversionPolicy,
    ) -> PyResult<PyMoney> {
        crate::core::market_data::fx::convert_money(self, to_currency, date, provider, &policy)
    }
}

impl PyMoney {
    /// Get the inner Money type
    pub fn inner(&self) -> CoreMoney {
        self.inner
    }

    /// Create a PyMoney from inner CoreMoney
    pub fn from_inner(inner: CoreMoney) -> Self {
        PyMoney { inner }
    }
}
