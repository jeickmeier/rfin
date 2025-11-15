//! Helper utilities for P&L attribution.
//!
//! Provides shared functions for market context manipulation, instrument repricing,
//! and currency conversion.

use crate::instruments::common::traits::Instrument;
use finstack_core::prelude::*;
use std::sync::Arc;

/// Reprice an instrument at a given date with a market context.
///
/// # Arguments
///
/// * `instrument` - Instrument to price
/// * `market` - Market data context
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Present value in the instrument's native currency.
///
/// # Errors
///
/// Returns error if pricing fails (missing curves, invalid parameters, etc.).
pub fn reprice_instrument(
    instrument: &Arc<dyn Instrument>,
    market: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    instrument.value(market, as_of)
}

/// Convert money to a target currency using FX rates from market context.
///
/// # Arguments
///
/// * `money` - Amount to convert
/// * `target_ccy` - Target currency
/// * `market` - Market context with FX matrix
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Converted amount in target currency.
///
/// # Errors
///
/// Returns error if FX matrix is missing or rate lookup fails.
pub fn convert_currency(
    money: Money,
    target_ccy: Currency,
    market: &MarketContext,
    as_of: Date,
) -> Result<Money> {
    if money.currency() == target_ccy {
        return Ok(money);
    }

    let fx_matrix = market
        .fx
        .as_ref()
        .ok_or_else(|| Error::Validation("FX matrix not available".to_string()))?;

    let query = FxQuery::new(money.currency(), target_ccy, as_of);
    let rate_result = fx_matrix.rate(query)?;

    Ok(Money::new(money.amount() * rate_result.rate, target_ccy))
}

/// Compute P&L between two valuations in target currency.
///
/// Converts both valuations to target currency before computing difference.
///
/// # Arguments
///
/// * `val_t0` - Value at T₀
/// * `val_t1` - Value at T₁
/// * `target_ccy` - Currency for P&L
/// * `market_t1` - Market context at T₁ (for FX conversion)
/// * `as_of_t1` - Date at T₁
///
/// # Returns
///
/// P&L in target currency (val_t1 - val_t0).
///
/// # Errors
///
/// Returns error if currency conversion fails.
pub fn compute_pnl(
    val_t0: Money,
    val_t1: Money,
    target_ccy: Currency,
    market_t1: &MarketContext,
    as_of_t1: Date,
) -> Result<Money> {
    let val_t0_converted = convert_currency(val_t0, target_ccy, market_t1, as_of_t1)?;
    let val_t1_converted = convert_currency(val_t1, target_ccy, market_t1, as_of_t1)?;

    val_t1_converted.checked_sub(val_t0_converted)
}

/// Compute P&L with explicit FX conversion for each date.
///
/// This allows proper isolation of FX translation effects by using
/// date-appropriate FX rates for conversion.
///
/// # Arguments
///
/// * `val_t0` - Value at T₀
/// * `val_t1` - Value at T₁
/// * `target_ccy` - Currency for P&L
/// * `market_fx_t0` - Market context at T₀ (for T₀ FX conversion)
/// * `market_fx_t1` - Market context at T₁ (for T₁ FX conversion)
/// * `as_of_t0` - Date at T₀
/// * `as_of_t1` - Date at T₁
///
/// # Returns
///
/// P&L in target currency with FX translation properly isolated.
///
/// # Errors
///
/// Returns error if currency conversion fails.
///
/// # Examples
///
/// ```rust,ignore
/// // For FX attribution: convert T₀ value with T₀ FX, T₁ value with T₁ FX
/// let fx_pnl = compute_pnl_with_fx(
///     pv_t0,
///     pv_t1,
///     Currency::USD,
///     &market_t0,
///     &market_t1,
///     as_of_t0,
///     as_of_t1,
/// )?;
/// ```
pub fn compute_pnl_with_fx(
    val_t0: Money,
    val_t1: Money,
    target_ccy: Currency,
    market_fx_t0: &MarketContext,
    market_fx_t1: &MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
) -> Result<Money> {
    let val_t0_converted = convert_currency(val_t0, target_ccy, market_fx_t0, as_of_t0)?;
    let val_t1_converted = convert_currency(val_t1, target_ccy, market_fx_t1, as_of_t1)?;

    val_t1_converted.checked_sub(val_t0_converted)
}

/// Clone a MarketContext (cheap operation due to Arc-based storage).
#[inline]
pub fn clone_market(market: &MarketContext) -> MarketContext {
    market.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
    use std::sync::Arc;
    use time::macros::date;

    // Simple test FX provider
    struct TestFx;
    impl FxProvider for TestFx {
        fn rate(
            &self,
            from: Currency,
            to: Currency,
            _on: Date,
            _policy: FxConversionPolicy,
        ) -> Result<f64> {
            if from == Currency::EUR && to == Currency::USD {
                Ok(1.1)
            } else if from == Currency::USD && to == Currency::EUR {
                Ok(1.0 / 1.1)
            } else if from == to {
                Ok(1.0)
            } else {
                Err(Error::Validation("FX rate not found".to_string()))
            }
        }
    }

    #[test]
    fn test_convert_currency_same_ccy() {
        let money = Money::new(1000.0, Currency::USD);
        let market = MarketContext::new();
        let as_of = date!(2025 - 01 - 15);

        let result = convert_currency(money, Currency::USD, &market, as_of)
            .expect("Currency conversion should succeed in test");
        assert_eq!(result, money);
    }

    #[test]
    fn test_convert_currency_with_fx() {
        let money = Money::new(1000.0, Currency::EUR);
        let fx = FxMatrix::new(Arc::new(TestFx));
        let market = MarketContext::new().insert_fx(fx);
        let as_of = date!(2025 - 01 - 15);

        let result = convert_currency(money, Currency::USD, &market, as_of)
            .expect("Currency conversion should succeed in test");
        assert_eq!(result.amount(), 1100.0);
        assert_eq!(result.currency(), Currency::USD);
    }

    #[test]
    fn test_compute_pnl() {
        let val_t0 = Money::new(1000.0, Currency::EUR);
        let val_t1 = Money::new(1100.0, Currency::EUR);
        let fx = FxMatrix::new(Arc::new(TestFx));
        let market = MarketContext::new().insert_fx(fx);
        let as_of = date!(2025 - 01 - 15);

        let pnl = compute_pnl(val_t0, val_t1, Currency::USD, &market, as_of)
            .expect("PNL computation should succeed in test");
        // (1100 - 1000) EUR * 1.1 = 110 USD
        assert_eq!(pnl.amount(), 110.0);
        assert_eq!(pnl.currency(), Currency::USD);
    }

    #[test]
    fn test_compute_pnl_with_fx() {
        // Test FX translation isolation
        let pv = Money::new(1000.0, Currency::EUR);

        // T0 market: EUR/USD = 1.1
        let fx_t0 = FxMatrix::new(Arc::new(TestFx));
        let market_t0 = MarketContext::new().insert_fx(fx_t0);

        // T1 market: EUR/USD = 1.2 (10% appreciation)
        struct TestFxT1;
        impl FxProvider for TestFxT1 {
            fn rate(
                &self,
                from: Currency,
                to: Currency,
                _on: Date,
                _policy: FxConversionPolicy,
            ) -> Result<f64> {
                if from == Currency::EUR && to == Currency::USD {
                    Ok(1.2)
                } else if from == Currency::USD && to == Currency::EUR {
                    Ok(1.0 / 1.2)
                } else if from == to {
                    Ok(1.0)
                } else {
                    Err(Error::Validation("FX rate not found".to_string()))
                }
            }
        }
        let fx_t1 = FxMatrix::new(Arc::new(TestFxT1));
        let market_t1 = MarketContext::new().insert_fx(fx_t1);

        let as_of_t0 = date!(2025 - 01 - 15);
        let as_of_t1 = date!(2025 - 01 - 16);

        // PV unchanged in EUR, but FX moved
        let pnl = compute_pnl_with_fx(
            pv,
            pv,
            Currency::USD,
            &market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
        )
        .expect("PNL computation with FX should succeed in test");

        // FX translation: 1000 EUR @ 1.2 - 1000 EUR @ 1.1 = 1200 - 1100 = 100 USD
        assert_eq!(pnl.amount(), 100.0);
        assert_eq!(pnl.currency(), Currency::USD);
    }
}
