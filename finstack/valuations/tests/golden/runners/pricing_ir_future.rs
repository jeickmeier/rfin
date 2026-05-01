//! Domain runner for `rates.ir_future` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Interest-rate future golden runner.
pub struct IrFutureRunner;

impl DomainRunner for IrFutureRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
