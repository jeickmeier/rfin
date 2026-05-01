//! Domain runner for `rates.irs` golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Interest rate swap golden runner skeleton.
pub struct IrsRunner;

impl DomainRunner for IrsRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        crate::golden::runners::pricing_common::run_pricing_fixture(fixture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_bloomberg_swpm_fixture() {
        let fixture: GoldenFixture = serde_json::from_str(include_str!(
            "../data/pricing/irs/usd_sofr_5y_receive_fixed_swpm.json"
        ))
        .expect("fixture parses");

        let actuals = IrsRunner.run(&fixture).expect("runner prices fixture");

        assert!(actuals.contains_key("npv"));
        assert!(actuals.contains_key("par_rate"));
        assert!(actuals.contains_key("dv01"));
    }
}
