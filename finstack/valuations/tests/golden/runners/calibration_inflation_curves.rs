//! Domain runner for inflation curve calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Inflation curve calibration golden runner.
pub struct CalibrationInflationCurvesRunner;

impl DomainRunner for CalibrationInflationCurvesRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::calibration_common::run_curve_fixture(fixture)
    }
}
