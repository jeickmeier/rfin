//! Static envelope validator and dependency-graph utilities.
//!
//! [`validate`] runs all structural checks (missing dependencies, undefined
//! `quote_set`s) and returns a [`ValidationReport`] listing every error found
//! plus the dependency graph of the steps. No solver is invoked — the
//! validator runs in microseconds.
//!
//! Genuine cycles cannot occur in the current `CalibrationPlan` model:
//! steps execute in declared order and read only from `market_data` /
//! `prior_market` or curves produced by *earlier* steps. A self-referential
//! step would surface as a [`EnvelopeError::MissingDependency`] rather than a
//! cycle.
//!
//! [`dry_run`] and [`dependency_graph_json`] are JSON-string wrappers for
//! cross-binding consumption (Python / WASM).

// `EnvelopeError` is intentionally large (carries available-IDs lists, etc.)
// because the cross-binding consumers want all the diagnostic context in
// one shot. Boxing the error would harm ergonomics on a cold error path.
#![allow(clippy::result_large_err)]

use serde::Serialize;
use std::collections::{BTreeSet, HashSet};

use crate::calibration::api::errors::EnvelopeError;
use crate::calibration::api::schema::{CalibrationEnvelope, CalibrationStep, StepParams};

/// Result of [`validate`]. Always contains the dependency graph; `errors` is
/// empty when the envelope is structurally valid.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    /// All errors found in a single pass; empty if the envelope is valid.
    pub errors: Vec<EnvelopeError>,
    /// Topological view of the steps' inputs and outputs.
    pub dependency_graph: DependencyGraph,
}

/// Static dependency graph derived from a [`CalibrationEnvelope`].
#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    /// Curve / surface IDs available at the start of execution, contributed
    /// by `market_data` and `prior_market`.
    pub initial_ids: Vec<String>,
    /// Per-step inputs and outputs in declared order.
    pub nodes: Vec<DependencyNode>,
}

/// A single step's view of the dependency graph.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyNode {
    /// Zero-based index in `plan.steps`.
    pub step_index: usize,
    /// Step identifier.
    pub step_id: String,
    /// Step kind (`"discount"`, `"forward"`, ...).
    pub kind: String,
    /// Curve / surface IDs the step depends on. Each must be either in
    /// `initial_ids` or produced by an earlier step.
    pub reads: Vec<String>,
    /// Curve / surface ID(s) the step produces.
    pub writes: Vec<String>,
}

/// Run all static validation checks.
///
/// Always returns a [`ValidationReport`]; inspect `errors` to see what failed.
/// The validator is solver-free — runs in microseconds, suitable as a
/// pre-flight check before invoking [`engine::execute`](super::engine::execute).
pub fn validate(envelope: &CalibrationEnvelope) -> ValidationReport {
    let mut errors = Vec::new();
    let initial_ids = collect_initial_ids(envelope);
    let nodes = build_nodes(&envelope.plan.steps);

    check_quote_sets(envelope, &mut errors);
    check_market_data_uniqueness(envelope, &mut errors);
    check_quote_sets_resolve(envelope, &mut errors);
    check_dependencies(&initial_ids, &nodes, &mut errors);

    let mut sorted_initial: Vec<String> = initial_ids.into_iter().collect();
    sorted_initial.sort();

    ValidationReport {
        errors,
        dependency_graph: DependencyGraph {
            initial_ids: sorted_initial,
            nodes,
        },
    }
}

/// Wrap [`validate`] to take a JSON string.
///
/// Returns the report serialized as pretty-printed JSON. Returns an
/// [`EnvelopeError::JsonParse`] if the envelope is malformed.
pub fn dry_run(envelope_json: &str) -> Result<String, EnvelopeError> {
    let envelope = parse_envelope_v3(envelope_json)?;
    let report = validate(&envelope);
    Ok(serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()))
}

/// JSON-friendly wrapper that returns just the dependency graph.
pub fn dependency_graph_json(envelope_json: &str) -> Result<String, EnvelopeError> {
    let envelope = parse_envelope_v3(envelope_json)?;
    let nodes = build_nodes(&envelope.plan.steps);
    let mut sorted_initial: Vec<String> = collect_initial_ids(&envelope).into_iter().collect();
    sorted_initial.sort();
    let graph = DependencyGraph {
        initial_ids: sorted_initial,
        nodes,
    };
    Ok(serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string()))
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Parse a JSON envelope, returning a friendly error for legacy v2 envelopes.
///
/// The v3 envelope (see [`CalibrationEnvelope`]) replaced the v2 `initial_market`
/// section with flat `market_data` / `prior_market` lists. Envelopes that still
/// carry the v2 `initial_market` key get a targeted message pointing at the
/// migration design doc rather than the generic serde `unknown field` error.
pub fn parse_envelope_v3(json: &str) -> Result<CalibrationEnvelope, EnvelopeError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| EnvelopeError::JsonParse {
            message: e.to_string(),
            line: Some(e.line() as u32),
            col: Some(e.column() as u32),
        })?;
    if value.get("initial_market").is_some() {
        return Err(EnvelopeError::JsonParse {
            message: "envelope schema v2 is no longer supported; see \
                      docs/2026-05-10-calibration-envelope-cleanup-design.md \
                      for the v3 shape"
                .to_string(),
            line: None,
            col: None,
        });
    }
    serde_json::from_value(value).map_err(|e| EnvelopeError::JsonParse {
        message: e.to_string(),
        line: None,
        col: None,
    })
}

fn collect_initial_ids(envelope: &CalibrationEnvelope) -> HashSet<String> {
    let mut ids = HashSet::new();
    for obj in &envelope.prior_market {
        ids.insert(obj.id().to_string());
    }
    ids
}

fn build_nodes(steps: &[CalibrationStep]) -> Vec<DependencyNode> {
    steps
        .iter()
        .enumerate()
        .map(|(idx, step)| {
            let (kind, reads, writes) = step_io(&step.params);
            DependencyNode {
                step_index: idx,
                step_id: step.id.clone(),
                kind,
                reads,
                writes,
            }
        })
        .collect()
}

/// `(kind, reads, writes)` triple for each step variant.
///
/// Mirrors `step_runtime::output_key` for the write side; reads are sourced
/// from the step's parameter struct (typically `*_curve_id` fields).
fn step_io(params: &StepParams) -> (String, Vec<String>, Vec<String>) {
    match params {
        StepParams::Discount(p) => (
            "discount".to_string(),
            Vec::new(),
            vec![p.curve_id.to_string()],
        ),
        StepParams::Forward(p) => (
            "forward".to_string(),
            vec![p.discount_curve_id.to_string()],
            vec![p.curve_id.to_string()],
        ),
        StepParams::Hazard(p) => (
            "hazard".to_string(),
            vec![p.discount_curve_id.to_string()],
            vec![p.curve_id.to_string()],
        ),
        StepParams::Inflation(p) => (
            "inflation".to_string(),
            vec![p.discount_curve_id.to_string()],
            vec![p.curve_id.to_string()],
        ),
        StepParams::VolSurface(p) => {
            let reads = p
                .discount_curve_id
                .as_ref()
                .map(|c| vec![c.to_string()])
                .unwrap_or_default();
            ("vol_surface".to_string(), reads, vec![p.surface_id.clone()])
        }
        StepParams::SwaptionVol(p) => (
            "swaption_vol".to_string(),
            vec![p.discount_curve_id.to_string()],
            vec![p.surface_id.clone()],
        ),
        StepParams::BaseCorrelation(p) => (
            "base_correlation".to_string(),
            vec![p.discount_curve_id.to_string()],
            // Mirror step_runtime::output_key: writes "{index_id}_CORR".
            vec![format!("{}_CORR", p.index_id)],
        ),
        StepParams::StudentT(p) => {
            let mut reads = vec![p.base_correlation_curve_id.clone()];
            if let Some(d) = &p.discount_curve_id {
                reads.push(d.to_string());
            }
            (
                "student_t".to_string(),
                reads,
                vec![format!("{}_STUDENT_T_DF", p.tranche_instrument_id)],
            )
        }
        StepParams::HullWhite(p) => (
            "hull_white".to_string(),
            vec![p.curve_id.to_string()],
            vec![format!("{}_HW1F", p.curve_id.as_str())],
        ),
        StepParams::CapFloorHullWhite(p) => {
            let mut reads = vec![p.discount_curve_id.to_string()];
            if p.forward_curve_id != p.discount_curve_id {
                reads.push(p.forward_curve_id.to_string());
            }
            (
                "cap_floor_hull_white".to_string(),
                reads,
                vec![format!("{}_CAPFLOOR_HW1F", p.discount_curve_id.as_str())],
            )
        }
        StepParams::SviSurface(p) => {
            let reads = p
                .discount_curve_id
                .as_ref()
                .map(|c| vec![c.to_string()])
                .unwrap_or_default();
            ("svi_surface".to_string(), reads, vec![p.surface_id.clone()])
        }
        StepParams::XccyBasis(p) => {
            let mut writes = vec![p.curve_id.to_string()];
            if let Some(b) = &p.basis_spread_curve_id {
                writes.push(b.to_string());
            }
            (
                "xccy_basis".to_string(),
                vec![p.domestic_discount_id.to_string()],
                writes,
            )
        }
        StepParams::Parametric(p) => {
            let reads = p
                .discount_curve_id
                .as_ref()
                .map(|c| vec![c.to_string()])
                .unwrap_or_default();
            (
                "parametric".to_string(),
                reads,
                vec![p.curve_id.to_string()],
            )
        }
    }
}

fn check_quote_sets(envelope: &CalibrationEnvelope, errors: &mut Vec<EnvelopeError>) {
    let mut available: Vec<String> = envelope.plan.quote_sets.keys().cloned().collect();
    available.sort();
    for (idx, step) in envelope.plan.steps.iter().enumerate() {
        if !envelope.plan.quote_sets.contains_key(&step.quote_set) {
            errors.push(EnvelopeError::UndefinedQuoteSet {
                step_index: idx,
                step_id: step.id.clone(),
                ref_name: step.quote_set.clone(),
                available: available.clone(),
                suggestion: closest_match(&step.quote_set, &available),
            });
        }
    }
}

fn check_market_data_uniqueness(envelope: &CalibrationEnvelope, errors: &mut Vec<EnvelopeError>) {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, BTreeSet<String>> = HashMap::new();
    for datum in &envelope.market_data {
        let key = if datum.is_quote() {
            "quote"
        } else {
            datum.kind_name()
        };
        if !seen.entry(key).or_default().insert(datum.id().to_string()) {
            errors.push(EnvelopeError::DuplicateMarketDatumId {
                datum_kind: key.to_string(),
                id: datum.id().to_string(),
            });
        }
    }
}

fn check_quote_sets_resolve(envelope: &CalibrationEnvelope, errors: &mut Vec<EnvelopeError>) {
    let quote_ids: HashSet<&str> = envelope
        .market_data
        .iter()
        .filter(|d| d.is_quote())
        .map(|d| d.id())
        .collect();
    for (set_name, ids) in &envelope.plan.quote_sets {
        for qid in ids {
            if !quote_ids.contains(qid.as_str()) {
                errors.push(EnvelopeError::QuoteIdNotInMarketData {
                    quote_set: set_name.clone(),
                    id: qid.to_string(),
                });
            }
        }
    }
}

fn check_dependencies(
    initial_ids: &HashSet<String>,
    nodes: &[DependencyNode],
    errors: &mut Vec<EnvelopeError>,
) {
    let mut available: BTreeSet<String> = initial_ids.iter().cloned().collect();
    for node in nodes {
        for read_id in &node.reads {
            if !available.contains(read_id) {
                errors.push(EnvelopeError::MissingDependency {
                    step_index: node.step_index,
                    step_id: node.step_id.clone(),
                    step_kind: node.kind.clone(),
                    missing_id: read_id.clone(),
                    missing_kind: "curve".to_string(),
                    available: available.iter().cloned().collect(),
                });
            }
        }
        for write_id in &node.writes {
            available.insert(write_id.clone());
        }
    }
}

/// Closest-match suggestion for a misspelled identifier (Levenshtein ≤ 3).
fn closest_match(target: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .map(|c| (c, levenshtein(target, c)))
        .filter(|(_, d)| *d > 0 && *d <= 3)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c.clone())
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::api::schema::{
        CalibrationPlan, CalibrationStep, DiscountCurveParams, StepParams, CALIBRATION_SCHEMA,
    };
    use finstack_core::HashMap;

    fn empty_envelope(id: &str) -> CalibrationEnvelope {
        CalibrationEnvelope {
            schema_url: None,
            schema: CALIBRATION_SCHEMA.to_string(),
            plan: CalibrationPlan {
                id: id.to_string(),
                description: None,
                quote_sets: HashMap::default(),
                steps: Vec::new(),
                settings: Default::default(),
            },
            market_data: Vec::new(),
            prior_market: Vec::new(),
        }
    }

    fn discount_step(id: &str, quote_set: &str, curve_id: &str) -> CalibrationStep {
        // Use serde to construct DiscountCurveParams since several fields rely
        // on serde defaults (interpolation, conventions, etc.).
        let params: DiscountCurveParams = serde_json::from_value(serde_json::json!({
            "curve_id": curve_id,
            "currency": "USD",
            "base_date": "2026-05-08",
        }))
        .expect("default discount params deserialize");
        CalibrationStep {
            id: id.to_string(),
            quote_set: quote_set.to_string(),
            params: StepParams::Discount(params),
        }
    }

    #[test]
    fn empty_envelope_validates_clean() {
        let env = empty_envelope("smoke");
        let report = validate(&env);
        assert!(report.errors.is_empty());
        assert!(report.dependency_graph.nodes.is_empty());
    }

    #[test]
    fn undefined_quote_set_is_caught_with_suggestion() {
        let mut env = empty_envelope("test");
        env.plan
            .quote_sets
            .insert("usd_quotes".to_string(), Vec::new());
        env.plan
            .steps
            .push(discount_step("d", "usd_quotess", "USD-OIS"));

        let report = validate(&env);
        let err = report
            .errors
            .iter()
            .find(|e| matches!(e, EnvelopeError::UndefinedQuoteSet { .. }))
            .expect("undefined quote_set error");
        match err {
            EnvelopeError::UndefinedQuoteSet {
                ref_name,
                suggestion,
                ..
            } => {
                assert_eq!(ref_name, "usd_quotess");
                assert_eq!(suggestion.as_deref(), Some("usd_quotes"));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn dry_run_returns_pretty_json() {
        let env = empty_envelope("smoke");
        let json = serde_json::to_string(&env).expect("serialize");
        let report_json = dry_run(&json).expect("dry_run");
        assert!(report_json.contains("\"errors\""));
        assert!(report_json.contains("\"dependency_graph\""));
    }

    #[test]
    fn dependency_graph_json_is_well_formed() {
        let env = empty_envelope("smoke");
        let json = serde_json::to_string(&env).expect("serialize");
        let graph_json = dependency_graph_json(&json).expect("dep graph");
        assert!(graph_json.contains("\"initial_ids\""));
        assert!(graph_json.contains("\"nodes\""));
    }

    #[test]
    fn missing_dependency_for_forward_step_without_discount() {
        // Build a forward step that reads "USD-OIS" without a producing step
        // and without initial_market — should surface MissingDependency.
        let mut env = empty_envelope("missing");
        env.plan
            .quote_sets
            .insert("fwd_quotes".to_string(), Vec::new());
        let params: crate::calibration::api::schema::ForwardCurveParams =
            serde_json::from_value(serde_json::json!({
                "curve_id": "USD-3M-LIBOR",
                "currency": "USD",
                "base_date": "2026-05-08",
                "tenor_years": 0.25,
                "discount_curve_id": "USD-OIS",
            }))
            .expect("forward params");
        env.plan.steps.push(CalibrationStep {
            id: "fwd".to_string(),
            quote_set: "fwd_quotes".to_string(),
            params: StepParams::Forward(params),
        });

        let report = validate(&env);
        let missing = report
            .errors
            .iter()
            .find(|e| matches!(e, EnvelopeError::MissingDependency { .. }))
            .expect("missing dependency error");
        if let EnvelopeError::MissingDependency { missing_id, .. } = missing {
            assert_eq!(missing_id, "USD-OIS");
        }
    }

    #[test]
    fn json_parse_error_surfaces_line_and_col() {
        let err = dry_run("not json").unwrap_err();
        match err {
            EnvelopeError::JsonParse { line, col, .. } => {
                assert!(line.is_some());
                assert!(col.is_some());
            }
            _ => panic!("expected JsonParse"),
        }
    }

    #[test]
    fn v2_envelope_yields_friendly_error() {
        let v2 = r#"{
            "schema":"finstack.calibration",
            "plan":{"id":"x","description":null,"quote_sets":{},"steps":[],"settings":{}},
            "initial_market":null
        }"#;
        let err = parse_envelope_v3(v2).unwrap_err();
        match err {
            EnvelopeError::JsonParse { message, .. } => {
                assert!(message.contains("v3 shape"), "message was: {message}")
            }
            other => panic!("expected JsonParse error, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod v3_validation_tests {
    use super::*;
    use crate::calibration::api::market_datum::{MarketDatum, PriceDatum};
    use crate::calibration::api::schema::{CalibrationPlan, CALIBRATION_SCHEMA};
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::HashMap;

    #[test]
    fn duplicate_price_id_is_flagged() {
        let envelope = CalibrationEnvelope {
            schema_url: None,
            schema: CALIBRATION_SCHEMA.to_string(),
            plan: CalibrationPlan {
                id: "t".into(),
                description: None,
                quote_sets: HashMap::default(),
                steps: vec![],
                settings: Default::default(),
            },
            market_data: vec![
                MarketDatum::Price(PriceDatum {
                    id: "AAPL".into(),
                    scalar: MarketScalar::Unitless(100.0),
                }),
                MarketDatum::Price(PriceDatum {
                    id: "AAPL".into(),
                    scalar: MarketScalar::Unitless(101.0),
                }),
            ],
            prior_market: vec![],
        };
        let report = validate(&envelope);
        assert!(
            report.errors.iter().any(|e| matches!(
                e,
                EnvelopeError::DuplicateMarketDatumId { datum_kind, id }
                    if datum_kind == "price" && id == "AAPL"
            )),
            "expected DuplicateMarketDatumId, got: {:?}",
            report.errors
        );
    }

    #[test]
    fn unresolved_quote_id_is_flagged() {
        use crate::market::quotes::ids::QuoteId;
        let mut quote_sets = HashMap::default();
        quote_sets.insert("usd".to_string(), vec![QuoteId::new("MISSING")]);
        let envelope = CalibrationEnvelope {
            schema_url: None,
            schema: CALIBRATION_SCHEMA.to_string(),
            plan: CalibrationPlan {
                id: "t".into(),
                description: None,
                quote_sets,
                steps: vec![],
                settings: Default::default(),
            },
            market_data: vec![],
            prior_market: vec![],
        };
        let report = validate(&envelope);
        assert!(
            report.errors.iter().any(|e| matches!(
                e,
                EnvelopeError::QuoteIdNotInMarketData { quote_set, id }
                    if quote_set == "usd" && id == "MISSING"
            )),
            "expected QuoteIdNotInMarketData, got: {:?}",
            report.errors
        );
    }
}
