//! WASM bindings for Date type.
//!
//! Minimal wrapper around `time::Date` (re-exported via `rfin_core::Date`).
//! Provides simple constructor and accessors so that JS/TS consumers can
//! create and inspect calendar dates.

use rfin_core::dates::DayCount as CoreDayCount;
use rfin_core::Date as CoreDate;
use rfin_core::DateExt;
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

    /// Fiscal year corresponding to the date (currently same as calendar year).
    #[wasm_bindgen(js_name = "fiscalYear")]
    pub fn fiscal_year(&self) -> i32 {
        self.inner.fiscal_year()
    }

    /// Add or subtract a number of business days and return a **new** `Date`.
    #[wasm_bindgen(js_name = "addBusinessDays")]
    pub fn add_business_days(&self, n: i32) -> Date {
        let new_inner = self.inner.add_business_days(n);
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
}

impl From<DayCount> for CoreDayCount {
    fn from(dc: DayCount) -> Self {
        match dc {
            DayCount::Act360 => CoreDayCount::Act360,
            DayCount::Act365F => CoreDayCount::Act365F,
            DayCount::Thirty360 => CoreDayCount::Thirty360,
            DayCount::ThirtyE360 => CoreDayCount::ThirtyE360,
            DayCount::ActAct => CoreDayCount::ActAct,
        }
    }
}

/// Return the number of days between two dates according to the given convention.
#[wasm_bindgen(js_name = "dayCountDays")]
pub fn day_count_days(convention: DayCount, start: &Date, end: &Date) -> Result<i32, JsValue> {
    let core = CoreDayCount::from(convention);
    core.days(start.inner(), end.inner())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Return the year fraction between two dates according to the given convention.
#[wasm_bindgen(js_name = "dayCountYearFraction")]
pub fn day_count_year_fraction(
    convention: DayCount,
    start: &Date,
    end: &Date,
) -> Result<f64, JsValue> {
    let core = CoreDayCount::from(convention);
    core.year_fraction(start.inner(), end.inner())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
