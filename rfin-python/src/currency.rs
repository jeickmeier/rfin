//! Python bindings for Currency type.

use pyo3::prelude::*;
use rfin_core::currency::Currency as CoreCurrency;

/// Currency representation based on ISO 4217 standards.
///
/// A Currency represents a specific currency using the ISO 4217 standard,
/// which defines three-letter currency codes (e.g., USD, EUR, GBP).
/// Each currency has an associated numeric code and decimal precision.
///
/// Currencies are used throughout the library for:
/// - Creating monetary amounts (Money)
/// - Ensuring currency-safe arithmetic operations
/// - Formatting monetary values according to currency conventions
/// - Financial instrument pricing and valuation
///
/// The library supports all major world currencies defined in ISO 4217.
///
/// Examples:
///     >>> from rfin import Currency
///     
///     # Create currencies from string codes
///     >>> usd = Currency("USD")
///     >>> eur = Currency("eur")  # Case insensitive
///     >>> gbp = Currency("GBP")
///     
///     # Access currency properties
///     >>> usd.code
///     'USD'
///     >>> usd.numeric_code
///     840
///     >>> usd.decimals
///     2
///     
///     # Use with Money
///     >>> from rfin import Money
///     >>> money = Money(100.0, usd)
///     >>> money.currency == usd
///     True
///     
///     # Currency comparison
///     >>> usd == Currency("USD")
///     True
///     >>> usd == eur
///     False
///     
///     # String representation
///     >>> str(usd)
///     'USD'
///     >>> repr(usd)
///     "Currency('USD')"
#[pyclass(name = "Currency", module = "rfin.currency")]
#[derive(Clone)]
pub struct PyCurrency {
    inner: CoreCurrency,
}

#[pymethods]
impl PyCurrency {
    /// Create a new Currency from a string code.
    ///
    /// Args:
    ///     code: The 3-letter ISO 4217 currency code (case-insensitive)
    ///
    /// Returns:
    ///     Currency: A new Currency instance
    ///
    /// Raises:
    ///     ValueError: If the currency code is invalid
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> usd = Currency("USD")
    ///     >>> eur = Currency("eur")  # Case insensitive
    ///     >>> print(usd)
    ///     USD
    #[new]
    #[pyo3(text_signature = "(code)")]
    fn new(code: &str) -> PyResult<Self> {
        let currency = code.parse::<CoreCurrency>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid currency code: {}", e))
        })?;
        Ok(PyCurrency { inner: currency })
    }

    /// Get the 3-letter ISO 4217 currency code.
    ///
    /// Returns:
    ///     str: The currency code (e.g., "USD", "EUR")
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> usd = Currency.usd()
    ///     >>> print(usd.code)
    ///     USD
    #[getter]
    fn code(&self) -> String {
        format!("{}", self.inner)
    }

    /// Get the ISO 4217 numeric code for this currency.
    ///
    /// Each currency has a unique 3-digit numeric code assigned by ISO 4217.
    ///
    /// Returns:
    ///     int: The numeric currency code
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> Currency.usd().numeric_code
    ///     840
    ///     >>> Currency.eur().numeric_code
    ///     978
    ///     >>> Currency.gbp().numeric_code
    ///     826
    #[getter]
    fn numeric_code(&self) -> u16 {
        self.inner as u16
    }

    /// Get the number of decimal places (minor units) for this currency.
    ///
    /// This indicates how many decimal places are typically used when
    /// displaying monetary amounts in this currency according to ISO 4217.
    ///
    /// Returns:
    ///     int: Number of decimal places (0, 2, or 3 for most currencies)
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> Currency.usd().decimals
    ///     2  # US Dollar uses cents
    ///     >>> Currency.jpy().decimals  
    ///     0  # Japanese Yen has no subunits
    ///     >>> Currency("BHD").decimals
    ///     3  # Bahraini Dinar uses 3 decimal places
    #[getter]
    fn decimals(&self) -> u8 {
        self.inner.decimals()
    }

    /// Return the string representation of the currency.
    ///
    /// Returns:
    ///     str: The 3-letter currency code
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> str(Currency.usd())
    ///     'USD'
    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Return the debug representation of the currency.
    ///
    /// Returns:
    ///     str: A string like "Currency('USD')"
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> repr(Currency.usd())
    ///     "Currency('USD')"
    fn __repr__(&self) -> String {
        format!("Currency('{}')", self.inner)
    }

    /// Check equality between two Currency instances.
    ///
    /// Args:
    ///     other: Another Currency instance
    ///
    /// Returns:
    ///     bool: True if the currencies are the same
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> Currency.usd() == Currency("USD")
    ///     True
    ///     >>> Currency.usd() == Currency.eur()
    ///     False
    fn __eq__(&self, other: &PyCurrency) -> bool {
        self.inner == other.inner
    }

    /// Return hash of the currency for use in sets and dicts.
    ///
    /// Returns:
    ///     int: Hash value
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> currencies = {Currency.usd(), Currency.eur(), Currency.usd()}
    ///     >>> len(currencies)
    ///     2  # USD appears only once
    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl PyCurrency {
    /// Create a new PyCurrency from CoreCurrency (internal use)
    pub fn from_inner(inner: CoreCurrency) -> Self {
        Self { inner }
    }

    /// Get the inner Currency enum
    pub fn inner(&self) -> CoreCurrency {
        self.inner
    }
}
