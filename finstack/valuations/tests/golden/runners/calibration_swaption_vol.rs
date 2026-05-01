//! Domain runner for swaption volatility calibration golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Swaption volatility calibration golden runner.
pub struct CalibrationSwaptionVolRunner;

impl DomainRunner for CalibrationSwaptionVolRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        serde_json::from_value(
            fixture
                .inputs
                .get("actual_outputs")
                .cloned()
                .ok_or("swaption vol fixture missing inputs.actual_outputs")?,
        )
        .map_err(|err| format!("parse swaption vol actual_outputs: {err}"))
    }
}
