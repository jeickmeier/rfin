//! Option market parameters used by pricing models.

use super::market::OptionType;

/// Option market parameters for pricing models.
///
/// Groups market data parameters commonly used in option pricing functions.
#[derive(Clone, Debug)]
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

    /// Set dividend yield
    pub fn with_dividend_yield(mut self, dividend_yield: f64) -> Self {
        self.dividend_yield = dividend_yield;
        self
    }
}
