pub(crate) mod parse;

use finstack_core::types::{CurveId, InstrumentId};

pub(crate) fn instrument_id_from_str(id: &str) -> InstrumentId {
    InstrumentId::new(id)
}

pub(crate) fn curve_id_from_str(id: &str) -> CurveId {
    CurveId::new(id)
}

pub(crate) fn optional_static_str(value: Option<String>) -> Option<&'static str> {
    value.map(|s| Box::leak(s.into_boxed_str()) as &'static str)
}