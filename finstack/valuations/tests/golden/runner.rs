//! Runner trait, fixture dispatch, and `run_golden!` test macro.

use crate::golden::schema::GoldenFixture;
use crate::golden::tolerance::{compare, ComparisonResult};
use std::collections::BTreeMap;

/// One runner per fixture domain. Runners build canonical API inputs and extract metrics.
pub trait DomainRunner {
    /// Run the canonical computation for this fixture.
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String>;
}

/// Dispatch a fixture to its domain runner by `domain` field.
pub fn dispatch(fixture: &GoldenFixture) -> Result<Box<dyn DomainRunner>, String> {
    match fixture.domain.as_str() {
        "rates.irs" => Ok(Box::new(crate::golden::runners::pricing_irs::IrsRunner)),
        other => Err(format!("no runner registered for domain '{other}'")),
    }
}

/// Run a fixture end-to-end and return one comparison result per expected metric.
pub fn run_fixture(fixture: &GoldenFixture) -> Result<Vec<ComparisonResult>, String> {
    let runner = dispatch(fixture)?;
    let actuals = runner.run(fixture)?;
    fixture
        .expected_outputs
        .iter()
        .map(|(metric, expected)| {
            let actual = actuals
                .get(metric)
                .copied()
                .ok_or_else(|| format!("runner did not produce metric '{metric}'"))?;
            let tolerance = fixture
                .tolerances
                .get(metric)
                .ok_or_else(|| format!("no tolerance for metric '{metric}'"))?;
            Ok(compare(metric, actual, *expected, tolerance))
        })
        .collect()
}

/// Run one golden fixture from a path relative to `tests/golden/data/`.
#[macro_export]
macro_rules! run_golden {
    ($relative_path:expr) => {{
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir)
            .join("tests/golden/data")
            .join($relative_path);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read fixture {:?}: {}", path, err));
        let fixture: $crate::golden::schema::GoldenFixture = serde_json::from_str(&raw)
            .unwrap_or_else(|err| panic!("parse fixture {:?}: {}", path, err));
        let results = $crate::golden::runner::run_fixture(&fixture)
            .unwrap_or_else(|err| panic!("run fixture {:?}: {}", path, err));
        let failures = results
            .iter()
            .filter(|result| !result.passed)
            .map(|result| result.failure_message(&path.display().to_string()))
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            panic!(
                "{} metric(s) failed:\n{}",
                failures.len(),
                failures.join("\n\n")
            );
        }
    }};
}
