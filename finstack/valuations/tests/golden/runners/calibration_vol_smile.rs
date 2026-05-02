//! Domain runner for equity and FX volatility smile calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Volatility smile calibration golden runner.
pub struct CalibrationVolSmileRunner;

impl DomainRunner for CalibrationVolSmileRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::reject_flattened_outputs("vol smile calibration runner", fixture)
    }
}
