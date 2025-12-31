//! Instrument-level shock adapters.
//!
//! Applies price and spread shocks to instrument collections via pricing overrides,
//! enabling `OperationSpec` variants to affect subsets of a portfolio by type.
//! When instruments support pricing_overrides_mut(), shocks are applied functionally.
//! Otherwise, shocks are stored as metadata attributes for downstream processing.

use crate::adapters::traits::{ScenarioAdapter, ScenarioEffect};
use crate::engine::ExecutionContext;
use crate::error::Result;
use crate::spec::OperationSpec;
use finstack_valuations::instruments::{Attributes, Instrument};
use finstack_valuations::pricer::InstrumentType;

/// Adapter for instrument operations.
pub struct InstrumentAdapter;

impl ScenarioAdapter for InstrumentAdapter {
    fn try_generate_effects(
        &self,
        op: &OperationSpec,
        _ctx: &ExecutionContext,
    ) -> Result<Option<Vec<ScenarioEffect>>> {
        match op {
            OperationSpec::InstrumentPricePctByType {
                instrument_types,
                pct,
            } => Ok(Some(vec![ScenarioEffect::InstrumentPriceShock {
                types: Some(instrument_types.clone()),
                attrs: None,
                pct: *pct,
            }])),
            OperationSpec::InstrumentPricePctByAttr { attrs, pct } => {
                Ok(Some(vec![ScenarioEffect::InstrumentPriceShock {
                    types: None,
                    attrs: Some(attrs.clone()),
                    pct: *pct,
                }]))
            }
            OperationSpec::InstrumentSpreadBpByType {
                instrument_types,
                bp,
            } => Ok(Some(vec![ScenarioEffect::InstrumentSpreadShock {
                types: Some(instrument_types.clone()),
                attrs: None,
                bp: *bp,
            }])),
            OperationSpec::InstrumentSpreadBpByAttr { attrs, bp } => {
                Ok(Some(vec![ScenarioEffect::InstrumentSpreadShock {
                    types: None,
                    attrs: Some(attrs.clone()),
                    bp: *bp,
                }]))
            }
            _ => Ok(None),
        }
    }
}

/// Apply a percentage price shock to instruments matching the provided types.
pub fn apply_instrument_type_price_shock(
    instruments: &mut [Box<dyn Instrument>],
    instrument_types: &[InstrumentType],
    pct: f64,
) -> Result<usize> {
    let mut count = 0;
    let shock_decimal = pct / 100.0; // Convert percentage to decimal

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            // Try to apply via scenario_overrides for functional pricing effect
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_price_shock_pct = Some(shock_decimal);
            } else {
                // Fallback: store as metadata for downstream processing
                let shock_str = format!("{:.6}", shock_decimal);
                instrument
                    .attributes_mut()
                    .meta
                    .insert("scenario_price_shock_pct".to_string(), shock_str);
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Apply a spread shock (basis points) to instruments matching the provided types.
pub fn apply_instrument_type_spread_shock(
    instruments: &mut [Box<dyn Instrument>],
    instrument_types: &[InstrumentType],
    bp: f64,
) -> Result<usize> {
    let mut count = 0;

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            // Try to apply via scenario_overrides for functional pricing effect
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_spread_shock_bp = Some(bp);
            } else {
                // Fallback: store as metadata for downstream processing
                let shock_str = format!("{:.2}", bp);
                instrument
                    .attributes_mut()
                    .meta
                    .insert("scenario_spread_shock_bp".to_string(), shock_str);
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Apply a percentage price shock to instruments matching the provided attributes.
pub fn apply_instrument_attr_price_shock(
    instruments: &mut [Box<dyn Instrument>],
    attrs: &indexmap::IndexMap<String, String>,
    pct: f64,
) -> Result<(usize, Vec<String>)> {
    let mut warnings = Vec::new();
    let shock_decimal = pct / 100.0;
    let filters = normalise_filters(attrs);
    let mut count = 0;

    for instrument in instruments.iter_mut() {
        if matches_attr_filter(instrument.attributes(), &filters) {
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_price_shock_pct = Some(shock_decimal);
            } else {
                let shock_str = format!("{:.6}", shock_decimal);
                instrument
                    .attributes_mut()
                    .meta
                    .insert("scenario_price_shock_pct".to_string(), shock_str);
            }
            count += 1;
        }
    }

    if count == 0 {
        warnings.push(format!(
            "No instruments matched attribute filter {:?}",
            attrs
        ));
    }

    Ok((count, warnings))
}

/// Apply a spread shock to instruments matching the provided attributes.
pub fn apply_instrument_attr_spread_shock(
    instruments: &mut [Box<dyn Instrument>],
    attrs: &indexmap::IndexMap<String, String>,
    bp: f64,
) -> Result<(usize, Vec<String>)> {
    let mut warnings = Vec::new();
    let filters = normalise_filters(attrs);
    let mut count = 0;

    for instrument in instruments.iter_mut() {
        if matches_attr_filter(instrument.attributes(), &filters) {
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_spread_shock_bp = Some(bp);
            } else {
                let shock_str = format!("{:.2}", bp);
                instrument
                    .attributes_mut()
                    .meta
                    .insert("scenario_spread_shock_bp".to_string(), shock_str);
            }
            count += 1;
        }
    }

    if count == 0 {
        warnings.push(format!(
            "No instruments matched attribute filter {:?}",
            attrs
        ));
    }

    Ok((count, warnings))
}

/// Case-insensitive AND match across provided filters.
fn matches_attr_filter(attrs: &Attributes, filters: &[(String, String)]) -> bool {
    if filters.is_empty() {
        return true;
    }

    filters.iter().all(|(key, value)| {
        attrs
            .meta
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case(key) && v.eq_ignore_ascii_case(value))
    })
}

fn normalise_filters(attrs: &indexmap::IndexMap<String, String>) -> Vec<(String, String)> {
    attrs
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v.to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    // Note: Full testing requires concrete instrument implementations
    // These tests are placeholders for when integration testing is added
}
