//! Domain runner for `fixed_income.convertible` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Convertible bond pricing golden runner.
pub struct ConvertibleRunner;

impl DomainRunner for ConvertibleRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
