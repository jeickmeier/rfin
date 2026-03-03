//! Interest rate option specific parameters.
//!
//! This module groups parameters used to construct cap/floor instruments.
//! It mirrors the structure used across other instruments (e.g., `cds`).

use super::types::RateOptionType;
use finstack_core::{
    dates::{BusinessDayConvention, DayCount, StubKind, Tenor},
    money::Money,
    types::Rate,
};
use rust_decimal::Decimal;

/// Interest rate option specific parameters.
///
/// Groups parameters specific to interest rate options (caps/floors).
#[derive(Debug, Clone)]
pub struct InterestRateOptionParams {
    /// Type of rate option (Cap/Floor)
    pub rate_option_type: RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate
    pub strike: Decimal,
    /// Payment frequency
    pub frequency: Tenor,
    /// Day count convention
    pub day_count: DayCount,
    /// Stub convention for schedule generation
    pub stub: StubKind,
    /// Business day convention for schedule generation
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier for schedule generation
    pub calendar_id: Option<&'static str>,
}

impl InterestRateOptionParams {
    /// Create cap parameters
    pub fn cap(notional: Money, strike: f64, frequency: Tenor, day_count: DayCount) -> Self {
        Self {
            rate_option_type: RateOptionType::Cap,
            notional,
            strike: Decimal::try_from(strike).unwrap_or_default(),
            frequency,
            day_count,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
        }
    }

    /// Create cap parameters using a typed strike rate.
    pub fn cap_rate(notional: Money, strike: Rate, frequency: Tenor, day_count: DayCount) -> Self {
        Self {
            rate_option_type: RateOptionType::Cap,
            notional,
            strike: Decimal::try_from(strike.as_decimal()).unwrap_or_default(),
            frequency,
            day_count,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
        }
    }

    /// Create floor parameters
    pub fn floor(notional: Money, strike: f64, frequency: Tenor, day_count: DayCount) -> Self {
        Self {
            rate_option_type: RateOptionType::Floor,
            notional,
            strike: Decimal::try_from(strike).unwrap_or_default(),
            frequency,
            day_count,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
        }
    }

    /// Create floor parameters using a typed strike rate.
    pub fn floor_rate(
        notional: Money,
        strike: Rate,
        frequency: Tenor,
        day_count: DayCount,
    ) -> Self {
        Self {
            rate_option_type: RateOptionType::Floor,
            notional,
            strike: Decimal::try_from(strike.as_decimal()).unwrap_or_default(),
            frequency,
            day_count,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
        }
    }
}
