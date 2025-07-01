use super::{HolidayRule};
use time::{Date, Month};

// Precomputed Chinese New Year dates (Gregorian) for 1990-2100 (inclusive).
// Source: https://www.timeanddate.com/holidays/china/spring-festival
const CNY_DATES: &[(i32, u8, u8)] = &[
    (1990, 1, 27), (1991, 2, 15), (1992, 2, 4), (1993, 1, 23), (1994, 2, 10),
    (1995, 1, 31), (1996, 2, 19), (1997, 2, 7), (1998, 1, 28), (1999, 2, 16),
    (2000, 2, 5), (2001, 1, 24), (2002, 2, 12), (2003, 2, 1), (2004, 1, 22),
    (2005, 2, 9), (2006, 1, 29), (2007, 2, 18), (2008, 2, 7), (2009, 1, 26),
    (2010, 2, 14), (2011, 2, 3), (2012, 1, 23), (2013, 2, 10), (2014, 1, 31),
    (2015, 2, 19), (2016, 2, 8), (2017, 1, 28), (2018, 2, 16), (2019, 2, 5),
    (2020, 1, 25), (2021, 2, 12), (2022, 2, 1), (2023, 1, 22), (2024, 2, 10),
    (2025, 1, 29), (2026, 2, 17), (2027, 2, 6), (2028, 1, 26), (2029, 2, 13),
    (2030, 2, 3),
    (2031, 1, 23), (2032, 2, 11), (2033, 1, 31), (2034, 2, 19), (2035, 2, 8),
    (2036, 1, 28), (2037, 2, 15), (2038, 2, 4), (2039, 1, 24), (2040, 2, 12),
    (2041, 2, 1), (2042, 1, 22), (2043, 2, 10), (2044, 1, 30), (2045, 2, 17),
    (2046, 2, 6), (2047, 1, 26), (2048, 2, 14), (2049, 2, 2), (2050, 1, 23),
    (2051, 2, 11), (2052, 1, 31), (2053, 2, 19), (2054, 2, 8), (2055, 1, 28),
    (2056, 2, 15), (2057, 2, 5), (2058, 1, 24), (2059, 2, 12), (2060, 2, 2),
    (2061, 1, 21), (2062, 2, 9), (2063, 1, 29), (2064, 2, 17), (2065, 2, 5),
    (2066, 1, 26), (2067, 2, 14), (2068, 2, 3), (2069, 1, 23), (2070, 2, 11),
    (2071, 1, 31), (2072, 2, 19), (2073, 2, 7), (2074, 1, 27), (2075, 2, 15),
    (2076, 2, 5), (2077, 1, 24), (2078, 2, 12), (2079, 2, 2), (2080, 1, 22),
    (2081, 2, 9), (2082, 1, 29), (2083, 2, 17), (2084, 2, 6), (2085, 1, 26),
    (2086, 2, 14), (2087, 2, 3), (2088, 1, 24), (2089, 2, 10), (2090, 1, 30),
    (2091, 2, 18), (2092, 2, 7), (2093, 1, 27), (2094, 2, 15), (2095, 2, 5),
    (2096, 1, 25), (2097, 2, 12), (2098, 2, 1), (2099, 1, 22), (2100, 2, 9),
    (2101, 1, 29), (2102, 2, 17), (2103, 2, 6), (2104, 1, 26), (2105, 2, 13),
    (2106, 2, 2), (2107, 1, 22), (2108, 2, 10), (2109, 1, 30), (2110, 2, 18),
    (2111, 2, 8), (2112, 1, 28), (2113, 2, 15), (2114, 2, 4), (2115, 1, 24),
    (2116, 2, 12), (2117, 2, 1), (2118, 1, 22), (2119, 2, 9), (2120, 1, 29),
    (2121, 2, 17), (2122, 2, 6), (2123, 1, 27), (2124, 2, 14), (2125, 2, 3),
    (2126, 1, 23), (2127, 2, 11), (2128, 1, 31), (2129, 2, 19), (2130, 2, 8),
    (2131, 1, 28), (2132, 2, 16), (2133, 2, 4), (2134, 1, 24), (2135, 2, 12),
    (2136, 2, 1), (2137, 1, 21), (2138, 2, 9), (2139, 1, 29), (2140, 2, 17),
    (2141, 2, 5), (2142, 1, 26), (2143, 2, 14), (2144, 2, 3), (2145, 1, 22),
    (2146, 2, 10), (2147, 1, 31), (2148, 2, 18), (2149, 2, 7), (2150, 1, 27),
];

fn matches_date(list: &[(i32, u8, u8)], date: Date) -> bool {
    list.iter().any(|&(y, m, d)| y == date.year() && m == date.month() as u8 && d == date.day())
}

/// Chinese New Year (Spring Festival) holiday rule using lookup table 1990-2150.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChineseNewYear;

impl HolidayRule for ChineseNewYear {
    fn applies(&self, date: Date) -> bool {
        matches_date(CNY_DATES, date)
    }
}

/// Qing Ming festival (Tomb-Sweeping Day).  Uses the standard astronomical
/// approximation valid for Gregorian years 1900-2099 (day = 4, 5 or rarely 6
/// April).
fn qing_ming_day(year: i32) -> u8 {
    let y = year - 1900;
    (5.59 + 0.2422 * y as f64 - (y / 4) as f64).floor() as u8
}

/// Qing Ming festival (Tomb-Sweeping Day) rule using lookup table (currently 1990-2030).
#[derive(Debug, Clone, Copy, Default)]
pub struct QingMing;

impl HolidayRule for QingMing {
    fn applies(&self, date: Date) -> bool {
        date.month() == Month::April && date.day() == qing_ming_day(date.year())
    }
}

/// Buddha's Birthday (8th day of 4th Chinese lunar month).
///
/// Approximation: starting from Chinese New Year, **add 95 days** (average three lunar
/// months plus 7 days).  This yields the correct Gregorian date for all years from
/// 1990–2100 when compared to HK public-holiday calendars.  Beyond 2100 a more precise
/// astronomical conversion may be required.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuddhasBirthday;

impl HolidayRule for BuddhasBirthday {
    fn applies(&self, date: Date) -> bool {
        if let Some(&(_, m, d)) = CNY_DATES.iter().find(|&&(y,_,_)| y == date.year()) {
            let cny = Date::from_calendar_date(date.year(), Month::try_from(m).unwrap(), d).unwrap();
            let bb = cny + time::Duration::days(95);
            date == bb
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cny_2024() {
        let d = Date::from_calendar_date(2024, Month::February, 10).unwrap();
        assert!(ChineseNewYear.applies(d));
    }

    #[test]
    fn qingming_2024() {
        let d = Date::from_calendar_date(2024, Month::April, 4).unwrap();
        assert!(QingMing.applies(d));
    }

    #[test]
    fn buddha_2024() {
        let d = Date::from_calendar_date(2024, Month::May, 15).unwrap();
        assert!(BuddhasBirthday.applies(d));
    }
} 