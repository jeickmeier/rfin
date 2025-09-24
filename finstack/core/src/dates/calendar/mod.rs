//! Holiday calendar DSL – unified design and semantics.
//!
//! ## Supported Date Range
//!
//! Holiday calendars are optimized for years **1970-2150** using generated bitsets.
//! Years outside this range fall back to runtime rule evaluation.
//!
//! **Chinese New Year (CNY) Coverage**: All CNY-dependent calendars (CNBE, HKHK, SGSI)
//! now support the full 1970-2150 range through externally-sourced data.
//! Previously, CNY was limited to 1990-2100, causing silent degradation outside that range.
//!
//! ## Semantics
//!
//! - "Holiday" refers to non-working dates as defined by a specific market
//!   calendar. Many calendars also label weekends as holidays for convenience,
//!   while some intentionally ignore weekends in `is_holiday`.
//! - Independent of the above, [`crate::dates::calendar::HolidayCalendar::is_business_day`]
//!   always treats Saturday/Sunday as non-business days and defers to
//!   `is_holiday` for market-specific closures.
//! - Prefer `is_business_day` for scheduling and adjustment logic. Use
//!   [`crate::dates::calendar::is_weekend`] if you need to only detect Saturday/Sunday.

pub mod composite;
pub mod core;
pub mod generated;
pub mod registry;
pub mod rule;

// ----------------------------------------------------------------------------------------------
// Macros used by generated calendar code
// ----------------------------------------------------------------------------------------------

/// Macro to define thin delegate calendars that mirror another calendar's rules.
#[macro_export]
macro_rules! impl_calendar_delegate {
    ($ty:ident, $id:expr, $delegate:ident) => {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $ty;
        impl $ty {
            #[inline]
            pub const fn id(self) -> &'static str {
                $id
            }
        }
        impl $crate::dates::calendar::HolidayCalendar for $ty {
            fn is_holiday(&self, date: $crate::dates::Date) -> bool {
                $delegate.is_holiday(date)
            }
        }
    };
}

/// Implement [`HolidayCalendar`](crate::dates::calendar::HolidayCalendar) using
/// generated rule-based bitsets for fast lookup.
///
/// - `ignore_weekends = false` (default): `is_holiday` may return `true` on
///   Saturdays/Sundays if the generated rules/bitsets include them (common for
///   many calendars).
/// - `ignore_weekends = true`: `is_holiday` will return `false` on
///   Saturdays/Sundays regardless of rule bits. This is useful for calendars
///   that intentionally do not label weekends as holidays.
///
/// Note: This setting only affects `is_holiday`. Business-day semantics always
/// exclude weekends via the blanket `is_business_day` implementation.
#[macro_export]
#[allow(missing_docs)]
macro_rules! impl_calendar_generated {
    ($ty:ident, $id:literal, $rules:path) => {
        $crate::impl_calendar_generated!($ty, $id, $rules, ignore_weekends = false);
    };
    ($ty:ident, $id:literal, $rules:path, ignore_weekends = $ignore_weekends:expr) => {
        impl $crate::dates::calendar::HolidayCalendar for $ty {
            fn is_holiday(&self, date: time::Date) -> bool {
                if ($crate::dates::calendar::generated::BASE_YEAR
                    ..=$crate::dates::calendar::generated::END_YEAR)
                    .contains(&date.year())
                {
                    static STORE: once_cell::sync::Lazy<
                        Vec<
                            once_cell::sync::OnceCell<$crate::dates::calendar::generated::YearBits>,
                        >,
                    > = once_cell::sync::Lazy::new(|| {
                        let mut v = Vec::with_capacity($crate::dates::calendar::generated::YEARS);
                        for _ in 0..$crate::dates::calendar::generated::YEARS {
                            v.push(once_cell::sync::OnceCell::new());
                        }
                        v
                    });
                    let idx =
                        (date.year() - $crate::dates::calendar::generated::BASE_YEAR) as usize;
                    let bits = STORE[idx].get_or_init(|| {
                        $crate::dates::calendar::generated::compute_year_bits_for_rules(
                            $rules,
                            date.year(),
                        )
                    });
                    let mut is_h = $crate::dates::calendar::generated::bit_test(
                        bits,
                        $crate::dates::calendar::generated::day_of_year_0_based(date),
                    );
                    if $ignore_weekends
                        && matches!(
                            date.weekday(),
                            time::Weekday::Saturday | time::Weekday::Sunday
                        )
                    {
                        is_h = false;
                    }
                    return is_h;
                }
                let mut res = $rules.is_holiday(date);
                if $ignore_weekends
                    && matches!(
                        date.weekday(),
                        time::Weekday::Saturday | time::Weekday::Sunday
                    )
                {
                    res = false;
                }
                res
            }
        }
    };
}

/// Macro that declares a calendar type, its id method, and wires it to the
/// generated rules using `impl_calendar_generated!`. This reduces boilerplate
/// in the generated registry file.
#[macro_export]
macro_rules! declare_calendar {
    ($ty:ident, $id:literal, $rules:path) => {
        #[allow(missing_docs)]
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $ty;
        #[allow(missing_docs)]
        impl $ty {
            #[inline]
            pub const fn id(self) -> &'static str {
                $id
            }
        }
        $crate::impl_calendar_generated!($ty, $id, $rules);
    };
    ($ty:ident, $id:literal, $rules:path, ignore_weekends = $ignore_weekends:expr) => {
        #[allow(missing_docs)]
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $ty;
        #[allow(missing_docs)]
        impl $ty {
            #[inline]
            pub const fn id(self) -> &'static str {
                $id
            }
        }
        $crate::impl_calendar_generated!($ty, $id, $rules, ignore_weekends = $ignore_weekends);
    };
}

// Re-export commonly used items for ergonomic imports.
pub use core::{adjust, available_calendars, BusinessDayConvention, HolidayCalendar};
pub use rule::{Direction, Observed, Rule};

// Include generated calendar types and registry helpers.
// Prefer static file; fallback to OUT_DIR when feature is enabled.
#[cfg(not(feature = "use_out_dir_generated"))]
include!("../../generated/calendar_generated.rs");
#[cfg(feature = "use_out_dir_generated")]
include!(concat!(env!("OUT_DIR"), "/generated_calendars.rs"));
