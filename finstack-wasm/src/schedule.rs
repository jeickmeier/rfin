use js_sys::Array;
use wasm_bindgen::prelude::*;

use finstack_core::dates::{Frequency as CoreFrequency, ScheduleBuilder, StubKind as CoreStubRule};

use crate::calendar::BusDayConvention;
use crate::dates::Date;
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
            Frequency::Annual => CoreFrequency::Months(12),
            Frequency::SemiAnnual => CoreFrequency::Months(6),
            Frequency::Quarterly => CoreFrequency::Months(3),
            Frequency::Monthly => CoreFrequency::Months(1),
            Frequency::BiWeekly => CoreFrequency::Days(14),
            Frequency::Weekly => CoreFrequency::Days(7),
            Frequency::Daily => CoreFrequency::Days(1),
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
    // Validate range; throw JS error early instead of panicking
    if start.inner() > end.inner() {
        wasm_bindgen::throw_str("Invalid date range: start must be before or equal to end");
    }
    let mut builder = ScheduleBuilder::new(start.inner(), end.inner()).frequency(frequency.into());

    if let Some(s) = stub {
        builder = builder.stub_rule(s.into());
    }

    let arr = Array::new();
    if let Some(conv) = convention {
        let cal = finstack_core::dates::Target2;
        for d in builder.adjust_with(conv.into(), &cal).build() {
            arr.push(&Date::from_core(d).into());
        }
    } else {
        for d in builder.build_raw() {
            arr.push(&Date::from_core(d).into());
        }
    }
    arr
}
