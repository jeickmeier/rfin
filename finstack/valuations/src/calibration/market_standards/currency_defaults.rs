//! Currency-specific default conventions for calibration.
//!
//! Provides market-standard defaults for settlement, day count, and calendar
//! conventions by currency. These are used as fallbacks when quotes do not
//! specify per-instrument conventions.

use finstack_core::dates::DayCount;
use finstack_core::prelude::*;

/// Get settlement days for a currency (default if index not found).
///
/// Market-standard settlement conventions:
/// - USD: T+2
/// - EUR: T+2
/// - GBP: T+0
/// - JPY: T+2
/// - Others: T+2 (default)
pub fn settlement_days_for_currency(currency: Currency) -> i32 {
    match currency {
        Currency::GBP => 0,                 // GBP settles same-day
        Currency::AUD | Currency::CAD => 1, // T+1 for AUD/CAD
        _ => 2,                             // T+2 for USD, EUR, JPY, CHF, and others
    }
}

/// Get the standard day count convention for a currency's discount curve.
///
/// Market conventions:
/// - USD, EUR, CHF: ACT/360
/// - GBP, JPY, AUD, CAD: ACT/365F
pub fn standard_day_count_for_currency(currency: Currency) -> DayCount {
    match currency {
        Currency::GBP | Currency::JPY | Currency::AUD | Currency::CAD | Currency::NZD => {
            DayCount::Act365F
        }
        _ => DayCount::Act360,
    }
}

/// Get the default settlement calendar ID for a currency.
///
/// Market-standard settlement calendars used for spot/settlement date calculation:
/// - USD: "usny" (US Federal Reserve / New York)
/// - EUR: "target2" (TARGET2 / ECB / SEPA)
/// - GBP: "gblo" (London Stock Exchange / UK Bank Holidays)
/// - JPY: "jpto" (Tokyo Stock Exchange / Japan)
/// - CHF: "chzu" (Zurich / Switzerland)
/// - AUD: "ausy" (Sydney / Australia)
/// - CAD: "cato" (Toronto / Canada)
/// - Others: "usny" (default to US calendar)
///
/// These IDs correspond to calendars in the `CalendarRegistry`.
pub fn default_calendar_for_currency(currency: Currency) -> &'static str {
    match currency {
        Currency::USD => "usny",
        Currency::EUR => "target2",
        Currency::GBP => "gblo",
        Currency::JPY => "jpto",
        Currency::CHF => "chzu",
        Currency::AUD => "ausy",
        Currency::CAD => "cato",
        Currency::NZD => "nzau", // Auckland/Wellington
        Currency::HKD => "hkex",
        Currency::SGD => "sgex",
        _ => "usny", // Default to US calendar for unlisted currencies
    }
}

