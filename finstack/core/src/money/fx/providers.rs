//! Standard FX provider implementations for simple quote storage.
//!
//! This module provides reusable FX provider types that can be shared across
//! bindings (Python, WASM, etc.) without duplication.

use super::{FxConversionPolicy, FxProvider};
use crate::collections::HashMap;
use crate::currency::Currency;
use crate::dates::Date;
use crate::error::InputError;
use parking_lot::RwLock;
use std::sync::Arc;

/// Simple FX provider backed by an in-memory quote store.
///
/// Supports:
/// - Direct quote lookup
/// - Automatic reciprocal calculation
/// - Thread-safe mutable quote insertion
///
/// # Examples
/// ```rust
/// use finstack_core::money::fx::SimpleFxProvider;
/// use finstack_core::money::fx::{FxProvider, FxConversionPolicy};
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use time::Month;
///
/// let provider = SimpleFxProvider::new();
/// provider.set_quote(Currency::EUR, Currency::USD, 1.1).expect("valid rate");
///
/// let date = Date::from_calendar_date(2024, Month::January, 2).expect("Valid date");
/// let rate = provider.rate(Currency::EUR, Currency::USD, date, FxConversionPolicy::CashflowDate).expect("FX rate lookup should succeed");
/// assert_eq!(rate, 1.1);
///
/// // Reciprocal works automatically
/// let rate_inv = provider.rate(Currency::USD, Currency::EUR, date, FxConversionPolicy::CashflowDate).expect("FX rate lookup should succeed");
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
    /// use finstack_core::money::fx::SimpleFxProvider;
    ///
    /// let provider = SimpleFxProvider::new();
    /// ```
    pub fn new() -> Self {
        Self {
            quotes: RwLock::new(HashMap::default()),
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
    /// use finstack_core::money::fx::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quote(Currency::GBP, Currency::USD, 1.25);
    /// ```
    pub fn set_quote(&self, from: Currency, to: Currency, rate: f64) -> crate::Result<()> {
        let rate = super::validate_fx_rate(from, to, rate)?;
        self.quotes.write().insert((from, to), rate);
        Ok(())
    }

    /// Builder-style quote insertion for ergonomic setup.
    ///
    /// Equivalent to `set_quote` but takes and returns `self`, allowing chained
    /// construction before wrapping in `Arc`. Panics if the rate is invalid
    /// (non-finite, NaN, zero, or negative).
    ///
    /// # Panics
    ///
    /// Panics if `rate` is not a valid positive finite FX rate.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    /// use std::sync::Arc;
    ///
    /// let provider = Arc::new(
    ///     SimpleFxProvider::new()
    ///         .with_quote(Currency::EUR, Currency::USD, 1.1)
    ///         .with_quote(Currency::GBP, Currency::USD, 1.25),
    /// );
    /// ```
    #[must_use]
    #[allow(clippy::panic)]
    pub fn with_quote(self, from: Currency, to: Currency, rate: f64) -> Self {
        let rate = super::validate_fx_rate(from, to, rate)
            .unwrap_or_else(|e| panic!("invalid FX rate {from}->{to} = {rate}: {e}"));
        self.quotes.write().insert((from, to), rate);
        self
    }

    /// Bulk insert or update FX quotes.
    ///
    /// # Parameters
    /// - `quotes`: Slice of `(from, to, rate)` tuples
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quotes(&[
    ///     (Currency::EUR, Currency::USD, 1.1),
    ///     (Currency::GBP, Currency::USD, 1.25),
    /// ]);
    /// ```
    pub fn set_quotes(&self, quotes: &[(Currency, Currency, f64)]) -> crate::Result<()> {
        let mut guard = self.quotes.write();
        for &(from, to, rate) in quotes {
            let rate = super::validate_fx_rate(from, to, rate)?;
            guard.insert((from, to), rate);
        }
        Ok(())
    }

    /// Retrieve a direct quote if available.
    ///
    /// Returns `None` if no direct quote exists for the pair.
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::SimpleFxProvider;
    /// use finstack_core::currency::Currency;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quote(Currency::EUR, Currency::USD, 1.1).expect("valid rate");
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
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - [`InputError::NotFound`](crate::error::InputError::NotFound): No direct quote
    ///   exists for `from→to` and no reciprocal `to→from` is available
    /// - [`InputError::NonFiniteValue`](crate::error::InputError::NonFiniteValue): The
    ///   stored rate or its reciprocal is non-finite
    ///
    /// # Examples
    /// ```rust
    /// use finstack_core::money::fx::SimpleFxProvider;
    /// use finstack_core::money::fx::{FxProvider, FxConversionPolicy};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let provider = SimpleFxProvider::new();
    /// provider.set_quote(Currency::EUR, Currency::USD, 1.1).expect("valid rate");
    ///
    /// let date = Date::from_calendar_date(2024, Month::January, 2).expect("Valid date");
    /// let rate = provider.rate(Currency::EUR, Currency::USD, date, FxConversionPolicy::CashflowDate).expect("FX rate lookup should succeed");
    /// assert_eq!(rate, 1.1);
    ///
    /// // Reciprocal works automatically
    /// let rate_inv = provider.rate(Currency::USD, Currency::EUR, date, FxConversionPolicy::CashflowDate).expect("FX rate lookup should succeed");
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
            return super::reciprocal_rate_or_err(rate, to, from);
        }
        Err(InputError::NotFound {
            id: format!("FX:{from}->{to}"),
        }
        .into())
    }
}

/// Wrapper provider that overrides a specific FX rate while delegating others.
///
/// This is useful for bumping FX rates in scenario analysis without
/// losing the rest of the FX matrix state.
pub struct BumpedFxProvider {
    /// Original provider to delegate to
    original: Arc<dyn FxProvider>,
    /// Override rate for specific pair
    override_from: Currency,
    /// Override rate for specific pair
    override_to: Currency,
    /// Override rate value
    override_rate: f64,
}

impl BumpedFxProvider {
    /// Create a new bumped provider that overrides one rate.
    ///
    /// # Parameters
    /// - `original`: Original FX provider to delegate to
    /// - `from`: Currency to override
    /// - `to`: Currency to override
    /// - `bumped_rate`: New rate for the overridden pair
    pub fn new(
        original: Arc<dyn FxProvider>,
        from: Currency,
        to: Currency,
        bumped_rate: f64,
    ) -> Self {
        Self {
            original,
            override_from: from,
            override_to: to,
            override_rate: bumped_rate,
        }
    }
}

impl FxProvider for BumpedFxProvider {
    /// Return an FX rate, using the bumped value for the overridden pair.
    ///
    /// The provider:
    /// 1. Returns the bumped rate if querying the overridden `from→to` pair
    /// 2. Returns the reciprocal of the bumped rate for `to→from`
    /// 3. Delegates to the original provider for all other pairs
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - The original provider fails for non-overridden pairs
    /// - Any error propagated from [`FxProvider::rate`] on the underlying provider
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<f64> {
        // Check if this is the overridden pair (or its reciprocal)
        if from == self.override_from && to == self.override_to {
            return Ok(self.override_rate);
        }
        if from == self.override_to && to == self.override_from && self.override_rate != 0.0 {
            return Ok(1.0 / self.override_rate);
        }

        // Delegate to original provider for all other pairs
        self.original.rate(from, to, on, policy)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 2).expect("Valid test date")
    }

    #[test]
    fn simple_provider_direct_quote() {
        let provider = SimpleFxProvider::new();
        provider
            .set_quote(Currency::EUR, Currency::USD, 1.1)
            .expect("valid test quote");

        let rate = provider
            .rate(
                Currency::EUR,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .expect("FX rate query should succeed in test");
        assert_eq!(rate, 1.1);
    }

    #[test]
    fn simple_provider_reciprocal() {
        let provider = SimpleFxProvider::new();
        provider
            .set_quote(Currency::EUR, Currency::USD, 1.1)
            .expect("valid test quote");

        let rate = provider
            .rate(
                Currency::USD,
                Currency::EUR,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .expect("FX rate query should succeed in test");
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
            .expect("FX rate query should succeed in test");
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
        provider
            .set_quotes(&[
                (Currency::EUR, Currency::USD, 1.1),
                (Currency::GBP, Currency::USD, 1.25),
            ])
            .expect("valid test quotes");

        let eur_usd = provider
            .rate(
                Currency::EUR,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .expect("FX rate query should succeed in test");
        assert_eq!(eur_usd, 1.1);

        let gbp_usd = provider
            .rate(
                Currency::GBP,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .expect("FX rate query should succeed in test");
        assert_eq!(gbp_usd, 1.25);
    }

    #[test]
    fn bumped_provider_overrides_rate() {
        let original = Arc::new(SimpleFxProvider::new());
        original
            .set_quote(Currency::EUR, Currency::USD, 1.1)
            .expect("valid test quote");
        original
            .set_quote(Currency::GBP, Currency::USD, 1.25)
            .expect("valid test quote");

        // Create bumped provider that overrides EUR/USD
        let bumped = BumpedFxProvider::new(original.clone(), Currency::EUR, Currency::USD, 1.2);

        // Overridden rate should return bumped value
        let eur_usd = bumped
            .rate(
                Currency::EUR,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .expect("FX rate query should succeed in test");
        assert_eq!(eur_usd, 1.2);

        // Other rates should delegate to original
        let gbp_usd = bumped
            .rate(
                Currency::GBP,
                Currency::USD,
                test_date(),
                FxConversionPolicy::CashflowDate,
            )
            .expect("FX rate query should succeed in test");
        assert_eq!(gbp_usd, 1.25);
    }
}
