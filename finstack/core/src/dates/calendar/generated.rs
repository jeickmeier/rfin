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

/// Words needed to cover 366 bits.
pub const BITSET_WORDS: usize = 366_usize.div_ceil(64); // 6 u64 words

/// Bitset type for one year (366 bits).
pub type YearBits = [u64; BITSET_WORDS];

#[inline]
/// Return the zero-based day-of-year index for `date`.
///
/// This helper is used to address the precomputed holiday bitsets, where
/// January 1 maps to `0` and December 31 maps to `364` or `365` depending on
/// whether the year is a leap year.
pub fn day_of_year_0_based(date: Date) -> u16 {
    date.ordinal() - 1
}

#[inline]
/// Test whether the bit at `idx` is set in a yearly holiday bitset.
///
/// The index is expected to come from [`day_of_year_0_based`] and therefore
/// address one of the 366 possible calendar days in a Gregorian year.
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
        let mut d = Date::from_calendar_date(year, month, 1)
            .unwrap_or_else(|_| unreachable!("first day of month is a valid Gregorian date"));
        while d.weekday() != weekday {
            d += Duration::days(1);
        }
        d + Duration::weeks((n as i64) - 1)
    } else {
        let (ny, nm) = if month == Month::December {
            (year + 1, Month::January)
        } else {
            (
                year,
                Month::try_from(month as u8 + 1).unwrap_or_else(|_| {
                    unreachable!("successor month exists for non-December months")
                }),
            )
        };
        let mut d = Date::from_calendar_date(ny, nm, 1).unwrap_or_else(|_| {
            unreachable!("first day of successor month is a valid Gregorian date")
        }) - Duration::days(1);
        while d.weekday() != weekday {
            d -= Duration::days(1);
        }
        let pos = (-n) as i64; // 1=last, 2=second-last
        d - Duration::weeks(pos - 1)
    }
}
