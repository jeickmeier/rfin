//! Domain runner for `rates.deposit` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Money-market deposit golden runner.
pub struct DepositRunner;

impl DomainRunner for DepositRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
