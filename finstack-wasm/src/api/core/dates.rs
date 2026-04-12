//! WASM bindings for date utilities from [`finstack_core::dates`].

use crate::utils::to_js_err;
use finstack_core::dates::{
    adjust, BusinessDayConvention, CalendarRegistry, DayCount as RustDayCount, DayCountCtx,
    Tenor as RustTenor,
};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// DayCount
// ---------------------------------------------------------------------------

/// Day-count convention for computing year fractions and day counts.
///
/// Dates are represented as epoch days (i32, days since 1970-01-01).
#[wasm_bindgen(js_name = DayCount)]
pub struct DayCount {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustDayCount,
}

#[wasm_bindgen(js_class = DayCount)]
impl DayCount {
    /// Parse a day-count convention from its string name
    /// (e.g. `"act_360"`, `"30_360"`, `"act_act"`).
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str) -> Result<DayCount, JsValue> {
        name.parse::<RustDayCount>()
            .map(|inner| DayCount { inner })
            .map_err(to_js_err)
    }

    /// Actual/360.
    #[wasm_bindgen(js_name = act360)]
    pub fn act360() -> DayCount {
        DayCount {
            inner: RustDayCount::Act360,
        }
    }

    /// Actual/365 Fixed.
    #[wasm_bindgen(js_name = act365f)]
    pub fn act365f() -> DayCount {
        DayCount {
            inner: RustDayCount::Act365F,
        }
    }

    /// 30/360 US (Bond Basis).
    #[wasm_bindgen(js_name = thirty360)]
    pub fn thirty360() -> DayCount {
        DayCount {
            inner: RustDayCount::Thirty360,
        }
    }

    /// 30E/360 (Eurobond Basis).
    #[wasm_bindgen(js_name = thirtyE360)]
    pub fn thirty_e360() -> DayCount {
        DayCount {
            inner: RustDayCount::ThirtyE360,
        }
    }

    /// Actual/Actual (ISDA).
    #[wasm_bindgen(js_name = actAct)]
    pub fn act_act() -> DayCount {
        DayCount {
            inner: RustDayCount::ActAct,
        }
    }

    /// Actual/Actual (ICMA/ISMA).
    #[wasm_bindgen(js_name = actActIsma)]
    pub fn act_act_isma() -> DayCount {
        DayCount {
            inner: RustDayCount::ActActIsma,
        }
    }

    /// Business/252.
    #[wasm_bindgen(js_name = bus252)]
    pub fn bus252() -> DayCount {
        DayCount {
            inner: RustDayCount::Bus252,
        }
    }

    /// Compute the year fraction between two dates given as epoch days.
    #[wasm_bindgen(js_name = yearFraction)]
    pub fn year_fraction(
        &self,
        start_epoch_days: i32,
        end_epoch_days: i32,
    ) -> Result<f64, JsValue> {
        let start = epoch_to_date(start_epoch_days)?;
        let end = epoch_to_date(end_epoch_days)?;
        self.inner
            .year_fraction(start, end, DayCountCtx::default())
            .map_err(to_js_err)
    }

    /// Count the calendar days between two dates (epoch days).
    #[wasm_bindgen(js_name = calendarDays)]
    pub fn calendar_days(
        &self,
        start_epoch_days: i32,
        end_epoch_days: i32,
    ) -> Result<i64, JsValue> {
        let start = epoch_to_date(start_epoch_days)?;
        let end = epoch_to_date(end_epoch_days)?;
        Ok(RustDayCount::calendar_days(start, end))
    }

    /// Convention name.
    #[wasm_bindgen(js_name = toString)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

// ---------------------------------------------------------------------------
// Tenor
// ---------------------------------------------------------------------------

/// A financial tenor such as `3M`, `1Y`, or `2W`.
#[wasm_bindgen(js_name = Tenor)]
pub struct Tenor {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustTenor,
}

#[wasm_bindgen(js_class = Tenor)]
impl Tenor {
    /// Parse a tenor string (e.g. `"3M"`, `"1Y"`, `"2W"`).
    #[wasm_bindgen(constructor)]
    pub fn new(s: &str) -> Result<Tenor, JsValue> {
        RustTenor::parse(s)
            .map(|inner| Tenor { inner })
            .map_err(to_js_err)
    }

    /// 1-day tenor.
    #[wasm_bindgen(js_name = daily)]
    pub fn daily() -> Tenor {
        Tenor {
            inner: RustTenor::daily(),
        }
    }

    /// 1-week tenor.
    #[wasm_bindgen(js_name = weekly)]
    pub fn weekly() -> Tenor {
        Tenor {
            inner: RustTenor::weekly(),
        }
    }

    /// 1-month tenor.
    #[wasm_bindgen(js_name = monthly)]
    pub fn monthly() -> Tenor {
        Tenor {
            inner: RustTenor::monthly(),
        }
    }

    /// 3-month (quarterly) tenor.
    #[wasm_bindgen(js_name = quarterly)]
    pub fn quarterly() -> Tenor {
        Tenor {
            inner: RustTenor::quarterly(),
        }
    }

    /// 6-month (semi-annual) tenor.
    #[wasm_bindgen(js_name = semiAnnual)]
    pub fn semi_annual() -> Tenor {
        Tenor {
            inner: RustTenor::semi_annual(),
        }
    }

    /// 12-month (annual) tenor.
    #[wasm_bindgen(js_name = annual)]
    pub fn annual() -> Tenor {
        Tenor {
            inner: RustTenor::annual(),
        }
    }

    /// Numeric count.
    #[wasm_bindgen(getter, js_name = count)]
    pub fn count(&self) -> u32 {
        self.inner.count
    }

    /// Approximate length in years (simple estimate, no calendar).
    #[wasm_bindgen(js_name = toYearsSimple)]
    pub fn to_years_simple(&self) -> f64 {
        self.inner.to_years_simple()
    }

    /// Tenor string representation.
    #[wasm_bindgen(js_name = toString)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Create a date and return it as epoch days (days since 1970-01-01).
#[wasm_bindgen(js_name = createDate)]
pub fn create_date(year: i32, month: u8, day: u8) -> Result<i32, JsValue> {
    let m = time::Month::try_from(month).map_err(to_js_err)?;
    let date = finstack_core::dates::create_date(year, m, day).map_err(to_js_err)?;
    Ok(finstack_core::dates::days_since_epoch(date))
}

/// Convert epoch days back to `[year, month, day]` as a JS array-compatible triple.
#[wasm_bindgen(js_name = dateFromEpochDays)]
pub fn date_from_epoch_days(days: i32) -> Result<Vec<i32>, JsValue> {
    let date = finstack_core::dates::date_from_epoch_days(days)
        .ok_or_else(|| JsValue::from_str("epoch days out of valid date range"))?;
    Ok(vec![date.year(), date.month() as i32, date.day() as i32])
}

/// Adjust a date (epoch days) according to a business-day convention and calendar.
///
/// Returns the adjusted date as epoch days.
#[wasm_bindgen(js_name = adjustBusinessDay)]
pub fn adjust_business_day(
    epoch_days: i32,
    convention: &str,
    calendar_code: &str,
) -> Result<i32, JsValue> {
    let date = epoch_to_date(epoch_days)?;
    let bdc: BusinessDayConvention = convention
        .parse()
        .map_err(|e: String| JsValue::from_str(&e))?;
    let registry = CalendarRegistry::global();
    let cal = registry
        .resolve_str(calendar_code)
        .ok_or_else(|| JsValue::from_str(&format!("unknown calendar: {calendar_code}")))?;
    let adjusted = adjust(date, bdc, cal).map_err(to_js_err)?;
    Ok(finstack_core::dates::days_since_epoch(adjusted))
}

/// Return the list of available calendar codes.
#[wasm_bindgen(js_name = availableCalendars)]
pub fn available_calendars() -> Vec<String> {
    CalendarRegistry::global()
        .available_ids()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert epoch days to a `time::Date`.
fn epoch_to_date(days: i32) -> Result<time::Date, JsValue> {
    finstack_core::dates::date_from_epoch_days(days)
        .ok_or_else(|| JsValue::from_str("epoch days out of valid date range"))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn epoch(y: i32, m: u8, d: u8) -> i32 {
        let month = time::Month::try_from(m).expect("valid month");
        let date = finstack_core::dates::create_date(y, month, d).expect("valid date");
        finstack_core::dates::days_since_epoch(date)
    }

    fn jan15() -> i32 {
        epoch(2024, 1, 15)
    }

    fn jul15() -> i32 {
        epoch(2024, 7, 15)
    }

    // -- DayCount -----------------------------------------------------------

    #[test]
    fn daycount_constructors() {
        let dc = DayCount::act360();
        assert_eq!(dc.to_string(), "act_360");
        let dc = DayCount::act365f();
        assert_eq!(dc.to_string(), "act_365f");
        let dc = DayCount::thirty360();
        assert_eq!(dc.to_string(), "30_360");
        let dc = DayCount::thirty_e360();
        assert_eq!(dc.to_string(), "30e_360");
        let dc = DayCount::act_act();
        assert_eq!(dc.to_string(), "act_act");
        let dc = DayCount::act_act_isma();
        assert_eq!(dc.to_string(), "act_act_isma");
        let dc = DayCount::bus252();
        assert_eq!(dc.to_string(), "bus_252");
    }

    #[test]
    fn daycount_from_string() {
        let dc = DayCount::new("act_360").expect("valid");
        assert_eq!(dc.to_string(), "act_360");
    }

    #[test]
    fn year_fraction_act365f() {
        let dc = DayCount::act365f();
        let yf = dc.year_fraction(jan15(), jul15()).expect("valid");
        assert!(yf > 0.49 && yf < 0.51, "yf={yf}");
    }

    #[test]
    fn calendar_days() {
        let dc = DayCount::act365f();
        let days = dc.calendar_days(jan15(), jul15()).expect("valid");
        assert_eq!(days, (jul15() - jan15()) as i64);
    }

    // -- Tenor --------------------------------------------------------------

    #[test]
    fn tenor_factories() {
        assert_eq!(Tenor::daily().count(), 1);
        assert_eq!(Tenor::weekly().count(), 1);
        assert_eq!(Tenor::monthly().count(), 1);
        assert_eq!(Tenor::quarterly().count(), 3);
        assert_eq!(Tenor::semi_annual().count(), 6);
        assert_eq!(Tenor::annual().count(), 1);
    }

    #[test]
    fn tenor_parse() {
        let t = Tenor::new("3M").expect("valid");
        assert_eq!(t.count(), 3);
        assert!(t.to_years_simple() > 0.24 && t.to_years_simple() < 0.26);
    }

    #[test]
    fn tenor_parse_year() {
        let t = Tenor::new("1Y").expect("valid");
        assert!((t.to_years_simple() - 1.0).abs() < 0.01);
    }

    #[test]
    fn tenor_to_string() {
        let t = Tenor::quarterly();
        let s = t.to_string();
        assert!(s.contains('M') || s.contains('Q'), "got: {s}");
    }

    // -- Free functions -----------------------------------------------------

    #[test]
    fn create_date_valid() {
        let e = create_date(2024, 1, 15).expect("valid");
        assert_eq!(e, jan15());
    }

    #[test]
    fn date_from_epoch_days_roundtrip() {
        let parts = date_from_epoch_days(jan15()).expect("valid");
        assert_eq!(parts, vec![2024, 1, 15]);
    }

    #[test]
    fn available_calendars_not_empty() {
        let cals = available_calendars();
        assert!(!cals.is_empty());
    }

    #[test]
    fn epoch_to_date_via_core() {
        let d = finstack_core::dates::date_from_epoch_days(jan15()).expect("valid");
        assert_eq!(d.year(), 2024);
    }

    #[test]
    fn year_fraction_act360() {
        let dc = DayCount::act360();
        let yf = dc.year_fraction(jan15(), jul15()).expect("valid");
        let days = (jul15() - jan15()) as f64;
        assert!((yf - days / 360.0).abs() < 1e-10);
    }

    #[test]
    fn year_fraction_thirty360() {
        let dc = DayCount::thirty360();
        let yf = dc.year_fraction(jan15(), jul15()).expect("valid");
        assert!(yf > 0.0);
    }

    #[test]
    fn tenor_weekly_to_string() {
        let t = Tenor::weekly();
        let s = t.to_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn tenor_semi_annual_years() {
        let t = Tenor::semi_annual();
        assert!((t.to_years_simple() - 0.5).abs() < 0.01);
    }

    #[test]
    fn tenor_annual_years() {
        let t = Tenor::annual();
        assert!((t.to_years_simple() - 1.0).abs() < 0.01);
    }

    #[test]
    fn tenor_daily_years() {
        let t = Tenor::daily();
        assert!(t.to_years_simple() < 0.01);
    }

    // -- Boundary tests ------------------------------------------------
    // Error paths through wasm-bindgen create JsValue, which panics on
    // native targets.  Test the underlying Rust types instead.

    #[test]
    fn create_date_invalid_month() {
        assert!(time::Month::try_from(13_u8).is_err());
        assert!(time::Month::try_from(0_u8).is_err());
    }

    #[test]
    fn create_date_invalid_day() {
        assert!(finstack_core::dates::create_date(2024, time::Month::February, 30).is_err());
    }

    #[test]
    fn date_from_epoch_days_extreme() {
        assert!(finstack_core::dates::date_from_epoch_days(i32::MAX).is_none());
        assert!(finstack_core::dates::date_from_epoch_days(i32::MIN).is_none());
    }

    #[test]
    fn daycount_invalid_string() {
        assert!("not_a_daycount".parse::<RustDayCount>().is_err());
    }

    #[test]
    fn tenor_invalid_string() {
        assert!(RustTenor::parse("").is_err());
        assert!(RustTenor::parse("XYZ").is_err());
    }
}
