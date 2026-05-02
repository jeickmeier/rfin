//! Domain runner for credit hazard calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Credit hazard calibration golden runner.
pub struct CalibrationHazardRunner;

impl DomainRunner for CalibrationHazardRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::calibration_common::run_hazard_fixture(fixture)
    }
}
