### Config (`/config`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, data scientists, and engineers (Rust/Python/WASM)
**Purpose:** Define user‑facing requirements for a workspace‑wide configuration system focused on numeric rounding and currency scale policy. This aligns with `docs/new/10_config/10_config_tdd.md` and ensures deterministic ingest/output behavior and reproducible results across bindings and hosts.

---

## 1) Executive Summary

The Config layer provides a single source of truth for numeric rounding and currency scale policy across the stack. It enforces deterministic `rust_decimal` behavior at ingest (inputs/deserialization/builders) and at output (serialization/export), and stamps the active policy into results metadata for auditability and reproducibility. Defaults follow market conventions (e.g., half‑to‑even, 2 decimals; ISO‑4217 overrides like JPY=0), and hosts can initialize the policy once per process, with scoped overrides for tests/utilities.

Key outcomes:
- **Deterministic numbers** across OS/hosts and serial vs parallel runs.
- **Stable interop** for CSV/JSON/WASM/Python via consistent scaling at IO boundaries.
- **Transparent policy**: active rounding/scale context is embedded in results metadata.

---

## 2) Goals and Non‑Goals

Goals:
- Provide a global, deterministic rounding/scale policy that governs ingest and output.
- Support currency‑aware scales with ISO‑4217‑style overrides and sensible defaults.
- Expose the active rounding context in all top‑level results for traceability.
- Offer a safe API for process‑wide initialization and scoped, temporary overrides for tests.
- Maintain stable, versioned wire shapes across Rust, Python, and WASM.

Non‑Goals:
- Locale display/formatting (names, separators) beyond numeric rounding/scale.
- Implicit runtime mutation of global policy in production flows.
- Per‑field scale policies (e.g., percent/bps) in this version.
- Time‑based policy migrations (versioning hooks are defined but not automated).

---

## 3) Target Users & Personas

- **Quant/Engineer (Rust):** Needs deterministic numeric handling and a single policy surface.
- **Data Scientist (Python):** Expects stable DataFrame‑friendly outputs with predictable decimals.
- **Analyst/PM:** Requires auditable, reproducible numbers across hosts and runs.
- **App Engineer (WASM):** Relies on small, deterministic JSON interfaces with clear metadata.

---

## 4) Primary Use Cases

- **Global initialization:** Host sets the rounding mode and currency scales once during startup.
- **Ingest normalization:** Builders and deserializers normalize `Decimal` inputs according to the ingest scale by currency.
- **Output normalization:** Serializers/export utilities apply the output scale by currency for stable CSV/JSON interop.
- **Metadata stamping:** Every top‑level result embeds the active rounding context for auditability/replay.
- **Scoped overrides (tests):** Temporarily swap config within a closure/context manager for test scenarios.

---

## 5) Scope (In / Out)

In‑Scope:
- Rounding modes: half‑to‑even (bankers), away/toward zero, floor, ceiling.
- Currency scale policy: default scale plus per‑currency overrides.
- Ingest and output application points with deterministic behavior.
- Results metadata containing a serializable rounding context and version.
- Cross‑crate visibility (valuations, statements, portfolio) via stable wire types.

Out‑of‑Scope (current release):
- Locale formatting profiles (names, separators, symbol placement).
- Per‑field scale policies (percent/bps) and time‑scheduled policy changes.
- Dynamic user/session‑level mutation of the global policy in production.

---

## 6) Functional Requirements

### 6.1 Rounding & Scale Policy
- Provide a closed set of rounding modes: Bankers (half‑to‑even), AwayFromZero, TowardZero, Floor, Ceil.
- Provide currency scale policy: a `default_scale` and a map of overrides keyed by currency code.
- Maintain separate policies for ingest and output to support asymmetric workflows when needed.

### 6.2 Defaults
- Default rounding mode is Bankers (half‑to‑even).
- Default scale is 2 decimals.
- ISO‑style overrides include: JPY=0, KWD=3, BHD=3, TND=3.
- Policy schema/version starts at 1; increment on semantic changes.

### 6.3 Application Points
- **Ingest:** Deserializers, CSV/JSON loaders, and builder methods normalize numeric inputs using the ingest scale by currency when available; otherwise use default scale.
- **Output:** Serializers/exporters (CSV/JSON/Arrow) apply output scale by currency; unitless scalars use default scale unless the caller specifies otherwise.

### 6.4 Results Metadata
- All top‑level result envelopes include `rounding` metadata with: rounding mode, ingest scale map, output scale map, and a policy version.
- Metadata is stable across bindings and included in golden tests for reproducibility.

### 6.5 Cross‑Crate Responsibilities
- **Valuations:** Stamp the rounding context in `ValuationResult` and apply output scale for exported amounts.
- **Statements:** Include the rounding context in result metadata; respect currency scales at export.
- **Portfolio:** Include the rounding context in `PortfolioResults` and in all exported tables.

### 6.6 API & Bindings
- Hosts initialize the process‑wide config once; subsequent reads are lock‑free and thread‑safe.
- Provide scoped overrides for tests/utilities that do not leak beyond the call boundary.
- Expose equivalent capabilities in Python and WASM with stable serde/JSON shapes.

### 6.7 Observability & Errors
- Clear errors when invalid rounding mode or currency codes are provided.
- Structured traces when applying ingest/output scaling at IO boundaries.

---

## 7) Non‑Functional Requirements

- **Determinism:** Identical Decimal results across OS/hosts; serial vs parallel parity.
- **Stability:** Semver‑governed public schemas; unknown fields rejected by default.
- **Portability:** Rust core with first‑class Python/WASM parity and stable JSON IO.
- **Safety & Security:** No `unsafe` in policy application; closed set of deterministic hooks.
- **Performance:** Negligible overhead for policy lookups and Decimal scaling.

---

## 8) User Experience Requirements

### 8.1 Python
- Simple configuration object mirrors the wire shape; Pydantic validation and friendly errors.
- Context manager for scoped overrides in tests/examples.
- DataFrame exports reflect output scale; rounding metadata available on result objects.

### 8.2 WASM
- JSON IO mirrors serde names; small bundles via feature flags; examples show ingest/output scaling.

### 8.3 Documentation & Examples
- Quickstart: initialize config, ingest values with currencies, export CSV/JSON with stable decimals.
- Examples: effect of different rounding modes; currency scale overrides; metadata inspection.

---

## 9) Success Metrics

- **Determinism:** Golden tests with fixed rounding context pass across CI matrices.
- **Transparency:** 100% of top‑level results include rounding metadata.
- **Interop stability:** CSV/JSON exports are byte‑stable across hosts for a fixed policy.
- **Adoption:** Most examples use the config defaults without additional code.

---

## 10) Acceptance Criteria (High‑Level)

- Process‑wide configuration is initialized once; reads are thread‑safe and fast.
- Ingest applies currency‑aware scaling; output applies currency‑aware scaling; unitless scalars use default scale unless overridden.
- All top‑level results in valuations/statements/portfolio include the rounding context.
- Property tests validate stable string serialization for a fixed policy across platforms.
- Golden tests include rounding metadata; replays match byte‑for‑byte.
- Negative tests: switching rounding mode or scales produces expected numeric deltas.
- Python and WASM bindings expose equivalent functionality with stable IO shapes.

---

## 11) Release Plan (Phased)

- **Phase A — Core policy:** Global init, rounding modes, currency scale defaults/overrides, ingest/output application, metadata stamping.
- **Phase B — Cross‑crate integration:** Ensure valuations, statements, and portfolio stamp and honor the rounding context in all outputs.
- **Phase C — UX polish:** Python/WASM examples, docs, and DataFrame export demonstrations.
- (Future) **Phase D — Extensions:** Locale formatting profiles; per‑field scale carriers; time‑based policy versioning.

---

## 12) Risks & Mitigations

- **Hidden rounding differences:** Make policy explicit, visible in metadata, and required to initialize; provide examples.
- **Numeric drift across hosts:** Default to Decimal and stable policy; include policy version in results; run golden/property tests.
- **Performance regressions:** Keep lookups O(1); benchmark IO hot paths; avoid per‑value allocations.
- **Misuse of overrides:** Restrict scoped overrides to tests/examples; document non‑production use.

---

## 13) References

- Technical design: `docs/new/10_config/10_config_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`
- Core PRD: `docs/new/02_core/02_core_prd.md`
- Related PRDs: `docs/new/03_valuations/03_valuations_prd.md`, `docs/new/04_statements/04_statements_prd.md`, `docs/new/07_portfolio/07_portfolio_prd.md`



