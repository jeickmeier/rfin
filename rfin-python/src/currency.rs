//! Python bindings for Currency type.

use pyo3::prelude::*;
use pyo3::types::PyType;
use rfin_core::currency::Currency as CoreCurrency;

/// Python wrapper for the Currency enum
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

    /// Create a US Dollar (USD) currency instance.
    ///
    /// Returns:
    ///     Currency: USD currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> usd = Currency.usd()
    ///     >>> print(usd.code)
    ///     USD
    ///     >>> print(usd.decimals)
    ///     2
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn usd(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::USD,
        }
    }

    /// Create a Euro (EUR) currency instance.
    ///
    /// Returns:
    ///     Currency: EUR currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> eur = Currency.eur()
    ///     >>> print(eur.numeric_code)
    ///     978
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn eur(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::EUR,
        }
    }

    /// Create a British Pound Sterling (GBP) currency instance.
    ///
    /// Returns:
    ///     Currency: GBP currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> gbp = Currency.gbp()
    ///     >>> print(gbp.code)
    ///     GBP
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn gbp(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::GBP,
        }
    }

    /// Create a Japanese Yen (JPY) currency instance.
    ///
    /// Returns:
    ///     Currency: JPY currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> jpy = Currency.jpy()
    ///     >>> print(jpy.decimals)
    ///     0  # Yen has no decimal places
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn jpy(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::JPY,
        }
    }

    /// Create a Swiss Franc (CHF) currency instance.
    ///
    /// Returns:
    ///     Currency: CHF currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> chf = Currency.chf()
    ///     >>> print(chf.code)
    ///     CHF
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn chf(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::CHF,
        }
    }

    /// Create an Australian Dollar (AUD) currency instance.
    ///
    /// Returns:
    ///     Currency: AUD currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> aud = Currency.aud()
    ///     >>> print(aud.numeric_code)
    ///     36
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn aud(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::AUD,
        }
    }

    /// Create a Canadian Dollar (CAD) currency instance.
    ///
    /// Returns:
    ///     Currency: CAD currency
    ///
    /// Examples:
    ///     >>> from rfin import Currency
    ///     >>> cad = Currency.cad()
    ///     >>> print(cad.code)
    ///     CAD
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn cad(_cls: &Bound<'_, PyType>) -> Self {
        PyCurrency {
            inner: CoreCurrency::CAD,
        }
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
