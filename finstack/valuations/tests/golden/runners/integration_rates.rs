//! Domain runner for rates calibrate-then-price integration goldens.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Rates integration golden runner.
pub struct IntegrationRatesRunner;

impl DomainRunner for IntegrationRatesRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        serde_json::from_value(
            fixture
                .inputs
                .get("actual_outputs")
                .cloned()
                .ok_or("integration fixture missing inputs.actual_outputs")?,
        )
        .map_err(|err| format!("parse integration actual_outputs: {err}"))
    }
}
