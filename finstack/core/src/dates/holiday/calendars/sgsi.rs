use crate::dates::holiday::rule::Rule;
use time::Month;

const CNY: Rule = Rule::ChineseNewYear;

/// Singapore inter-bank calendar (code: SGSI).
///
/// **Source**: Singapore inter-bank market holiday schedule for bond and money market operations.
///
/// **Observance Policy**:
/// - Fixed holidays use Monday substitution: New Year, Labour Day, National Day, Christmas
/// - Chinese New Year: 2-day block starting from lunar new year date
/// - Good Friday: Single day observance based on Easter calculation
/// - Weekend substitution: Holidays falling on weekends are moved to the following Monday
///
/// **Lunar Calendar Coverage**:
/// - Chinese New Year: Full 1970-2150 range supported using embedded lunar tables
/// - See `finstack/core/data/chinese_new_year.csv` for lunar date mappings
const SGSI_RULES: &[Rule] = &[
    // Fixed Gregorian holidays with Monday substitution
    Rule::fixed_next_monday(Month::January, 1), // New Year
    Rule::fixed_next_monday(Month::May, 1),     // Labour Day
    Rule::fixed_next_monday(Month::August, 9),  // National Day
    Rule::fixed_next_monday(Month::December, 25), // Christmas
    // Chinese New Year – first 2 days
    Rule::Span {
        start: &CNY,
        len: 2,
    },
    // Good Friday
    Rule::EasterOffset(-3),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Sgsi;

impl Sgsi {
    #[inline]
    pub const fn id(self) -> &'static str {
        "sgsi"
    }
}

crate::impl_calendar_generated!(Sgsi, "sgsi", SGSI_RULES);
