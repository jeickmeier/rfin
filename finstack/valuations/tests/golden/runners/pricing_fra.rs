//! Domain runner for `rates.fra` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Forward rate agreement golden runner.
pub struct FraRunner;

impl DomainRunner for FraRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
