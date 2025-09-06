//! WASM bindings for Date type.
//!
//! Minimal wrapper around `time::Date` (re-exported via `finstack_core::dates::Date`).
//! Provides simple constructor and accessors so that JS/TS consumers can
//! create and inspect calendar dates.

use finstack_core::dates::Date as CoreDate;
use finstack_core::dates::DateExt;
use finstack_core::dates::DayCount as CoreDayCount;
use finstack_core::dates::{
    next_cds_date as core_next_cds, next_imm as core_next_imm, third_wednesday as core_third_wed,
};
use time::Month;
use wasm_bindgen::prelude::*;

/// A calendar date (YYYY-MM-DD) exposed to JavaScript.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Date {
    inner: CoreDate,
}

#[wasm_bindgen]
impl Date {
    /// Construct a new date from numeric components.
    ///
    /// Throws a JS `TypeError` when the components are out of range or do not
    /// form a valid calendar date (e.g. 2025-02-30).
    #[wasm_bindgen(constructor)]
    pub fn new(year: i32, month: u8, day: u8) -> Result<Date, JsValue> {
        let month_enum =
            Month::try_from(month).map_err(|_| JsValue::from_str("Month must be in range 1-12"))?;
        let date = CoreDate::from_calendar_date(year, month_enum, day)
            .map_err(|e| JsValue::from_str(&format!("Invalid date components: {}", e)))?;
        Ok(Date { inner: date })
    }

    /// Year component (e.g. 2025).
    #[wasm_bindgen(getter)]
    pub fn year(&self) -> i32 {
        self.inner.year()
    }

    /// Month component (1-12).
    #[wasm_bindgen(getter)]
    pub fn month(&self) -> u8 {
        self.inner.month() as u8
    }

    /// Day of month (1-31).
    #[wasm_bindgen(getter)]
    pub fn day(&self) -> u8 {
        self.inner.day()
    }

    /// ISO-8601 `YYYY-MM-DD` string.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }

    /// Equality check with another Date.
    #[wasm_bindgen]
    pub fn equals(&self, other: &Date) -> bool {
        self.inner == other.inner
    }

    /// Check if the date is a weekend.
    #[wasm_bindgen(js_name = "isWeekend")]
    pub fn is_weekend(&self) -> bool {
        self.inner.is_weekend()
    }

    /// Calendar quarter (1–4).
    #[wasm_bindgen]
    pub fn quarter(&self) -> u8 {
        self.inner.quarter()
    }

    /// Fiscal year corresponding to the date.
    ///
    /// Currently uses calendar year configuration (fiscal year = calendar year).
    /// TODO: Add FiscalConfig support to WASM bindings for custom fiscal years.
    #[wasm_bindgen(js_name = "fiscalYear")]
    pub fn fiscal_year(&self) -> i32 {
        use finstack_core::dates::{DateExt, FiscalConfig};
        self.inner.fiscal_year(FiscalConfig::calendar_year())
    }

    /// Add or subtract a number of weekdays and return a **new** `Date`.
    ///
    /// Weekdays exclude weekends (Saturday and Sunday) but do NOT account for holidays.
    /// For true business day adjustments that respect holidays, use a proper holiday calendar.
    #[wasm_bindgen(js_name = "addWeekdays")]
    pub fn add_weekdays(&self, n: i32) -> Date {
        use finstack_core::dates::DateExt;
        let new_inner = self.inner.add_weekdays(n);
        Date { inner: new_inner }
    }
}

impl Date {
    /// Internal helper to access the inner `time::Date` value.
    pub fn inner(&self) -> CoreDate {
        self.inner
    }

    /// Internal helper to create a `Date` from a core value.
    pub(crate) fn from_core(inner: CoreDate) -> Self {
        Date { inner }
    }
}

// -------------------------------------------------------------------------------------------------
// DayCount enum bindings
// -------------------------------------------------------------------------------------------------

/// Supported day-count conventions.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum DayCount {
    Act360,
    Act365F,
    Thirty360,
    ThirtyE360,
    ActAct,
    ActActIsma,
}

impl From<DayCount> for CoreDayCount {
    fn from(dc: DayCount) -> Self {
        match dc {
            DayCount::Act360 => CoreDayCount::Act360,
            DayCount::Act365F => CoreDayCount::Act365F,
            DayCount::Thirty360 => CoreDayCount::Thirty360,
            DayCount::ThirtyE360 => CoreDayCount::ThirtyE360,
            DayCount::ActAct => CoreDayCount::ActAct,
            DayCount::ActActIsma => CoreDayCount::ActActIsma,
        }
    }
}

/// Return the number of days between two dates according to the given convention.
#[wasm_bindgen(js_name = "dayCountDays")]
pub fn day_count_days(convention: DayCount, start: &Date, end: &Date) -> Result<i32, JsValue> {
    let core = CoreDayCount::from(convention);
    let s = start.inner();
    let e = end.inner();
    let days = match core {
        CoreDayCount::Act360
        | CoreDayCount::Act365F
        | CoreDayCount::ActAct
        | CoreDayCount::ActActIsma => (e - s).whole_days() as i32,
        CoreDayCount::Act365L => (e - s).whole_days() as i32,
        CoreDayCount::Thirty360 => days_30_360_us(s, e),
        CoreDayCount::ThirtyE360 => days_30_360_eu(s, e),
        // Bus/252 requires a calendar – mirror core behaviour and return error
        _ => return Err(JsValue::from_str("Bus/252 requires a holiday calendar")),
    };
    Ok(days)
}

#[inline]
fn days_30_360_us(start: CoreDate, end: CoreDate) -> i32 {
    let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
    let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

    let d1_adj = if d1 == 31 { 30 } else { d1 };
    let d2_adj = if d2 == 31 && d1_adj == 30 { 30 } else { d2 };

    (y2 - y1) * 360 + (m2 - m1) * 30 + (d2_adj - d1_adj)
}

#[inline]
fn days_30_360_eu(start: CoreDate, end: CoreDate) -> i32 {
    let (y1, m1, d1) = (start.year(), start.month() as i32, start.day() as i32);
    let (y2, m2, d2) = (end.year(), end.month() as i32, end.day() as i32);

    let d1_adj = if d1 == 31 { 30 } else { d1 };
    let d2_adj = if d2 == 31 { 30 } else { d2 };

    (y2 - y1) * 360 + (m2 - m1) * 30 + (d2_adj - d1_adj)
}

/// Return the year fraction between two dates according to the given convention.
#[wasm_bindgen(js_name = "dayCountYearFraction")]
pub fn day_count_year_fraction(
    convention: DayCount,
    start: &Date,
    end: &Date,
) -> Result<f64, JsValue> {
    let core = CoreDayCount::from(convention);
    core.year_fraction(
        start.inner(),
        end.inner(),
        finstack_core::dates::DayCountCtx::default(),
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Return the third Wednesday of the given `month` (1-12) and `year`.
#[wasm_bindgen(js_name = "thirdWednesday")]
pub fn third_wednesday(month: u8, year: i32) -> Result<Date, JsValue> {
    let month_enum =
        Month::try_from(month).map_err(|_| JsValue::from_str("Month must be in range 1-12"))?;
    let d = core_third_wed(month_enum, year);
    Ok(Date::from_core(d))
}

/// Return the next IMM date strictly after `date`.
#[wasm_bindgen(js_name = "nextImm")]
pub fn next_imm(date: &Date) -> Date {
    Date::from_core(core_next_imm(date.inner()))
}

/// Return the next CDS roll date strictly after `date`.
#[wasm_bindgen(js_name = "nextCdsDate")]
pub fn next_cds_date(date: &Date) -> Date {
    Date::from_core(core_next_cds(date.inner()))
}
