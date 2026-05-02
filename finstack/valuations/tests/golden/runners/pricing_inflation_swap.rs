//! Domain runner for `rates.inflation_swap` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Inflation swap pricing golden runner.
pub struct InflationSwapRunner;

impl DomainRunner for InflationSwapRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
