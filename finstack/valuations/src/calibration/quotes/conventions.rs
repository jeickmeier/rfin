//! Per-instrument conventions for calibration quotes.

#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Per-instrument conventions for calibration.
///
/// These optional fields allow each quote to specify its own settlement,
/// payment, and fixing conventions. When not specified, the calibrator
/// uses currency-specific defaults.
///
/// # Example
///
/// ```ignore
/// let conventions = InstrumentConventions {
///     settlement_days: Some(0),  // T+0 for this instrument
///     calendar_id: Some("gblo".to_string()),
///     ..Default::default()
/// };
/// ```
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(default)]
pub struct InstrumentConventions {
    /// Settlement lag in business days from trade date (e.g., 0 for T+0, 2 for T+2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_days: Option<i32>,
    /// Payment delay in business days after period end
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_delay_days: Option<i32>,
    /// Reset lag in business days for floating rate fixings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_lag: Option<i32>,
    /// Calendar identifier for schedule generation and business day adjustments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
}

impl InstrumentConventions {
    /// Create conventions with settlement days.
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Create conventions with payment delay.
    pub fn with_payment_delay(mut self, days: i32) -> Self {
        self.payment_delay_days = Some(days);
        self
    }

    /// Create conventions with reset lag.
    pub fn with_reset_lag(mut self, days: i32) -> Self {
        self.reset_lag = Some(days);
        self
    }

    /// Create conventions with calendar ID.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Check if all fields are None (i.e., use defaults).
    pub fn is_empty(&self) -> bool {
        self.settlement_days.is_none()
            && self.payment_delay_days.is_none()
            && self.reset_lag.is_none()
            && self.calendar_id.is_none()
    }
}

