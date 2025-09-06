use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::dates::{
    adjust, BusinessDayConvention as BusDayConv, CompositeCalendar, HolidayCalendar,
};
use js_sys::Array;
use wasm_bindgen::prelude::*;

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

#[wasm_bindgen]
pub struct Calendar {
    ids: Vec<String>,
}

#[wasm_bindgen]
impl Calendar {
    // ----------------------------
    // Constructors / factories
    // ----------------------------

    /// Create calendar from identifier (e.g. "gblo", "target2").
    #[wasm_bindgen(js_name = "fromId")]
    pub fn from_id(id: &str) -> Result<Calendar, JsValue> {
        if calendar_by_id(id).is_some() {
            Ok(Calendar {
                ids: vec![id.to_lowercase()],
            })
        } else {
            Err(JsValue::from_str(&format!("Unknown calendar id '{id}'")))
        }
    }

    /// Return a calendar representing the union of `self` and `other`.
    #[wasm_bindgen(js_name = "union")]
    pub fn union(&self, other: &Calendar) -> Calendar {
        let mut ids = self.ids.clone();
        for id in &other.ids {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        Calendar { ids }
    }

    #[wasm_bindgen(js_name = "isHoliday")]
    pub fn is_holiday(&self, date: &Date) -> bool {
        self.ids.iter().any(|id| {
            calendar_by_id(id.as_str())
                .map(|cal| cal.is_holiday(date.inner()))
                .unwrap_or(false)
        })
    }

    #[wasm_bindgen]
    pub fn adjust(&self, date: &Date, convention: BusDayConvention) -> Date {
        use finstack_core::dates::Date as CoreDate;
        let mut refs: Vec<&dyn HolidayCalendar> = Vec::new();
        for id in &self.ids {
            if let Some(cal) = calendar_by_id(id.as_str()) {
                refs.push(cal);
            }
        }
        if refs.is_empty() {
            return date.clone();
        }
        let adj: CoreDate = if refs.len() == 1 {
            adjust(date.inner(), convention.into(), refs[0])
                .expect("Date adjustment should not fail")
        } else {
            let comp = CompositeCalendar::new(&refs);
            adjust(date.inner(), convention.into(), &comp).expect("Date adjustment should not fail")
        };
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
    for &name in finstack_core::dates::available_calendars() {
        arr.push(&JsValue::from_str(name));
    }
    arr
}
