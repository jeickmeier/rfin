//! Domain runner for `rates.cap_floor` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Cap/floor golden runner.
pub struct CapFloorRunner;

impl DomainRunner for CapFloorRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
