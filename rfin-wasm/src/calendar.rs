use wasm_bindgen::prelude::*;
use rfin_core::dates::{BusDayConv, HolidayCalendar, Target2, adjust};
use js_sys::Array;

use crate::dates::Date;

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum BusDayConvention {
    Unadjusted,
    Following,
    ModifiedFollowing,
    Preceding,
    ModifiedPreceding,
}

impl From<BusDayConvention> for BusDayConv {
    fn from(c: BusDayConvention) -> Self {
        match c {
            BusDayConvention::Unadjusted => BusDayConv::Unadjusted,
            BusDayConvention::Following => BusDayConv::Following,
            BusDayConvention::ModifiedFollowing => BusDayConv::ModifiedFollowing,
            BusDayConvention::Preceding => BusDayConv::Preceding,
            BusDayConvention::ModifiedPreceding => BusDayConv::ModifiedPreceding,
        }
    }
}

#[derive(Clone, Copy)]
enum CalendarKind {
    Target2,
}

impl CalendarKind {
    fn calendar(&self) -> Target2 { Target2::new() }
}

#[wasm_bindgen]
pub struct Calendar {
    kind: CalendarKind,
}

#[wasm_bindgen]
impl Calendar {
    #[wasm_bindgen(js_name = "target2")]
    pub fn target2() -> Calendar {
        Calendar { kind: CalendarKind::Target2 }
    }

    #[wasm_bindgen(js_name = "isHoliday")]
    pub fn is_holiday(&self, date: &Date) -> bool {
        let cal = self.kind.calendar();
        cal.is_holiday(date.inner())
    }

    #[wasm_bindgen]
    pub fn adjust(&self, date: &Date, convention: BusDayConvention) -> Date {
        let cal = self.kind.calendar();
        let adj = adjust(date.inner(), convention.into(), &cal);
        Date::from_core(adj)
    }
}

// -----------------------------------------------------------------------------
// Module-level helpers (WASM)
// -----------------------------------------------------------------------------

/// Return the list of available built-in calendar identifiers.
#[wasm_bindgen(js_name = "availableCalendars")]
pub fn available_calendars() -> Array {
    let arr = Array::new();
    for &name in rfin_core::dates::available_calendars() {
        arr.push(&JsValue::from_str(name));
    }
    arr
} 