//! Interest rate option specific parameters.

use super::types::RateOptionType;
use finstack_core::{
    dates::{DayCount, Frequency},
    money::Money,
    F,
};

/// Interest rate option specific parameters.
///
/// Groups parameters specific to interest rate options (caps/floors).
#[derive(Clone, Debug)]
pub struct InterestRateOptionParams {
    /// Type of rate option (Cap/Floor)
    pub rate_option_type: RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate
    pub strike_rate: F,
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
}

impl InterestRateOptionParams {
    /// Create new interest rate option parameters
    pub fn new(
        rate_option_type: RateOptionType,
        notional: Money,
        strike_rate: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self {
            rate_option_type,
            notional,
            strike_rate,
            frequency,
            day_count,
        }
    }

    /// Create cap parameters
    pub fn cap(notional: Money, strike_rate: F, frequency: Frequency, day_count: DayCount) -> Self {
        Self::new(
            RateOptionType::Cap,
            notional,
            strike_rate,
            frequency,
            day_count,
        )
    }

    /// Create floor parameters
    pub fn floor(
        notional: Money,
        strike_rate: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self::new(
            RateOptionType::Floor,
            notional,
            strike_rate,
            frequency,
            day_count,
        )
    }
}
