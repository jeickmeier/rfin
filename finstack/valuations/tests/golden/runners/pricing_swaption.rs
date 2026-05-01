//! Domain runner for `rates.swaption` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Swaption golden runner.
pub struct SwaptionRunner;

impl DomainRunner for SwaptionRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
