//! Build script for the rfin-core crate – code-gen helpers.

mod currency_build;

use std::io;

fn main() -> io::Result<()> {
    currency_build::generate()?;
    generate_calendars()
}

// -----------------------------------------------------------------------------
// Auto-discover calendar modules (src/dates/calendars/*.rs)
// -----------------------------------------------------------------------------

fn generate_calendars() -> io::Result<()> {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path: PathBuf = Path::new(&out_dir).join("generated_calendars.rs");

    // Scan directory for .rs files (excluding mod.rs)
    let mut lines = String::new();
    let mut names = Vec::<String>::new();
    let cal_dir = Path::new("src/dates/calendars");
    if cal_dir.exists() {
        for entry in fs::read_dir(cal_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            if path.file_name().and_then(|s| s.to_str()) == Some("mod.rs") {
                continue;
            }
            let stem = path.file_stem().unwrap().to_str().unwrap();
            // Embed each calendar file as an inline module so the compiler can find it even
            // though this code is expanded from OUT_DIR (which would otherwise break the
            // standard module path resolution logic).
            lines.push_str(&format!(
                "pub mod {name} {{\n    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/dates/calendars/{name}.rs\"));\n}}\n",
                name = stem
            ));
            lines.push_str(&format!("pub use {name}::*;\n", name = stem));

            // Keep track of all discovered calendar names so we can expose a list later.
            names.push(stem.to_string());
        }
    }

    // Expose list of available calendars for runtime discovery
    if !names.is_empty() {
        names.sort();
        lines.push_str("\n/// List of built-in holiday calendar identifiers.\n");
        lines.push_str("pub const AVAILABLE_CALENDARS: &[&str] = &[\n");
        for name in &names {
            lines.push_str(&format!("    \"{}\",\n", name));
        }
        lines.push_str("];");

        lines.push_str("\n\n/// Returns slice of available calendar identifiers.\n");
        lines.push_str("#[inline]\n");
        lines.push_str("pub const fn available_calendars() -> &'static [&'static str] {\n    AVAILABLE_CALENDARS\n}\n");
    }

    let mut file = File::create(&dest_path)?;
    file.write_all(lines.as_bytes())?;

    // Rebuild if directory or its files change
    println!("cargo:rerun-if-changed=src/dates/calendars");

    Ok(())
}
