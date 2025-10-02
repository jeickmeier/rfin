use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::core::utils::js_array_from_iter;
use crate::core::error::js_error;
use finstack_core::market_data::dividends::{
    DividendEvent, DividendKind, DividendSchedule, DividendScheduleBuilder,
};
use std::mem;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = DividendEvent)]
#[derive(Clone)]
pub struct JsDividendEvent {
    inner: DividendEvent,
}

impl JsDividendEvent {
    pub(crate) fn from_inner(inner: DividendEvent) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = DividendEvent)]
impl JsDividendEvent {
    #[wasm_bindgen(getter)]
    pub fn date(&self) -> JsDate {
        JsDate::from_core(self.inner.date)
    }

    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        match self.inner.kind {
            DividendKind::Cash(_) => "cash".to_string(),
            DividendKind::Yield(_) => "yield".to_string(),
            DividendKind::Stock { .. } => "stock".to_string(),
        }
    }

    #[wasm_bindgen(getter, js_name = cashAmount)]
    pub fn cash_amount(&self) -> Option<JsMoney> {
        match self.inner.kind {
            DividendKind::Cash(m) => Some(JsMoney::from_inner(m)),
            _ => None,
        }
    }

    #[wasm_bindgen(getter, js_name = dividendYield)]
    pub fn dividend_yield(&self) -> Option<f64> {
        match self.inner.kind {
            DividendKind::Yield(v) => Some(v),
            _ => None,
        }
    }

    #[wasm_bindgen(getter, js_name = stockRatio)]
    pub fn stock_ratio(&self) -> Option<f64> {
        match self.inner.kind {
            DividendKind::Stock { ratio } => Some(ratio),
            _ => None,
        }
    }
}

#[wasm_bindgen(js_name = DividendSchedule)]
#[derive(Clone)]
pub struct JsDividendSchedule {
    inner: Arc<DividendSchedule>,
}

impl JsDividendSchedule {
    pub(crate) fn from_inner(inner: DividendSchedule) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub(crate) fn from_arc(inner: Arc<DividendSchedule>) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Arc<DividendSchedule> {
        Arc::clone(&self.inner)
    }
}

#[wasm_bindgen(js_class = DividendSchedule)]
impl JsDividendSchedule {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn underlying(&self) -> Option<String> {
        self.inner.underlying.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> Option<JsCurrency> {
        self.inner.currency.map(JsCurrency::from_inner)
    }

    #[wasm_bindgen(getter)]
    pub fn events(&self) -> js_sys::Array {
        let events = self
            .inner
            .events
            .iter()
            .cloned()
            .map(JsDividendEvent::from_inner);
        js_array_from_iter(events.map(JsValue::from))
    }

    #[wasm_bindgen(js_name = eventsBetween)]
    pub fn events_between(&self, start: &JsDate, end: &JsDate) -> js_sys::Array {
        let list = self
            .inner
            .events_between(start.inner(), end.inner())
            .into_iter()
            .cloned()
            .map(JsDividendEvent::from_inner);
        js_array_from_iter(list.map(JsValue::from))
    }
}

#[wasm_bindgen(js_name = DividendScheduleBuilder)]
pub struct JsDividendScheduleBuilder {
    inner: DividendScheduleBuilder,
}

#[wasm_bindgen(js_class = DividendScheduleBuilder)]
impl JsDividendScheduleBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str) -> JsDividendScheduleBuilder {
        Self {
            inner: DividendScheduleBuilder::new(id),
        }
    }

    #[wasm_bindgen]
    pub fn underlying(&mut self, name: &str) {
        let builder = mem::replace(&mut self.inner, DividendScheduleBuilder::new("temp"));
        self.inner = builder.underlying(name);
    }

    #[wasm_bindgen]
    pub fn currency(&mut self, currency: &JsCurrency) {
        let builder = mem::replace(&mut self.inner, DividendScheduleBuilder::new("temp"));
        self.inner = builder.currency(currency.inner());
    }

    #[wasm_bindgen]
    pub fn cash(&mut self, date: &JsDate, amount: &JsMoney) {
        let builder = mem::replace(&mut self.inner, DividendScheduleBuilder::new("temp"));
        self.inner = builder.cash(date.inner(), amount.inner());
    }

    #[wasm_bindgen(js_name = yieldDividend)]
    pub fn yield_dividend(&mut self, date: &JsDate, dividend_yield: f64) {
        let builder = mem::replace(&mut self.inner, DividendScheduleBuilder::new("temp"));
        self.inner = builder.yield_div(date.inner(), dividend_yield);
    }

    #[wasm_bindgen]
    pub fn stock(&mut self, date: &JsDate, ratio: f64) {
        let builder = mem::replace(&mut self.inner, DividendScheduleBuilder::new("temp"));
        self.inner = builder.stock(date.inner(), ratio);
    }

    #[wasm_bindgen]
    pub fn build(&mut self) -> Result<JsDividendSchedule, JsValue> {
        let builder = mem::replace(&mut self.inner, DividendScheduleBuilder::new("temp"));
        let schedule = builder.build().map_err(|e| js_error(e.to_string()))?;
        self.inner = DividendScheduleBuilder::new(schedule.id.to_string());
        Ok(JsDividendSchedule::from_inner(schedule))
    }
}
