//! WASM bindings for the RustFin library.

use wasm_bindgen::prelude::*;

mod currency;
mod dates;
mod money;
mod utils;
mod calendar;
mod schedule;

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

// Re-export key types for ergonomic JS imports (`import { Date, Money, Currency } …`).
pub use currency::Currency;
pub use dates::{
    Date,
    DayCount,
    day_count_days as dayCountDays,
    day_count_year_fraction as dayCountYearFraction,
    third_wednesday as thirdWednesday,
    next_imm as nextImm,
    next_cds_date as nextCdsDate,
};
pub use money::Money;
pub use calendar::{Calendar, BusDayConvention};
pub use schedule::{Frequency, StubRule, generate_schedule as generateSchedule};
