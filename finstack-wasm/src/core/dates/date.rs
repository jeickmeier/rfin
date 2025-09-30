use crate::core::utils::js_error;
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
    #[wasm_bindgen(constructor)]
    pub fn new(year: i32, month: u8, day: u8) -> Result<JsDate, JsValue> {
        let month_enum =
            Month::try_from(month).map_err(|_| js_error("Month must be in range 1-12"))?;
        let date = CoreDate::from_calendar_date(year, month_enum, day)
            .map_err(|e| js_error(format!("Invalid date components: {e}")))?;
        Ok(Self { inner: date })
    }

    #[wasm_bindgen(getter)]
    pub fn year(&self) -> i32 {
        self.inner.year()
    }

    #[wasm_bindgen(getter)]
    pub fn month(&self) -> u8 {
        self.inner.month() as u8
    }

    #[wasm_bindgen(getter)]
    pub fn day(&self) -> u8 {
        self.inner.day()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen]
    pub fn equals(&self, other: &JsDate) -> bool {
        self.inner == other.inner
    }

    #[wasm_bindgen(js_name = isWeekend)]
    pub fn is_weekend(&self) -> bool {
        self.inner.is_weekend()
    }

    #[wasm_bindgen]
    pub fn quarter(&self) -> u8 {
        self.inner.quarter()
    }

    #[wasm_bindgen(js_name = fiscalYear)]
    pub fn fiscal_year(&self) -> i32 {
        self.inner.fiscal_year(FiscalConfig::calendar_year())
    }

    #[wasm_bindgen(js_name = addWeekdays)]
    pub fn add_weekdays(&self, offset: i32) -> JsDate {
        JsDate::from_core(self.inner.add_weekdays(offset))
    }
}
