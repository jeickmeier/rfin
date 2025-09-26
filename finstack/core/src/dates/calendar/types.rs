//! Clean calendar implementation using a single Calendar struct.

use super::business_days::HolidayCalendar;
use super::generated::{bit_test, day_of_year_0_based, YearBits, BASE_YEAR, END_YEAR};
use super::rule::Rule;
use crate::dates::DateExt;
use time::Date;

/// A holiday calendar implementation that uses rule-based evaluation
/// with optional precomputed bitsets for performance.
#[derive(Debug, Clone, Copy)]
pub struct Calendar {
    /// Unique identifier (e.g., "target2", "gblo")
    pub id: &'static str,

    /// Display name for the calendar
    pub name: &'static str,

    /// Whether weekends should be ignored in is_holiday checks
    pub ignore_weekends: bool,

    /// The rules defining this calendar's holidays
    pub rules: &'static [Rule],

    /// Optional precomputed bitsets for fast lookup (1970-2150)
    pub bitsets: Option<&'static [YearBits]>,
}

impl Calendar {
    /// Create a new calendar with the given parameters.
    pub const fn new(
        id: &'static str,
        name: &'static str,
        ignore_weekends: bool,
        rules: &'static [Rule],
    ) -> Self {
        Self {
            id,
            name,
            ignore_weekends,
            rules,
            bitsets: None,
        }
    }

    /// Add precomputed bitsets to this calendar for fast lookup.
    pub const fn with_bitsets(mut self, bitsets: &'static [YearBits]) -> Self {
        self.bitsets = Some(bitsets);
        self
    }

    /// Get the calendar identifier.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Get the calendar display name.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl HolidayCalendar for Calendar {
    fn is_holiday(&self, date: Date) -> bool {
        // Use fast bitset lookup if available and date is in range
        if let Some(bitsets) = self.bitsets {
            if (BASE_YEAR..=END_YEAR).contains(&date.year()) {
                let year_idx = (date.year() - BASE_YEAR) as usize;
                if year_idx < bitsets.len() {
                    let day_idx = day_of_year_0_based(date);
                    let mut is_holiday = bit_test(&bitsets[year_idx], day_idx);

                    // Apply weekend ignore logic
                    if self.ignore_weekends && date.is_weekend() {
                        is_holiday = false;
                    }

                    return is_holiday;
                }
            }
        }

        // Fall back to rule-based evaluation for dates outside bitset range
        // or when bitsets are not available
        let mut is_holiday = self.rules.iter().any(|rule| rule.applies(date));

        // Apply weekend ignore logic
        if self.ignore_weekends && date.is_weekend() {
            is_holiday = false;
        }

        is_holiday
    }
}

// Also implement HolidayCalendar for &Calendar for convenience
impl HolidayCalendar for &Calendar {
    fn is_holiday(&self, date: Date) -> bool {
        (*self).is_holiday(date)
    }
}
