//! Standard FX provider implementations for simple quote storage.
//!
//! This module provides reusable FX provider types that can be shared across
//! bindings (Python, WASM, etc.) without duplication.

use super::{FxConversionPolicy, FxProvider};
use crate::currency::Currency;
use crate::dates::Date;
use crate::error::InputError;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Simple FX provider backed by an in-memory quote store.
///
/// Supports:
/// - Direct quote lookup
/// - Automatic reciprocal calculation
/// - Thread-safe mutable quote insertion
///
/// # Examples
/// ```rust
/// use finstack_core::money::fx::providers::SimpleFxProvider;
/// use finstack_core::money::fx::{FxProvider, FxConversionPolicy};
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let provider = SimpleFxProvider::new();
/// provider.set_quote(Currency::EUR, Currency::USD, 1.1);
///
/// let date = Date::from_calendar_date(2024, Month::January, 2).unwrap();
/// let rate = provider.rate(Currency::EUR, Currency::USD, date, FxConversionPolicy::CashflowDate).unwrap();
/// assert_eq!(rate, 1.1);
///
/// // Reciprocal works automatically
/// let rate_inv = provider.rate(Currency::USD, Currency::EUR, date, FxConversionPolicy::CashflowDate).unwrap();
/// assert!((rate_inv - 1.0/1.1).abs() < 1e-12);
/// ```
#[derive(Default)]
pub struct SimpleFxProvider {
    quotes: RwLock<HashMap<(Currency, Currency), f64>>,
}

impl SimpleFxProvider {
    /// Create a new empty provider.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::providers::SimpleFxProvider;
    ///
    /// let provider = SimpleFxProvider::new();
    /// ```
    pub fn new() -> Self {
        Self {
            quotes: RwLock::new(HashMap::new()),
        }
    }

    /// Insert or update a single FX quote.
    ///
    /// # Parameters
    /// - `from`: Base currency
    /// - `to`: Quote currency
    /// - `rate`: FX rate (from → to)
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::providers::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quote(Currency::GBP, Currency::USD, 1.25);
    /// ```
    pub fn set_quote(&self, from: Currency, to: Currency, rate: f64) {
        self.quotes.write().insert((from, to), rate);
    }

    /// Bulk insert or update FX quotes.
    ///
    /// # Parameters
    /// - `quotes`: Slice of `(from, to, rate)` tuples
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::providers::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quotes(&[
    ///     (Currency::EUR, Currency::USD, 1.1),
    ///     (Currency::GBP, Currency::USD, 1.25),
    /// ]);
    /// ```
    pub fn set_quotes(&self, quotes: &[(Currency, Currency, f64)]) {
        let mut guard = self.quotes.write();
        for &(from, to, rate) in quotes {
            guard.insert((from, to), rate);
        }
    }

    /// Retrieve a direct quote if available.
    ///
    /// Returns `None` if no direct quote exists for the pair.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::providers::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quote(Currency::EUR, Currency::USD, 1.1);
    ///
    /// assert_eq!(provider.get_direct(Currency::EUR, Currency::USD), Some(1.1));
    /// assert_eq!(provider.get_direct(Currency::USD, Currency::EUR), None);
    /// ```
    pub fn get_direct(&self, from: Currency, to: Currency) -> Option<f64> {
        self.quotes.read().get(&(from, to)).copied()
    }
}

impl FxProvider for SimpleFxProvider {
    /// Return an FX rate with automatic reciprocal fallback.
    ///
    /// The provider:
    /// 1. Returns 1.0 for identical currencies
    /// 2. Checks for a direct quote
    /// 3. Falls back to reciprocal if available
    /// 4. Returns `NotFound` error otherwise
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::providers::SimpleFxProvider;
    /// use finstack_core::money::fx::{FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quote(Currency::EUR, Currency::USD, 1.1);
    ///
    /// let date = Date::from_calendar_date(2024, Month::January, 2).unwrap();
    /// let rate = provider.rate(Currency::EUR, Currency::USD, date, FxConversionPolicy::CashflowDate).unwrap();
    /// assert_eq!(rate, 1.1);
    ///
    /// // Reciprocal works automatically
    /// let rate_inv = provider.rate(Currency::USD, Currency::EUR, date, FxConversionPolicy::CashflowDate).unwrap();
    /// assert!((rate_inv - 1.0/1.1).abs() < 1e-12);
    /// ```
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> crate::Result<f64> {
        if from == to {
            return Ok(1.0);
        }
        if let Some(rate) = self.get_direct(from, to) {
            return Ok(rate);
        }
        if let Some(rate) = self.get_direct(to, from) {
            if rate != 0.0 {
                return Ok(1.0 / rate);
            }
        }
        Err(InputError::NotFound {
            id: format!("FX:{from}->{to}"),
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 2).unwrap()
    }

    #[test]
    fn simple_provider_direct_quote() {
        let provider = SimpleFxProvider::new();
        provider.set_quote(Currency::EUR, Currency::USD, 1.1);

        let rate = provider
            .rate(
                Currency::EUR,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .unwrap();
        assert_eq!(rate, 1.1);
    }

    #[test]
    fn simple_provider_reciprocal() {
        let provider = SimpleFxProvider::new();
        provider.set_quote(Currency::EUR, Currency::USD, 1.1);

        let rate = provider
            .rate(
                Currency::USD,
                Currency::EUR,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .unwrap();
        assert!((rate - 1.0 / 1.1).abs() < 1e-12);
    }

    #[test]
    fn simple_provider_identity() {
        let provider = SimpleFxProvider::new();

        let rate = provider
            .rate(
                Currency::USD,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .unwrap();
        assert_eq!(rate, 1.0);
    }

    #[test]
    fn simple_provider_not_found() {
        let provider = SimpleFxProvider::new();

        let result = provider.rate(
            Currency::EUR,
            Currency::USD,
            test_date(),
            FxConversionPolicy::CashflowDate,
        );
        assert!(result.is_err());
    }

    #[test]
    fn simple_provider_bulk_quotes() {
        let provider = SimpleFxProvider::new();
        provider.set_quotes(&[
            (Currency::EUR, Currency::USD, 1.1),
            (Currency::GBP, Currency::USD, 1.25),
        ]);

        let eur_usd = provider
            .rate(
                Currency::EUR,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .unwrap();
        assert_eq!(eur_usd, 1.1);

        let gbp_usd = provider
            .rate(
                Currency::GBP,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .unwrap();
        assert_eq!(gbp_usd, 1.25);
    }
}
