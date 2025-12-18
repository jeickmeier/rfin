//! Per-instrument conventions for calibration quotes.
//!
//! Shared instrument-level conventions.

use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};
#[cfg(feature = "ts_export")]
use ts_rs::TS;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::json_registry::{build_lookup_map_mapped, RegistryFile};


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

// ============================================================================
// Instrument-centric convention registries
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DepositConventions {
    pub calendar_id: String,
    pub settlement_days: i32,
    pub business_day_convention: BusinessDayConvention,
    pub day_count: DayCount,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DepositConventionsRecord {
    calendar_id: String,
    settlement_days: i32,
    business_day_convention: BusinessDayConvention,
    day_count: DayCount,
}

impl DepositConventionsRecord {
    fn into_conventions(self) -> DepositConventions {
        DepositConventions {
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            business_day_convention: self.business_day_convention,
            day_count: self.day_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FraConventions {
    pub calendar_id: String,
    pub settlement_days: i32,
    pub business_day_convention: BusinessDayConvention,
    pub day_count: DayCount,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FraConventionsRecord {
    calendar_id: String,
    settlement_days: i32,
    business_day_convention: BusinessDayConvention,
    day_count: DayCount,
}

impl FraConventionsRecord {
    fn into_conventions(self) -> FraConventions {
        FraConventions {
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            business_day_convention: self.business_day_convention,
            day_count: self.day_count,
        }
    }
}

fn normalize_currency_key(id: &str) -> String {
    id.trim().to_uppercase()
}

fn deposit_registry() -> &'static HashMap<String, DepositConventions> {
    static REGISTRY: OnceLock<HashMap<String, DepositConventions>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let json = include_str!("../../../data/conventions/deposit_conventions.json");
        let file: RegistryFile<DepositConventionsRecord> =
            serde_json::from_str(json).expect("Failed to parse embedded deposit conventions JSON");
        build_lookup_map_mapped(file, normalize_currency_key, |rec| rec.clone().into_conventions())
    })
}

fn fra_registry() -> &'static HashMap<String, FraConventions> {
    static REGISTRY: OnceLock<HashMap<String, FraConventions>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let json = include_str!("../../../data/conventions/fra_conventions.json");
        let file: RegistryFile<FraConventionsRecord> =
            serde_json::from_str(json).expect("Failed to parse embedded FRA conventions JSON");
        build_lookup_map_mapped(file, normalize_currency_key, |rec| rec.clone().into_conventions())
    })
}

impl DepositConventions {
    pub(crate) fn for_currency(currency: Currency) -> &'static Self {
        let key = currency.to_string();
        deposit_registry()
            .get(&key)
            .or_else(|| deposit_registry().get("DEFAULT"))
            .unwrap_or_else(|| {
                panic!(
                    "Missing deposit conventions for '{}' and missing DEFAULT entry",
                    key
                )
            })
    }

    /// Resolve deposit conventions using an optional index key (e.g., "USD-SOFR", "EUR-ESTR-OIS").
    ///
    /// Resolution order:
    /// 1) `index` (if provided) against `deposit_conventions.json` entry IDs
    /// 2) `currency` against `deposit_conventions.json` entry IDs
    /// 3) "DEFAULT"
    pub(crate) fn for_currency_or_index(currency: Currency, index: Option<&IndexId>) -> &'static Self {
        if let Some(index) = index {
            let key = normalize_currency_key(index.as_str());
            if let Some(found) = deposit_registry().get(&key) {
                return found;
            }
        }
        Self::for_currency(currency)
    }
}

impl FraConventions {
    pub(crate) fn for_currency(currency: Currency) -> &'static Self {
        let key = currency.to_string();
        fra_registry()
            .get(&key)
            .or_else(|| fra_registry().get("DEFAULT"))
            .unwrap_or_else(|| {
                panic!(
                    "Missing FRA conventions for '{}' and missing DEFAULT entry",
                    key
                )
            })
    }
}
