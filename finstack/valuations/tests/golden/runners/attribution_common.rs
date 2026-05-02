//! Domain runner for flattened attribution golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Attribution golden runner.
pub struct AttributionRunner;

impl DomainRunner for AttributionRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::reject_flattened_outputs("attribution runner", fixture)
    }
}
