//! Build script for the rfin-core crate – code-gen helpers.

#[path = "build/currency_build.rs"]
mod currency_build;
#[path = "build/generate_cny.rs"]
mod generate_cny;
#[path = "build/generate_calendars_from_json.rs"]
mod generate_calendars_from_json;
#[path = "build/generate_holidays.rs"]
mod generate_holidays;

use std::io;

fn main() -> io::Result<()> {
    currency_build::generate()?;
    // native calendar registry generation removed in favor of JSON-driven calendars
    generate_holidays::generate()?;
    generate_cny::generate()?;
    generate_calendars_from_json::generate()
}
