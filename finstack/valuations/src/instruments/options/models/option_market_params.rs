//! Option market parameters used by pricing models.

use crate::instruments::options::OptionType;
use finstack_core::F;

/// Option market parameters for pricing models.
///
/// Groups market data parameters commonly used in option pricing functions.
#[derive(Clone, Debug)]
pub struct OptionMarketParams {
    /// Current spot/forward price
    pub spot: F,
    /// Strike price
    pub strike: F,
    /// Risk-free rate
    pub rate: F,
    /// Volatility
    pub volatility: F,
    /// Time to expiry in years
    pub time_to_expiry: F,
    /// Dividend yield or cost of carry
    pub dividend_yield: F,
    /// Option type (Call/Put)
    pub option_type: OptionType,
}

impl OptionMarketParams {
    /// Create option market parameters
    pub fn new(
        spot: F,
        strike: F,
        rate: F,
        volatility: F,
        time_to_expiry: F,
        dividend_yield: F,
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

    /// Create call option market parameters
    pub fn call(spot: F, strike: F, rate: F, volatility: F, time_to_expiry: F) -> Self {
        Self::new(spot, strike, rate, volatility, time_to_expiry, 0.0, OptionType::Call)
    }

    /// Create put option market parameters
    pub fn put(spot: F, strike: F, rate: F, volatility: F, time_to_expiry: F) -> Self {
        Self::new(spot, strike, rate, volatility, time_to_expiry, 0.0, OptionType::Put)
    }

    /// Set dividend yield
    pub fn with_dividend_yield(mut self, dividend_yield: F) -> Self {
        self.dividend_yield = dividend_yield;
        self
    }
}
