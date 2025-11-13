//! Core types and common engine for Total Return Swaps.

use crate::cashflow::builder::date_generation::build_dates;
use crate::cashflow::builder::ScheduleParams;
use finstack_core::dates::Date;

/// Side of the TRS trade from the party's perspective.
///
/// See unit tests and `examples/` for usage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrsSide {
    /// Receive total return, pay financing.
    ReceiveTotalReturn,
    /// Pay total return, receive financing.
    PayTotalReturn,
}

impl std::fmt::Display for TrsSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrsSide::ReceiveTotalReturn => write!(f, "receive_total_return"),
            TrsSide::PayTotalReturn => write!(f, "pay_total_return"),
        }
    }
}

impl std::str::FromStr for TrsSide {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "receive_total_return" | "receive" => Ok(TrsSide::ReceiveTotalReturn),
            "pay_total_return" | "pay" => Ok(TrsSide::PayTotalReturn),
            other => Err(format!("Unknown TRS side: {}", other)),
        }
    }
}

impl TrsSide {
    /// Gets the sign multiplier for present value calculation.
    ///
    /// # Returns
    /// 1.0 for ReceiveTotalReturn, -1.0 for PayTotalReturn.
    pub fn sign(&self) -> f64 {
        match self {
            TrsSide::ReceiveTotalReturn => 1.0,
            TrsSide::PayTotalReturn => -1.0,
        }
    }
}

/// Schedule specification for TRS payment periods.
///
/// Defines the payment schedule and frequency for both legs of the TRS.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct TrsScheduleSpec {
    /// Start date for the TRS leg.
    pub start: Date,
    /// End date for the TRS leg.
    pub end: Date,
    /// Schedule parameters (frequency, day count, bdc, calendar, stub).
    pub params: ScheduleParams,
}

impl TrsScheduleSpec {
    /// Creates a schedule specification from start/end dates and schedule parameters.
    pub fn from_params(start: Date, end: Date, schedule: ScheduleParams) -> Self {
        Self {
            start,
            end,
            params: schedule,
        }
    }

    /// Builds the period date schedule in a canonical way.
    pub fn period_schedule(&self) -> crate::cashflow::builder::date_generation::PeriodSchedule {
        build_dates(
            self.start,
            self.end,
            self.params.freq,
            self.params.stub,
            self.params.bdc,
            self.params.calendar_id.as_deref(),
        )
    }
}

// Re-export common parameter types for backward compatibility
pub use crate::instruments::common::parameters::legs::{FinancingLegSpec, TotalReturnLegSpec};
pub use crate::instruments::common::parameters::underlying::IndexUnderlyingParams;
