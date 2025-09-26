//! Copy pre-generated holiday/cny/calendar artifacts into OUT_DIR for builds.

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

const FILES: &[&str] = &[
    "src/generated/holiday_generated.rs",
    "src/generated/cny_generated.rs",
    "src/generated/generated_calendars.rs",
];

pub(crate) fn generate() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    for file in FILES {
        let source = manifest_dir.join(file);
        let filename = source.file_name().unwrap();
        let dest = out_dir.join(filename);
        fs::copy(&source, &dest)?;
    }
    Ok(())
}

