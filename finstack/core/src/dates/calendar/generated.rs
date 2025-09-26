//! Generated holidays support: fast year-indexed bitsets and helper macro.
//!
//! Design:
//! - Years covered: BASE_YEAR..=END_YEAR (configured via build-generated constants)
//! - Precomputed 366-bit bitsets per year are generated at build-time for fast O(1) lookup.
//! - Years outside the range fall back to evaluating rules at runtime for correctness.

use time::{Date, Duration, Month, Weekday};

// Rule import no longer needed at runtime; kept for IDE hints in generated constants.

// Include generated constants directly from src/generated for IDE discoverability.
include!("../../generated/holiday_generated.rs");

/// Number of years covered.
pub const YEARS: usize = (END_YEAR - BASE_YEAR + 1) as usize;

/// Words needed to cover 366 bits.
pub const BITSET_WORDS: usize = (366 + 63) / 64; // 6 u64 words

/// Bitset type for one year (366 bits).
pub type YearBits = [u64; BITSET_WORDS];

#[inline]
#[allow(missing_docs)]
pub fn day_of_year_0_based(date: Date) -> u16 {
    let jan1 = Date::from_calendar_date(date.year(), Month::January, 1).unwrap();
    (date - jan1).whole_days() as u16
}

#[inline]
#[allow(missing_docs)]
pub fn bit_test(bits: &YearBits, idx: u16) -> bool {
    let i = idx as usize;
    let word = i >> 6;
    let off = i & 63;
    ((bits[word] >> off) & 1) == 1
}

// Build-time precomputation provides static bitsets; no runtime materialization.

/// Helper to compute nth weekday of month.
#[inline]
pub fn nth_weekday_of_month(year: i32, month: Month, weekday: Weekday, n: i8) -> Date {
    if n > 0 {
        let mut d = Date::from_calendar_date(year, month, 1).unwrap();
        while d.weekday() != weekday {
            d += Duration::days(1);
        }
        d + Duration::weeks((n as i64) - 1)
    } else {
        let (ny, nm) = if month == Month::December {
            (year + 1, Month::January)
        } else {
            (year, Month::try_from(month as u8 + 1).unwrap())
        };
        let mut d = Date::from_calendar_date(ny, nm, 1).unwrap() - Duration::days(1);
        while d.weekday() != weekday {
            d -= Duration::days(1);
        }
        let pos = (-n) as i64; // 1=last, 2=second-last
        d - Duration::weeks(pos - 1)
    }
}
