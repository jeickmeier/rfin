use js_sys::Array;
use rfin_core::dates::calendars::Gblo;
use rfin_core::dates::{adjust, BusDayConv, CompositeCalendar, HolidayCalendar, Target2};
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

#[derive(Clone)]
enum CalendarKind {
    Target2,
    Gblo,
    Union(Vec<CalendarKind>),
}

impl CalendarKind {
    fn is_holiday(&self, date: rfin_core::dates::Date) -> bool {
        match self {
            CalendarKind::Target2 => Target2::new().is_holiday(date),
            CalendarKind::Gblo => Gblo::new().is_holiday(date),
            CalendarKind::Union(list) => list.iter().any(|c| c.is_holiday(date)),
        }
    }

    fn collect_refs<'a>(&'a self, out: &mut Vec<Box<dyn HolidayCalendar + 'a>>) {
        match self {
            CalendarKind::Target2 => out.push(Box::new(Target2::new())),
            CalendarKind::Gblo => out.push(Box::new(Gblo::new())),
            CalendarKind::Union(list) => {
                for c in list {
                    c.collect_refs(out);
                }
            }
        }
    }
}

#[wasm_bindgen]
pub struct Calendar {
    kind: CalendarKind,
}

#[wasm_bindgen]
impl Calendar {
    #[wasm_bindgen(js_name = "target2")]
    pub fn target2() -> Calendar {
        Calendar {
            kind: CalendarKind::Target2,
        }
    }

    #[wasm_bindgen(js_name = "gblo")]
    pub fn gblo() -> Calendar {
        Calendar {
            kind: CalendarKind::Gblo,
        }
    }

    /// Return a calendar representing the union of `self` and `other`.
    #[wasm_bindgen(js_name = "union")]
    pub fn union(&self, other: &Calendar) -> Calendar {
        Calendar {
            kind: CalendarKind::Union(vec![self.kind.clone(), other.kind.clone()]),
        }
    }

    #[wasm_bindgen(js_name = "isHoliday")]
    pub fn is_holiday(&self, date: &Date) -> bool {
        self.kind.is_holiday(date.inner())
    }

    #[wasm_bindgen]
    pub fn adjust(&self, date: &Date, convention: BusDayConvention) -> Date {
        use rfin_core::dates::Date as CoreDate;
        let adj: CoreDate = match &self.kind {
            CalendarKind::Target2 => adjust(date.inner(), convention.into(), &Target2::new()),
            CalendarKind::Gblo => adjust(date.inner(), convention.into(), &Gblo::new()),
            CalendarKind::Union(list) => {
                let mut refs: Vec<Box<dyn HolidayCalendar>> = Vec::new();
                for c in list {
                    c.collect_refs(&mut refs);
                }
                // transform to &dyn slice
                let ref_slice: Vec<&dyn HolidayCalendar> =
                    refs.iter().map(|b| b.as_ref()).collect();
                let comp = CompositeCalendar::merge(&ref_slice);
                adjust(date.inner(), convention.into(), &comp)
            }
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
    for &name in rfin_core::dates::available_calendars() {
        arr.push(&JsValue::from_str(name));
    }
    arr
}
