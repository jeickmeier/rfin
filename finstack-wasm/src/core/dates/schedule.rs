use crate::core::common::parse::ParseFromString;
use crate::core::dates::calendar::{resolve_calendar_ref, JsBusinessDayConvention, JsCalendar};
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsTenor;
use crate::core::error::js_error;
use finstack_core::dates::Date as CoreDate;
use finstack_core::dates::{ScheduleBuilder as CoreScheduleBuilder, ScheduleSpec, StubKind};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = StubKind)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsStubKind {
    inner: StubKind,
}

impl JsStubKind {
    pub(crate) fn inner(&self) -> StubKind {
        self.inner
    }

    fn new(inner: StubKind) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = StubKind)]
impl JsStubKind {
    #[wasm_bindgen(constructor)]
    pub fn new_from_name(name: &str) -> Result<JsStubKind, JsValue> {
        JsStubKind::from_name(name)
    }

    #[wasm_bindgen(js_name = none)]
    pub fn none() -> JsStubKind {
        JsStubKind::new(StubKind::None)
    }

    #[wasm_bindgen(js_name = shortFront)]
    pub fn short_front() -> JsStubKind {
        JsStubKind::new(StubKind::ShortFront)
    }

    #[wasm_bindgen(js_name = shortBack)]
    pub fn short_back() -> JsStubKind {
        JsStubKind::new(StubKind::ShortBack)
    }

    #[wasm_bindgen(js_name = longFront)]
    pub fn long_front() -> JsStubKind {
        JsStubKind::new(StubKind::LongFront)
    }

    #[wasm_bindgen(js_name = longBack)]
    pub fn long_back() -> JsStubKind {
        JsStubKind::new(StubKind::LongBack)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsStubKind, JsValue> {
        StubKind::parse_from_string(name).map(JsStubKind::new)
    }

    #[wasm_bindgen(js_name = name)]
    pub fn name(&self) -> String {
        match self.inner {
            StubKind::None => "none",
            StubKind::ShortFront => "short_front",
            StubKind::ShortBack => "short_back",
            StubKind::LongFront => "long_front",
            StubKind::LongBack => "long_back",
            _ => "custom",
        }
        .to_string()
    }
}

#[wasm_bindgen(js_name = ScheduleBuilder)]
pub struct JsScheduleBuilder {
    inner: CoreScheduleBuilder<'static>,
    start: CoreDate,
    end: CoreDate,
}

impl JsScheduleBuilder {
    fn new_with(builder: CoreScheduleBuilder<'static>, start: CoreDate, end: CoreDate) -> Self {
        Self {
            inner: builder,
            start,
            end,
        }
    }
}

#[wasm_bindgen(js_class = ScheduleBuilder)]
impl JsScheduleBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(start: &JsDate, end: &JsDate) -> Result<JsScheduleBuilder, JsValue> {
        let start_date = start.inner();
        let end_date = end.inner();
        let builder =
            CoreScheduleBuilder::new(start_date, end_date).map_err(|e| js_error(e.to_string()))?;
        Ok(Self::new_with(builder, start_date, end_date))
    }

    #[wasm_bindgen(js_name = frequency)]
    pub fn frequency(self, frequency: &JsTenor) -> JsScheduleBuilder {
        JsScheduleBuilder::new_with(
            self.inner.frequency(frequency.inner()),
            self.start,
            self.end,
        )
    }

    #[wasm_bindgen(js_name = stubRule)]
    pub fn stub_rule(self, stub: &JsStubKind) -> JsScheduleBuilder {
        JsScheduleBuilder::new_with(self.inner.stub_rule(stub.inner()), self.start, self.end)
    }

    #[wasm_bindgen(js_name = adjustWith)]
    pub fn adjust_with(
        self,
        convention: JsBusinessDayConvention,
        calendar: &JsCalendar,
    ) -> Result<JsScheduleBuilder, JsValue> {
        let cal = resolve_calendar_ref(&calendar.code())?;
        Ok(JsScheduleBuilder::new_with(
            self.inner.adjust_with(convention.into(), cal),
            self.start,
            self.end,
        ))
    }

    #[wasm_bindgen(js_name = endOfMonth)]
    pub fn end_of_month(self, enabled: bool) -> JsScheduleBuilder {
        JsScheduleBuilder::new_with(self.inner.end_of_month(enabled), self.start, self.end)
    }

    #[wasm_bindgen(js_name = cdsImm)]
    pub fn cds_imm(self) -> JsScheduleBuilder {
        JsScheduleBuilder::new_with(self.inner.cds_imm(), self.start, self.end)
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsSchedule, JsValue> {
        self.inner
            .build()
            .map(|schedule| JsSchedule {
                dates: schedule.dates,
            })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("ScheduleBuilder(start={}, end={})", self.start, self.end)
    }
}

#[wasm_bindgen(js_name = Schedule)]
#[derive(Clone, Debug)]
pub struct JsSchedule {
    dates: Vec<CoreDate>,
}

#[wasm_bindgen(js_class = Schedule)]
impl JsSchedule {
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.dates.len()
    }

    #[wasm_bindgen(js_name = toArray)]
    pub fn to_array(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        for date in &self.dates {
            array.push(&JsValue::from(JsDate::from_core(*date)));
        }
        array
    }
}

#[wasm_bindgen(js_name = ScheduleSpec)]
#[derive(Clone, Debug)]
pub struct JsScheduleSpec {
    inner: ScheduleSpec,
}

#[wasm_bindgen(js_class = ScheduleSpec)]
impl JsScheduleSpec {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)] // WASM bindings often require many constructor arguments
    pub fn new(
        start: &JsDate,
        end: &JsDate,
        frequency: &JsTenor,
        stub: Option<JsStubKind>,
        business_day_convention: Option<JsBusinessDayConvention>,
        calendar_id: Option<String>,
        end_of_month: bool,
        cds_imm_mode: bool,
        graceful: bool,
    ) -> JsScheduleSpec {
        let inner = ScheduleSpec {
            start: start.inner(),
            end: end.inner(),
            frequency: frequency.inner(),
            stub: stub.map(|s| s.inner()).unwrap_or(StubKind::None),
            business_day_convention: business_day_convention.map(Into::into),
            calendar_id,
            end_of_month,
            imm_mode: false,
            cds_imm_mode,
            graceful,
            allow_missing_calendar: false,
        };
        JsScheduleSpec { inner }
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(&self) -> Result<JsSchedule, JsValue> {
        self.inner
            .build()
            .map(|schedule| JsSchedule {
                dates: schedule.dates,
            })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = calendarId)]
    pub fn calendar_id(&self) -> Option<String> {
        self.inner.calendar_id.clone()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(payload: &str) -> Result<JsScheduleSpec, JsValue> {
        serde_json::from_str(payload)
            .map(|inner| JsScheduleSpec { inner })
            .map_err(|e| js_error(e.to_string()))
    }
}
