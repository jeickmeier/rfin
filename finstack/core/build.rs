//! Build script for the rfin-core crate – code-gen helpers.

#[path = "build/currency_build.rs"]
mod currency_build;
#[path = "build/generate_calendars.rs"]
mod generate_calendars;

use std::io;

fn main() -> io::Result<()> {
    currency_build::generate()?;
    generate_calendars::generate()
}
