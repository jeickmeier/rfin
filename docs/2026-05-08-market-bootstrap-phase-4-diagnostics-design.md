# Market Bootstrap Phase 4 — Diagnostics

**Status:** Draft
**Date:** 2026-05-08
**Owner:** finstack/valuations + finstack-py + finstack-wasm
**Phase:** 4 of 5 (diagnostics)
**Depends on:** Phase 1 foundation (independent of Phases 2 and 3)
**Related specs:**
- Phase 1 — Canonical-path foundation: [2026-05-08-market-bootstrap-phase-1-foundation-design.md](2026-05-08-market-bootstrap-phase-1-foundation-design.md)
- Phase 2 — Reference catalog: [2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md](2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md)
- Phase 3 — IDE autocomplete: [2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md](2026-05-08-market-bootstrap-phase-3-ide-autocomplete-design.md)

## 1. Motivation

When a user's envelope fails — bad reference, undefined quote_set, type mismatch, solver non-convergence — the error today is generic and rarely points at the specific issue. There is no pre-flight validator, no dependency-graph view, and no machine-readable error structure for tooling. This phase adds rich, structured diagnostics across the validate / dry-run / execute / fail paths so analysts can debug envelopes confidently.

## 2. Goals

- A structured `EnvelopeError` enum with variants for the common failure modes, both human-readable and machine-readable.
- A static envelope validator that catches all structural issues in one pass before solving — fast (microseconds, no solver invocation).
- Public `dry_run` and `dependency_graph_json` entry points across Rust, Python, and WASM.
- Solver non-convergence errors identify the worst-fitting quote(s) by ID.
- Python `CalibrationEnvelopeError(RuntimeError)` exception class with structured attributes.
- WASM errors carry structured `cause` for `try/catch` consumers.

## 3. Non-Goals

- Improving the solver itself or changing convergence criteria.
- Adding new step kinds.
- Diff or regression-comparison utilities for `MarketContext` (out of scope; would be its own spec).
- Backwards-incompatible Python exception changes — `CalibrationEnvelopeError` inherits from `RuntimeError` so existing `except RuntimeError` callers continue to work.

## 4. Architectural Baseline

Today's diagnostic surface (per Phase 1 §4):

- `engine::execute` returns `Result<CalibrationResultEnvelope, finstack_core::Error>`.
- `validate_calibration_json(json) -> String` canonicalizes via serde.
- `CalibrationReport` per-step holds `step_id`, `success`, `iterations`, `max_residual`, `rmse`, `reason`.
- Python `CalibrationResult` exposes `report_json`, `step_report_json(id)`, `report_to_dataframe()`, `market_json`.

This phase extends but does not replace these surfaces.

## 5. Scope — file-by-file

### 5.1 Structured `EnvelopeError`

[finstack/valuations/src/calibration/api/errors.rs](../finstack/valuations/src/calibration/api/errors.rs) — new module:

```rust
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnvelopeError {
    JsonParse {
        message: String,
        line: Option<u32>,
        col: Option<u32>,
    },
    UnknownStepKind {
        step_index: usize,
        step_id: String,
        found: String,
        expected_one_of: Vec<String>,
    },
    MissingDependency {
        step_index: usize,
        step_id: String,
        kind: String,
        missing_id: String,
        missing_kind: String,
        available: Vec<String>,
    },
    UndefinedQuoteSet {
        step_index: usize,
        step_id: String,
        ref_name: String,
        available: Vec<String>,
        suggestion: Option<String>,
    },
    QuoteClassMismatch {
        step_index: usize,
        step_id: String,
        kind: String,
        expected_class: String,
        breakdown: Vec<(String, usize)>,
    },
    SolverNotConverged {
        step_id: String,
        max_residual: f64,
        tolerance: f64,
        iterations: u32,
        worst_quote_id: Option<String>,
        worst_quote_residual: Option<f64>,
    },
    QuoteDataInvalid {
        step_id: String,
        quote_id: String,
        reason: String,
    },
    StepCycle {
        involved_step_ids: Vec<String>,
    },
}

impl Display for EnvelopeError { /* human-readable, with hints */ }
impl EnvelopeError {
    pub fn to_json(&self) -> String { /* serde_json::to_string_pretty */ }
}
impl From<EnvelopeError> for finstack_core::Error { /* backward compat */ }
```

### 5.2 Static envelope validator

Extend `validate_calibration_json` in [finstack/valuations/src/calibration/api/engine.rs](../finstack/valuations/src/calibration/api/engine.rs) (or a new `validate.rs` sibling under `calibration/api/`):

```rust
pub struct ValidationReport {
    pub canonical_envelope: Option<String>,  // present iff valid
    pub errors: Vec<EnvelopeError>,
    pub dependency_graph: DependencyGraph,
}

pub fn validate(envelope: &CalibrationEnvelope) -> ValidationReport;
pub fn validate_json(json: &str) -> ValidationReport;
```

The validator:

1. Parses the envelope via serde (collects JSON parse errors as `EnvelopeError::JsonParse`).
2. Builds the dependency graph from `initial_market` (curve IDs available at start) plus each step's declared inputs and outputs.
3. For each step, checks: `quote_set` exists in `plan.quote_sets`, referenced curve IDs are reachable (in `initial_market` or produced by an earlier step in topological order), quote classes match the step's expected class.
4. Detects cycles between steps.
5. Returns all errors found, not just the first.

`dry_run(envelope_json) -> String` returns `ValidationReport` JSON (for binding-friendly serialization).

### 5.3 Dependency graph dump

Add `dependency_graph(envelope) -> DependencyGraph` and `dependency_graph_json(envelope_json) -> String`:

- `DependencyGraph` lists nodes (curve IDs sourced from `initial_market` + step outputs), edges (step inputs), and the topologically-sorted step execution order.
- JSON form is structured with both `nodes`/`edges` (for graph rendering) and a flat `topo_order` list (for human reading).

### 5.4 Solver diagnostic enrichment

Per-step `CalibrationReport` already carries residuals. Add `worst_quote_id: Option<String>` and `worst_quote_residual: Option<f64>` populated at the solver-loop level. When `engine::execute` returns a non-converged step, lift those fields into a `SolverNotConverged` `EnvelopeError`.

(First step in implementation: audit how the solver currently surfaces per-quote residuals. If they are already accessible, this is a small data-plumbing change. If not, it requires deeper restructuring — re-estimate accordingly.)

### 5.5 Python binding

[finstack-py/src/bindings/valuations/calibration.rs](../finstack-py/src/bindings/valuations/calibration.rs):

```python
class CalibrationEnvelopeError(RuntimeError):
    """Raised when a calibration envelope fails validation or solving."""
    kind: str
    step_id: str | None
    envelope_path: str | None  # e.g., "plan.steps[2]"
    details: dict  # full structured details from EnvelopeError::Serialize

def dry_run(envelope_json: str) -> str: ...  # returns ValidationReport JSON
def dependency_graph_json(envelope_json: str) -> str: ...
```

`calibrate` and `validate_calibration_json` raise `CalibrationEnvelopeError` (still inheriting `RuntimeError`, so existing `except RuntimeError` callers continue to work).

[finstack-py/finstack/valuations/**init**.pyi](../finstack-py/finstack/valuations/__init__.pyi):
- Add `CalibrationEnvelopeError`, `dry_run`, `dependency_graph_json` to `__all__` with stubs.

### 5.6 WASM binding

[finstack-wasm/src/api/valuations/calibration.rs](../finstack-wasm/src/api/valuations/calibration.rs):
- Add `dryRun(envelopeJson: string): string` and `dependencyGraphJson(envelopeJson: string): string`.
- On error, return a `JsValue` constructed from a JS `Error` whose `name = "CalibrationEnvelopeError"` and whose `cause` property is set to the structured `EnvelopeError` JSON. Standard JS `try/catch (e)` then exposes `e.name` and `e.cause`.

[finstack-wasm/exports/valuations.js](../finstack-wasm/exports/valuations.js): re-export `dryRun` and `dependencyGraphJson`.

[finstack-wasm/index.d.ts](../finstack-wasm/index.d.ts): typed signatures for the new functions.

### 5.7 Notebook section

Notebook gets a "When envelopes fail" section showing:

1. A deliberately-broken envelope (e.g., misspelled `quote_set` reference).
2. `try: calibrate(...) except CalibrationEnvelopeError as e:` — print `e.kind`, `e.step_id`, `e.details`.
3. Run `dry_run(envelope_json)` to see the full error list and dependency graph in one call.
4. Fix and retry.

## 6. Acceptance Criteria

- [ ] `EnvelopeError` enum exists with all variants from §5.1, with `Display`, `Serialize`, and `to_json`.
- [ ] `validate_json` returns all errors in one pass (not just first); covered by unit tests for each variant.
- [ ] `dry_run` and `dependency_graph_json` callable from Rust, Python, and JavaScript.
- [ ] Solver non-convergence errors include `worst_quote_id`.
- [ ] Python: `CalibrationEnvelopeError(RuntimeError)` raised correctly; structured attributes accessible; `except RuntimeError` continues to catch it (backward compat).
- [ ] WASM: thrown errors have `name = "CalibrationEnvelopeError"` and `cause` containing structured details; documented in `index.d.ts`.
- [ ] Notebook section catches a deliberately-broken envelope and pretty-prints `dry_run` output.

## 7. Verification Commands

- `cargo test -p finstack-valuations --test calibration envelope_errors`
- `cargo test -p finstack-valuations --test calibration validator`
- `uv run pytest -v finstack-py/tests/test_envelope_errors.py`
- `npm --prefix finstack-wasm run test`
- `mise run all-test`

## 8. Risks

- **Effort uncertainty.** The estimated 3–5 days assumes most current error pathways are stringly-typed and need wrapping. If parts already have structure, the work shrinks. If the solver loop needs deep restructuring to expose worst-quote info, work grows. **First step in implementation: audit current error pathways and re-estimate before starting the structured-error work.**
- **Backward-compat tightrope.** `CalibrationEnvelopeError(RuntimeError)` keeps existing Python catchers working but changes the type they catch. Users who type-narrow with `isinstance(e, RuntimeError) and not isinstance(e, CalibrationEnvelopeError)` would see behavior change. This is unlikely in practice but flag in the changelog.
- **`dry_run` performance expectations.** "Microseconds" is the goal; actual depends on dependency-graph traversal cost. Should be linear in envelope size; verify with a benchmark in the same crate as other calibration benches.
