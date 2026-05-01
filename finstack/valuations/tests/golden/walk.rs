//! Walk-test for validating every committed golden fixture.

use crate::golden::schema::{GoldenFixture, SCHEMA_VERSION};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DATA_ROOT: &str = "tests/golden/data";
const MANUAL_SCREENSHOT_SOURCES: &[&str] = &["bloomberg-screen", "intex"];
const VALID_SOURCES: &[&str] = &[
    "quantlib",
    "bloomberg-api",
    "bloomberg-screen",
    "intex",
    "formula",
    "textbook",
];

fn collect_fixture_paths() -> Vec<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root = Path::new(manifest_dir).join(DATA_ROOT);
    if !root.exists() {
        return Vec::new();
    }

    let mut paths = Vec::new();
    walk_dir(&root, &mut paths);
    paths.sort();
    paths
}

fn walk_dir(dir: &Path, paths: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            paths.push(PathBuf::from(format!(
                "__read_dir_error__:{}:{}",
                dir.display(),
                err
            )));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("screenshots") {
                walk_dir(&path, paths);
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            paths.push(path);
        }
    }
}

fn validate_fixture(path: &Path) -> Result<(), String> {
    if path.to_string_lossy().starts_with("__read_dir_error__:") {
        return Err(path.display().to_string());
    }

    let raw = fs::read_to_string(path).map_err(|err| format!("read failed: {err}"))?;
    let fixture: GoldenFixture =
        serde_json::from_str(&raw).map_err(|err| format!("parse failed: {err}"))?;

    if fixture.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "schema_version is '{}', expected '{}'",
            fixture.schema_version, SCHEMA_VERSION
        ));
    }

    if !VALID_SOURCES.contains(&fixture.provenance.source.as_str()) {
        return Err(format!(
            "provenance.source '{}' is not recognized",
            fixture.provenance.source
        ));
    }

    validate_non_empty("name", &fixture.name)?;
    validate_non_empty("domain", &fixture.domain)?;
    validate_non_empty("description", &fixture.description)?;
    validate_non_empty("provenance.as_of", &fixture.provenance.as_of)?;
    validate_non_empty(
        "provenance.source_detail",
        &fixture.provenance.source_detail,
    )?;
    validate_non_empty("provenance.captured_by", &fixture.provenance.captured_by)?;
    validate_non_empty("provenance.captured_on", &fixture.provenance.captured_on)?;
    validate_non_empty(
        "provenance.last_reviewed_by",
        &fixture.provenance.last_reviewed_by,
    )?;
    validate_non_empty(
        "provenance.last_reviewed_on",
        &fixture.provenance.last_reviewed_on,
    )?;

    for metric in fixture.expected_outputs.keys() {
        if !fixture.tolerances.contains_key(metric) {
            return Err(format!(
                "expected_outputs has '{}' but tolerances does not",
                metric
            ));
        }
    }

    for (metric, tolerance) in &fixture.tolerances {
        if !fixture.expected_outputs.contains_key(metric) {
            return Err(format!(
                "tolerances has '{}' but expected_outputs does not",
                metric
            ));
        }
        if tolerance.abs.is_none() && tolerance.rel.is_none() {
            return Err(format!(
                "tolerance for '{}' has neither abs nor rel",
                metric
            ));
        }
    }

    if MANUAL_SCREENSHOT_SOURCES.contains(&fixture.provenance.source.as_str())
        && fixture.provenance.screenshots.is_empty()
    {
        return Err(format!(
            "source '{}' requires at least one screenshot",
            fixture.provenance.source
        ));
    }

    let parent = path.parent().ok_or("fixture has no parent dir")?;
    for shot in &fixture.provenance.screenshots {
        let shot_path = parent.join(&shot.path);
        if !shot_path.exists() {
            return Err(format!(
                "screenshot '{}' does not exist (resolved to {:?})",
                shot.path, shot_path
            ));
        }
        if !is_git_tracked(&shot_path) {
            return Err(format!(
                "screenshot '{}' exists but is not tracked by git",
                shot.path
            ));
        }
    }

    Ok(())
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} is empty"));
    }
    Ok(())
}

fn is_git_tracked(path: &Path) -> bool {
    Command::new("git")
        .arg("ls-files")
        .arg("--error-unmatch")
        .arg(path)
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn all_fixtures_well_formed() {
    let failures = collect_fixture_paths()
        .iter()
        .filter_map(|path| {
            validate_fixture(path).err().map(|msg| {
                if msg.starts_with("__read_dir_error__:") {
                    msg
                } else {
                    format!("{}: {}", path.display(), msg)
                }
            })
        })
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "{} fixture(s) failed validation:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
