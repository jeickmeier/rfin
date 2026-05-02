//! Domain runner for credit calibrate-then-price integration goldens.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Credit integration golden runner.
pub struct IntegrationCreditRunner;

impl DomainRunner for IntegrationCreditRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::integration_common::run_credit_integration(fixture)
    }
}
