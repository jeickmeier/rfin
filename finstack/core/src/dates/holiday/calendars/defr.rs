use crate::dates::holiday::rule::Rule;
use time::Month;

/// German XETRA stock-exchange calendar (code: DEFR).
///
/// **Source**: German XETRA electronic trading system holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays: New Year, Labour Day, Christmas (2 days)
/// - Easter holidays: Good Friday, Easter Monday
/// - No weekend substitution: Holidays are observed on their actual dates regardless of day of week
/// - Minimal holiday set: Only major national holidays observed
///
/// **Coverage**: Full year range supported (1970-2150).
const DEFR_RULES: &[Rule] = &[
    Rule::fixed(Month::January, 1),   // New Year
    Rule::EasterOffset(-3),           // Good Friday
    Rule::EasterOffset(0),            // Easter Monday
    Rule::fixed(Month::May, 1),       // Labour Day
    Rule::fixed(Month::December, 25), // Christmas
    Rule::fixed(Month::December, 26), // St. Stephen
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Defr;

impl Defr {
    #[inline]
    pub const fn id(self) -> &'static str {
        "defr"
    }
}

crate::impl_calendar_generated!(Defr, "defr", DEFR_RULES);
