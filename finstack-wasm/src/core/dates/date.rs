use crate::core::error::js_error;
use finstack_core::dates::{Date as CoreDate, DateExt, FiscalConfig};
use time::Month;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Date)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsDate {
    inner: CoreDate,
}

impl JsDate {
    pub(crate) fn from_core(inner: CoreDate) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> CoreDate {
        self.inner
    }
}

#[wasm_bindgen(js_class = Date)]
impl JsDate {
    /// Create a calendar date from year, month, and day components.
    ///
    /// @param {number} year - Four-digit calendar year
    /// @param {number} month - Month number (1-based: 1=January, 12=December)
    /// @param {number} day - Day of month (1-31 depending on month)
    /// @returns {Date} Date instance representing the calendar day
    /// @throws {Error} If components are invalid (e.g., February 30)
    ///
    /// @example
    /// ```javascript
    /// const date = new Date(2024, 9, 30);  // September 30, 2024
    /// console.log(date.year);    // 2024
    /// console.log(date.month);   // 9
    /// console.log(date.day);     // 30
    /// console.log(date.toString());  // "2024-09-30"
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(year: i32, month: u8, day: u8) -> Result<JsDate, JsValue> {
        let month_enum =
            Month::try_from(month).map_err(|_| js_error("Month must be in range 1-12"))?;
        let date = CoreDate::from_calendar_date(year, month_enum, day)
            .map_err(|e| js_error(format!("Invalid date components: {e}")))?;
        Ok(Self { inner: date })
    }

    /// Four-digit calendar year.
    ///
    /// @type {number}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn year(&self) -> i32 {
        self.inner.year()
    }

    /// Month number (1-based: 1 = January, 12 = December).
    ///
    /// @type {number}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn month(&self) -> u8 {
        self.inner.month() as u8
    }

    /// Day of month (1-31).
    ///
    /// @type {number}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn day(&self) -> u8 {
        self.inner.day()
    }

    /// ISO-8601 string representation (YYYY-MM-DD).
    ///
    /// @returns {string} Date formatted as "2024-09-30"
    ///
    /// @example
    /// ```javascript
    /// const date = new Date(2024, 1, 15);
    /// console.log(date.toString());  // "2024-01-15"
    /// ```
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }

    /// Check equality with another date.
    ///
    /// @param {Date} other - Date to compare against
    /// @returns {boolean} True if dates represent the same calendar day
    ///
    /// @example
    /// ```javascript
    /// const d1 = new Date(2024, 1, 1);
    /// const d2 = new Date(2024, 1, 1);
    /// const d3 = new Date(2024, 1, 2);
    /// console.log(d1.equals(d2));  // true
    /// console.log(d1.equals(d3));  // false
    /// ```
    #[wasm_bindgen]
    pub fn equals(&self, other: &JsDate) -> bool {
        self.inner == other.inner
    }

    /// Check if this date falls on a weekend (Saturday or Sunday).
    ///
    /// @returns {boolean} True if the date is Saturday or Sunday
    ///
    /// @example
    /// ```javascript
    /// const saturday = new Date(2024, 1, 6);   // Saturday
    /// const monday = new Date(2024, 1, 8);     // Monday
    /// console.log(saturday.isWeekend());  // true
    /// console.log(monday.isWeekend());    // false
    /// ```
    #[wasm_bindgen(js_name = isWeekend)]
    pub fn is_weekend(&self) -> bool {
        self.inner.is_weekend()
    }

    /// Calendar quarter (1-4).
    ///
    /// @returns {number} Quarter number (1=Q1 Jan-Mar, 2=Q2 Apr-Jun, etc.)
    ///
    /// @example
    /// ```javascript
    /// const jan = new Date(2024, 1, 15);
    /// const jul = new Date(2024, 7, 15);
    /// console.log(jan.quarter());  // 1
    /// console.log(jul.quarter());  // 3
    /// ```
    #[wasm_bindgen]
    pub fn quarter(&self) -> u8 {
        self.inner.quarter()
    }

    /// Fiscal year for this date (assuming calendar-year fiscal config).
    ///
    /// @returns {number} Fiscal year (typically same as calendar year for standard configs)
    ///
    /// @example
    /// ```javascript
    /// const date = new Date(2024, 11, 1);  // November
    /// console.log(date.fiscalYear());  // 2024 (for calendar-year fiscal)
    /// ```
    #[wasm_bindgen(js_name = fiscalYear)]
    pub fn fiscal_year(&self) -> i32 {
        self.inner.fiscal_year(FiscalConfig::calendar_year())
    }

    /// Add business days (weekdays) to this date, skipping weekends.
    ///
    /// @param {number} offset - Number of weekdays to add (negative to subtract)
    /// @returns {Date} New date after adding the specified weekdays
    ///
    /// @example
    /// ```javascript
    /// const friday = new Date(2024, 1, 5);  // Friday, January 5
    /// const nextMon = friday.addWeekdays(1);  // Skips weekend
    /// console.log(nextMon.toString());  // "2024-01-08" (Monday)
    ///
    /// const fiveDaysLater = friday.addWeekdays(5);
    /// console.log(fiveDaysLater.toString());  // "2024-01-12" (Friday)
    /// ```
    #[wasm_bindgen(js_name = addWeekdays)]
    pub fn add_weekdays(&self, offset: i32) -> JsDate {
        JsDate::from_core(self.inner.add_weekdays(offset))
    }
}
