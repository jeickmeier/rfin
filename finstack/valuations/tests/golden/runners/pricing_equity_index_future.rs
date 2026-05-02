//! Domain runner for `equity.equity_index_future` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Equity index future pricing golden runner.
pub struct EquityIndexFutureRunner;

impl DomainRunner for EquityIndexFutureRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
