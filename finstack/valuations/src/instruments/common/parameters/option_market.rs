//! Option market parameters used by pricing models.

use super::market::OptionType;
use finstack_core::types::{Percentage, Rate};

/// Option market parameters for pricing models.
///
/// Groups market data parameters commonly used in option pricing functions.
#[derive(Debug, Clone)]
pub struct OptionMarketParams {
    /// Current spot/forward price
    pub spot: f64,
    /// Strike price
    pub strike: f64,
    /// Risk-free rate
    pub rate: f64,
    /// Volatility
    pub volatility: f64,
    /// Time to expiry in years
    pub time_to_expiry: f64,
    /// Dividend yield or cost of carry
    pub dividend_yield: f64,
    /// Option type (Call/Put)
    pub option_type: OptionType,
}

impl OptionMarketParams {
    /// Create option market parameters
    pub fn new(
        spot: f64,
        strike: f64,
        rate: f64,
        volatility: f64,
        time_to_expiry: f64,
        dividend_yield: f64,
        option_type: OptionType,
    ) -> Self {
        Self {
            spot,
            strike,
            rate,
            volatility,
            time_to_expiry,
            dividend_yield,
            option_type,
        }
    }

    /// Create option market parameters using typed rates/volatility.
    pub fn new_typed(
        spot: f64,
        strike: f64,
        rate: Rate,
        volatility: Percentage,
        time_to_expiry: f64,
        dividend_yield: Percentage,
        option_type: OptionType,
    ) -> Self {
        Self {
            spot,
            strike,
            rate: rate.as_decimal(),
            volatility: volatility.as_decimal(),
            time_to_expiry,
            dividend_yield: dividend_yield.as_decimal(),
            option_type,
        }
    }

    /// Create call option market parameters
    pub fn call(spot: f64, strike: f64, rate: f64, volatility: f64, time_to_expiry: f64) -> Self {
        Self::new(
            spot,
            strike,
            rate,
            volatility,
            time_to_expiry,
            0.0,
            OptionType::Call,
        )
    }

    /// Create call option market parameters using typed rates/volatility.
    pub fn call_typed(
        spot: f64,
        strike: f64,
        rate: Rate,
        volatility: Percentage,
        time_to_expiry: f64,
    ) -> Self {
        Self {
            spot,
            strike,
            rate: rate.as_decimal(),
            volatility: volatility.as_decimal(),
            time_to_expiry,
            dividend_yield: Percentage::ZERO.as_decimal(),
            option_type: OptionType::Call,
        }
    }

    /// Create put option market parameters
    pub fn put(spot: f64, strike: f64, rate: f64, volatility: f64, time_to_expiry: f64) -> Self {
        Self::new(
            spot,
            strike,
            rate,
            volatility,
            time_to_expiry,
            0.0,
            OptionType::Put,
        )
    }

    /// Create put option market parameters using typed rates/volatility.
    pub fn put_typed(
        spot: f64,
        strike: f64,
        rate: Rate,
        volatility: Percentage,
        time_to_expiry: f64,
    ) -> Self {
        Self {
            spot,
            strike,
            rate: rate.as_decimal(),
            volatility: volatility.as_decimal(),
            time_to_expiry,
            dividend_yield: Percentage::ZERO.as_decimal(),
            option_type: OptionType::Put,
        }
    }

    /// Set dividend yield
    pub fn with_dividend_yield(mut self, dividend_yield: f64) -> Self {
        self.dividend_yield = dividend_yield;
        self
    }

    /// Set dividend yield using a typed percentage.
    pub fn with_dividend_yield_pct(mut self, dividend_yield: Percentage) -> Self {
        self.dividend_yield = dividend_yield.as_decimal();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_and_typed_builders_set_all_fields() {
        let plain = OptionMarketParams::new(100.0, 95.0, 0.03, 0.25, 1.5, 0.01, OptionType::Put);
        assert_eq!(plain.spot, 100.0);
        assert_eq!(plain.strike, 95.0);
        assert_eq!(plain.rate, 0.03);
        assert_eq!(plain.volatility, 0.25);
        assert_eq!(plain.time_to_expiry, 1.5);
        assert_eq!(plain.dividend_yield, 0.01);
        assert_eq!(plain.option_type, OptionType::Put);

        let typed = OptionMarketParams::new_typed(
            100.0,
            105.0,
            Rate::from_percent(4.0),
            Percentage::new(20.0),
            2.0,
            Percentage::new(1.5),
            OptionType::Call,
        );
        assert!((typed.rate - 0.04).abs() < 1e-12);
        assert!((typed.volatility - 0.20).abs() < 1e-12);
        assert!((typed.dividend_yield - 0.015).abs() < 1e-12);
        assert_eq!(typed.option_type, OptionType::Call);
    }

    #[test]
    fn call_and_put_helpers_default_dividend_yield_to_zero() {
        let call = OptionMarketParams::call(100.0, 100.0, 0.05, 0.30, 1.0);
        let put = OptionMarketParams::put(100.0, 100.0, 0.05, 0.30, 1.0);
        let typed_call = OptionMarketParams::call_typed(
            100.0,
            100.0,
            Rate::from_bps(500),
            Percentage::new(30.0),
            1.0,
        );
        let typed_put = OptionMarketParams::put_typed(
            100.0,
            100.0,
            Rate::from_bps(500),
            Percentage::new(30.0),
            1.0,
        );

        for params in [call, put, typed_call, typed_put] {
            assert_eq!(params.dividend_yield, 0.0);
        }
    }

    #[test]
    fn dividend_yield_setters_override_existing_value() {
        let plain =
            OptionMarketParams::call(100.0, 100.0, 0.05, 0.30, 1.0).with_dividend_yield(0.02);
        let typed = OptionMarketParams::put(100.0, 100.0, 0.05, 0.30, 1.0)
            .with_dividend_yield_pct(Percentage::new(2.5));

        assert!((plain.dividend_yield - 0.02).abs() < 1e-12);
        assert!((typed.dividend_yield - 0.025).abs() < 1e-12);
    }
}
