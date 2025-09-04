//! Unified forward function builders for volatility surface calibration.
//!
//! This module consolidates the forward function construction logic that was
//! duplicated between the orchestrator and vol surface calibrator.

use finstack_core::market_data::context::MarketContext;
use finstack_core::prelude::*;
use finstack_core::F;

/// Build forward function for equity underlyings: F(t) = S0 * exp((r - q) * t)
pub fn forward_fn_equity<'a>(
    context: &'a MarketContext,
    underlying: &str,
    base_currency: Currency,
) -> Result<Box<dyn Fn(F) -> F + 'a>> {
    // Get spot price
    let spot_scalar = context.market_scalar(underlying)?;
    let spot = match spot_scalar {
        finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
    };

    // Get dividend yield (default to 0.0 if not available)
    let div_yield_key = format!("{}-DIVYIELD", underlying);
    let dividend_yield = context
        .market_scalar(&div_yield_key)
        .map(|scalar| match scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(yield_val) => {
                *yield_val
            }
            _ => 0.0,
        })
        .unwrap_or(0.0);

    // Get risk-free rate from discount curve
    let disc_curve_id = format!("{}-OIS", base_currency);
    let discount_curve = context.discount(&disc_curve_id)?;

    Ok(Box::new(move |t: F| -> F {
        let risk_free_rate = discount_curve.zero(t);
        spot * ((risk_free_rate - dividend_yield) * t).exp()
    }))
}

/// Build forward function for FX underlyings: F(t) = S0 * exp((r_dom - r_for) * t)
pub fn forward_fn_fx<'a>(
    context: &'a MarketContext,
    underlying: &str,
) -> Result<Box<dyn Fn(F) -> F + 'a>> {
    // Parse FX pair (assume 6-char format like "EURUSD")
    if underlying.len() != 6 {
        return Err(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        ));
    }

    let foreign_ccy = &underlying[0..3];
    let domestic_ccy = &underlying[3..6];

    // Get spot rate
    let spot_scalar = context.market_scalar(underlying)?;
    let spot = match spot_scalar {
        finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        finstack_core::market_data::primitives::MarketScalar::Unitless(value) => *value,
    };

    // Get domestic and foreign discount curves
    let dom_disc_id = format!("{}-OIS", domestic_ccy);
    let for_disc_id = format!("{}-OIS", foreign_ccy);
    let dom_curve = context.discount(&dom_disc_id)?;
    let for_curve = context.discount(&for_disc_id)?;

    Ok(Box::new(move |t: F| -> F {
        let domestic_rate = dom_curve.zero(t);
        let foreign_rate = for_curve.zero(t);
        spot * ((domestic_rate - foreign_rate) * t).exp()
    }))
}

/// Build forward function for interest rate underlyings: F(t) = forward_curve.rate(t)
pub fn forward_fn_rates<'a>(
    context: &'a MarketContext,
    underlying: &str,
) -> Result<Box<dyn Fn(F) -> F + 'a>> {
    // Get forward curve for this index
    let forward_curve = context.forecast(underlying)?;

    Ok(Box::new(move |t: F| -> F { forward_curve.rate(t) }))
}

/// Auto-detect asset class and build appropriate forward function.
///
/// Determines asset class from underlying identifier and constructs
/// appropriate forward calculation using market data.
pub fn forward_fn_auto<'a>(
    context: &'a MarketContext,
    underlying: &str,
    base_currency: Currency,
) -> Result<Box<dyn Fn(F) -> F + 'a>> {
    // Detect asset class from underlying identifier
    if underlying.contains("-")
        && (underlying.contains("SOFR")
            || underlying.contains("EURIBOR")
            || underlying.contains("SONIA"))
    {
        // Interest rate underlying (e.g., "USD-SOFR3M", "EUR-EURIBOR3M")
        forward_fn_rates(context, underlying)
    } else if underlying.len() == 6 && underlying.chars().all(|c| c.is_ascii_alphabetic()) {
        // FX pair (e.g., "EURUSD", "GBPJPY")
        forward_fn_fx(context, underlying)
    } else {
        // Equity underlying (e.g., "SPY", "AAPL")
        forward_fn_equity(context, underlying, base_currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::market_data::primitives::MarketScalar;
    use finstack_core::dates::Date;
    use time::Month;

    fn create_test_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        
        // Create discount curve
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
            .linear_df()
            .build()
            .unwrap();

        // Create forward curve
        let fwd_curve = ForwardCurve::builder("USD-SOFR3M", 0.25)
            .base_date(base_date)
            .knots([(0.0, 0.03), (1.0, 0.035), (5.0, 0.04)])
            .linear_df()
            .build()
            .unwrap();

        MarketContext::new()
            .with_discount(disc_curve)
            .with_forecast(fwd_curve)
            .with_price("SPY", MarketScalar::Unitless(100.0))
            .with_price("SPY-DIVYIELD", MarketScalar::Unitless(0.02))
            .with_price("EURUSD", MarketScalar::Unitless(1.1))
    }

    #[test]
    fn test_equity_forward_function() {
        let context = create_test_context();
        let forward_fn = forward_fn_equity(&context, "SPY", Currency::USD).unwrap();
        
        // Test forward price calculation
        let forward_1y = forward_fn(1.0);
        
        // Should be positive and reasonable
        assert!(forward_1y > 0.0);
        assert!(forward_1y > 90.0 && forward_1y < 110.0); // Reasonable range around spot
    }

    #[test]
    fn test_rates_forward_function() {
        let context = create_test_context();
        let forward_fn = forward_fn_rates(&context, "USD-SOFR3M").unwrap();
        
        // Test forward rate
        let forward_rate_1y = forward_fn(1.0);
        
        // Should match the forward curve
        assert!((forward_rate_1y - 0.035).abs() < 1e-6);
    }

    #[test]
    fn test_auto_detection_equity() {
        let context = create_test_context();
        let forward_fn = forward_fn_auto(&context, "SPY", Currency::USD).unwrap();
        
        let forward_1y = forward_fn(1.0);
        assert!(forward_1y > 0.0);
    }

    #[test]
    fn test_auto_detection_rates() {
        let context = create_test_context();
        let forward_fn = forward_fn_auto(&context, "USD-SOFR3M", Currency::USD).unwrap();
        
        let forward_rate_1y = forward_fn(1.0);
        assert!((forward_rate_1y - 0.035).abs() < 1e-6);
    }

    #[test]
    fn test_auto_detection_fx() {
        // Create additional discount curve for EUR
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let eur_disc = DiscountCurve::builder("EUR-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.82)])
            .linear_df()
            .build()
            .unwrap();

        let context = create_test_context().with_discount(eur_disc);
        let forward_fn = forward_fn_auto(&context, "EURUSD", Currency::USD).unwrap();
        
        let forward_1y = forward_fn(1.0);
        assert!(forward_1y > 0.0);
    }

    #[test]
    fn test_invalid_fx_pair() {
        let context = create_test_context();
        let result = forward_fn_fx(&context, "INVALID");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_market_data() {
        let context = MarketContext::new(); // Empty context
        
        let result = forward_fn_equity(&context, "SPY", Currency::USD);
        assert!(result.is_err()); // Should fail due to missing spot price
    }
}
