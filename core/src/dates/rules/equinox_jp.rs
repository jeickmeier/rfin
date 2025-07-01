use super::HolidayRule;
use time::{Date, Month};

/// Vernal Equinox Day (Japanese calendar approximation).
///
/// Formula: floor(20.8431 + 0.242194*(Y-1980) - floor((Y-1980)/4)) March
#[derive(Debug, Clone, Copy, Default)]
pub struct VernalEquinoxJP;

impl VernalEquinoxJP {
    #[inline]
    fn date(year: i32) -> Date {
        let y = year - 1980;
        let day = (20.8431 + 0.242194 * y as f64 - ((y / 4) as f64).floor()).floor() as u8;
        Date::from_calendar_date(year, Month::March, day).unwrap()
    }
}

impl HolidayRule for VernalEquinoxJP {
    fn applies(&self, date: Date) -> bool {
        date == Self::date(date.year())
    }
}

/// Autumnal Equinox Day (Japanese calendar approximation).
///
/// Formula: floor(23.2488 + 0.242194*(Y-1980) - floor((Y-1980)/4)) September
#[derive(Debug, Clone, Copy, Default)]
pub struct AutumnalEquinoxJP;

impl AutumnalEquinoxJP {
    #[inline]
    fn date(year: i32) -> Date {
        let y = year - 1980;
        let day = (23.2488 + 0.242194 * y as f64 - ((y / 4) as f64).floor()).floor() as u8;
        Date::from_calendar_date(year, Month::September, day).unwrap()
    }
}

impl HolidayRule for AutumnalEquinoxJP {
    fn applies(&self, date: Date) -> bool {
        date == Self::date(date.year())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vernal_2024() {
        let d = Date::from_calendar_date(2024, Month::March, 20).unwrap();
        assert!(VernalEquinoxJP.applies(d));
    }

    #[test]
    fn vernal_2023() {
        let d = Date::from_calendar_date(2023, Month::March, 21).unwrap();
        assert!(VernalEquinoxJP.applies(d));
    }

    #[test]
    fn autumn_2023() {
        let d = Date::from_calendar_date(2023, Month::September, 23).unwrap();
        assert!(AutumnalEquinoxJP.applies(d));
    }
} 