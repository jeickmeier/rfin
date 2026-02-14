//! Contract specifications and general parameter types.

use serde::{Deserialize, Serialize};

/// Contract size information for derivatives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSpec {
    /// Number of units per contract
    pub contract_size: f64,
    /// Optional multiplier for pricing
    pub multiplier: Option<f64>,
}

impl ContractSpec {
    /// Create a new contract specification
    pub fn new(contract_size: f64) -> Self {
        Self {
            contract_size,
            multiplier: None,
        }
    }

    /// Standard single-unit contract
    pub fn unit() -> Self {
        Self::new(1.0)
    }

    /// Set the contract multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = Some(multiplier);
        self
    }
}

impl Default for ContractSpec {
    fn default() -> Self {
        Self::unit()
    }
}

/// Schedule specification for payment periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    /// Start date for the schedule
    pub start: Date,
    /// End date for the schedule
    pub end: Date,
    /// Payment frequency
    pub frequency: Tenor,
    /// Stub period handling
    #[serde(default = "crate::serde_defaults::stub_short_front")]
    pub stub: StubKind,
    /// Business day convention
    #[serde(default = "crate::serde_defaults::bdc_modified_following")]
    pub bdc: BusinessDayConvention,
    /// Optional calendar for adjustments
    pub calendar_id: Option<&'static str>,
}

impl ScheduleSpec {
    /// Create a new schedule specification
    pub fn new(start: Date, end: Date, frequency: Tenor) -> Self {
        Self {
            start,
            end,
            frequency,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
        }
    }

    /// Set business day convention
    pub fn with_bdc(mut self, bdc: BusinessDayConvention) -> Self {
        self.bdc = bdc;
        self
    }

    /// Set stub handling
    pub fn with_stub(mut self, stub: StubKind) -> Self {
        self.stub = stub;
        self
    }

    /// Set calendar for adjustments
    pub fn with_calendar(mut self, calendar_id: &'static str) -> Self {
        self.calendar_id = Some(calendar_id);
        self
    }
}

// Need to import these for the ScheduleSpec
use finstack_core::dates::{BusinessDayConvention, Date, StubKind, Tenor};
