//! Domain runner for flattened attribution golden fixtures.

use crate::golden::runner::DomainRunner;
use crate::golden::schema::GoldenFixture;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Attribution golden runner.
pub struct AttributionRunner;

#[derive(Debug, Deserialize)]
struct AttributionInputs {
    #[serde(default)]
    components: BTreeMap<String, f64>,
    #[serde(default)]
    sums: BTreeMap<String, Vec<String>>,
}

impl DomainRunner for AttributionRunner {
    fn run(&self, fixture: &GoldenFixture) -> Result<BTreeMap<String, f64>, String> {
        let inputs: AttributionInputs = serde_json::from_value(fixture.inputs.clone())
            .map_err(|err| format!("parse attribution inputs: {err}"))?;
        let mut actuals = inputs.components;
        let mut pending = inputs.sums;
        while !pending.is_empty() {
            let ready = pending
                .iter()
                .filter_map(|(output, terms)| {
                    terms
                        .iter()
                        .map(|term| actuals.get(term).copied())
                        .sum::<Option<f64>>()
                        .map(|total| (output.clone(), total))
                })
                .collect::<Vec<_>>();
            if ready.is_empty() {
                let missing = pending
                    .iter()
                    .map(|(output, terms)| {
                        let unresolved = terms
                            .iter()
                            .filter(|term| !actuals.contains_key(*term))
                            .cloned()
                            .collect::<Vec<_>>();
                        format!("{output}: {}", unresolved.join(", "))
                    })
                    .collect::<Vec<_>>();
                return Err(format!(
                    "attribution sums contain unresolved references: {}",
                    missing.join("; ")
                ));
            }
            for (output, total) in ready {
                pending.remove(&output);
                actuals.insert(output, total);
            }
        }
        Ok(actuals)
    }
}
