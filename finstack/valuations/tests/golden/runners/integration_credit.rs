//! Domain runner for credit calibrate-then-price integration goldens.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Credit integration golden runner.
pub struct IntegrationCreditRunner;

impl DomainRunner for IntegrationCreditRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::reject_flattened_outputs("credit integration runner", fixture)
    }
}
