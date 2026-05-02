//! Domain runner for swaption volatility calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Swaption volatility calibration golden runner.
pub struct CalibrationSwaptionVolRunner;

impl DomainRunner for CalibrationSwaptionVolRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::reject_flattened_outputs("swaption vol calibration runner", fixture)
    }
}
