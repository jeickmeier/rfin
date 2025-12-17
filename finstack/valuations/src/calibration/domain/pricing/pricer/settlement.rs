//! Settlement and start-date logic for `CalibrationPricer`.

use super::CalibrationPricer;
use crate::calibration::domain::quotes::InstrumentConventions;
use finstack_core::dates::{adjust, CalendarRegistry, Date, DateExt};
use finstack_core::types::Currency;

use super::super::convention_resolution as conv;

impl CalibrationPricer {
    /// Resolve settlement date using strictly provided quote conventions (no defaults).
    pub fn settlement_date_for_quote_strict(
        &self,
        quote_conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        let settled = conv::resolve_settlement_strict(quote_conventions, currency)?;
        let days = settled.settlement_days;
        let calendar_id = settled.calendar_id;

        let registry = CalendarRegistry::global();
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                adjust(self.base_date, settled.bdc, calendar)
            } else {
                let spot = self.base_date.add_business_days(days, calendar)?;
                adjust(spot, settled.bdc, calendar)
            }
        } else {
            Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("calendar '{}'", calendar_id),
                },
            ))
        }
    }

    /// Get the effective start date for instruments.
    pub fn effective_start_date(
        &self,
        conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        if self.use_settlement_start {
            if self.strict_pricing {
                self.settlement_date_for_quote_explicit(conventions, currency)
            } else {
                self.settlement_date_for_quote(conventions, currency)
            }
        } else {
            Ok(self.base_date)
        }
    }

    /// Calculate settlement date from base date using business-day calendar.
    pub fn settlement_date(&self, currency: Currency) -> finstack_core::Result<Date> {
        self.settlement_date_for_quote(&InstrumentConventions::default(), currency)
    }

    /// Calculate settlement date for a specific quote's conventions.
    pub fn settlement_date_for_quote(
        &self,
        quote_conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        let common = conv::resolve_common(self, quote_conventions, currency);
        let days = common.settlement_days;
        let calendar_id = common.calendar_id;
        let bdc = common.bdc;

        let registry = CalendarRegistry::global();

        // If we have a valid calendar, use business-day arithmetic
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                // T+0: just ensure base_date is a business day (use consistent BDC)
                adjust(self.base_date, bdc, calendar)
            } else {
                // Add business days and adjust result
                let spot = self.base_date.add_business_days(days, calendar)?;
                // Final adjustment ensures we land on a business day
                adjust(spot, bdc, calendar)
            }
        } else if self.allow_calendar_fallback {
            // Fallback: calendar not found, use calendar-day addition with warning.
            tracing::warn!(
                calendar_id = calendar_id,
                currency = ?currency,
                "Calendar not found, falling back to calendar-day settlement"
            );
            Ok(if days == 0 {
                self.base_date
            } else {
                self.base_date + time::Duration::days(days as i64)
            })
        } else {
            Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("calendar '{}'", calendar_id),
                },
            ))
        }
    }

    /// Calculate settlement date for a specific quote using only explicitly provided conventions.
    ///
    /// Resolution order is: quote conventions → pricer (step-level) conventions.
    /// No currency-based defaults are applied. If still missing, returns a validation error.
    pub fn settlement_date_for_quote_explicit(
        &self,
        quote_conventions: &InstrumentConventions,
        currency: Currency,
    ) -> finstack_core::Result<Date> {
        let days = quote_conventions
            .settlement_days
            .or(self.settlement_days)
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Strict pricing requires settlement_days to be set (quote or step)".to_string(),
                )
            })?;

        let calendar_id = quote_conventions
            .calendar_id
            .as_deref()
            .or(self.calendar_id.as_deref())
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Strict pricing requires calendar_id to be set (quote or step)".to_string(),
                )
            })?;

        let bdc = quote_conventions
            .business_day_convention
            .or(self.business_day_convention)
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Strict pricing requires business_day_convention to be set (quote or step)"
                        .to_string(),
                )
            })?;

        let registry = CalendarRegistry::global();
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            if days == 0 {
                adjust(self.base_date, bdc, calendar)
            } else {
                let spot = self.base_date.add_business_days(days, calendar)?;
                adjust(spot, bdc, calendar)
            }
        } else if self.allow_calendar_fallback {
            tracing::warn!(
                calendar_id = calendar_id,
                currency = ?currency,
                "Calendar not found, falling back to calendar-day settlement (strict pricing)"
            );
            Ok(if days == 0 {
                self.base_date
            } else {
                self.base_date + time::Duration::days(days as i64)
            })
        } else {
            Err(finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: format!("calendar '{}'", calendar_id),
                },
            ))
        }
    }

    /// Compute FRA fixing date using signed reset lag and calendars.
    ///
    /// Returns the fixing date and a flag indicating whether the calendar was found.
    pub(in crate::calibration::domain::pricing::pricer) fn compute_fra_fixing_date(
        &self,
        start: Date,
        reset_lag: i32,
        calendar_id: &str,
        allow_calendar_fallback: bool,
    ) -> finstack_core::Result<(Date, bool)> {
        let registry = CalendarRegistry::global();
        if let Some(calendar) = registry.resolve_str(calendar_id) {
            let fixing_date = if start >= self.base_date {
                start.add_business_days(reset_lag, calendar)?
            } else {
                self.base_date
            };
            Ok((fixing_date, true))
        } else if allow_calendar_fallback {
            let candidate = start + time::Duration::days(reset_lag as i64);
            let fixing_date = if candidate >= self.base_date {
                candidate
            } else {
                self.base_date
            };
            Ok((fixing_date, false))
        } else {
            Err(finstack_core::Error::calendar_not_found_with_suggestions(
                calendar_id.to_string(),
                registry.available_ids(),
            ))
        }
    }
}


