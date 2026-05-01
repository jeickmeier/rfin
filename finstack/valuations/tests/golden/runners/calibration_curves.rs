//! Domain runner for rates curve calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Rates curve calibration golden runner.
pub struct CalibrationCurvesRunner;

impl DomainRunner for CalibrationCurvesRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        serde_json::from_value(
            fixture
                .inputs
                .get("actual_outputs")
                .cloned()
                .ok_or("calibration fixture missing inputs.actual_outputs")?,
        )
        .map_err(|err| format!("parse calibration actual_outputs: {err}"))
    }
}
