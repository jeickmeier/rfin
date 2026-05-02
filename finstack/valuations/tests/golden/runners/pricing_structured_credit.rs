//! Domain runner for `fixed_income.structured_credit` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Structured credit pricing golden runner.
pub struct StructuredCreditRunner;

impl DomainRunner for StructuredCreditRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}
