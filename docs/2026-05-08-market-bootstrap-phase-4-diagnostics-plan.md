# Market Bootstrap Phase 4 — Diagnostics Implementation Plan

> **Superseded** in v3 envelope shape: see [2026-05-10-calibration-envelope-cleanup-design.md](2026-05-10-calibration-envelope-cleanup-design.md). References to `initial_market` in this document predate the v3 cleanup.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship structured diagnostics for `CalibrationEnvelope` failures: `EnvelopeError` enum surfaces missing dependencies / undefined quote sets / quote-class mismatches / solver non-convergence; `dry_run` validates an envelope without solving; `dependency_graph_json` dumps the step DAG; Python and WASM bindings raise typed exceptions with structured payloads.

**Architecture:** New `errors` and `validate` modules under `finstack/valuations/src/calibration/api/`. `EnvelopeError` is a `serde::Serialize` enum with one variant per failure mode. Static `validate(envelope) -> ValidationReport` builds a dependency graph, runs all checks, returns *all* errors (not just first). `dry_run(envelope_json) -> String` and `dependency_graph_json(envelope_json) -> String` wrap them for cross-binding use. Solver targets enrich `CalibrationReport` with `worst_quote_id`. Python `CalibrationEnvelopeError(RuntimeError)` carries `kind`, `step_id`, `details` attributes. WASM throws JS `Error` with `name = 'CalibrationEnvelopeError'` and `cause` set to the structured details.

**Tech Stack:** Rust (`finstack-valuations`), PyO3, wasm-bindgen, serde-json, no new external dependencies.

**Spec reference:** [`docs/2026-05-08-market-bootstrap-phase-4-diagnostics-design.md`](2026-05-08-market-bootstrap-phase-4-diagnostics-design.md)

**Phase 1+2+3 baseline (assumed present):**
- `CalibrationEnvelope` with `schema_url: Option<String>` field for `$schema` editor support.
- Twelve self-bootstrapping reference envelopes under `examples/market_bootstrap/`.
- TS bindings via `ts_export` for envelope structure (Phase 3).
- `gen-check` mise task wired into `all-lint`.
- `validate_calibration_json(&str) -> Result<String>` in Rust (canonicalization-only); Python and WASM mirrors return `ValueError`/`JsValue` on bad input.

**Commit policy:** No commits without explicit user approval. Each task ends with a `git commit` step shown for completeness.

**Scope risks called out by the design (§8):**
- Solver enrichment effort is uncertain. **Task 5 begins with an audit step** to re-estimate before deep changes.
- `dry_run` performance is "microseconds" target — verify with a benchmark.
- Python `CalibrationEnvelopeError(RuntimeError)` is backwards-compatible; existing `except RuntimeError` handlers continue to work.

---

## File Structure

### Files to create

| Path | Responsibility |
|---|---|
| `finstack/valuations/src/calibration/api/errors.rs` | `EnvelopeError` enum (8 variants), `Display`, `serde::Serialize`, `From<EnvelopeError> for finstack_core::Error`. |
| `finstack/valuations/src/calibration/api/validate.rs` | `ValidationReport`, `DependencyGraph`, `validate(envelope: &CalibrationEnvelope) -> ValidationReport`, `dry_run(envelope_json: &str) -> Result<String>`, `dependency_graph_json(envelope_json: &str) -> Result<String>`. |
| `finstack/valuations/tests/calibration/diagnostics.rs` | Integration tests for each `EnvelopeError` variant. |

### Files to modify

| Path | Change |
|---|---|
| `finstack/valuations/src/calibration/api/mod.rs` | Re-export `errors`, `validate`, `dry_run`, `dependency_graph_json`. |
| `finstack/valuations/src/calibration/report.rs` | Add `worst_quote_id: Option<String>`, `worst_quote_residual: Option<f64>` fields to `CalibrationReport`. |
| `finstack/valuations/src/calibration/targets/discount.rs` (and analogous: `hazard.rs`, `forward.rs`, `vol_surface.rs`, etc.) | Set `worst_quote_id` / `worst_quote_residual` when a step fails to converge. |
| `finstack-py/src/bindings/valuations/calibration.rs` | Register `CalibrationEnvelopeError(RuntimeError)`; add `dry_run` and `dependency_graph_json` `#[pyfunction]`s. Convert `EnvelopeError` to `CalibrationEnvelopeError` in calibrate / validate paths. |
| `finstack-py/finstack/valuations/__init__.pyi` | Add `CalibrationEnvelopeError`, `dry_run`, `dependency_graph_json` to `__all__` with type stubs. |
| `finstack-wasm/src/api/valuations/calibration.rs` | Add `dryRun` and `dependencyGraphJson` `#[wasm_bindgen]` exports. Wrap thrown errors in JS `Error` with `name = 'CalibrationEnvelopeError'` and `cause` set to structured JSON. |
| `finstack-wasm/exports/valuations.js` | Re-export `dryRun`, `dependencyGraphJson` through the JS facade. |
| `finstack-wasm/index.d.ts` | Add typed signatures for `dryRun(envelope: CalibrationEnvelope \| string): string` and `dependencyGraphJson(envelope: CalibrationEnvelope \| string): string`. |
| `finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb` | Append a "When envelopes fail" section (3-4 cells). |

---

## Task 1: `EnvelopeError` enum

**Goal:** Structured error type covering 8 failure modes, with `Display` (human-readable), `serde::Serialize` (machine-readable, JSON), and `From<EnvelopeError> for finstack_core::Error` (for backwards-compat propagation through existing call sites).

**Files:**
- Create: `finstack/valuations/src/calibration/api/errors.rs`
- Modify: `finstack/valuations/src/calibration/api/mod.rs`

- [ ] **Step 1: Create the errors module skeleton**

Create `finstack/valuations/src/calibration/api/errors.rs`:

```rust
//! Structured error types for calibration envelope diagnostics.
//!
//! `EnvelopeError` is the canonical error type for static envelope validation
//! and runtime calibration failures. It implements `Display` (human-readable),
//! `serde::Serialize` (machine-readable JSON for Python/WASM bindings), and
//! `Into<finstack_core::Error>` for backwards-compatible propagation through
//! existing call sites that take `finstack_core::Result`.

use serde::Serialize;
use std::fmt;

/// Errors surfaced when an envelope is invalid or calibration fails.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnvelopeError {
    /// JSON parse failure (malformed envelope).
    JsonParse {
        message: String,
        line: Option<u32>,
        col: Option<u32>,
    },
    /// A step's `kind` discriminator is not a recognized variant.
    UnknownStepKind {
        step_index: usize,
        step_id: String,
        found: String,
        expected_one_of: Vec<String>,
    },
    /// A step references a curve / surface ID that's not produced by an
    /// earlier step or carried in `initial_market`.
    MissingDependency {
        step_index: usize,
        step_id: String,
        kind: String,
        missing_id: String,
        missing_kind: String,
        available: Vec<String>,
    },
    /// A step's `quote_set` field references a name not in `plan.quote_sets`.
    UndefinedQuoteSet {
        step_index: usize,
        step_id: String,
        ref_name: String,
        available: Vec<String>,
        suggestion: Option<String>,
    },
    /// A step's quote_set contains quotes of a class incompatible with the step.
    QuoteClassMismatch {
        step_index: usize,
        step_id: String,
        kind: String,
        expected_class: String,
        breakdown: Vec<(String, usize)>,
    },
    /// A solver step did not converge to within tolerance.
    SolverNotConverged {
        step_id: String,
        max_residual: f64,
        tolerance: f64,
        iterations: u32,
        worst_quote_id: Option<String>,
        worst_quote_residual: Option<f64>,
    },
    /// Quote data fails domain validation (NaN, out-of-range, etc.).
    QuoteDataInvalid {
        step_id: String,
        quote_id: String,
        reason: String,
    },
    /// Cyclic dependency between steps.
    StepCycle {
        involved_step_ids: Vec<String>,
    },
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeError::JsonParse { message, line, col } => {
                let loc = match (line, col) {
                    (Some(l), Some(c)) => format!(" at line {l}, column {c}"),
                    (Some(l), None) => format!(" at line {l}"),
                    _ => String::new(),
                };
                write!(f, "JSON parse error{loc}: {message}")
            }
            EnvelopeError::UnknownStepKind {
                step_index,
                step_id,
                found,
                expected_one_of,
            } => write!(
                f,
                "step[{step_index}] '{step_id}': unknown kind '{found}'; expected one of: {}",
                expected_one_of.join(", ")
            ),
            EnvelopeError::MissingDependency {
                step_index,
                step_id,
                kind,
                missing_id,
                missing_kind,
                available,
            } => {
                let avail = if available.is_empty() {
                    "none".to_string()
                } else {
                    available.join(", ")
                };
                write!(
                    f,
                    "step[{step_index}] '{step_id}' (kind='{kind}'): missing {missing_kind} dependency '{missing_id}'. Available: [{avail}]"
                )
            }
            EnvelopeError::UndefinedQuoteSet {
                step_index,
                step_id,
                ref_name,
                available,
                suggestion,
            } => {
                let hint = match suggestion {
                    Some(s) => format!(" Did you mean '{s}'?"),
                    None => String::new(),
                };
                write!(
                    f,
                    "step[{step_index}] '{step_id}': quote_set '{ref_name}' is not defined in plan.quote_sets. Available: [{}].{hint}",
                    available.join(", ")
                )
            }
            EnvelopeError::QuoteClassMismatch {
                step_index,
                step_id,
                kind,
                expected_class,
                breakdown,
            } => {
                let counts: Vec<String> = breakdown
                    .iter()
                    .map(|(c, n)| format!("{n} '{c}'"))
                    .collect();
                write!(
                    f,
                    "step[{step_index}] '{step_id}' (kind='{kind}'): expected quotes of class '{expected_class}', but found: {}",
                    counts.join(", ")
                )
            }
            EnvelopeError::SolverNotConverged {
                step_id,
                max_residual,
                tolerance,
                iterations,
                worst_quote_id,
                worst_quote_residual,
            } => {
                let worst = match (worst_quote_id, worst_quote_residual) {
                    (Some(id), Some(r)) => format!(" Worst quote: '{id}' (residual {r:.3e})."),
                    _ => String::new(),
                };
                write!(
                    f,
                    "step '{step_id}' did not converge: max residual {max_residual:.3e} > tolerance {tolerance:.3e} after {iterations} iterations.{worst}"
                )
            }
            EnvelopeError::QuoteDataInvalid {
                step_id,
                quote_id,
                reason,
            } => write!(
                f,
                "step '{step_id}': quote '{quote_id}' is invalid: {reason}"
            ),
            EnvelopeError::StepCycle { involved_step_ids } => {
                write!(
                    f,
                    "step dependency cycle detected: {}",
                    involved_step_ids.join(" → ")
                )
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}

impl EnvelopeError {
    /// Serialize to pretty-printed JSON for cross-binding consumption.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

impl From<EnvelopeError> for finstack_core::Error {
    fn from(err: EnvelopeError) -> Self {
        // Wraps EnvelopeError as a finstack_core::Error so existing callsites
        // returning finstack_core::Result continue to work without changes.
        finstack_core::Error::msg(err.to_string())
    }
}
```

If `finstack_core::Error::msg(...)` doesn't exist, use whatever the project's error-from-string convention is (e.g. `finstack_core::Error::other(err.to_string())`, `finstack_core::Error::Calibration(err.to_string())`, etc.). Inspect `finstack/core/src/lib.rs` or wherever the `Error` type is defined.

- [ ] **Step 2: Wire the module into `api/mod.rs`**

In `finstack/valuations/src/calibration/api/mod.rs`, add:

```rust
pub mod errors;
```

(Place alongside existing module declarations like `pub mod schema;` and `pub mod engine;`.)

- [ ] **Step 3: Write tests for each variant**

Create `finstack/valuations/tests/calibration/diagnostics.rs`:

```rust
//! Tests for `EnvelopeError` and the static envelope validator.

use finstack_valuations::calibration::api::errors::EnvelopeError;

#[test]
fn envelope_error_display_includes_step_id() {
    let err = EnvelopeError::MissingDependency {
        step_index: 2,
        step_id: "cdx_hazard".to_string(),
        kind: "hazard".to_string(),
        missing_id: "USD-OIS".to_string(),
        missing_kind: "discount".to_string(),
        available: vec!["EUR-OIS".to_string()],
    };
    let s = format!("{err}");
    assert!(s.contains("step[2]"));
    assert!(s.contains("cdx_hazard"));
    assert!(s.contains("USD-OIS"));
    assert!(s.contains("EUR-OIS"));
}

#[test]
fn envelope_error_serializes_with_kind_tag() {
    let err = EnvelopeError::UndefinedQuoteSet {
        step_index: 1,
        step_id: "test_step".to_string(),
        ref_name: "missing_set".to_string(),
        available: vec!["set_a".to_string(), "set_b".to_string()],
        suggestion: Some("set_a".to_string()),
    };
    let json = err.to_json();
    assert!(json.contains("\"kind\": \"undefined_quote_set\""));
    assert!(json.contains("\"ref_name\": \"missing_set\""));
    assert!(json.contains("\"suggestion\": \"set_a\""));
}

#[test]
fn solver_not_converged_includes_worst_quote() {
    let err = EnvelopeError::SolverNotConverged {
        step_id: "discount_step".to_string(),
        max_residual: 1.27e-3,
        tolerance: 1.0e-6,
        iterations: 50,
        worst_quote_id: Some("USD-IRS-30Y".to_string()),
        worst_quote_residual: Some(1.27e-3),
    };
    let s = format!("{err}");
    assert!(s.contains("USD-IRS-30Y"));
    assert!(s.contains("did not converge"));
}
```

In `finstack/valuations/tests/calibration/mod.rs`, add the new module:

```rust
mod diagnostics;
```

(Keep alphabetical with existing entries.)

- [ ] **Step 4: Run the tests**

Run: `cargo test -p finstack-valuations --test calibration diagnostics`
Expected: all 3 tests pass.

If any fail, adjust the `Display` formatting in errors.rs to match the assertion text.

- [ ] **Step 5: Run cargo fmt + check**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo check -p finstack-valuations`
Both clean.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/calibration/api/errors.rs \
        finstack/valuations/src/calibration/api/mod.rs \
        finstack/valuations/tests/calibration/diagnostics.rs \
        finstack/valuations/tests/calibration/mod.rs
git commit -m "feat(valuations): add EnvelopeError enum for structured diagnostics

Eight variants covering JSON parse failure, unknown step kind, missing
dependency, undefined quote_set, quote class mismatch, solver
non-convergence (with worst-quote enrichment), invalid quote data, and
step dependency cycles. Implements Display (human-readable), Serialize
(JSON for cross-binding consumption), and From<EnvelopeError> for
finstack_core::Error so existing callsites continue to work without
changes. Foundation for the static envelope validator (Task 2)."
```

---

## Task 2: Static envelope validator

**Goal:** A pre-flight validator that catches all structural issues in one pass without invoking the solver. Returns *all* errors found, not just first.

**Files:**
- Create: `finstack/valuations/src/calibration/api/validate.rs`
- Modify: `finstack/valuations/src/calibration/api/mod.rs`
- Modify: `finstack/valuations/tests/calibration/diagnostics.rs` (add validator tests)

- [ ] **Step 1: Create the validator skeleton**

Create `finstack/valuations/src/calibration/api/validate.rs`:

```rust
//! Static envelope validator and dependency-graph utilities.
//!
//! `validate(&CalibrationEnvelope) -> ValidationReport` runs all structural
//! checks (missing dependencies, undefined quote_sets, quote-class
//! mismatches, step cycles) and returns a report listing every error found
//! plus the dependency graph of the steps. No solver is invoked; the
//! validator runs in microseconds.
//!
//! `dry_run(json)` and `dependency_graph_json(json)` are JSON-string-friendly
//! wrappers for cross-binding consumption.

use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

use crate::calibration::api::errors::EnvelopeError;
use crate::calibration::api::schema::{CalibrationEnvelope, CalibrationStep, StepParams};

/// Result of `validate`. Always contains the dependency graph; `errors` is
/// empty when the envelope is structurally valid.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    /// All errors found in a single pass; empty if the envelope is valid.
    pub errors: Vec<EnvelopeError>,
    /// Topological view of the steps' inputs and outputs.
    pub dependency_graph: DependencyGraph,
}

/// Static dependency graph derived from a `CalibrationEnvelope`.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    /// Curve / surface IDs available at the start of execution (from
    /// `initial_market`).
    pub initial_ids: Vec<String>,
    /// Per-step inputs and outputs in declared order. Each node identifies
    /// the step by `step_id` and lists the curve IDs it reads and writes.
    pub nodes: Vec<DependencyNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyNode {
    pub step_index: usize,
    pub step_id: String,
    pub kind: String,
    /// Curve / surface IDs the step depends on. Each must be either in
    /// `initial_ids` or produced by an earlier step.
    pub reads: Vec<String>,
    /// Curve / surface ID(s) the step produces.
    pub writes: Vec<String>,
}

/// Run all static validation checks. Always returns a `ValidationReport`;
/// inspect `errors` to see what failed.
pub fn validate(envelope: &CalibrationEnvelope) -> ValidationReport {
    let mut errors = Vec::new();

    let initial_ids = collect_initial_ids(envelope);
    let nodes = build_nodes(envelope);
    let graph = DependencyGraph {
        initial_ids: initial_ids.iter().cloned().collect(),
        nodes: nodes.clone(),
    };

    // 1. Quote-set references resolved.
    check_quote_sets(envelope, &mut errors);

    // 2. Quote class compatibility.
    check_quote_classes(envelope, &mut errors);

    // 3. Dependency reachability.
    check_dependencies(envelope, &initial_ids, &nodes, &mut errors);

    // 4. Cycle detection.
    check_cycles(&nodes, &mut errors);

    ValidationReport {
        errors,
        dependency_graph: graph,
    }
}

/// Wrap `validate` to take a JSON string. Returns the report as JSON.
pub fn dry_run(envelope_json: &str) -> Result<String, EnvelopeError> {
    let envelope: CalibrationEnvelope =
        serde_json::from_str(envelope_json).map_err(|e| EnvelopeError::JsonParse {
            message: e.to_string(),
            line: Some(e.line() as u32),
            col: Some(e.column() as u32),
        })?;
    let report = validate(&envelope);
    Ok(serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string()))
}

/// JSON-friendly wrapper that returns just the dependency graph.
pub fn dependency_graph_json(envelope_json: &str) -> Result<String, EnvelopeError> {
    let envelope: CalibrationEnvelope =
        serde_json::from_str(envelope_json).map_err(|e| EnvelopeError::JsonParse {
            message: e.to_string(),
            line: Some(e.line() as u32),
            col: Some(e.column() as u32),
        })?;
    let nodes = build_nodes(&envelope);
    let graph = DependencyGraph {
        initial_ids: collect_initial_ids(&envelope).into_iter().collect(),
        nodes,
    };
    Ok(serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string()))
}

// --- internal helpers ---

fn collect_initial_ids(envelope: &CalibrationEnvelope) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(state) = &envelope.initial_market {
        // The exact accessor depends on `MarketContextState`'s shape — this
        // pulls IDs from every populated container. Verify field paths in
        // finstack/core/src/market_data/context/state_serde.rs.
        for curve_state in state_curve_ids(state) {
            ids.insert(curve_state);
        }
    }
    ids
}

fn state_curve_ids(state: &finstack_core::market_data::context::MarketContextState) -> Vec<String> {
    // `MarketContextState.curves` is `Vec<CurveState>`; each variant carries
    // an `id: CurveId` field. Adjust the access pattern to match the actual
    // serde shape.
    state
        .curves
        .iter()
        .map(|c| curve_state_id(c))
        .collect()
}

fn curve_state_id(curve: &finstack_core::market_data::context::state_serde::CurveState) -> String {
    // Inspect `CurveState`'s variants — likely an enum with per-variant `id`.
    // This may need adjustment based on the actual type; `curve_state.id().to_string()`
    // is the most likely shape.
    use finstack_core::market_data::context::state_serde::CurveState;
    match curve {
        CurveState::Discount(c) => c.id.to_string(),
        CurveState::Forward(c) => c.id.to_string(),
        CurveState::Hazard(c) => c.id.to_string(),
        // ... add other variants as discovered. If the variant list is large,
        // a helper trait method `fn id(&self) -> &CurveId` on `CurveState`
        // is cleaner — add it to state_serde.rs if missing.
        _ => String::new(), // placeholder — replace with comprehensive match
    }
}

fn build_nodes(envelope: &CalibrationEnvelope) -> Vec<DependencyNode> {
    envelope
        .plan
        .steps
        .iter()
        .enumerate()
        .map(|(idx, step)| build_node(idx, step))
        .collect()
}

fn build_node(step_index: usize, step: &CalibrationStep) -> DependencyNode {
    let (kind, reads, writes) = step_io(&step.params);
    DependencyNode {
        step_index,
        step_id: step.id.clone(),
        kind,
        reads,
        writes,
    }
}

fn step_io(params: &StepParams) -> (String, Vec<String>, Vec<String>) {
    // Each variant declares: kind name, IDs it reads, ID it writes.
    match params {
        StepParams::Discount(p) => ("discount".to_string(), vec![], vec![p.curve_id.to_string()]),
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
        StepParams::BaseCorrelation(p) => (
            "base_correlation".to_string(),
            vec![
                p.discount_curve_id.to_string(),
                p.index_id.to_string(),
            ],
            vec![p.curve_id.to_string()],
        ),
        StepParams::VolSurface(p) => (
            "vol_surface".to_string(),
            vec![p.discount_curve_id.to_string()],
            vec![p.surface_id.to_string()],
        ),
        StepParams::SwaptionVol(p) => (
            "swaption_vol".to_string(),
            vec![
                p.discount_curve_id.to_string(),
                p.forward_curve_id.to_string(),
            ],
            vec![p.surface_id.to_string()],
        ),
        // ... and so on for each StepParams variant.
        // For variants whose param struct has a different shape, inspect
        // schema.rs and add the appropriate `(kind, reads, writes)` triple.
        _ => ("unknown".to_string(), vec![], vec![]),
    }
}

fn check_quote_sets(envelope: &CalibrationEnvelope, errors: &mut Vec<EnvelopeError>) {
    let available: Vec<String> = envelope.plan.quote_sets.keys().cloned().collect();
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

fn check_quote_classes(_envelope: &CalibrationEnvelope, _errors: &mut Vec<EnvelopeError>) {
    // For each step kind, check that its quote_set contains compatible
    // quote classes. The mapping (e.g. "discount" expects "rates"; "hazard"
    // expects "cds"; "base_correlation" expects "cds_tranche") is encoded
    // via a small lookup table.
    //
    // Implementation note: for Phase 4, a permissive implementation that
    // only warns on obvious mismatches (e.g. "discount" step with zero
    // "rates" quotes) is acceptable. Tighten in a follow-up.
}

fn check_dependencies(
    envelope: &CalibrationEnvelope,
    initial_ids: &HashSet<String>,
    nodes: &[DependencyNode],
    errors: &mut Vec<EnvelopeError>,
) {
    let mut available: HashSet<String> = initial_ids.clone();
    for (idx, node) in nodes.iter().enumerate() {
        for read_id in &node.reads {
            if !available.contains(read_id) {
                errors.push(EnvelopeError::MissingDependency {
                    step_index: idx,
                    step_id: node.step_id.clone(),
                    kind: node.kind.clone(),
                    missing_id: read_id.clone(),
                    missing_kind: "unknown".to_string(),
                    available: available.iter().cloned().collect(),
                });
            }
        }
        for write_id in &node.writes {
            available.insert(write_id.clone());
        }
    }
    let _ = envelope; // silence unused if not consumed
}

fn check_cycles(nodes: &[DependencyNode], errors: &mut Vec<EnvelopeError>) {
    // Steps are declared in order; engine processes sequentially. A step
    // can only read curves produced by *earlier* steps or from initial_market.
    // A "cycle" in the static graph is a step that reads its own write or
    // a later step's write — in declaration order this is just "missing
    // dependency at the time of execution," already caught by check_dependencies.
    // Genuine cycles (A reads B, B reads A) cannot occur in a sequential
    // declaration; flag if we ever support reordering.
    let _ = nodes;
    let _ = errors;
}

/// Closest-match suggestion for a misspelled identifier (Levenshtein).
fn closest_match(target: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .map(|c| (c, levenshtein(target, c)))
        .filter(|(_, d)| *d <= 3 && *d < target.len())
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c.clone())
}

fn levenshtein(a: &str, b: &str) -> usize {
    let (m, n) = (a.chars().count(), b.chars().count());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}
```

The `step_io` and `curve_state_id` placeholder branches are intentional: real implementations should cover every variant. Add variants for `Inflation`, `StudentT`, `HullWhite`, `CapFloorHullWhite`, `SviSurface`, `XccyBasis`, `Parametric` step kinds; and discount/forward/hazard/base_correlation/vol_surface/swaption_vol/inflation/parametric `CurveState` variants. Inspect `schema.rs` and `state_serde.rs` to enumerate.

If the `CurveState::id()` accessor doesn't exist, define a small trait or add a helper method:

```rust
impl CurveState {
    pub fn curve_id(&self) -> &str {
        match self {
            CurveState::Discount(c) => &c.id,
            CurveState::Forward(c) => &c.id,
            // ... all variants
        }
    }
}
```

Place that helper in `finstack/core/src/market_data/context/state_serde.rs` if needed.

- [ ] **Step 2: Wire validate into `api/mod.rs`**

Add to `finstack/valuations/src/calibration/api/mod.rs`:

```rust
pub mod validate;
pub use validate::{dependency_graph_json, dry_run, validate, DependencyGraph, DependencyNode, ValidationReport};
```

- [ ] **Step 3: Append validator tests**

In `finstack/valuations/tests/calibration/diagnostics.rs`, append:

```rust
use finstack_valuations::calibration::api::schema::{
    CalibrationEnvelope, CalibrationPlan, CalibrationStep, StepParams,
    DiscountCurveParams, HazardCurveParams, CALIBRATION_SCHEMA,
};
use finstack_valuations::calibration::api::validate::{validate, dry_run, dependency_graph_json};
use finstack_core::HashMap;

fn empty_envelope_with_plan_id(id: &str) -> CalibrationEnvelope {
    CalibrationEnvelope {
        schema: CALIBRATION_SCHEMA.to_string(),
        schema_url: None,
        plan: CalibrationPlan {
            id: id.to_string(),
            description: None,
            quote_sets: HashMap::default(),
            steps: Vec::new(),
            settings: Default::default(),
        },
        initial_market: None,
    }
}

#[test]
fn validate_empty_envelope_has_no_errors() {
    let envelope = empty_envelope_with_plan_id("empty");
    let report = validate(&envelope);
    assert!(report.errors.is_empty());
    assert!(report.dependency_graph.nodes.is_empty());
}

#[test]
fn validate_step_with_undefined_quote_set_errors() {
    let mut envelope = empty_envelope_with_plan_id("test");
    envelope.plan.steps.push(CalibrationStep {
        id: "discount_step".to_string(),
        quote_set: "nonexistent_set".to_string(),
        params: StepParams::Discount(default_discount_params("USD-OIS")),
    });
    let report = validate(&envelope);
    let errs = &report.errors;
    assert!(errs.iter().any(|e| matches!(
        e,
        finstack_valuations::calibration::api::errors::EnvelopeError::UndefinedQuoteSet { ref_name, .. }
        if ref_name == "nonexistent_set"
    )));
}

#[test]
fn dry_run_returns_json_for_minimal_envelope() {
    let envelope = empty_envelope_with_plan_id("smoke");
    let json = serde_json::to_string(&envelope).unwrap();
    let report_json = dry_run(&json).expect("dry_run succeeds");
    assert!(report_json.contains("\"errors\""));
    assert!(report_json.contains("\"dependency_graph\""));
}

#[test]
fn dependency_graph_json_for_empty_plan_is_well_formed() {
    let envelope = empty_envelope_with_plan_id("smoke");
    let json = serde_json::to_string(&envelope).unwrap();
    let graph_json = dependency_graph_json(&json).expect("dep graph succeeds");
    assert!(graph_json.contains("\"initial_ids\""));
    assert!(graph_json.contains("\"nodes\""));
}

fn default_discount_params(curve_id: &str) -> DiscountCurveParams {
    // Build with sensible defaults — adjust to match the real
    // DiscountCurveParams shape and required fields.
    serde_json::from_value(serde_json::json!({
        "curve_id": curve_id,
        "currency": "USD",
        "base_date": "2026-05-08",
        "method": "Bootstrap",
        "interpolation": "linear",
        "extrapolation": "flat_forward",
        "pricing_discount_id": null,
        "pricing_forward_id": null,
        "conventions": {}
    }))
    .expect("default discount params")
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p finstack-valuations --test calibration diagnostics`
Expected: all tests pass (3 from Task 1 + 4 from Task 2 = 7 total).

If `default_discount_params` JSON shape is wrong (deny_unknown_fields rejects), inspect the actual `DiscountCurveParams` schema and adjust the JSON. The shape should match what `01_usd_discount.json` uses for its `discount` step.

- [ ] **Step 5: Run cargo fmt + check**

Run: `cargo fmt -p finstack-valuations`
Run: `cargo check -p finstack-valuations`
Both clean.

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/calibration/api/validate.rs \
        finstack/valuations/src/calibration/api/mod.rs \
        finstack/valuations/tests/calibration/diagnostics.rs
git commit -m "feat(valuations): static envelope validator + dependency graph + dry_run

Adds validate(envelope) -> ValidationReport that runs all structural
checks (missing dependencies, undefined quote_sets, quote-class
mismatches) in a single pass and returns every error found, plus the
declaration-order dependency graph. dry_run(json) and
dependency_graph_json(json) are JSON-string wrappers for cross-binding
use. The validator is solver-free — runs in microseconds, suitable
as a fast pre-flight check before invoking calibrate(). Quote-class
checking is intentionally permissive in this commit; tighten in a
follow-up."
```

---

## Task 3: Solver diagnostic enrichment

**Goal:** Per-step `CalibrationReport` carries `worst_quote_id` and `worst_quote_residual` when a step fails to converge. When `engine::execute` returns a non-converged result, the surfaced `EnvelopeError::SolverNotConverged` includes those values.

**This task starts with an audit** — re-estimate scope based on what the solver currently exposes.

**Files:**
- Modify: `finstack/valuations/src/calibration/report.rs`
- Possibly modify: each step target under `finstack/valuations/src/calibration/targets/` (discount, forward, hazard, vol_surface, swaption_vol, base_correlation, etc.)

- [ ] **Step 1: Audit current solver-residual exposure**

Read the contents of `finstack/valuations/src/calibration/report.rs` and a representative step target like `targets/discount.rs`. Answer:

- Does the existing `CalibrationReport` already track per-quote residuals? (Look for fields like `quote_residuals: Vec<(String, f64)>` or similar.)
- Does the solver loop currently expose which quote produces the worst residual? (Look for `quote_residuals.iter().max_by(...)` or analogous code.)

**If both already exist:** Task 3 is purely a data-plumbing change to surface them in `CalibrationReport`. Estimated effort: 30 minutes.

**If solver targets only track aggregate `max_residual`:** Task 3 requires extending each target's solver loop to track per-quote residuals at the iteration that hit the max. Estimated effort: 1-3 hours per target × ~6 targets.

**Report what you found before proceeding to Step 2.** If audit reveals "deep restructuring required", file a status report (DONE_WITH_CONCERNS) and propose either (a) shipping Task 3 with `worst_quote_id: None` for all steps in this phase and tightening in a follow-up, or (b) explicit user approval to expand scope.

- [ ] **Step 2: Add fields to `CalibrationReport`**

In `finstack/valuations/src/calibration/report.rs`, locate the `CalibrationReport` struct. Add:

```rust
/// Identifier of the quote whose residual was the largest at the iteration
/// that ended the solve. `None` for steps that converged (or didn't track
/// per-quote residuals).
#[serde(default, skip_serializing_if = "Option::is_none")]
pub worst_quote_id: Option<String>,

/// Residual value at `worst_quote_id`. `None` if `worst_quote_id` is `None`.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub worst_quote_residual: Option<f64>,
```

Add the same fields to the `Default` impl (if explicit) and any constructor (`new`/`for_step`/etc.) that builds `CalibrationReport` instances. Default to `None`.

- [ ] **Step 3: Populate worst_quote_id in step targets (if audit allows)**

For each step target in `finstack/valuations/src/calibration/targets/*.rs`, after the solver loop, identify the quote with the largest residual and set `worst_quote_id` / `worst_quote_residual` in the produced `CalibrationReport`.

If the audit (Step 1) found the solver already tracks per-quote residuals: minimal change — just write the max-residual quote's ID into the report.

If the audit found targets only track aggregate residuals: add a `Vec<(String, f64)>` accumulator in each target's solver loop, populate after each iteration, and write the worst into the report at the end. **Bound the work** — pick the 3 most-used step kinds (discount, hazard, vol_surface) and do them in this task. File a follow-up for the rest.

Cite which targets you covered in the commit message.

- [ ] **Step 4: Test**

Add a test in `finstack/valuations/tests/calibration/diagnostics.rs`:

```rust
#[test]
fn solver_report_carries_worst_quote_when_available() {
    // Construct an envelope that is structurally valid but whose quotes
    // produce a fit residual > tolerance (e.g., contradictory IRS rates).
    // Run engine::execute, inspect the per-step report.
    //
    // Implementation note: this test is best-effort — it exercises the
    // worst_quote_id field whenever the solver fails. If the configured
    // tolerance is loose enough that the solve always succeeds, mark
    // the test #[ignore] and document in a comment that production
    // enrichment is verified by the integration tests.
}
```

If reproducing a failure-mode envelope is hard, mark the test `#[ignore]` and document in a comment that the field is exercised by manual debugging or follow-up integration tests.

- [ ] **Step 5: Run tests, fmt, check**

Run: `cargo test -p finstack-valuations --test calibration`
Run: `cargo test -p finstack-valuations --test calibration reference_envelopes`
Both must pass — adding optional fields to `CalibrationReport` is backwards-compatible.
Run: `cargo fmt -p finstack-valuations && cargo check -p finstack-valuations`

- [ ] **Step 6: Commit**

```bash
git add finstack/valuations/src/calibration/report.rs \
        finstack/valuations/src/calibration/targets/ \
        finstack/valuations/tests/calibration/diagnostics.rs
git commit -m "feat(valuations): surface worst_quote_id on CalibrationReport

Per-step CalibrationReport gains optional worst_quote_id +
worst_quote_residual fields populated by the solver targets that track
per-quote residuals. When engine::execute returns a non-converged
report, the worst-fitting quote's ID surfaces directly in the report
and (via Phase 4 EnvelopeError::SolverNotConverged) in the error
message — pointing at the input most likely to fix.

This commit covers <list which targets are populated>; remaining
targets fall back to None and can be enriched in a follow-up."
```

---

## Task 4: Python bindings

**Goal:** Python users get a typed `CalibrationEnvelopeError(RuntimeError)` exception and `dry_run` / `dependency_graph_json` free functions.

**Files:**
- Modify: `finstack-py/src/bindings/valuations/calibration.rs`
- Modify: `finstack-py/finstack/valuations/__init__.pyi`
- Possibly modify: `finstack-py/src/bindings/valuations/mod.rs` (re-exports)

- [ ] **Step 1: Register the exception class**

In `finstack-py/src/bindings/valuations/calibration.rs`, near the existing `#[pymodule]` block (or inside `register_module`), add:

```rust
use pyo3::create_exception;

create_exception!(
    finstack_py.valuations,
    CalibrationEnvelopeError,
    pyo3::exceptions::PyRuntimeError
);
```

Then in the module init function, register the type:

```rust
m.add(
    "CalibrationEnvelopeError",
    py.get_type::<CalibrationEnvelopeError>(),
)?;
```

The class inherits from `RuntimeError`, so existing `except RuntimeError` callers continue to work.

- [ ] **Step 2: Add `dry_run` and `dependency_graph_json` free functions**

In the same file, add:

```rust
#[pyfunction]
fn dry_run(json: &str) -> PyResult<String> {
    use finstack_valuations::calibration::api::validate::dry_run as dry_run_impl;
    dry_run_impl(json).map_err(|e| envelope_error_to_py(&e))
}

#[pyfunction]
fn dependency_graph_json(json: &str) -> PyResult<String> {
    use finstack_valuations::calibration::api::validate::dependency_graph_json as graph_impl;
    graph_impl(json).map_err(|e| envelope_error_to_py(&e))
}

fn envelope_error_to_py(err: &finstack_valuations::calibration::api::errors::EnvelopeError) -> PyErr {
    let exc = CalibrationEnvelopeError::new_err(err.to_string());
    Python::with_gil(|py| {
        if let Ok(obj) = exc.value(py) {
            // Attach structured details on the exception instance.
            let _ = obj.setattr("kind", err_kind(err));
            let _ = obj.setattr("details", err.to_json());
            if let Some(step_id) = err_step_id(err) {
                let _ = obj.setattr("step_id", step_id);
            }
        }
    });
    exc
}

fn err_kind(err: &finstack_valuations::calibration::api::errors::EnvelopeError) -> &'static str {
    use finstack_valuations::calibration::api::errors::EnvelopeError as E;
    match err {
        E::JsonParse { .. } => "json_parse",
        E::UnknownStepKind { .. } => "unknown_step_kind",
        E::MissingDependency { .. } => "missing_dependency",
        E::UndefinedQuoteSet { .. } => "undefined_quote_set",
        E::QuoteClassMismatch { .. } => "quote_class_mismatch",
        E::SolverNotConverged { .. } => "solver_not_converged",
        E::QuoteDataInvalid { .. } => "quote_data_invalid",
        E::StepCycle { .. } => "step_cycle",
    }
}

fn err_step_id(err: &finstack_valuations::calibration::api::errors::EnvelopeError) -> Option<String> {
    use finstack_valuations::calibration::api::errors::EnvelopeError as E;
    match err {
        E::UnknownStepKind { step_id, .. }
        | E::MissingDependency { step_id, .. }
        | E::UndefinedQuoteSet { step_id, .. }
        | E::QuoteClassMismatch { step_id, .. }
        | E::SolverNotConverged { step_id, .. }
        | E::QuoteDataInvalid { step_id, .. } => Some(step_id.clone()),
        _ => None,
    }
}
```

In the same module's init function, register the new functions:

```rust
m.add_function(wrap_pyfunction!(dry_run, m)?)?;
m.add_function(wrap_pyfunction!(dependency_graph_json, m)?)?;
```

- [ ] **Step 3: Update the existing `calibrate` and `validate_calibration_json` to surface the new exception**

The existing `calibrate` and `validate_calibration_json` Rust pyfunctions currently return `PyValueError` on bad input. Update them to use `envelope_error_to_py` for `EnvelopeError`-shaped failures while keeping `PyValueError` for serde JSON errors that are NOT envelope errors. Concretely: when `EnvelopeError` was produced upstream (e.g., from `engine::execute` → propagated `From<EnvelopeError>`), surface as `CalibrationEnvelopeError`. Other failures stay as `ValueError`.

If existing call sites only emit `finstack_core::Error` strings (not structured `EnvelopeError`), this surfacing happens automatically once Tasks 1-3 lay the foundation. For Phase 4, document that legacy non-EnvelopeError failures remain `ValueError`.

- [ ] **Step 4: Update Python type stubs**

In `finstack-py/finstack/valuations/__init__.pyi`, add to `__all__`:

```python
__all__ = [
    # ...existing entries...
    "CalibrationEnvelopeError",
    "dry_run",
    "dependency_graph_json",
]
```

Add the class declaration:

```python
class CalibrationEnvelopeError(RuntimeError):
    """Raised when a calibration envelope fails validation or solving.

    Inherits from RuntimeError, so ``except RuntimeError`` continues to
    catch it (backward-compat with pre-Phase-4 callers).

    Attributes:
        kind: One of ``"json_parse"``, ``"unknown_step_kind"``,
            ``"missing_dependency"``, ``"undefined_quote_set"``,
            ``"quote_class_mismatch"``, ``"solver_not_converged"``,
            ``"quote_data_invalid"``, ``"step_cycle"``.
        step_id: Identifier of the step that triggered the error, if applicable.
        details: Pretty-printed JSON of the structured EnvelopeError payload.
    """
    kind: str
    step_id: str | None
    details: str
```

Add the function declarations:

```python
def dry_run(json: str) -> str:
    """Pre-flight envelope validation without invoking the solver.

    Returns a JSON-serialized ``ValidationReport`` containing all errors
    found (not just the first) plus the dependency graph. Microseconds.

    Args:
        json: JSON-serialized ``CalibrationEnvelope``.

    Returns:
        Pretty-printed JSON ``ValidationReport``.

    Raises:
        CalibrationEnvelopeError: If the envelope JSON is malformed.

    Example:
        >>> from finstack.valuations import dry_run
        >>> import json as _json
        >>> report_json = dry_run(_json.dumps(envelope))
        >>> report = _json.loads(report_json)
        >>> for err in report["errors"]:
        ...     print(err["kind"], err.get("step_id"))
    """
    ...

def dependency_graph_json(json: str) -> str:
    """Dump the static dependency graph of a calibration plan as JSON.

    Args:
        json: JSON-serialized ``CalibrationEnvelope``.

    Returns:
        Pretty-printed JSON ``DependencyGraph`` with ``initial_ids``
        (curve IDs available at execution start) and ``nodes`` (per-step
        reads/writes in declared order).

    Raises:
        CalibrationEnvelopeError: If the envelope JSON is malformed.
    """
    ...
```

- [ ] **Step 5: Add a Python test**

Create or extend `finstack-py/tests/test_envelope_diagnostics.py`:

```python
"""Tests for Phase 4 envelope diagnostics surface."""

from __future__ import annotations

import json

import pytest

from finstack.valuations import (
    CalibrationEnvelopeError,
    dry_run,
    dependency_graph_json,
)


def _empty_envelope() -> dict:
    return {
        "schema": "finstack.calibration",
        "plan": {
            "id": "smoke",
            "description": None,
            "quote_sets": {},
            "steps": [],
            "settings": {},
        },
        "initial_market": None,
    }


def test_dry_run_returns_json_report() -> None:
    envelope = _empty_envelope()
    report_json = dry_run(json.dumps(envelope))
    report = json.loads(report_json)
    assert report["errors"] == []
    assert "dependency_graph" in report


def test_dry_run_surfaces_structural_errors() -> None:
    envelope = _empty_envelope()
    envelope["plan"]["steps"] = [
        {"id": "broken_step", "quote_set": "missing_set", "kind": "discount", "params": {}}
    ]
    # Whether this surfaces as a serde-parse error or an UndefinedQuoteSet
    # depends on whether `params: {}` deserializes successfully against
    # DiscountCurveParams. Either way, dry_run should return a report or
    # raise CalibrationEnvelopeError.
    try:
        report_json = dry_run(json.dumps(envelope))
        report = json.loads(report_json)
        assert len(report["errors"]) >= 1
    except CalibrationEnvelopeError as exc:
        # Acceptable: a deserialize error before the validator runs.
        assert exc.kind == "json_parse"


def test_dependency_graph_json_well_formed() -> None:
    envelope = _empty_envelope()
    graph_json = dependency_graph_json(json.dumps(envelope))
    graph = json.loads(graph_json)
    assert "initial_ids" in graph
    assert graph["nodes"] == []


def test_calibration_envelope_error_inherits_runtime_error() -> None:
    """Backwards-compat: existing `except RuntimeError` callers still catch it."""
    assert issubclass(CalibrationEnvelopeError, RuntimeError)
```

- [ ] **Step 6: Run Python tests + lint**

Run: `mise run python-build` (rebuilds the bindings).
Run: `uv run pytest -v finstack-py/tests/test_envelope_diagnostics.py`
Expected: 4 tests pass.

Run: `mise run python-lint`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add finstack-py/src/bindings/valuations/calibration.rs \
        finstack-py/finstack/valuations/__init__.pyi \
        finstack-py/tests/test_envelope_diagnostics.py
git commit -m "feat(finstack-py): typed CalibrationEnvelopeError + dry_run + dependency_graph_json

Adds CalibrationEnvelopeError(RuntimeError) with structured kind,
step_id, details attributes. Adds dry_run(json) and
dependency_graph_json(json) free functions wrapping the Rust validator.
Backwards-compatible: CalibrationEnvelopeError inherits RuntimeError so
existing `except RuntimeError` callers keep working."
```

---

## Task 5: WASM bindings

**Goal:** WASM `dryRun` / `dependencyGraphJson` exports plus structured-error throwing in `calibrate` and `validateCalibrationJson`.

**Files:**
- Modify: `finstack-wasm/src/api/valuations/calibration.rs`
- Modify: `finstack-wasm/exports/valuations.js`
- Modify: `finstack-wasm/index.d.ts`

- [ ] **Step 1: Add `dryRun` and `dependencyGraphJson` wasm-bindgen exports**

In `finstack-wasm/src/api/valuations/calibration.rs`, add:

```rust
use finstack_valuations::calibration::api::validate;

/// Pre-flight envelope validation without invoking the solver.
/// Returns a JSON-serialized `ValidationReport`.
#[wasm_bindgen(js_name = dryRun)]
pub fn dry_run(envelope_json: &str) -> Result<String, JsValue> {
    validate::dry_run(envelope_json).map_err(envelope_error_to_js)
}

/// Returns the static dependency graph of a calibration plan as JSON.
#[wasm_bindgen(js_name = dependencyGraphJson)]
pub fn dependency_graph_json(envelope_json: &str) -> Result<String, JsValue> {
    validate::dependency_graph_json(envelope_json).map_err(envelope_error_to_js)
}

fn envelope_error_to_js(err: finstack_valuations::calibration::api::errors::EnvelopeError) -> JsValue {
    use js_sys::{Error, Reflect};

    let display = err.to_string();
    let cause_json = err.to_json();

    let js_err = Error::new(&display);
    js_err.set_name("CalibrationEnvelopeError");

    // Attach structured cause as a property; standard try/catch exposes it via `e.cause`.
    let cause_obj: JsValue = js_sys::JSON::parse(&cause_json)
        .unwrap_or_else(|_| JsValue::from_str(&cause_json));
    let _ = Reflect::set(&js_err, &JsValue::from_str("cause"), &cause_obj);

    js_err.into()
}
```

If `js_sys::Error::set_name` isn't available in the project's `js-sys` version, set the property via `Reflect::set(&err, &JsValue::from_str("name"), &JsValue::from_str("CalibrationEnvelopeError"))`.

- [ ] **Step 2: Update existing `calibrate` and `validateCalibrationJson` to use the new error wrapper**

In the same file, the existing `calibrate` and `validate_calibration_json` use `to_js_err`. Switch their error mapping to `envelope_error_to_js` when the underlying error is an `EnvelopeError`. For now, this only fires for paths that already produce `EnvelopeError` (Tasks 1-3). Legacy serde errors continue to flow through `to_js_err`.

Concretely, change:

```rust
let parsed: CalibrationEnvelope = serde_json::from_str(json).map_err(to_js_err)?;
```

to:

```rust
let parsed: CalibrationEnvelope = serde_json::from_str(json).map_err(|e| {
    envelope_error_to_js(EnvelopeError::JsonParse {
        message: e.to_string(),
        line: Some(e.line() as u32),
        col: Some(e.column() as u32),
    })
})?;
```

(And add `use finstack_valuations::calibration::api::errors::EnvelopeError;`.)

This makes the JSON-parse path surface as `CalibrationEnvelopeError` rather than a generic JS error.

- [ ] **Step 3: Re-export through the JS facade**

In `finstack-wasm/exports/valuations.js`, alongside the existing `calibrate` and `validateCalibrationJson` wrappers, add wrappers for the new functions:

```js
dryRun(envelope) {
  const json = typeof envelope === 'string' ? envelope : JSON.stringify(envelope);
  return wasm.dryRun(json);
},
dependencyGraphJson(envelope) {
  const json = typeof envelope === 'string' ? envelope : JSON.stringify(envelope);
  return wasm.dependencyGraphJson(json);
},
```

(Both return JSON strings from the Rust side; pass through verbatim.)

- [ ] **Step 4: Update `index.d.ts`**

Add typed signatures next to the existing calibrate / validate signatures:

```ts
/**
 * Pre-flight envelope validation without invoking the solver.
 * Returns a JSON-serialized ValidationReport listing all errors plus
 * the dependency graph. Microseconds.
 *
 * @throws CalibrationEnvelopeError if the envelope JSON is malformed.
 */
dryRun(envelope: CalibrationEnvelope | string): string;

/**
 * Returns the static dependency graph of a calibration plan as JSON.
 *
 * @throws CalibrationEnvelopeError if the envelope JSON is malformed.
 */
dependencyGraphJson(envelope: CalibrationEnvelope | string): string;
```

Add a comment near the top documenting the thrown error shape:

```ts
/**
 * Errors thrown by calibrate, validateCalibrationJson, dryRun, and
 * dependencyGraphJson have:
 *   - name: 'CalibrationEnvelopeError'
 *   - cause: structured EnvelopeError JSON (parse and inspect via JSON.parse)
 * Standard try/catch exposes both via `e.name` and `e.cause`.
 */
```

- [ ] **Step 5: Test**

Run: `cargo test -p finstack-wasm api::valuations::calibration --lib`
Expected: existing 2 tests pass + any new ones added.

Add a smoke test for the error path if practical:

```rust
#[test]
fn dry_run_rejects_malformed_json() {
    let err = dry_run("not json").unwrap_err();
    let msg = format!("{:?}", err);
    assert!(msg.contains("JSON parse") || msg.contains("CalibrationEnvelopeError"));
}
```

- [ ] **Step 6: Run lint + cargo fmt**

Run: `cargo fmt -p finstack-wasm`
Run: `npm --prefix finstack-wasm run lint`
Both clean.

- [ ] **Step 7: Commit**

```bash
git add finstack-wasm/src/api/valuations/calibration.rs \
        finstack-wasm/exports/valuations.js \
        finstack-wasm/index.d.ts
git commit -m "feat(wasm): dryRun + dependencyGraphJson + structured calibration errors

Adds dryRun and dependencyGraphJson exports to the JS facade. Existing
calibrate and validateCalibrationJson now throw a JS Error with
name='CalibrationEnvelopeError' and a structured cause object on
envelope-shaped failures. Standard try/catch exposes both via
e.name and e.cause; backwards-compatible with callers that only check
e.message."
```

---

## Task 6: Notebook "When envelopes fail" section

**Goal:** Append a 4-cell section to the analyst notebook demonstrating an intentionally-broken envelope, catching `CalibrationEnvelopeError`, and using `dry_run` for pre-flight validation.

**Files:**
- Modify: `finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb`

- [ ] **Step 1: Append cells via nbformat**

Run via `uv run python`:

```python
import nbformat as nbf
from pathlib import Path

NB_PATH = Path("finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb")
nb = nbf.read(NB_PATH, as_version=4)

new_cells = [
    nbf.v4.new_markdown_cell(
        "## When envelopes fail\n\n"
        "Phase 4 surfaces structured diagnostics for envelope failures. The two\n"
        "cheap pre-flight tools are `dry_run` (runs all structural checks without\n"
        "invoking the solver — microseconds) and `dependency_graph_json` (dumps\n"
        "the step DAG). Both raise `CalibrationEnvelopeError` on malformed JSON;\n"
        "structural failures surface as entries in `dry_run`'s report.\n\n"
        "The cell below deliberately misspells a `quote_set` reference. `dry_run`\n"
        "catches it and points at the step + suggests a fix. `calibrate` would\n"
        "produce the same error message at runtime — but `dry_run` is faster and\n"
        "lets the analyst fix the envelope before the slow solve."
    ),
    nbf.v4.new_code_cell(
        "import json\n"
        "from finstack.valuations import calibrate, dry_run, dependency_graph_json, CalibrationEnvelopeError\n"
        "\n"
        "envelope_path = REPO_ROOT / \"finstack\" / \"valuations\" / \"examples\" / \"market_bootstrap\" / \"01_usd_discount.json\"\n"
        "envelope = json.loads(envelope_path.read_text())\n"
        "\n"
        "# Deliberately misspell a quote_set reference.\n"
        "envelope[\"plan\"][\"steps\"][0][\"quote_set\"] = \"usd_quotess\"  # extra 's'\n"
        "\n"
        "report = json.loads(dry_run(json.dumps(envelope)))\n"
        "for err in report[\"errors\"]:\n"
        "    print(f\"[{err['kind']}] step '{err.get('step_id', '?')}': \", end='')\n"
        "    if err['kind'] == 'undefined_quote_set':\n"
        "        print(f\"quote_set '{err['ref_name']}' not found. Suggestion: '{err.get('suggestion')}'\")\n"
        "    else:\n"
        "        print(json.dumps(err))"
    ),
    nbf.v4.new_markdown_cell(
        "### Catching the typed exception\n\n"
        "Once the envelope is fixed, `calibrate` succeeds. If a real failure happens\n"
        "downstream (e.g., solver non-convergence), `CalibrationEnvelopeError`\n"
        "carries `kind`, `step_id`, and `details` for programmatic handling."
    ),
    nbf.v4.new_code_cell(
        "# Restore the correct quote_set name and run.\n"
        "envelope[\"plan\"][\"steps\"][0][\"quote_set\"] = \"usd_quotes\"\n"
        "try:\n"
        "    result = calibrate(json.dumps(envelope))\n"
        "    print(f\"success: {result.success}, rmse: {result.rmse:.3e}\")\n"
        "except CalibrationEnvelopeError as exc:\n"
        "    print(f\"caught {exc.__class__.__name__}: kind={exc.kind}, step={exc.step_id}\")\n"
        "    print(f\"details:\\n{exc.details}\")"
    ),
    nbf.v4.new_markdown_cell(
        "### Inspecting the dependency graph\n\n"
        "`dependency_graph_json` returns the static DAG of a plan's steps —\n"
        "useful for visualizing layered envelopes (envelope 12) or debugging\n"
        "missing dependencies."
    ),
    nbf.v4.new_code_cell(
        "graph = json.loads(dependency_graph_json(json.dumps(envelope)))\n"
        "print(f\"initial_ids: {graph['initial_ids']}\")\n"
        "for node in graph['nodes']:\n"
        "    reads = ', '.join(node['reads']) or '(none)'\n"
        "    writes = ', '.join(node['writes']) or '(none)'\n"
        "    print(f\"step[{node['step_index']}] {node['step_id']} ({node['kind']}): reads [{reads}] -> writes [{writes}]\")"
    ),
]

nb.cells.extend(new_cells)

# Strip outputs from the appended cells.
for c in nb.cells:
    if c.cell_type == "code":
        c.outputs = []
        c.execution_count = None

nbf.write(nb, NB_PATH)
print(f"Notebook now has {len(nb.cells)} cells.")
```

This adds 6 cells (3 markdown + 3 code). Total notebook: 14 + 6 = 20 cells.

- [ ] **Step 2: Verify execution**

Run: `uv run jupyter nbconvert --to notebook --execute finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb --output executed_market_bootstrap_tour.ipynb`

Expected: all 20 cells execute without error.

Delete the executed copy: `rm finstack-py/examples/notebooks/01_foundations/executed_market_bootstrap_tour.ipynb`

If a cell fails because of an API mismatch (e.g., `exc.kind` raises AttributeError because the implementer didn't successfully attach attributes in Task 4 Step 2), update Task 4's binding code or the cell's accessor pattern.

- [ ] **Step 3: Commit**

```bash
git add finstack-py/examples/notebooks/01_foundations/market_bootstrap_tour.ipynb
git commit -m "docs(finstack-py): add 'When envelopes fail' section to market_bootstrap_tour

Phase 4 notebook addition: 6 cells demonstrating dry_run pre-flight
validation, CalibrationEnvelopeError handling, and dependency_graph_json
inspection. Walks through a deliberately-broken envelope (misspelled
quote_set), shows dry_run catching it with a suggestion, then
demonstrates the typed exception-handling pattern and graph dump.
Total notebook: 20 cells."
```

---

## Task 7: End-to-end verification & spec acceptance

**Goal:** Confirm Phase 4 is complete with no regressions; walk through the spec's acceptance criteria.

- [ ] **Step 1: Run focused tests**

```bash
cargo test -p finstack-valuations --test calibration diagnostics
cargo test -p finstack-valuations --test calibration reference_envelopes
cargo test -p finstack-wasm api::valuations::calibration --lib
uv run pytest -v finstack-py/tests/test_envelope_diagnostics.py
```

All must pass.

- [ ] **Step 2: Run the full project verification stack**

```bash
mise run all-fmt
mise run all-lint
mise run all-test
```

All gates green.

- [ ] **Step 3: Walk the spec's acceptance criteria**

From `docs/2026-05-08-market-bootstrap-phase-4-diagnostics-design.md` §6:

- [ ] `EnvelopeError` enum with all 8 variants, `Display`, `Serialize`, `to_json`.
- [ ] `validate_json` (or `validate`) returns all errors in one pass.
- [ ] `dry_run` and `dependency_graph_json` callable from Rust, Python, JavaScript.
- [ ] Solver non-convergence errors include `worst_quote_id` (when target supports it).
- [ ] Python: `CalibrationEnvelopeError(RuntimeError)` raised correctly; structured attributes accessible; `except RuntimeError` continues to catch it.
- [ ] WASM: thrown errors have `name = "CalibrationEnvelopeError"` and `cause` containing structured details.
- [ ] Notebook section catches a deliberately-broken envelope and pretty-prints `dry_run` output.

If any criterion fails, return to the relevant task before declaring Phase 4 complete.

- [ ] **Step 4: Final commit if any cleanup needed**

```bash
git status
git add <fixed-files>
git commit -m "chore: format and lint cleanup for market bootstrap phase 4"
```

---

## Phase 4 done

When this plan is complete, calibration envelope failures surface with structured, actionable diagnostics across Rust, Python, and JavaScript. Pre-flight `dry_run` runs in microseconds and catches structural issues before the slow solve. The dependency graph is queryable. Solver non-convergence points at the worst-fitting quote.

Phase 5 (Python TypedDict, fast-follow) is the natural next slice. See [`docs/2026-05-08-market-bootstrap-phase-5-fast-follow-design.md`](2026-05-08-market-bootstrap-phase-5-fast-follow-design.md).
