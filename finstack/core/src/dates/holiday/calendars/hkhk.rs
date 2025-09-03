use crate::dates::holiday::rule::Rule;
use time::Month;

const CNY: Rule = Rule::ChineseNewYear;

/// Hong Kong banking calendar (code: HKHK).
///
/// **Source**: Hong Kong banking and financial market holiday schedule.
///
/// **Observance Policy**:
/// - Fixed holidays: New Year, Labour Day, HKSAR Establishment Day, National Day, Christmas, Boxing Day
/// - Chinese New Year: 3-day block starting from lunar new year date
/// - Qing Ming (Tomb Sweeping Day): Single day observance based on solar term calculation
/// - Buddha's Birthday: Single day observance based on lunar calendar
/// - Weekend substitution: Holidays falling on weekends are not substituted (no make-up days)
///
/// **Lunar Calendar Coverage**:
/// - Chinese New Year: Full 1970-2150 range supported using embedded lunar tables
/// - Qing Ming: Full 1970-2150 range supported using solar term calculations
/// - Buddha's Birthday: Full 1970-2150 range supported using lunar calendar calculations
/// - See `finstack/core/data/chinese_new_year.csv` for lunar date mappings
const HKHK_RULES: &[Rule] = &[
    // Fixed-date Gregorian holidays
    Rule::fixed(Month::January, 1),
    Rule::fixed(Month::May, 1),
    Rule::fixed(Month::July, 1),
    Rule::fixed(Month::October, 1),
    Rule::fixed(Month::December, 25),
    Rule::fixed(Month::December, 26),
    // Lunar/solar term holidays
    Rule::Span {
        start: &CNY,
        len: 3,
    },
    Rule::QingMing,
    Rule::BuddhasBirthday,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Hkhk;

impl Hkhk {
    #[inline]
    pub const fn id(self) -> &'static str {
        "hkhk"
    }
}

crate::impl_calendar_generated!(Hkhk, "hkhk", HKHK_RULES);
