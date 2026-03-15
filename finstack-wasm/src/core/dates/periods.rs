use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use finstack_core::dates::{build_fiscal_periods, build_periods, FiscalConfig, Period, PeriodId};
use std::str::FromStr;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = FiscalConfig)]
#[derive(Clone, Copy, Debug)]
pub struct JsFiscalConfig {
    inner: FiscalConfig,
}

impl JsFiscalConfig {
    pub(crate) fn inner(&self) -> FiscalConfig {
        self.inner
    }

    fn new(inner: FiscalConfig) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = FiscalConfig)]
impl JsFiscalConfig {
    #[wasm_bindgen(constructor)]
    pub fn new_config(start_month: u8, start_day: u8) -> Result<JsFiscalConfig, JsValue> {
        FiscalConfig::new(start_month, start_day)
            .map(JsFiscalConfig::new)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = calendarYear)]
    pub fn calendar_year() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::calendar_year())
    }

    #[wasm_bindgen(js_name = usFederal)]
    pub fn us_federal() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::us_federal())
    }

    #[wasm_bindgen(js_name = uk)]
    pub fn uk() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::uk())
    }

    #[wasm_bindgen(js_name = japan)]
    pub fn japan() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::japan())
    }

    #[wasm_bindgen(js_name = canada)]
    pub fn canada() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::canada())
    }

    #[wasm_bindgen(js_name = australia)]
    pub fn australia() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::australia())
    }

    #[wasm_bindgen(js_name = germany)]
    pub fn germany() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::germany())
    }

    #[wasm_bindgen(js_name = france)]
    pub fn france() -> JsFiscalConfig {
        JsFiscalConfig::new(FiscalConfig::france())
    }

    #[wasm_bindgen(getter, js_name = startMonth)]
    pub fn start_month(&self) -> u8 {
        self.inner.start_month
    }

    #[wasm_bindgen(getter, js_name = startDay)]
    pub fn start_day(&self) -> u8 {
        self.inner.start_day
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FiscalConfig(start_month={}, start_day={})",
            self.inner.start_month, self.inner.start_day
        )
    }
}

#[wasm_bindgen(js_name = PeriodId)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsPeriodId {
    inner: PeriodId,
}

impl JsPeriodId {
    fn new(inner: PeriodId) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = PeriodId)]
impl JsPeriodId {
    #[wasm_bindgen(constructor)]
    pub fn parse(code: &str) -> Result<JsPeriodId, JsValue> {
        PeriodId::from_str(code)
            .map(JsPeriodId::new)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = quarter)]
    pub fn quarter(year: i32, quarter: u8) -> Result<JsPeriodId, JsValue> {
        if !(1..=4).contains(&quarter) {
            return Err(js_error("Quarter must be in 1..=4"));
        }
        Ok(JsPeriodId::new(PeriodId::quarter(year, quarter)))
    }

    #[wasm_bindgen(js_name = month)]
    pub fn month(year: i32, month: u8) -> Result<JsPeriodId, JsValue> {
        if !(1..=12).contains(&month) {
            return Err(js_error("Month must be in 1..=12"));
        }
        Ok(JsPeriodId::new(PeriodId::month(year, month)))
    }

    #[wasm_bindgen(js_name = week)]
    pub fn week(year: i32, week: u8) -> Result<JsPeriodId, JsValue> {
        let code = format!("{year}W{week:02}");
        PeriodId::from_str(&code)
            .map(JsPeriodId::new)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = half)]
    pub fn half(year: i32, half: u8) -> Result<JsPeriodId, JsValue> {
        if !(1..=2).contains(&half) {
            return Err(js_error("Half must be 1 or 2"));
        }
        Ok(JsPeriodId::new(PeriodId::half(year, half)))
    }

    #[wasm_bindgen(js_name = annual)]
    pub fn annual(year: i32) -> JsPeriodId {
        JsPeriodId::new(PeriodId::annual(year))
    }

    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn year(&self) -> i32 {
        self.inner.year
    }

    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u16 {
        self.inner.index
    }

    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        let code = self.inner.to_string();
        if code.contains('D') {
            "day".to_string()
        } else if code.contains('Q') {
            "quarter".to_string()
        } else if code.contains('M') {
            "month".to_string()
        } else if code.contains('W') {
            "week".to_string()
        } else if code.contains('H') {
            "half".to_string()
        } else {
            "year".to_string()
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

#[wasm_bindgen(js_name = Period)]
#[derive(Clone, Debug)]
pub struct JsPeriod {
    inner: Period,
}

impl JsPeriod {
    fn new(inner: Period) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = Period)]
impl JsPeriod {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> JsPeriodId {
        JsPeriodId::new(self.inner.id)
    }

    #[wasm_bindgen(getter)]
    pub fn start(&self) -> JsDate {
        JsDate::from_core(self.inner.start)
    }

    #[wasm_bindgen(getter)]
    pub fn end(&self) -> JsDate {
        JsDate::from_core(self.inner.end)
    }

    #[wasm_bindgen(getter, js_name = isActual)]
    pub fn is_actual(&self) -> bool {
        self.inner.is_actual
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Period(id='{}', start={}, end={}, actual={})",
            self.inner.id, self.inner.start, self.inner.end, self.inner.is_actual
        )
    }
}

#[wasm_bindgen(js_name = PeriodPlan)]
#[derive(Clone, Debug)]
pub struct JsPeriodPlan {
    periods: Vec<Period>,
}

impl JsPeriodPlan {
    fn new(periods: Vec<Period>) -> Self {
        Self { periods }
    }
}

#[wasm_bindgen(js_class = PeriodPlan)]
impl JsPeriodPlan {
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.periods.len()
    }

    #[wasm_bindgen(js_name = toArray)]
    pub fn to_array(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        for period in &self.periods {
            array.push(&JsValue::from(JsPeriod::new(period.clone())));
        }
        array
    }
}

#[wasm_bindgen(js_name = buildPeriods)]
pub fn build_periods_js(
    range: &str,
    actuals_until: Option<String>,
) -> Result<JsPeriodPlan, JsValue> {
    let plan =
        build_periods(range, actuals_until.as_deref()).map_err(|e| js_error(e.to_string()))?;
    Ok(JsPeriodPlan::new(plan.periods))
}

#[wasm_bindgen(js_name = buildFiscalPeriods)]
pub fn build_fiscal_periods_js(
    range: &str,
    config: &JsFiscalConfig,
    actuals_until: Option<String>,
) -> Result<JsPeriodPlan, JsValue> {
    let plan = build_fiscal_periods(range, config.inner(), actuals_until.as_deref())
        .map_err(|e| js_error(e.to_string()))?;
    Ok(JsPeriodPlan::new(plan.periods))
}
