//! Build script for finstack-core: generates calendar implementations from JSON.

#[path = "build/generate_calendars.rs"]
mod generate_calendars;
#[path = "build/currency_build.rs"]
mod currency_build;

use std::io;

fn main() -> io::Result<()> {
    currency_build::generate()?;
    generate_calendars::generate()
}
