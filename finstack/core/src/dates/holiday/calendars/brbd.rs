use crate::dates::holiday::rule::Rule;
use time::Month;

/// Brazil B3 exchange holiday calendar (code: BRBD).
///
/// **Source**: Brazil B3 (Brasil, Bolsa, Balcão) exchange official holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays: New Year, Tiradentes Day, Labour Day, Independence Day, Our Lady of Aparecida, All Souls' Day, Republic Day, Black Consciousness Day, Christmas
/// - Moveable holidays: Good Friday, Carnival Monday/Tuesday, Corpus Christi
/// - **Special Rule**: Market ignores weekends for holiday purposes (if holiday falls on Sat/Sun, market remains open)
/// - No weekend substitution: Holidays are observed on their actual dates regardless of day of week
///
/// **Coverage**: Full year range supported (1970-2150).
const BRBD_RULES: &[Rule] = &[
    // Fixed-date holidays (no substitution)
    Rule::fixed(Month::January, 1),
    Rule::fixed(Month::April, 21),
    Rule::fixed(Month::May, 1),
    Rule::fixed(Month::September, 7),
    Rule::fixed(Month::October, 12),
    Rule::fixed(Month::November, 2),
    Rule::fixed(Month::November, 15),
    Rule::fixed(Month::November, 20),
    Rule::fixed(Month::December, 25),
    // Moveable holidays
    Rule::EasterOffset(-3),  // Good Friday
    Rule::EasterOffset(-49), // Carnival Monday
    Rule::EasterOffset(-48), // Carnival Tuesday
    Rule::EasterOffset(59),  // Corpus Christi
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Brbd;

impl Brbd {
    #[inline]
    pub const fn id(self) -> &'static str {
        "brbd"
    }
}

// BRBD ignores weekends for holiday purposes.
crate::impl_calendar_generated!(Brbd, "brbd", BRBD_RULES, ignore_weekends = true);
