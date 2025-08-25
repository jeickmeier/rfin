### Finstack Analysis — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, risk, app engineers (Python/WASM/Rust)
**Purpose:** Define user-facing requirements for the Analysis crate. This aligns with `docs/new/06_analysis/06_analysis_tdd.md` while remaining accessible to non-Rust users.

---

## 1) Executive Summary

The Analysis crate provides extensible, plugin-based analysis capabilities on top of statements, valuations, scenarios, and portfolios. It enables users to validate models, explain calculations, run sensitivities and parameter grids, attribute changes (waterfalls), evaluate recovery and implied ratings, and introspect scenario plans. Results are deterministic, auditable, and available consistently in Rust, Python, and WASM.

Outcomes:
- **Faster insights:** One interface for validation, sensitivities, waterfalls, and scenario introspection.
- **Auditability:** Clear parameters, schemas, and metadata for every analysis run.
- **Determinism & parity:** Stable results across languages and hosts with the same inputs.

---

## 2) Goals and Non-Goals

- **Goal: Extensible plugin architecture** with discoverable analyzers and stable metadata (id, version, category, tags).
- **Goal: Schema-driven UX** where parameters/results validate against JSON Schemas with helpful error messages.
- **Goal: Built-in analyzers** covering validation, node explanation, sensitivity/grid, waterfall, recovery, implied ratings, and scenario explainer.
- **Goal: Composition** via pipelines with dependencies, error strategies, and deterministic execution.
- **Goal: Cross-language parity** (Rust, Python, WASM) with matching behavior and JSON shapes.
- **Goal: Performance with determinism** including parallel execution options that do not change numeric outputs.
- **Goal: Caching** for repeat analyses with content-addressed keys.

Non-Goals:
- Implement pricing kernels, cashflow math, or scenario engines (reused from other crates).
- Introduce new time-series or numeric backends (reuse Polars and math from dependent crates).
- Ship a GUI or external data connectors.

---

## 3) Target Users & Personas

- **Financial Analyst:** Validates models, runs what-ifs, explains results for reviews.
- **Quant/Developer:** Automates sensitivity studies, parameter sweeps, and diagnostic tooling.
- **Risk/PM:** Compares scenarios, attributes period-over-period changes, aggregates outcomes.
- **Model Governance/Audit:** Requires traceability, schemas, and reproducible reports.
- **App/Web Engineer:** Embeds analyzers in notebooks or browsers with predictable JSON IO.

---

## 4) Primary Use Cases

- **Model validation:** Run comprehensive checks (articulation, formulas, period alignment, currency consistency).
- **Explain a number:** Show formula, dependencies, calculation path, and final values for a node/period.
- **Sensitivity analysis:** Sweep one or more variables and evaluate specific output nodes.
- **Parameter grid:** Multi-dimensional grid sweeps with stable outputs and aggregations.
- **Waterfall attribution:** Explain changes between two periods with optional FX/price/volume mixes.
- **Recovery/LGD:** Evaluate recovery scenarios with costs and timing; compute expected recovery and LGD.
- **Implied ratings:** Derive a rating/score from financial metrics and peer comparisons.
- **Scenario explainer:** Preview composition, conflicts, expansions, and optional before/after impacts.
- **Pipelines:** Chain multiple analyses with dependencies and shared context.

---

## 5) Scope (In/Out)

In-Scope:
- Analyzer registry and discovery; analyzer metadata and categories.
- JSON Schemas for parameters/results; runtime validation and clear diagnostics.
- Built-in analyzers (see §6.3) and pipeline composition.
- Deterministic parallel execution, stable aggregation, and optional caching.
- Cross-language interfaces (Rust/Python/WASM) with consistent behavior.

Out-of-Scope (handled by other crates):
- Pricing/cashflow generation, statements evaluation, and scenario execution engines.
- Raw data connectors or storage systems; UI components.

---

## 6) Functional Requirements

### 6.1 Analyzer Registry & Discovery
- List available analyzers with metadata: id, name, version, category, tags, capabilities.
- Retrieve analyzers by id; unique id/version pairs are enforced.
- Support manual and link-time/runtime registration for extensibility.

### 6.2 Schemas & Validation
- Every analyzer exposes a parameter schema; optionally exposes a result schema.
- Validate inputs before execution; errors point to exact fields/paths and remediation hints.

### 6.3 Built-in Analyzers
- **Validation Report:** Returns pass/fail, error/warning lists, coverage metrics, optional articulation checks; tolerance and strict modes supported.
- **Node Explainer:** For a node (and optional period), return formula, dependencies, calculation path, and final value; optionally include values and traces.
- **Sensitivity:** Accepts variables with ranges (percent/absolute/bps/custom), output nodes, optional parallel flag; returns base case and sensitivity table with basic stats.
- **Grid:** Multi-dimensional sweeps with stable indexing; optional aggregations; optional caching of intermediate points.
- **Waterfall:** Start/end period attribution with categories (price, volume, mix, FX, etc.) and bridge-chart-friendly output.
- **Recovery:** Entity-specific recovery scenarios (rate, timing, costs); returns expected recovery, LGD, and optional waterfalls.
- **Implied Ratings:** Methodology selection, optional peer group and weights; returns implied rating, score, confidence interval, and key metric assessments.
- **Scenario Explainer:** Accepts DSL or structured spec; returns composition rules, final ordered operations, glob expansions, conflicts, and optional impact previews; supports as-of date and optional market/portfolio inputs.

### 6.4 Composition & Pipelines
- Fluent pipeline builder to chain analyzers with dependencies and per-step error strategies (fail/skip/retry/use-default).
- Deterministic stage ordering; DAG validation with helpful messages on cycles.
- Pipeline results merge named step outputs with stable keys.

### 6.5 Parallel Execution & Scheduling
- Deterministic parallel mode that preserves output order and values.
- Complexity hints inform batching/scheduling for predictable performance.

### 6.6 Caching
- Optional TTL-based, content-addressed cache keyed by analyzer id, version, model hash, and params hash.
- Ability to enable/disable and configure max entries/size; return cached results transparently when valid.

### 6.7 Cross-Language Interfaces
- **Python:** Wheels for major OSes; type-hinted API; GIL released on heavy work; schemas accessible; pass/return dicts and Pydantic-friendly shapes.
- **WASM/TypeScript:** JSON parity with Rust; factory to create analyzers by id; TypeScript types for analyzer metadata and pipeline API.

### 6.8 Observability & Metadata
- Each result includes run metadata: analyzer id/version, numeric/parallel mode, start/end timestamps, and optional cache hit information.
- Structured tracing spans surround analyzer and pipeline steps.

---

## 7) Non-Functional Requirements

- **Determinism:** Parallel vs serial produce identical numeric outputs by default.
- **Performance targets:** Single analysis on a 10k-node model < 500 ms; 5-step pipeline < 2 s; 10×10 sensitivity grid < 5 s; 100-model parallel batch < 10 s on reference hardware.
- **Portability:** Rust core on stable toolchains; Python 3.12+ wheels; WASM for `wasm32-unknown-unknown`.
- **Security/Safety:** No arbitrary code execution from params; strict schema validation; bounded resource usage.
- **Auditability:** Parameters, schemas, and composition/conflict decisions are accessible and exportable.
- **Stability:** Semver-governed public APIs/schemas; migrations documented for changes.

---

## 8) User Experience Requirements

- **Simple calls:** One or two function calls to run an analyzer or pipeline in Rust, Python, or JS.
- **Helpful errors:** Validation messages identify the field and expected shape; link to docs/examples.
- **Docs & examples:** Quickstarts for each built-in analyzer and pipelines; runnable end-to-end in Python and browser.
- **Result shapes:** Stable keys suitable for DataFrame construction in downstream layers.

---

## 9) Success Metrics

- **Determinism:** Golden tests confirm serial vs parallel parity across OS/architectures.
- **Performance:** Meets targets in CI benchmarks for representative models.
- **Adoption:** Analysts can run validation + sensitivity + waterfall in < 15 minutes from examples.
- **Stability:** No breaking schema changes without a semver bump and migration notes.
- **Transparency:** Scenario explainer used in >70% of scenario-related support sessions to diagnose plans.

---

## 10) Release Plan (Phased)

- **Phase 1:** Analyzer registry, schemas/validation, Validation Report, Node Explainer, Python/WASM parity.
- **Phase 2:** Sensitivity and Grid analyzers with deterministic parallel execution and initial caching.
- **Phase 3:** Waterfall analyzer; Scenario Explainer; expanded examples and docs.
- **Phase 4:** Recovery and Implied Ratings analyzers; pipeline builder GA; performance hardening.

Each phase ships with docs, examples, deterministic outputs, and CI coverage matching acceptance criteria.

---

## 11) Acceptance Criteria (High-Level)

- Registry lists built-ins with correct metadata; analyzers are retrievable by id in all languages.
- Parameter validation errors include JSON pointer paths and clear remediation text.
- Validation Report flags articulation or currency consistency issues with reproducible outputs.
- Node Explainer returns formula, dependency graph summary, and optional values for a given node/period.
- Sensitivity/Grid produce stable tables and base-case summaries; parallel and serial match.
- Waterfall returns start/end values, total change, and categorized components suitable for bridge charts.
- Scenario Explainer shows composition rules, expansions, conflicts, and optional impacts consistently with the scenarios engine.
- Python and WASM examples run end-to-end and match Rust outputs byte-for-byte in JSON.

---

## 12) Risks & Mitigations

- **Numeric drift:** Reuse statements/valuations/scenarios engines; enforce deterministic modes; golden tests.
- **Schema creep:** Centralize schemas; semver governance; CI checks with golden files.
- **Performance regressions:** Criterion benchmarks and gates on representative analyses.
- **Cross-language divergence:** Shared schemas and parity tests across Rust/Python/WASM.
- **Plugin misconfiguration:** Clear registration errors and analyzer capability metadata to guide usage.

---

## 13) References

- Technical design: `docs/new/06_analysis/06_analysis_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`
- Related designs: Statements, Valuations, Scenarios, Portfolio crates (see docs/new)


