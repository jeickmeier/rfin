//! Holiday calendar DSL – new unified design.

pub mod calendars;
pub mod rule;
pub mod generated;

// Re-export commonly used items for ergonomic imports.
pub use rule::{Direction, Observed, Rule};

// Convenience alias so users can `use finstack_core::dates::holiday::Calendar`.
// We re-export the existing `HolidayCalendar` trait from the parent module.
// This keeps the public API surface small while allowing direct usage.
//
// Example:
//
// fn foo(cal: &impl Calendar) { /* ... */ }

pub use crate::dates::calendar::HolidayCalendar as Calendar;

// Re-export most used calendars at holiday root level
pub use calendars::*;

// Export a macro so calendar modules can use it without path gymnastics.
#[macro_export]
#[allow(missing_docs)]
macro_rules! impl_calendar_generated {
    ($ty:ident, $id:literal, $rules:path) => {
        $crate::impl_calendar_generated!($ty, $id, $rules, ignore_weekends = false);
    };
    ($ty:ident, $id:literal, $rules:path, ignore_weekends = $ignore_weekends:expr) => {
        impl $crate::dates::calendar::HolidayCalendar for $ty {
            fn is_holiday(&self, date: time::Date) -> bool {
                if ($crate::dates::holiday::generated::BASE_YEAR..=$crate::dates::holiday::generated::END_YEAR)
                    .contains(&date.year())
                {
                    static STORE: once_cell::sync::Lazy<
                        Vec<once_cell::sync::OnceCell<$crate::dates::holiday::generated::YearBits>>,
                    > = once_cell::sync::Lazy::new(|| {
                        let mut v = Vec::with_capacity($crate::dates::holiday::generated::YEARS);
                        for _ in 0..$crate::dates::holiday::generated::YEARS {
                            v.push(once_cell::sync::OnceCell::new());
                        }
                        v
                    });
                    let idx = (date.year() - $crate::dates::holiday::generated::BASE_YEAR) as usize;
                    let bits = STORE[idx].get_or_init(|| {
                        $crate::dates::holiday::generated::compute_year_bits_for_rules($rules, date.year())
                    });
                    let mut is_h = $crate::dates::holiday::generated::bit_test(
                        bits,
                        $crate::dates::holiday::generated::day_of_year_0_based(date),
                    );
                    if $ignore_weekends
                        && matches!(date.weekday(), time::Weekday::Saturday | time::Weekday::Sunday)
                    {
                        is_h = false;
                    }
                    return is_h;
                }
                let mut res = $rules.is_holiday(date);
                if $ignore_weekends
                    && matches!(date.weekday(), time::Weekday::Saturday | time::Weekday::Sunday)
                {
                    res = false;
                }
                res
            }
        }
    };
}

// Export a macro for calendars driven by build-time CSV ordinals.
#[macro_export]
#[allow(missing_docs)]
macro_rules! impl_calendar_generated_from_ords {
    ($ty:ident, $id:literal, $ords:path, $offs:path, $rules:path) => {
        impl $crate::dates::calendar::HolidayCalendar for $ty {
            fn is_holiday(&self, date: time::Date) -> bool {
                if ($crate::dates::holiday::generated::BASE_YEAR..=$crate::dates::holiday::generated::END_YEAR)
                    .contains(&date.year())
                {
                    static STORE: once_cell::sync::Lazy<
                        Vec<once_cell::sync::OnceCell<$crate::dates::holiday::generated::YearBits>>,
                    > = once_cell::sync::Lazy::new(|| {
                        let mut v = Vec::with_capacity($crate::dates::holiday::generated::YEARS);
                        for _ in 0..$crate::dates::holiday::generated::YEARS {
                            v.push(once_cell::sync::OnceCell::new());
                        }
                        v
                    });
                    let idx = (date.year() - $crate::dates::holiday::generated::BASE_YEAR) as usize;
                    let bits = STORE[idx].get_or_init(|| {
                        let start = $offs[idx] as usize;
                        let end = $offs[idx + 1] as usize;
                        let mut b: $crate::dates::holiday::generated::YearBits = [0u64; $crate::dates::holiday::generated::BITSET_WORDS];
                        if start < end {
                            for &doy in &$ords[start..end] {
                                let i = doy as usize;
                                b[i >> 6] |= 1u64 << (i & 63);
                            }
                        } else {
                            // Fallback to rules if no CSV entries for this year.
                            let tmp = $crate::dates::holiday::generated::compute_year_bits_for_rules($rules, (idx as i32) + $crate::dates::holiday::generated::BASE_YEAR);
                            b = tmp;
                        }
                        b
                    });
                    return $crate::dates::holiday::generated::bit_test(bits, $crate::dates::holiday::generated::day_of_year_0_based(date));
                }
                $rules.is_holiday(date)
            }
        }
    };
}
