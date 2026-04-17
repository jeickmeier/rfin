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
use finstack_valuations::instruments::{Attributes, DynInstrument};
use finstack_valuations::pricer::InstrumentType;

fn accumulate_optional_shock(current: Option<f64>, delta: f64) -> f64 {
    current.unwrap_or(0.0) + delta
}

/// Accumulate a shock into an instrument's metadata map.
///
/// The serialized representation uses the default `f64` display (shortest
/// round-trippable decimal), which avoids the silent precision loss that
/// occurs when repeatedly parsing and re-formatting with a fixed precision.
fn accumulate_meta_shock(attrs: &mut Attributes, key: &str, delta: f64) {
    let current = attrs
        .meta
        .get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    attrs
        .meta
        .insert(key.to_string(), format!("{}", current + delta));
}

fn instrument_label(attrs: &Attributes) -> String {
    attrs
        .meta
        .get("id")
        .or_else(|| attrs.meta.get("instrument_id"))
        .or_else(|| attrs.meta.get("name"))
        .cloned()
        .unwrap_or_else(|| "<unidentified>".to_string())
}

fn fallback_warning(kind: &str, inst_type: &str, label: &str) -> String {
    format!(
        "Instrument {kind} shock fell back to metadata for instrument '{label}' \
         (type {inst_type}): the pricer does not expose scenario_overrides_mut(), \
         so the shock is recorded under scenario_{kind}_shock_* but will not affect \
         valuation unless the downstream consumer reads that metadata."
    )
}

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
///
/// `pct` is supplied in percentage points (`5.0 = +5%`) and converted into a
/// decimal shock before storage. When an instrument exposes
/// `scenario_overrides_mut()`, the shock is applied through pricing overrides.
/// Otherwise, the helper stores the shock in instrument metadata under
/// `scenario_price_shock_pct` for downstream consumers.
///
/// # Arguments
///
/// - `instruments`: Instrument collection to mutate.
/// - `instrument_types`: Instrument types to match.
/// - `pct`: Percentage-point price shock to apply.
///
/// # Returns
///
/// The number of matched instruments that were updated.
pub fn apply_instrument_type_price_shock(
    instruments: &mut [Box<DynInstrument>],
    instrument_types: &[InstrumentType],
    pct: f64,
) -> Result<(usize, Vec<String>)> {
    let mut count = 0;
    let mut warnings = Vec::new();
    let shock_decimal = pct / 100.0;

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_price_shock_pct = Some(accumulate_optional_shock(
                    overrides.scenario_price_shock_pct,
                    shock_decimal,
                ));
            } else {
                let label = instrument_label(instrument.attributes());
                let type_name = format!("{inst_type:?}");
                accumulate_meta_shock(
                    instrument.attributes_mut(),
                    "scenario_price_shock_pct",
                    shock_decimal,
                );
                warnings.push(fallback_warning("price", &type_name, &label));
            }
            count += 1;
        }
    }

    Ok((count, warnings))
}

/// Apply a spread shock (basis points) to instruments matching the provided types.
///
/// `bp` is additive in basis points. As with price shocks, the helper prefers
/// functional scenario overrides and falls back to metadata storage under
/// `scenario_spread_shock_bp` when direct overrides are unavailable.
///
/// # Arguments
///
/// - `instruments`: Instrument collection to mutate.
/// - `instrument_types`: Instrument types to match.
/// - `bp`: Additive spread shock in basis points.
///
/// # Returns
///
/// The number of matched instruments that were updated.
pub fn apply_instrument_type_spread_shock(
    instruments: &mut [Box<DynInstrument>],
    instrument_types: &[InstrumentType],
    bp: f64,
) -> Result<(usize, Vec<String>)> {
    let mut count = 0;
    let mut warnings = Vec::new();

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_spread_shock_bp = Some(accumulate_optional_shock(
                    overrides.scenario_spread_shock_bp,
                    bp,
                ));
            } else {
                let label = instrument_label(instrument.attributes());
                let type_name = format!("{inst_type:?}");
                accumulate_meta_shock(instrument.attributes_mut(), "scenario_spread_shock_bp", bp);
                warnings.push(fallback_warning("spread", &type_name, &label));
            }
            count += 1;
        }
    }

    Ok((count, warnings))
}

/// Apply a percentage price shock to instruments matching the provided attributes.
///
/// Matching is case-insensitive and uses AND semantics across the provided
/// `attrs` map. Only the metadata map on [`Attributes`] is compared; tag sets
/// are ignored. `pct` is supplied in percentage points and stored internally as
/// a decimal shock.
///
/// # Arguments
///
/// - `instruments`: Instrument collection to mutate.
/// - `attrs`: Metadata filters to match.
/// - `pct`: Percentage-point price shock to apply.
///
/// # Returns
///
/// A tuple `(matched_count, warnings)`. `warnings` contains a single message if
/// no instruments matched the attribute filter.
pub fn apply_instrument_attr_price_shock(
    instruments: &mut [Box<DynInstrument>],
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
                overrides.scenario_price_shock_pct = Some(accumulate_optional_shock(
                    overrides.scenario_price_shock_pct,
                    shock_decimal,
                ));
            } else {
                let label = instrument_label(instrument.attributes());
                let type_name = format!("{:?}", instrument.key());
                accumulate_meta_shock(
                    instrument.attributes_mut(),
                    "scenario_price_shock_pct",
                    shock_decimal,
                );
                warnings.push(fallback_warning("price", &type_name, &label));
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
///
/// Matching is case-insensitive and uses AND semantics across the provided
/// `attrs` map. Only the metadata map on [`Attributes`] is compared; tag sets
/// are ignored.
///
/// # Arguments
///
/// - `instruments`: Instrument collection to mutate.
/// - `attrs`: Metadata filters to match.
/// - `bp`: Additive spread shock in basis points.
///
/// # Returns
///
/// A tuple `(matched_count, warnings)`. `warnings` contains a single message if
/// no instruments matched the attribute filter.
pub fn apply_instrument_attr_spread_shock(
    instruments: &mut [Box<DynInstrument>],
    attrs: &indexmap::IndexMap<String, String>,
    bp: f64,
) -> Result<(usize, Vec<String>)> {
    let mut warnings = Vec::new();
    let filters = normalise_filters(attrs);
    let mut count = 0;

    for instrument in instruments.iter_mut() {
        if matches_attr_filter(instrument.attributes(), &filters) {
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_spread_shock_bp = Some(accumulate_optional_shock(
                    overrides.scenario_spread_shock_bp,
                    bp,
                ));
            } else {
                let label = instrument_label(instrument.attributes());
                let type_name = format!("{:?}", instrument.key());
                accumulate_meta_shock(instrument.attributes_mut(), "scenario_spread_shock_bp", bp);
                warnings.push(fallback_warning("spread", &type_name, &label));
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
///
/// Only the `meta` map on `Attributes` is compared; tag sets are ignored.
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
