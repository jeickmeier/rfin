use crate::core::dates::date::JsDate;
use crate::core::error::calendar_not_found;
use crate::core::error::js_error;
use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::{adjust as core_adjust, BusinessDayConvention};
use finstack_core::dates::{CalendarMetadata, HolidayCalendar};
use std::str::FromStr;
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
    BusinessDayConvention::from_str(name)
        .map(Into::into)
        .map_err(|e| js_error(e.to_string()))
}

/// Check if a business day convention name is valid without throwing an error.
///
/// # Arguments
/// * `name` - Convention name (e.g., "following", "modified_following", "unadjusted")
///
/// # Returns
/// `true` if the string can be parsed as a valid convention, `false` otherwise.
#[wasm_bindgen(js_name = isValidBusinessDayConvention)]
pub fn is_valid_business_day_convention(name: &str) -> bool {
    BusinessDayConvention::from_str(name).is_ok()
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

/// Get all available business day convention names.
///
/// # Returns
/// Array of valid convention name strings.
#[wasm_bindgen(js_name = allBusinessDayConventions)]
pub fn all_business_day_conventions() -> Vec<String> {
    vec![
        "unadjusted".to_string(),
        "following".to_string(),
        "modified_following".to_string(),
        "preceding".to_string(),
        "modified_preceding".to_string(),
    ]
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

// ======================================================================
// CompositeCalendar
// ======================================================================

/// Mode for combining multiple holiday calendars.
#[wasm_bindgen(js_name = CompositeMode)]
#[derive(Clone, Copy, Debug)]
pub struct JsCompositeMode {
    inner: finstack_core::dates::CompositeMode,
}

impl JsCompositeMode {
    pub(crate) fn inner(&self) -> finstack_core::dates::CompositeMode {
        self.inner
    }
}

#[wasm_bindgen(js_class = CompositeMode)]
impl JsCompositeMode {
    /// Holiday if any sub-calendar marks the date as holiday (set union).
    #[wasm_bindgen(js_name = Union)]
    pub fn union() -> JsCompositeMode {
        JsCompositeMode {
            inner: finstack_core::dates::CompositeMode::Union,
        }
    }

    /// Holiday only if all sub-calendars mark the date as holiday (set intersection).
    #[wasm_bindgen(js_name = Intersection)]
    pub fn intersection() -> JsCompositeMode {
        JsCompositeMode {
            inner: finstack_core::dates::CompositeMode::Intersection,
        }
    }

    /// String representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Composite calendar combining multiple market calendars.
///
/// Supports union (holiday if ANY subcalendar is closed) and intersection
/// (holiday only if ALL subcalendars are closed) semantics.
///
/// @example
/// ```javascript
/// const cal = new CompositeCalendar(["TARGET2", "GBLO"], CompositeMode.Union());
/// const isHoliday = cal.isHoliday(date);
/// ```
#[wasm_bindgen(js_name = CompositeCalendar)]
pub struct JsCompositeCalendar {
    /// Stored as static references resolved from the global registry.
    calendars: Vec<&'static dyn HolidayCalendar>,
    mode: finstack_core::dates::CompositeMode,
    codes: Vec<String>,
}

#[wasm_bindgen(js_class = CompositeCalendar)]
impl JsCompositeCalendar {
    /// Create a composite calendar from calendar codes and combination mode.
    ///
    /// @param {string[]} codes - Calendar codes (e.g., `["TARGET2", "GBLO"]`)
    /// @param {CompositeMode} mode - Union or Intersection
    #[wasm_bindgen(constructor)]
    pub fn new(codes: Vec<String>, mode: &JsCompositeMode) -> Result<JsCompositeCalendar, JsValue> {
        let mut calendars = Vec::with_capacity(codes.len());
        for code in &codes {
            calendars.push(resolve_calendar_ref(code)?);
        }
        Ok(JsCompositeCalendar {
            calendars,
            mode: mode.inner(),
            codes,
        })
    }

    /// Check if a date is a holiday under this composite calendar.
    #[wasm_bindgen(js_name = isHoliday)]
    pub fn is_holiday(&self, date: &JsDate) -> bool {
        let d = date.inner();
        let refs: Vec<&dyn HolidayCalendar> = self.calendars.to_vec();
        let composite = finstack_core::dates::CompositeCalendar::with_mode(&refs, self.mode);
        composite.is_holiday(d)
    }

    /// Check if a date is a business day under this composite calendar.
    #[wasm_bindgen(js_name = isBusinessDay)]
    pub fn is_business_day(&self, date: &JsDate) -> bool {
        let d = date.inner();
        let refs: Vec<&dyn HolidayCalendar> = self.calendars.to_vec();
        let composite = finstack_core::dates::CompositeCalendar::with_mode(&refs, self.mode);
        composite.is_business_day(d)
    }

    /// Calendar codes in this composite.
    #[wasm_bindgen(getter)]
    pub fn codes(&self) -> Vec<String> {
        self.codes.clone()
    }

    /// Combination mode.
    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> JsCompositeMode {
        JsCompositeMode { inner: self.mode }
    }
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
