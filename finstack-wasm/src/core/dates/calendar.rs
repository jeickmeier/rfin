use crate::core::common::parse::ParseFromString;
use crate::core::dates::date::JsDate;
use crate::core::error::calendar_not_found;
use crate::core::error::js_error;
use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::{adjust as core_adjust, BusinessDayConvention};
use finstack_core::dates::{CalendarMetadata, HolidayCalendar};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = BusinessDayConvention)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsBusinessDayConvention {
    Unadjusted,
    Following,
    ModifiedFollowing,
    Preceding,
    ModifiedPreceding,
}

impl From<JsBusinessDayConvention> for BusinessDayConvention {
    fn from(value: JsBusinessDayConvention) -> Self {
        match value {
            JsBusinessDayConvention::Unadjusted => BusinessDayConvention::Unadjusted,
            JsBusinessDayConvention::Following => BusinessDayConvention::Following,
            JsBusinessDayConvention::ModifiedFollowing => BusinessDayConvention::ModifiedFollowing,
            JsBusinessDayConvention::Preceding => BusinessDayConvention::Preceding,
            JsBusinessDayConvention::ModifiedPreceding => BusinessDayConvention::ModifiedPreceding,
        }
    }
}

impl From<BusinessDayConvention> for JsBusinessDayConvention {
    fn from(value: BusinessDayConvention) -> Self {
        match value {
            BusinessDayConvention::Unadjusted => JsBusinessDayConvention::Unadjusted,
            BusinessDayConvention::Following => JsBusinessDayConvention::Following,
            BusinessDayConvention::ModifiedFollowing => JsBusinessDayConvention::ModifiedFollowing,
            BusinessDayConvention::Preceding => JsBusinessDayConvention::Preceding,
            BusinessDayConvention::ModifiedPreceding => JsBusinessDayConvention::ModifiedPreceding,
            _ => JsBusinessDayConvention::Unadjusted,
        }
    }
}

#[wasm_bindgen(js_name = businessDayConventionFromName)]
pub fn business_day_convention_from_name(name: &str) -> Result<JsBusinessDayConvention, JsValue> {
    BusinessDayConvention::parse_from_string(name).map(Into::into)
}

#[wasm_bindgen(js_name = businessDayConventionName)]
pub fn business_day_convention_name(value: JsBusinessDayConvention) -> String {
    match value {
        JsBusinessDayConvention::Unadjusted => "unadjusted",
        JsBusinessDayConvention::Following => "following",
        JsBusinessDayConvention::ModifiedFollowing => "modified_following",
        JsBusinessDayConvention::Preceding => "preceding",
        JsBusinessDayConvention::ModifiedPreceding => "modified_preceding",
    }
    .to_string()
}

#[wasm_bindgen(js_name = Calendar)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsCalendar {
    code: String,
    name: String,
    ignore_weekends: bool,
}

#[wasm_bindgen(js_class = Calendar)]
impl JsCalendar {
    #[wasm_bindgen(constructor)]
    pub fn new(code: &str) -> Result<JsCalendar, JsValue> {
        build_calendar(code)
    }

    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(getter, js_name = ignoreWeekends)]
    pub fn ignore_weekends(&self) -> bool {
        self.ignore_weekends
    }

    #[wasm_bindgen(js_name = isBusinessDay)]
    pub fn is_business_day(&self, date: &JsDate) -> Result<bool, JsValue> {
        let cal = resolve_calendar_ref(&self.code)?;
        Ok(cal.is_business_day(date.inner()))
    }

    #[wasm_bindgen(js_name = isHoliday)]
    pub fn is_holiday(&self, date: &JsDate) -> Result<bool, JsValue> {
        let cal = resolve_calendar_ref(&self.code)?;
        Ok(cal.is_holiday(date.inner()))
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{} ({})", self.code, self.name)
    }
}

#[wasm_bindgen(js_name = availableCalendars)]
pub fn available_calendars() -> Result<Vec<JsCalendar>, JsValue> {
    let registry = CalendarRegistry::global();
    registry
        .available_ids()
        .iter()
        .map(|code| build_calendar(code))
        .collect()
}

#[wasm_bindgen(js_name = availableCalendarCodes)]
pub fn available_calendar_codes() -> Vec<String> {
    let registry = CalendarRegistry::global();
    registry
        .available_ids()
        .iter()
        .map(|code| code.to_string())
        .collect()
}

#[wasm_bindgen(js_name = getCalendar)]
pub fn get_calendar(code: &str) -> Result<JsCalendar, JsValue> {
    build_calendar(code)
}

#[wasm_bindgen(js_name = adjust)]
pub fn adjust(
    date: &JsDate,
    convention: JsBusinessDayConvention,
    calendar: &JsCalendar,
) -> Result<JsDate, JsValue> {
    let cal = resolve_calendar_ref(&calendar.code)?;
    let adjusted =
        core_adjust(date.inner(), convention.into(), cal).map_err(|e| js_error(e.to_string()))?;
    Ok(JsDate::from_core(adjusted))
}

pub(crate) fn resolve_calendar_ref(code: &str) -> Result<&'static dyn HolidayCalendar, JsValue> {
    let registry = CalendarRegistry::global();
    let normalized = code.to_ascii_lowercase();
    registry
        .resolve_str(&normalized)
        .ok_or_else(|| calendar_not_found(code))
}

fn build_calendar(code: &str) -> Result<JsCalendar, JsValue> {
    let normalized = code.to_ascii_lowercase();
    let calendar = resolve_calendar_ref(&normalized)?;
    if let Some(CalendarMetadata {
        id,
        name,
        ignore_weekends,
    }) = calendar.metadata()
    {
        Ok(JsCalendar {
            code: id.to_string(),
            name: name.to_string(),
            ignore_weekends,
        })
    } else {
        Ok(JsCalendar {
            code: normalized,
            name: code.to_string(),
            ignore_weekends: false,
        })
    }
}
