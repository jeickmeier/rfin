//! Domain runner for swaption volatility calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Swaption volatility calibration golden runner.
pub struct CalibrationSwaptionVolRunner;

impl DomainRunner for CalibrationSwaptionVolRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::calibration_common::run_sabr_cube_fixture(fixture)
    }
}
