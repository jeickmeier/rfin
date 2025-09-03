use crate::dates::holiday::rule::Rule;
use time::Month;

/// European TARGET2 settlement calendar (code: TARGET2).
///
/// **Source**: European Central Bank (ECB) TARGET2 payment system holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays: New Year, Labour Day, Christmas (2 days)
/// - Easter holidays: Good Friday, Easter Monday
/// - No weekend substitution: Holidays are observed on their actual dates regardless of day of week
/// - Eurozone-wide: Applies to all TARGET2 participating countries
/// - Minimal holiday set: Only major holidays that affect payment system operations
///
/// **Coverage**: Full year range supported (1970-2150).
const TARGET2_RULES: &[Rule] = &[
    Rule::fixed(Month::January, 1),   // New Year's Day
    Rule::EasterOffset(-3),           // Good Friday
    Rule::EasterOffset(0),            // Easter Monday
    Rule::fixed(Month::May, 1),       // Labour Day
    Rule::fixed(Month::December, 25), // Christmas Day
    Rule::fixed(Month::December, 26), // Boxing Day
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Target2;

impl Target2 {
    #[inline]
    pub const fn id(self) -> &'static str {
        "target2"
    }
}

crate::impl_calendar_generated!(Target2, "target2", TARGET2_RULES);
