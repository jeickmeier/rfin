use crate::dates::holiday::rule::Rule;
use time::Month;

const JAN1: Rule = Rule::fixed(Month::January, 1);
const MAY1: Rule = Rule::fixed(Month::May, 1);
const OCT1: Rule = Rule::fixed(Month::October, 1);
const CNY: Rule = Rule::ChineseNewYear;

/// China inter-bank settlement calendar (code: CNBE).
///
/// **Source**: China inter-bank market holiday schedule for bond and money market operations.
///
/// **Observance Policy**:
/// - Fixed holidays use multi-day blocks (New Year: 3 days, Labour Day: 5 days, National Day: 7 days)
/// - Chinese New Year (Spring Festival): 7-day block starting from lunar new year date
/// - Qing Ming (Tomb Sweeping Day): Single day observance based on solar term calculation
/// - Weekend substitution: Holidays falling on weekends are included in the multi-day blocks
///
/// **Lunar Calendar Coverage**:
/// - Chinese New Year: Full 1970-2150 range supported using embedded lunar tables
/// - Qing Ming: Full 1970-2150 range supported using solar term calculations
/// - See `finstack/core/data/chinese_new_year.csv` for lunar date mappings
const CNBE_RULES: &[Rule] = &[
    // New Year – 3 day block starting 1 Jan
    Rule::Span {
        start: &JAN1,
        len: 3,
    },
    // Spring Festival – 7-day block from Chinese New Year
    Rule::Span {
        start: &CNY,
        len: 7,
    },
    // Qing Ming
    Rule::QingMing,
    // Labour Day – 5-day block from 1-May
    Rule::Span {
        start: &MAY1,
        len: 5,
    },
    // National Day – 7-day block from 1-Oct
    Rule::Span {
        start: &OCT1,
        len: 7,
    },
];

// Bitset macro using build-time CSV ordinals (falls back to rules if empty year).

#[derive(Debug, Clone, Copy, Default)]
pub struct Cnbe;

impl Cnbe {
    #[inline]
    pub const fn id(self) -> &'static str {
        "cnbe"
    }
}
crate::impl_calendar_generated!(Cnbe, "cnbe", CNBE_RULES);
