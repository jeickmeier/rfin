//! Domain runner for `fixed_income.inflation_linked_bond` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Inflation-linked bond pricing golden runner.
pub struct InflationLinkedBondRunner;

impl DomainRunner for InflationLinkedBondRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
