//! Interest rate option specific parameters.
//!
//! This module groups parameters used to construct cap/floor instruments.
//! It mirrors the structure used across other instruments (e.g., `cds`).

use super::types::RateOptionType;
use finstack_core::{
    dates::{BusinessDayConvention, DayCount, Frequency, StubKind},
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
    /// Stub convention for schedule generation
    pub stub_kind: StubKind,
    /// Business day convention for schedule generation
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier for schedule generation
    pub calendar_id: Option<&'static str>,
}

impl InterestRateOptionParams {
    /// Create cap parameters
    pub fn cap(notional: Money, strike_rate: F, frequency: Frequency, day_count: DayCount) -> Self {
        Self {
            rate_option_type: RateOptionType::Cap,
            notional,
            strike_rate,
            frequency,
            day_count,
            stub_kind: StubKind::None,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
        }
    }

    /// Create floor parameters
    pub fn floor(
        notional: Money,
        strike_rate: F,
        frequency: Frequency,
        day_count: DayCount,
    ) -> Self {
        Self {
            rate_option_type: RateOptionType::Floor,
            notional,
            strike_rate,
            frequency,
            day_count,
            stub_kind: StubKind::None,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
        }
    }
}
