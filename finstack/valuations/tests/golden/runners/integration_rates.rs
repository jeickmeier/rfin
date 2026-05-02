//! Domain runner for rates calibrate-then-price integration goldens.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Rates integration golden runner.
pub struct IntegrationRatesRunner;

impl DomainRunner for IntegrationRatesRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::reject_flattened_outputs("rates integration runner", fixture)
    }
}
