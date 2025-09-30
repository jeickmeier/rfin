use crate::core::dates::calendar::{resolve_calendar_ref, JsCalendar};
use crate::core::dates::date::JsDate;
use crate::core::utils::js_error;
use finstack_core::dates::{DayCount, DayCountCtx, Frequency};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = Frequency)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsFrequency {
    inner: Frequency,
}

impl JsFrequency {
    pub(crate) fn inner(&self) -> Frequency {
        self.inner
    }

    pub(crate) fn from_inner(inner: Frequency) -> Self {
        Self { inner }
    }
}

impl From<Frequency> for JsFrequency {
    fn from(value: Frequency) -> Self {
        Self::from_inner(value)
    }
}

#[wasm_bindgen(js_class = Frequency)]
impl JsFrequency {
    #[wasm_bindgen(constructor)]
    pub fn new(months: u8) -> Result<JsFrequency, JsValue> {
        JsFrequency::from_months(months)
    }

    #[wasm_bindgen(js_name = fromMonths)]
    pub fn from_months(months: u8) -> Result<JsFrequency, JsValue> {
        if months == 0 || months > 12 {
            return Err(js_error("Months must be in the range 1..=12"));
        }
        Ok(Self::from_inner(Frequency::Months(months)))
    }

    #[wasm_bindgen(js_name = fromDays)]
    pub fn from_days(days: u16) -> Result<JsFrequency, JsValue> {
        if days == 0 {
            return Err(js_error("Days must be greater than zero"));
        }
        Ok(Self::from_inner(Frequency::Days(days)))
    }

    #[wasm_bindgen(js_name = fromPaymentsPerYear)]
    pub fn from_payments_per_year(payments: u32) -> Result<JsFrequency, JsValue> {
        Frequency::from_payments_per_year(payments)
            .map(Self::from_inner)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = annual)]
    pub fn annual() -> JsFrequency {
        Self::from_inner(Frequency::annual())
    }

    #[wasm_bindgen(js_name = semiAnnual)]
    pub fn semi_annual() -> JsFrequency {
        Self::from_inner(Frequency::semi_annual())
    }

    #[wasm_bindgen(js_name = quarterly)]
    pub fn quarterly() -> JsFrequency {
        Self::from_inner(Frequency::quarterly())
    }

    #[wasm_bindgen(js_name = monthly)]
    pub fn monthly() -> JsFrequency {
        Self::from_inner(Frequency::monthly())
    }

    #[wasm_bindgen(js_name = biMonthly)]
    pub fn bi_monthly() -> JsFrequency {
        Self::from_inner(Frequency::bimonthly())
    }

    #[wasm_bindgen(js_name = biWeekly)]
    pub fn bi_weekly() -> JsFrequency {
        Self::from_inner(Frequency::biweekly())
    }

    #[wasm_bindgen(js_name = weekly)]
    pub fn weekly() -> JsFrequency {
        Self::from_inner(Frequency::weekly())
    }

    #[wasm_bindgen(js_name = daily)]
    pub fn daily() -> JsFrequency {
        Self::from_inner(Frequency::daily())
    }

    #[wasm_bindgen(getter)]
    pub fn months(&self) -> Option<u8> {
        self.inner.months()
    }

    #[wasm_bindgen(getter)]
    pub fn days(&self) -> Option<u16> {
        self.inner.days()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        if let Some(m) = self.inner.months() {
            format!("Frequency.months({m})")
        } else if let Some(d) = self.inner.days() {
            format!("Frequency.days({d})")
        } else {
            "Frequency(?)".to_string()
        }
    }
}

#[wasm_bindgen(js_name = DayCountContext)]
#[derive(Clone, Debug, Default)]
pub struct JsDayCountContext {
    calendar: Option<String>,
    frequency: Option<Frequency>,
}

#[wasm_bindgen(js_class = DayCountContext)]
impl JsDayCountContext {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsDayCountContext {
        Self::default()
    }

    #[wasm_bindgen(js_name = setCalendar)]
    pub fn set_calendar(&mut self, calendar: &JsCalendar) {
        self.calendar = Some(calendar.code().to_ascii_lowercase());
    }

    #[wasm_bindgen(js_name = setCalendarCode)]
    pub fn set_calendar_code(&mut self, code: &str) {
        self.calendar = Some(code.to_ascii_lowercase());
    }

    #[wasm_bindgen(js_name = clearCalendar)]
    pub fn clear_calendar(&mut self) {
        self.calendar = None;
    }

    #[wasm_bindgen(js_name = setFrequency)]
    pub fn set_frequency(&mut self, frequency: &JsFrequency) {
        self.frequency = Some(frequency.inner());
    }

    #[wasm_bindgen(js_name = clearFrequency)]
    pub fn clear_frequency(&mut self) {
        self.frequency = None;
    }

    #[wasm_bindgen(getter, js_name = calendarCode)]
    pub fn calendar_code(&self) -> Option<String> {
        self.calendar.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn frequency(&self) -> Option<JsFrequency> {
        self.frequency.map(JsFrequency::from)
    }
}

impl JsDayCountContext {
    pub(crate) fn into_core(self) -> Result<DayCountCtx<'static>, JsValue> {
        let calendar = match self.calendar {
            Some(code) => Some(resolve_calendar_ref(&code)?),
            None => None,
        };
        Ok(DayCountCtx {
            calendar,
            frequency: self.frequency,
        })
    }
}

#[wasm_bindgen(js_name = DayCount)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsDayCount {
    inner: DayCount,
}

impl JsDayCount {
    pub(crate) fn inner(&self) -> DayCount {
        self.inner
    }

    fn new(inner: DayCount) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = DayCount)]
impl JsDayCount {
    #[wasm_bindgen(constructor)]
    pub fn new_from_name(name: &str) -> Result<JsDayCount, JsValue> {
        JsDayCount::from_name(name)
    }

    #[wasm_bindgen(js_name = act360)]
    pub fn act_360() -> JsDayCount {
        JsDayCount::new(DayCount::Act360)
    }

    #[wasm_bindgen(js_name = act365f)]
    pub fn act_365f() -> JsDayCount {
        JsDayCount::new(DayCount::Act365F)
    }

    #[wasm_bindgen(js_name = act365l)]
    pub fn act_365l() -> JsDayCount {
        JsDayCount::new(DayCount::Act365L)
    }

    #[wasm_bindgen(js_name = thirty360)]
    pub fn thirty_360() -> JsDayCount {
        JsDayCount::new(DayCount::Thirty360)
    }

    #[wasm_bindgen(js_name = thirtyE360)]
    pub fn thirty_e_360() -> JsDayCount {
        JsDayCount::new(DayCount::ThirtyE360)
    }

    #[wasm_bindgen(js_name = actAct)]
    pub fn act_act() -> JsDayCount {
        JsDayCount::new(DayCount::ActAct)
    }

    #[wasm_bindgen(js_name = actActIsma)]
    pub fn act_act_isma() -> JsDayCount {
        JsDayCount::new(DayCount::ActActIsma)
    }

    #[wasm_bindgen(js_name = bus252)]
    pub fn bus_252() -> JsDayCount {
        JsDayCount::new(DayCount::Bus252)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsDayCount, JsValue> {
        parse_day_count_label(name)
            .map(JsDayCount::new)
            .ok_or_else(|| js_error(format!("Unknown day-count convention: {name}")))
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        match self.inner {
            DayCount::Act360 => "act_360",
            DayCount::Act365F => "act_365f",
            DayCount::Act365L => "act_365l",
            DayCount::Thirty360 => "thirty_360",
            DayCount::ThirtyE360 => "thirty_e_360",
            DayCount::ActAct => "act_act",
            DayCount::ActActIsma => "act_act_isma",
            DayCount::Bus252 => "bus_252",
            _ => "custom",
        }
        .to_string()
    }

    #[wasm_bindgen(js_name = yearFraction)]
    pub fn year_fraction(
        &self,
        start: &JsDate,
        end: &JsDate,
        context: Option<JsDayCountContext>,
    ) -> Result<f64, JsValue> {
        let ctx = match context {
            Some(ctx) => ctx.into_core()?,
            None => DayCountCtx::default(),
        };

        self.inner
            .year_fraction(start.inner(), end.inner(), ctx)
            .map_err(|e| js_error(e.to_string()))
    }
}

pub(crate) fn parse_day_count_label(label: &str) -> Option<DayCount> {
    let norm = label.to_ascii_lowercase().replace([' ', '-', '/'], "_");
    match norm.as_str() {
        "act_360" | "actual_360" | "act360" => Some(DayCount::Act360),
        "act_365f" | "actual_365f" | "act365f" => Some(DayCount::Act365F),
        "act_365l" | "actual_365l" | "act365l" | "act_365afb" => Some(DayCount::Act365L),
        "30_360" | "30u_360" | "thirty_360" => Some(DayCount::Thirty360),
        "30e_360" | "30_360e" | "thirty_e_360" => Some(DayCount::ThirtyE360),
        "act_act" | "actual_actual" | "actact" | "act_act_isda" => Some(DayCount::ActAct),
        "act_act_isma" | "actactisma" | "icma" => Some(DayCount::ActActIsma),
        "bus_252" | "business_252" | "bus252" => Some(DayCount::Bus252),
        _ => None,
    }
}
