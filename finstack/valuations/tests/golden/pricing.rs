//! Pricing-domain golden tests.

use crate::golden::runner::run_golden_at_path;
use crate::golden::walk::collect_fixture_paths_under;

#[test]
fn golden_pricing_fixtures_from_existing_json_files() {
    let paths =
        collect_fixture_paths_under("pricing").expect("pricing fixture discovery should succeed");
    assert!(
        !paths.is_empty(),
        "pricing fixture discovery did not find any JSON files"
    );

    let mut failures = Vec::new();
    for path in paths {
        match run_golden_at_path(&path) {
            Ok(results) => {
                failures.extend(
                    results
                        .iter()
                        .filter(|result| !result.passed)
                        .map(|result| result.failure_message(&path.display().to_string())),
                );
            }
            Err(err) => failures.push(format!("run fixture {path:?}: {err}")),
        }
    }

    assert!(
        failures.is_empty(),
        "{} pricing golden fixture failure(s):\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
