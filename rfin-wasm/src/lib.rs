//! WASM bindings for the RustFin library.

use wasm_bindgen::prelude::*;

mod calendar;
mod currency;
mod dates;
mod money;
mod schedule;
mod utils;

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

// Re-export key types for ergonomic JS imports (`import { Date, Money, Currency } …`).
pub use calendar::{BusDayConvention, Calendar};
pub use currency::Currency;
pub use dates::{
    day_count_days as dayCountDays, day_count_year_fraction as dayCountYearFraction,
    next_cds_date as nextCdsDate, next_imm as nextImm, third_wednesday as thirdWednesday, Date,
    DayCount,
};
pub use money::Money;
pub use schedule::{generate_schedule as generateSchedule, Frequency, StubRule};
