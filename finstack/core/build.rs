//! Build script for finstack-core: generates calendar implementations from JSON.

#[path = "build/currency_build.rs"]
mod currency_build;
#[path = "build/generate_calendars.rs"]
mod generate_calendars;

use std::io;

fn main() -> io::Result<()> {
    currency_build::generate()?;
    generate_calendars::generate()
}
