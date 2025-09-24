//! Build script for the rfin-core crate – code-gen helpers.

// Build helpers are consolidated under `src/generated` and data under `data/`.
// The older build helper modules under `build/` are no longer used here.

use std::io;
use std::fs;
use std::path::Path;

fn main() -> io::Result<()> {
    // No-op build script to keep cargo happy in environments where the legacy
    // build helpers are not present. Validate that required data files exist
    // so downstream modules that include generated files compile.
    let data_dir = Path::new("data");
    assert!(data_dir.join("iso_4217.csv").exists(), "missing iso_4217.csv");
    assert!(data_dir.join("chinese_new_year.csv").exists(), "missing chinese_new_year.csv");
    assert!(data_dir.join("calendars").exists(), "missing calendars directory");
    // Touch a file in OUT_DIR if needed by future steps
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let _ = fs::write(Path::new(&out_dir).join("build_ok.txt"), b"ok");
    }
    Ok(())
}
