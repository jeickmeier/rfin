//! Shared dividend schedules for equity/ETF underlyings.
//!
//! Stores dated dividend events that can be referenced by multiple instruments
//! via `MarketContext`. Supports cash, proportional yield, and stock dividends.
//! Schedules are keyed by a `CurveId` in the market context for consistency
//! with other market data components.

use crate::currency::Currency;
use crate::dates::Date;
use crate::money::Money;
use crate::types::CurveId;
use crate::{Error, Result, F};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Type of dividend event.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DividendKind {
    /// Fixed cash amount in a currency.
    Cash(Money),
    /// Proportional yield (fraction per annum) to be applied over an accrual.
    /// This is metadata for models that approximate dividends as a yield.
    Yield(F),
    /// Stock dividend specified as a ratio (e.g., 0.05 = 5% stock dividend).
    Stock {
        /// Stock distribution ratio; 0.05 corresponds to a 5% stock dividend.
        ratio: F,
    },
}

/// A dated dividend event.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DividendEvent {
    /// Ex-dividend date.
    pub date: Date,
    /// Event kind.
    pub kind: DividendKind,
}

/// Shared dividend schedule identified by `CurveId` (e.g., "AAPL-DIVS").
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DividendSchedule {
    /// Unique identifier of this schedule in the market context.
    pub id: CurveId,
    /// Optional display symbol/ticker for convenience.
    pub underlying: Option<String>,
    /// Sorted events by date (ascending).
    pub events: Vec<DividendEvent>,
    /// Quote currency for cash dividends (optional metadata).
    pub currency: Option<Currency>,
}

impl DividendSchedule {
    /// Create a new empty schedule with identifier `id`.
    pub fn new(id: impl Into<CurveId>) -> Self {
        Self {
            id: id.into(),
            underlying: None,
            events: Vec::new(),
            currency: None,
        }
    }

    /// Set human-readable underlying/ticker symbol.
    pub fn with_underlying(mut self, underlying: impl Into<String>) -> Self {
        self.underlying = Some(underlying.into());
        self
    }

    /// Set the default currency for cash dividends.
    pub fn with_currency(mut self, ccy: Currency) -> Self {
        self.currency = Some(ccy);
        self
    }

    /// Add a cash dividend event.
    pub fn add_cash(mut self, date: Date, amount: Money) -> Self {
        self.events.push(DividendEvent {
            date,
            kind: DividendKind::Cash(amount),
        });
        self
    }

    /// Add a proportional yield event (metadata for models using yields).
    pub fn add_yield(mut self, date: Date, dividend_yield: F) -> Self {
        self.events.push(DividendEvent {
            date,
            kind: DividendKind::Yield(dividend_yield),
        });
        self
    }

    /// Add a stock dividend event given a ratio (e.g., 0.05 for 5%).
    pub fn add_stock(mut self, date: Date, ratio: F) -> Self {
        self.events.push(DividendEvent {
            date,
            kind: DividendKind::Stock { ratio },
        });
        self
    }

    /// Sort events by date ascending; call after bulk insertion.
    pub fn sort_by_date(&mut self) {
        self.events.sort_by_key(|e| e.date);
    }

    /// Return events filtered to a date range inclusive.
    pub fn events_between(&self, start: Date, end: Date) -> Vec<&DividendEvent> {
        self.events
            .iter()
            .filter(|e| e.date >= start && e.date <= end)
            .collect()
    }

    /// Convenience: cash dividends only (ignoring yield/stock entries).
    pub fn cash_events(&self) -> impl Iterator<Item = (Date, &Money)> {
        self.events.iter().filter_map(|e| match &e.kind {
            DividendKind::Cash(m) => Some((e.date, m)),
            _ => None,
        })
    }

    /// Validate schedule content (positive cash amounts, non-negative ratios).
    pub fn validate(&self) -> Result<()> {
        for ev in &self.events {
            match &ev.kind {
                DividendKind::Cash(m) => {
                    if m.amount() < 0.0 {
                        return Err(Error::Input(crate::error::InputError::NegativeValue));
                    }
                }
                DividendKind::Yield(y) => {
                    if !y.is_finite() {
                        return Err(Error::Input(crate::error::InputError::Invalid));
                    }
                }
                DividendKind::Stock { ratio } => {
                    if *ratio < 0.0 {
                        return Err(Error::Input(crate::error::InputError::NegativeValue));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Builder for [`DividendSchedule`].
pub struct DividendScheduleBuilder {
    id: CurveId,
    underlying: Option<String>,
    currency: Option<Currency>,
    events: Vec<DividendEvent>,
}

impl DividendScheduleBuilder {
    /// Start a new builder with identifier `id`.
    pub fn new(id: impl Into<CurveId>) -> Self {
        Self {
            id: id.into(),
            underlying: None,
            currency: None,
            events: Vec::new(),
        }
    }

    /// Optional underlying/ticker.
    pub fn underlying(mut self, name: impl Into<String>) -> Self {
        self.underlying = Some(name.into());
        self
    }

    /// Optional default currency for cash dividends.
    pub fn currency(mut self, ccy: Currency) -> Self {
        self.currency = Some(ccy);
        self
    }

    /// Add a cash dividend.
    pub fn cash(mut self, date: Date, amount: Money) -> Self {
        self.events.push(DividendEvent {
            date,
            kind: DividendKind::Cash(amount),
        });
        self
    }

    /// Add a yield dividend.
    pub fn yield_div(mut self, date: Date, y: F) -> Self {
        self.events.push(DividendEvent {
            date,
            kind: DividendKind::Yield(y),
        });
        self
    }

    /// Add a stock dividend.
    pub fn stock(mut self, date: Date, ratio: F) -> Self {
        self.events.push(DividendEvent {
            date,
            kind: DividendKind::Stock { ratio },
        });
        self
    }

    /// Build the schedule (events are sorted by date).
    pub fn build(mut self) -> Result<DividendSchedule> {
        self.events.sort_by_key(|e| e.date);
        let schedule = DividendSchedule {
            id: self.id,
            underlying: self.underlying,
            events: self.events,
            currency: self.currency,
        };
        schedule.validate()?;
        Ok(schedule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    #[test]
    fn build_and_filter_schedule() {
        let d1 = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let d2 = Date::from_calendar_date(2025, Month::March, 15).unwrap();
        let d3 = Date::from_calendar_date(2025, Month::June, 15).unwrap();

        let sched = DividendScheduleBuilder::new("AAPL-DIVS")
            .underlying("AAPL")
            .cash(d1, Money::new(0.24, Currency::USD))
            .cash(d2, Money::new(0.24, Currency::USD))
            .stock(d3, 0.02)
            .build()
            .unwrap();

        assert_eq!(sched.events.len(), 3);
        let between = sched.events_between(d1, d2);
        assert_eq!(between.len(), 2);
    }
}
