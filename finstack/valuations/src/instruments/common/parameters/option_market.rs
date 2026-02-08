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
