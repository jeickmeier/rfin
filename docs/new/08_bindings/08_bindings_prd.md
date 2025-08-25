### Bindings (Python & WASM) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, data scientists, web engineers
**Purpose:** Define user‑facing requirements for the Python and WASM bindings that expose the Finstack Rust library, aligned with `08_bindings_tdd.md` and overall product direction.

---

## 1) Executive Summary

The bindings packages (`rfin-python`, `rfin-wasm`) make Finstack’s Rust capabilities accessible and natural to Python and JavaScript/TypeScript users. They are reuse‑first: they do not implement new financial logic; they orchestrate and expose existing functionality from the Rust crates with idiomatic, predictable APIs, stable schemas, and cross‑platform distribution.

Outcomes:
- **Idiomatic developer experience** in Python and JavaScript/TypeScript for core workflows (statements, valuations, scenarios, portfolio).
- **Stable, transparent data models** that round‑trip cleanly via JSON and Pydantic/TypeScript definitions.
- **Deterministic, currency‑safe results** that match the Rust core across hosts and runs.
- **High performance** with minimal overhead, releasing the GIL (Python) and supporting tree‑shakable WASM builds for small bundles.

---

## 2) Goals and Non‑Goals

- **Goal: Reuse‑first surface area.** Bindings expose existing Rust functionality and composition patterns without re‑implementing algorithms.
- **Goal: Idiomatic APIs.** Python feels Pythonic; JS/TS feels like modern ESM/CJS—simple constructors, clear methods, predictable errors.
- **Goal: Stable wire formats.** 100% serde coverage with strict, versioned schemas for long‑lived pipelines and browser apps.
- **Goal: Determinism & currency safety.** Results match Rust core; no silent cross‑currency operations; explicit FX.
- **Goal: Cross‑platform delivery.** Python wheels for major OS/arch; WASM targets for web, Node.js, and bundlers.
- **Goal: Great docs & examples.** Quickstarts and end‑to‑end examples for the major workflows.

Non‑Goals:
- Implement new pricing/analytics algorithms in Python/JS.
- Build a UI. Examples are minimal and meant for learning.
- Ship market data connectors; data is provided by the host environment.

---

## 3) Target Users & Personas

- **Analyst/Data Scientist (Python):** Builds statements, runs forecasts and scenarios, exports to DataFrames. Values friendly errors, validation, and deterministic numbers.
- **Quant Developer (Python/Rust):** Prices instruments, computes risk, and automates runs; expects performance and schema stability.
- **Portfolio/Risk (PM team):** Aggregates positions and views consistent, reproducible totals across scenarios.
- **Web/App Engineer (WASM):** Embeds previews and calculators; requires small bundles, stable JSON IO, and TypeScript types.

---

## 4) Primary Use Cases

- **Statements modeling:** Define nodes (values/forecasts/formulas), evaluate across periods, and inspect metrics.
- **Instrument pricing & risk:** Price a core instrument set, retrieve PV/greeks, and consume tagged cashflows.
- **Scenario analysis:** Apply deterministic shocks to models and market data; preview and run consistently.
- **Portfolio evaluation:** Build entities/positions, align to a period plan, aggregate by book/entity/currency.
- **Data interchange:** Round‑trip models, portfolios, market data, and results with stable, versioned schemas.

---

## 5) Scope (In/Out)

In‑Scope:
- Python and WASM packages that wrap the Rust crates for core workflows.
- Feature‑gated modules: statements, valuations, scenarios, portfolio (enabled as needed).
- Validation and data modeling aligned to serde (Pydantic v2 for Python; TypeScript declarations for WASM).
- Error mapping to readable, actionable messages.
- Distribution for major platforms and environments (wheels and WASM targets).

Out‑of‑Scope (initial releases):
- Real‑time data connectors and streaming transports.
- Non‑deterministic simulation engines beyond documented roadmap.
- Rich GUI components.

---

## 6) Functional Requirements

### 6.1 Python Bindings (`rfin-python`)
- **Pythonic API surface:** Simple constructors, classmethods for advanced cases, clear property/getter patterns.
- **Validation parity:** Pydantic models mirror Rust serde shapes; unknown fields are rejected; helpful validation messages.
- **Deterministic results:** Numeric mode, parallel flag, FX policy, and rounding context are available in results metadata.
- **Currency safety:** Monetary arithmetic requires matching currencies; FX conversion is explicit.
- **High‑level facades:** Provide orchestration helpers (e.g., portfolio runs) that compose existing Rust crates without duplicating logic.
- **Compatibility:** Wheels for supported Python versions and platforms; installation via standard Python tooling.

### 6.2 WASM Bindings (`rfin-wasm`)
- **Modern module formats:** ESM and CJS imports; Node.js and browser targets.
- **TypeScript support:** Accurate `.d.ts` declarations for all public exports.
- **JSON IO parity:** `serde-wasm-bindgen`‑compatible serialization for complex objects; stable field names.
- **Tree‑shakable features:** Optional modules (statements/valuations/scenarios/portfolio) for minimal bundles.
- **Readable errors:** Exceptions include helpful messages that mirror Rust errors.

### 6.3 Shared Requirements
- **Stable wire schemas:** 100% serde coverage; versioned envelopes; strict inbound parsing.
- **Determinism:** Serial and parallel runs yield identical results in default numeric mode.
- **Documentation:** Quickstarts and examples for each major workflow, per language.
- **Interoperability:** Parity of inputs/outputs with Rust; results designed for DataFrame/tabular export.

---

## 7) Non‑Functional Requirements

- **Performance:** Minimal overhead vs. Rust; heavy computations release the GIL (Python). WASM supports batch calls to minimize boundary crossings. Core WASM bundle for base features targets < 500KB.
- **Portability & Distribution:**
  - Python: Wheels for Linux/macOS/Windows; x86_64 and aarch64 where feasible.
  - WASM: Web, Node.js, and bundler targets; side‑effect‑free packaging to enable tree‑shaking.
- **Stability:** Semver‑governed public APIs and schemas; unknown inbound fields denied by default.
- **Security & Safety:** No implicit I/O; strict deserialization; safe execution model consistent with browser and server environments.
- **Quality:** Unit, parity, and golden tests validate round‑trips, determinism, and schema stability across bindings.

---

## 8) User Experience Requirements

### 8.1 Python UX
- Install via standard tooling; imports are discoverable and consistent (e.g., `rfin.currency`, `rfin.valuations`).
- Clear exceptions with context (node/period/path) and links to docs/examples.
- Pydantic validation errors are concise, pointing to the exact field and expected shape.
- DataFrame‑friendly outputs with stable column names for CSV/Parquet/Arrow export.

### 8.2 JavaScript/WASM UX
- Small, feature‑scoped bundles; works in browser and Node.js.
- Errors are readable and consistent with Python and Rust messages.
- TypeScript autocompletion is accurate; examples run in common toolchains (Vite/webpack).

### 8.3 Documentation & Examples
- Quickstarts for statements, valuations, scenarios, and portfolio in both languages.
- End‑to‑end examples that can be executed locally or in the browser.
- Reference guides mapping common tasks to underlying Rust concepts.

---

## 9) Success Metrics

- **Determinism:** Serial vs. parallel runs match across OSes and bindings (golden tests pass in CI).
- **Performance:** Python wrappers add negligible overhead; WASM core bundle for base features < 500KB and loads quickly in sample apps.
- **Adoption:** A new user can complete a quickstart in < 15 minutes and accomplish an end‑to‑end portfolio run in < 30 minutes.
- **Stability:** Zero breaking schema changes without a documented semver bump and migration notes.
- **Documentation:** Example flows verified in CI; docs link directly from common error messages.

---

## 10) Release Plan (Phased)

- **Phase 1 — Core + Statements:** Currency/money, dates/periods/day‑count, statements modeling, validation, and basic quickstarts.
- **Phase 2 — Valuations + Scenarios:** Core instrument pricing and risk outputs; scenarios DSL preview and execution.
- **Phase 3 — Portfolio + IO:** Portfolio model and aggregation; DataFrame/CSV/Parquet interoperability; expanded examples.
- Each phase ships with deterministic behavior, docs, and CI coverage matching acceptance criteria.

---

## 11) Acceptance Criteria (High‑Level)

### Python
- All core types and primary workflows exposed via an idiomatic API.
- Pydantic round‑trip parity with Rust serde shapes; unknown fields rejected.
- Heavy computations release the GIL; observable performance comparable to Rust.
- Optional modules are feature‑gated; wheels published for supported versions and platforms.

### WASM
- JavaScript‑friendly constructors and methods; TypeScript declarations generated.
- Complex types serialize/deserialize via `serde-wasm-bindgen` without loss.
- Build outputs for web, Node.js, and bundlers; core feature bundle size target < 500KB.
- Cross‑browser and Node.js parity tests pass.

### Shared
- 100% wire format stability and visible schema versioning.
- Currency safety enforced and FX policies explicit.
- Deterministic results preserved (serial = parallel) under default numeric mode.
- Helpful, user‑oriented error messages across languages.
- Documentation includes runnable quickstarts and end‑to‑end examples.

---

## 12) Risks & Mitigations

- **Numeric drift across hosts:** Default to deterministic numerics; stamp numeric mode and rounding context; golden tests in CI.
- **Hidden FX assumptions:** Require explicit conversion and surface policy metadata alongside results.
- **Schema creep:** Enforce strict serde, semver checks, and golden file diffs.
- **Package matrix complexity:** Automate builds with a CI matrix for OS/arch/targets; pre‑publish smoke tests.
- **Boundary performance (WASM/Python):** Encourage batch operations; document best practices; benchmark regularly.

---

## 13) References

- Technical design: `docs/new/08_bindings/08_bindings_tdd.md`
- Overall PRD/TDD: `docs/new/01_overall/01_overall_prd.md`, `docs/new/01_overall/01_overall_tdd.md`
- Related designs: `docs/new/02_core`, `03_valuations`, `05_scenarios`, `07_portfolio`


