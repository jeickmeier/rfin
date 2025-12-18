//! Per-instrument conventions for calibration quotes.
//!
//! Shared instrument-level conventions.

use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Per-instrument conventions for calibration.
///
/// These optional fields allow each quote to specify its own settlement,
/// payment, and fixing conventions. When not specified, the calibrator
/// uses safe defaults (e.g., T+2 settlement).
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct InstrumentConventions {
    /// Settlement lag in business days from trade date (e.g., 0 for T+0, 2 for T+2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement_days: Option<i32>,

    /// Payment delay in business days after period end
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_delay_days: Option<i32>,

    /// Reset lag as a **signed** business-day offset for floating rate fixings.
    ///
    /// Examples:
    /// - `-2` = fixing two business days **before** period start (T-2)
    /// - `+2` = fixing two business days **after** period start (T+2)
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

    /// Payment frequency for coupon schedules (overrides instrument defaults).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub payment_frequency: Option<Tenor>,

    /// Business day convention for date adjustments (e.g., ModifiedFollowing).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub business_day_convention: Option<BusinessDayConvention>,

    /// Day count convention (e.g., Act360, Act365F, Thirty360).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub day_count: Option<DayCount>,

    /// Explicit currency for this instrument (overrides market defaults).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub currency: Option<Currency>,

    /// Underlying index identifier (e.g., "USD-SOFR-3M") for float legs.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts_export", ts(type = "string | null"))]
    pub index: Option<IndexId>,

    /// Recovery rate assumption for credit instruments (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_rate: Option<f64>,
}

impl InstrumentConventions {
    /// Set the settlement lag in business days.
    pub fn with_settlement_days(mut self, days: i32) -> Self {
        self.settlement_days = Some(days);
        self
    }

    /// Set the payment delay in business days after period end.
    pub fn with_payment_delay(mut self, days: i32) -> Self {
        self.payment_delay_days = Some(days);
        self
    }

    /// Set the signed business-day reset lag.
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

    /// Create conventions with payment frequency.
    pub fn with_payment_frequency(mut self, freq: Tenor) -> Self {
        self.payment_frequency = Some(freq);
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

    /// Create conventions with currency.
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = Some(currency);
        self
    }

    /// Create conventions with index identifier.
    pub fn with_index(mut self, index: impl Into<IndexId>) -> Self {
        self.index = Some(index.into());
        self
    }

    /// Create conventions with recovery rate.
    pub fn with_recovery_rate(mut self, recovery_rate: f64) -> Self {
        self.recovery_rate = Some(recovery_rate);
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
            && self.payment_frequency.is_none()
            && self.business_day_convention.is_none()
            && self.day_count.is_none()
            && self.currency.is_none()
            && self.index.is_none()
            && self.recovery_rate.is_none()
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

    // =========================================================================
    // Currency-Specific Default Conventions
    // =========================================================================

    /// Default day count for money market instruments (deposits, FRAs) by currency.
    ///
    /// Market conventions:
    /// - GBP, JPY, AUD, NZD, HKD, SGD: ACT/365F
    /// - USD, EUR, CHF, CAD, and others: ACT/360
    #[inline]
    pub fn default_money_market_day_count(currency: Currency) -> DayCount {
        match currency {
            Currency::GBP
            | Currency::JPY
            | Currency::AUD
            | Currency::NZD
            | Currency::HKD
            | Currency::SGD => DayCount::Act365F,
            _ => DayCount::Act360,
        }
    }

    /// Default fixed leg day count for swaps by currency.
    ///
    /// Market conventions:
    /// - GBP: ACT/365F
    /// - EUR, CHF: 30/360 (ISDA)
    /// - USD, others: 30/360
    #[inline]
    pub fn default_fixed_leg_day_count(currency: Currency) -> DayCount {
        match currency {
            Currency::GBP => DayCount::Act365F,
            _ => DayCount::Thirty360,
        }
    }

    /// Default float leg day count for swaps by currency.
    ///
    /// Market conventions:
    /// - GBP: ACT/365F
    /// - USD, EUR, CHF, others: ACT/360
    #[inline]
    pub fn default_float_leg_day_count(currency: Currency) -> DayCount {
        match currency {
            Currency::GBP | Currency::JPY | Currency::AUD => DayCount::Act365F,
            _ => DayCount::Act360,
        }
    }

    /// Default fixed leg payment frequency for swaps by currency.
    ///
    /// Market conventions:
    /// - GBP (SONIA): Annual
    /// - USD, EUR, others: Semi-annual
    #[inline]
    pub fn default_fixed_leg_frequency(currency: Currency) -> Tenor {
        match currency {
            Currency::GBP => Tenor::annual(),
            _ => Tenor::semi_annual(),
        }
    }

    /// Default float leg payment frequency for swaps by currency.
    ///
    /// For OIS swaps, this is typically annual (paid at maturity for short tenors).
    /// For IBOR-style swaps, this matches the index tenor (e.g., 3M for 3M LIBOR).
    #[inline]
    pub fn default_float_leg_frequency(_currency: Currency) -> Tenor {
        // Default to quarterly for most swaps; OIS typically annual
        Tenor::quarterly()
    }

    /// Get effective day count, using provided value or currency default for money market.
    #[inline]
    pub fn effective_day_count_or_default(&self, currency: Currency) -> DayCount {
        self.day_count
            .unwrap_or_else(|| Self::default_money_market_day_count(currency))
    }

    /// Get effective payment frequency, using provided value or currency default.
    #[inline]
    pub fn effective_payment_frequency_or_default(
        &self,
        currency: Currency,
        is_fixed: bool,
    ) -> Tenor {
        self.payment_frequency.unwrap_or_else(|| {
            if is_fixed {
                Self::default_fixed_leg_frequency(currency)
            } else {
                Self::default_float_leg_frequency(currency)
            }
        })
    }

    /// Get effective day count for a swap leg, using provided value or currency default.
    #[inline]
    pub fn effective_swap_day_count_or_default(
        &self,
        currency: Currency,
        is_fixed: bool,
    ) -> DayCount {
        self.day_count.unwrap_or_else(|| {
            if is_fixed {
                Self::default_fixed_leg_day_count(currency)
            } else {
                Self::default_float_leg_day_count(currency)
            }
        })
    }

    /// Get the effective payment delay in business days after period end.
    ///
    /// Market convention: 0 by default unless explicitly provided on the quote.
    #[inline]
    pub fn effective_payment_delay_days(&self) -> i32 {
        self.payment_delay_days.unwrap_or(0)
    }

    /// Get the effective reset lag in business days before period start for fixings.
    ///
    /// Market convention: -2 by default (T-2 fixing); positive values indicate
    /// fixing after the accrual start.
    #[inline]
    pub fn effective_reset_lag_days(&self) -> i32 {
        self.reset_lag.unwrap_or(-2)
    }
}
