//! Domain runner for rates curve calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Rates curve calibration golden runner.
pub struct CalibrationCurvesRunner;

impl DomainRunner for CalibrationCurvesRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::calibration_common::run_curve_fixture(fixture)
    }
}
