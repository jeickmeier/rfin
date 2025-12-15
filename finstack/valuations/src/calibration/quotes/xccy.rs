//! Cross-currency basis swap quotes.

use super::conventions::InstrumentConventions;
use crate::instruments::xccy_swap::NotionalExchange;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::types::{Currency, CurveId};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Which leg receives the quoted basis spread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
pub enum SpreadOn {
    /// Spread is applied to the domestic leg.
    Domestic,
    /// Spread is applied to the foreign leg (common quoting).
    Foreign,
}

/// Market quote for an XCCY basis swap used in curve calibration.
///
/// All conventions can be customized via `InstrumentConventions`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
pub struct XccyBasisQuote {
    /// Swap maturity date.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub maturity: Date,
    /// Basis spread quote in basis points.
    pub spread_bp: f64,
    /// Domestic currency (reporting currency for PV=0).
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub domestic_currency: Currency,
    /// Foreign currency (curve being calibrated).
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub foreign_currency: Currency,
    /// Domestic notional.
    pub domestic_notional: f64,
    /// Foreign notional.
    pub foreign_notional: f64,
    /// Domestic discount curve identifier (must exist in market).
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub domestic_discount_curve_id: CurveId,
    /// Domestic projection forward curve identifier.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub domestic_forward_curve_id: CurveId,
    /// Foreign projection forward curve identifier.
    #[cfg_attr(feature = "ts_export", ts(type = "string"))]
    pub foreign_forward_curve_id: CurveId,

    /// Where the basis spread is applied.
    pub spread_on: SpreadOn,
    /// Principal exchange convention.
    pub notional_exchange: NotionalExchange,

    /// Instrument-wide conventions (settlement, calendar, etc.).
    #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
    pub conventions: InstrumentConventions,

    /// Domestic leg specific conventions.
    #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
    pub domestic_leg_conventions: InstrumentConventions,

    /// Foreign leg specific conventions.
    #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
    pub foreign_leg_conventions: InstrumentConventions,
}

impl XccyBasisQuote {
    /// Get the effective domestic day count.
    pub fn domestic_day_count(&self) -> DayCount {
        self.domestic_leg_conventions
            .effective_swap_day_count_or_default(self.domestic_currency, false)
    }

    /// Get the effective foreign day count.
    pub fn foreign_day_count(&self) -> DayCount {
        self.foreign_leg_conventions
            .effective_swap_day_count_or_default(self.foreign_currency, false)
    }

    /// Get the effective domestic frequency.
    pub fn domestic_frequency(&self) -> Tenor {
        self.domestic_leg_conventions
            .effective_payment_frequency_or_default(self.domestic_currency, false)
    }

    /// Get the effective foreign frequency.
    pub fn foreign_frequency(&self) -> Tenor {
        self.foreign_leg_conventions
            .effective_payment_frequency_or_default(self.foreign_currency, false)
    }

    /// Get the effective domestic business day convention.
    pub fn domestic_bdc(&self) -> BusinessDayConvention {
        self.domestic_leg_conventions
            .business_day_convention
            .unwrap_or(BusinessDayConvention::ModifiedFollowing)
    }

    /// Get the effective foreign business day convention.
    pub fn foreign_bdc(&self) -> BusinessDayConvention {
        self.foreign_leg_conventions
            .business_day_convention
            .unwrap_or(BusinessDayConvention::ModifiedFollowing)
    }

    /// Get the effective domestic payment lag.
    pub fn domestic_payment_lag(&self) -> i32 {
        self.domestic_leg_conventions
            .payment_delay_days
            .unwrap_or(0)
    }

    /// Get the effective foreign payment lag.
    pub fn foreign_payment_lag(&self) -> i32 {
        self.foreign_leg_conventions.payment_delay_days.unwrap_or(0)
    }

    /// Get the effective domestic calendar.
    ///
    /// Falls back to instrument-wide calendar if not specified on the leg.
    pub fn domestic_calendar_id(&self) -> Option<&str> {
        self.domestic_leg_conventions
            .calendar_id
            .as_deref()
            .or(self.conventions.calendar_id.as_deref())
    }

    /// Get the effective foreign calendar.
    ///
    /// Falls back to instrument-wide calendar if not specified on the leg.
    pub fn foreign_calendar_id(&self) -> Option<&str> {
        self.foreign_leg_conventions
            .calendar_id
            .as_deref()
            .or(self.conventions.calendar_id.as_deref())
    }

    /// Get the effective spot lag.
    pub fn spot_lag_days(&self) -> u32 {
        self.conventions.settlement_days.unwrap_or(2) as u32
    }

    /// Get the effective spot business day convention.
    pub fn spot_bdc(&self) -> BusinessDayConvention {
        self.conventions
            .business_day_convention
            .unwrap_or(BusinessDayConvention::Following)
    }
}
