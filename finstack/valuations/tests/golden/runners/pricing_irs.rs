//! Domain runner for `rates.irs` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Interest rate swap golden runner skeleton.
pub struct IrsRunner;

impl DomainRunner for IrsRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn runs_bloomberg_swpm_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/data/pricing/irs/usd_sofr_5y_receive_fixed_swpm.json");
        let results = crate::golden::runner::run_golden_at_path(&path)
            .expect("IRS golden fixture should run end-to-end");
        let failures = results
            .iter()
            .filter(|result| !result.passed)
            .map(|result| result.failure_message(&path.display().to_string()))
            .collect::<Vec<_>>();
        assert!(
            failures.is_empty(),
            "IRS golden fixture mismatch(es):\n{}",
            failures.join("\n\n")
        );
    }
}
