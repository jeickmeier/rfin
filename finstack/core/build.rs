//! Build script for the rfin-core crate – code-gen helpers.

#[path = "build/currency_build.rs"]
mod currency_build;
#[path = "build/generate_calendars.rs"]
mod generate_calendars;
#[path = "build/generate_cny.rs"]
mod generate_cny;
#[path = "build/generate_holidays.rs"]
mod generate_holidays;

use std::io;

fn main() -> io::Result<()> {
    currency_build::generate()?;
    generate_calendars::generate()?;
    generate_holidays::generate()?;
    generate_cny::generate()
}
