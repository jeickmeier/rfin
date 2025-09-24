//! Generated holidays support: fast year-indexed bitsets and helper macro.
//!
//! Design:
//! - Years covered: BASE_YEAR..=END_YEAR (configured via build-generated constants)
//! - For each calendar, we lazily compute a 366-bit bitset per year from its Rule slice
//!   on first use, then serve O(1) lookups with no locking.
//! - Years outside the range fall back to evaluating rules at runtime for correctness.

use smallvec::SmallVec;
use time::{Date, Duration, Month, Weekday};

use crate::dates::calendar::rule::Rule;

// Prefer static checked-in generated file; fall back to OUT_DIR for legacy.
#[cfg(not(feature = "use_out_dir_generated"))]
include!("../../generated/holiday_generated.rs");
#[cfg(feature = "use_out_dir_generated")]
include!(concat!(env!("OUT_DIR"), "/holiday_generated.rs"));

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

#[inline]
fn bit_set(bits: &mut YearBits, idx: u16) {
    let i = idx as usize;
    let word = i >> 6;
    let off = i & 63;
    bits[word] |= 1u64 << off;
}

/// Compute the 366-bit bitset for a given year from the provided rules.
pub fn compute_year_bits_for_rules(rules: &[Rule], year: i32) -> YearBits {
    let mut out: YearBits = [0u64; BITSET_WORDS];

    // Materialize rule dates for the year using the Rule DSL directly.
    let mut dates: SmallVec<[Date; 64]> = SmallVec::new();
    for r in rules.iter() {
        r.materialize_year(year, &mut dates);
    }
    // Deduplicate in case rules overlap.
    dates.sort_unstable();
    dates.dedup();

    // Set bits for each holiday day and any spans covered by rules that spill across years.
    for d in dates {
        if d.year() == year {
            bit_set(&mut out, day_of_year_0_based(d));
        }
    }

    out
}

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
