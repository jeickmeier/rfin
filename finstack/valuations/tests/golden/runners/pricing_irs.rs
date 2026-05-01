//! Domain runner for `rates.irs` golden fixtures.
//!
//! The real IRS pricing adapter lands in Phase 2 with the first IRS fixture.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use std::collections::BTreeMap;

/// Interest rate swap golden runner skeleton.
pub struct IrsRunner;

impl DomainRunner for IrsRunner {
    fn run(&self, _fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        Err("IrsRunner::run is implemented in Phase 2".to_string())
    }
}
