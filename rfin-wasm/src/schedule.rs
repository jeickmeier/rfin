use wasm_bindgen::prelude::*;
use js_sys::Array;

use rfin_core::dates::{ScheduleBuilder, Frequency as CoreFrequency, StubRule as CoreStubRule};

use crate::dates::Date;
use crate::calendar::BusDayConvention;
// No business-day adjustment in this MVP

/// Coupon/payment frequency.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum Frequency {
    Annual,
    SemiAnnual,
    Quarterly,
    Monthly,
    BiWeekly,
    Weekly,
    Daily,
}

impl From<Frequency> for CoreFrequency {
    fn from(f: Frequency) -> Self {
        match f {
            Frequency::Annual => CoreFrequency::Annual,
            Frequency::SemiAnnual => CoreFrequency::SemiAnnual,
            Frequency::Quarterly => CoreFrequency::Quarterly,
            Frequency::Monthly => CoreFrequency::Monthly,
            Frequency::BiWeekly => CoreFrequency::BiWeekly,
            Frequency::Weekly => CoreFrequency::Weekly,
            Frequency::Daily => CoreFrequency::Daily,
        }
    }
}

/// Stub rule controlling handling of irregular periods (front/back stubs).
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub enum StubRule {
    None,
    ShortFront,
    ShortBack,
}

impl From<StubRule> for CoreStubRule {
    fn from(s: StubRule) -> Self {
        match s {
            StubRule::None => CoreStubRule::None,
            StubRule::ShortFront => CoreStubRule::ShortFront,
            StubRule::ShortBack => CoreStubRule::ShortBack,
        }
    }
}

/// Generate an inclusive date schedule between `start` and `end`.
/// Optionally supply a calendar & convention for business-day adjustment.
#[wasm_bindgen(js_name = "generateSchedule")]
pub fn generate_schedule(
    start: &Date,
    end: &Date,
    frequency: Frequency,
    convention: Option<BusDayConvention>,
    stub: Option<StubRule>,
) -> Array {
    let mut builder = ScheduleBuilder::new(start.inner(), end.inner(), frequency.into());

    if let Some(s) = stub {
        builder = builder.stub(s.into());
    }

    let mut sched = builder.generate();

    if let Some(conv) = convention {
        let cal = rfin_core::dates::Target2::new();
        for d in sched.iter_mut() {
            *d = rfin_core::dates::adjust(*d, conv.into(), &cal);
        }
    }

    let arr = Array::new();
    for d in sched {
        arr.push(&Date::from_core(d).into());
    }
    arr
} 