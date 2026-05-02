//! Domain runner for `credit.cds_tranche` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// CDS tranche pricing golden runner.
pub struct CdsTrancheRunner;

impl DomainRunner for CdsTrancheRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
