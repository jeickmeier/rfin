//! Per-instrument conventions for calibration quotes.

use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Per-instrument conventions for calibration.
///
/// These optional fields allow each quote to specify its own settlement,
/// payment, and fixing conventions. When not specified, the calibrator
/// uses safe defaults (e.g., T+2 settlement).
///
/// # Example
///
/// ```ignore
/// let conventions = InstrumentConventions {
///     settlement_days: Some(0),  // T+0 for this instrument
///     calendar_id: Some("gblo".to_string()),
///     business_day_convention: Some(BusinessDayConvention::ModifiedFollowing),
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

    /// Reset lag in business days for floating rate fixings (e.g., -2 for T-2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_lag: Option<i32>,

    /// General calendar identifier for schedule generation and business day adjustments.
    /// Used as fallback when specific calendars (fixing/payment) are not provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,

    /// Calendar identifier for fixing date adjustments (e.g., for floating rate resets).
    /// Falls back to `calendar_id` if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixing_calendar_id: Option<String>,

    /// Calendar identifier for payment date adjustments.
    /// Falls back to `calendar_id` if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_calendar_id: Option<String>,

    /// Reset frequency for floating legs (e.g., 3M for quarterly resets).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub reset_frequency: Option<Tenor>,

    /// Business day convention for date adjustments (e.g., ModifiedFollowing).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub business_day_convention: Option<BusinessDayConvention>,

    /// Day count convention (e.g., Act360, Act365F, Thirty360).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub day_count: Option<DayCount>,
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

    /// Create conventions with general calendar ID.
    pub fn with_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Create conventions with fixing calendar ID.
    pub fn with_fixing_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.fixing_calendar_id = Some(calendar_id.into());
        self
    }

    /// Create conventions with payment calendar ID.
    pub fn with_payment_calendar_id(mut self, calendar_id: impl Into<String>) -> Self {
        self.payment_calendar_id = Some(calendar_id.into());
        self
    }

    /// Create conventions with reset frequency.
    pub fn with_reset_frequency(mut self, freq: Tenor) -> Self {
        self.reset_frequency = Some(freq);
        self
    }

    /// Create conventions with business day convention.
    pub fn with_business_day_convention(mut self, bdc: BusinessDayConvention) -> Self {
        self.business_day_convention = Some(bdc);
        self
    }

    /// Create conventions with day count.
    pub fn with_day_count(mut self, dc: DayCount) -> Self {
        self.day_count = Some(dc);
        self
    }

    /// Check if all fields are None (i.e., use defaults).
    pub fn is_empty(&self) -> bool {
        self.settlement_days.is_none()
            && self.payment_delay_days.is_none()
            && self.reset_lag.is_none()
            && self.calendar_id.is_none()
            && self.fixing_calendar_id.is_none()
            && self.payment_calendar_id.is_none()
            && self.reset_frequency.is_none()
            && self.business_day_convention.is_none()
            && self.day_count.is_none()
    }

    /// Get the effective fixing calendar ID (falls back to general calendar_id).
    pub fn effective_fixing_calendar_id(&self) -> Option<&str> {
        self.fixing_calendar_id
            .as_deref()
            .or(self.calendar_id.as_deref())
    }

    /// Get the effective payment calendar ID (falls back to general calendar_id).
    pub fn effective_payment_calendar_id(&self) -> Option<&str> {
        self.payment_calendar_id
            .as_deref()
            .or(self.calendar_id.as_deref())
    }
}

