### Finstack (Rust) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, quants, analysts, data scientists, and developers (Python/WASM)
**Purpose:** Define user‑facing requirements for the Finstack library suite in alignment with the technical design in `01_overall_tdd.md`.

---

## 1) Executive Summary

Finstack is a deterministic, cross‑platform financial computation engine for modeling statements, pricing instruments, running scenarios, and aggregating results at portfolio scale. It provides a single Rust core with first‑class Python and WASM bindings, clear currency‑safety, predictable rounding behavior, and stable schemas suitable for pipelines, notebooks, and web apps.

Outcomes:
- **Deterministic results** across hosts and runs.
- **Currency‑safe calculations** with explicit FX policies.
- **High performance** with vectorized/parallel execution that keeps results stable.
- **Stable wire formats** and **end‑to‑end Python/WASM experiences**.

---

## 2) Goals and Non‑Goals

- **Goal: Deterministic, reproducible results** by default (Decimal numerics) across OS, CPU, and language bindings.
- **Goal: Currency‑safe computation** (no silent cross‑currency math) and explicit, visible FX conversion policies.
- **Goal: Stable schemas** and strict serialization for reliable pipelines and long‑lived golden tests.
- **Goal: High‑performance evaluation** of statements, valuations, and portfolios; parallel when requested without changing numeric outputs in default mode.
- **Goal: Simple, ergonomic bindings** for Python and WASM with parity to Rust APIs.

Non‑Goals:
- Build a full GL/ledger system.
- Provide real‑time market data connectors (inputs are host‑provided).
- Ship a GUI; Finstack is headless and integrates with downstream tools.

---

## 3) Target Users & Personas

- **Quant Developer (Rust/Python):** Builds valuation models and risk reports, integrates with curves/vols, needs determinism and speed.
- **Financial Analyst:** Creates financial statements, models credit / private equity style deals, runs scenarios, conducts analysis on multiple deal structures.
- **Analyst/Data Scientist (Python):** Prototypes scenarios, runs forecasts, exports results to DataFrames; expects transparent FX and rounding.
- **Risk/PM (Portfolio):** Aggregates positions across books/entities, runs what‑if scenarios, and expects repeatable numbers for reviews.
- **Web/App Engineer (WASM):** Embeds calculators and previews in a browser; needs stable JSON IO and small bundles via features.

---

## 4) Primary Use Cases

- **Financial statements modeling:** Define nodes (values, forecasts, formulas), evaluate over periods, and derive metrics.
- **Instrument pricing and risk:** Build schedules, price instruments, compute risk measures, and tag cashflows for aggregation.
- **Scenario analysis:** Apply deterministic, auditable shocks across statements, instruments, and market data.
- **Portfolio aggregation:** Sum positions by book/entity/currency; consistently collapse to a base currency when requested.
- **Data interchange:** Import/export models, portfolios, and results with stable schemas; interop via Polars DataFrames.

---

## 5) Scope (In/Out)

In‑Scope:
- Core numerics (Decimal default), calendars/day‑count, period planning.
- Statements engine with values/forecasts/formulas and deterministic evaluation.
- Valuation of core instrument set; currency‑preserving cashflow aggregation.
- Scenarios DSL (deterministic, composable), preview, and execution.
- Portfolio model (books, positions, base currency) and aggregation.
- Python and WASM bindings with strict serde compatibility.
- Observability: structured tracing, results metadata, and stable rounding policy.

Out‑of‑Scope (for now):
- Real‑time data connectors; external sources are provided by host.
- Rich UI; examples are console/notebooks/browser demos.
- Non‑deterministic engines (e.g., Monte Carlo) beyond documented follow‑ups.

---

## 6) Functional Requirements

### 6.1 Determinism & Reproducibility
- Default numeric mode is Decimal; serial and parallel runs produce identical results.
- Each result bundle includes metadata: numeric mode, parallel flag, seed, model/portfolio currency, and active rounding context.

### 6.2 Currency Safety & FX Policy
- All arithmetic on monetary amounts requires matching currencies.
- FX conversion requires explicit policy and provider; the applied policy is stamped in result metadata.
- Period aggregation is currency‑preserving by default; collapse to a single currency is explicit.

### 6.3 Periods, Calendars, and Day Count
- Users can define period ranges (monthly/quarterly/yearly) and mark actuals; evaluation honors ordering and identity.
- Calendar holidays for major markets.
- Business‑day conventions, EOM, and standard day‑count fractions are available and cached.

### 6.4 Statements Engine
- Nodes support values, forecasts, and formulas with a clear resolution order: Value > Forecast > Formula per period.
- Deterministic evaluation order; vectorized execution where possible; strict behavior for missing values and division by zero.
- Built‑in metrics registry with namespacing to avoid collisions and allow for standardization of node naming (revenue vs rev./total revenue/etc.).

### 6.5 Valuations
- Core instrument set: deposits/FRAs, swaps, FX spot/forwards, CDS, vanilla options, and equities (spot).
- Fixed income set: fixed / float with cash/pik/mixed coupon (optional schedules) with index floor/ceiling, amortization schedules, call schedules, fees.
- Pricing and risk outputs expose PV, greeks (when applicable), and tagged cashflows.
- Aggregation to model currency requires explicit FX; currency‑preserving totals are available at all times.

### 6.6 Scenarios DSL
- Deterministic path grammar with quoting and globs; includes assignment, percentage, basis‑point, multiplicative, and shift modifiers.
- Preview shows expansion, ordering, and conflict strategy; strict/lenient modes are supported.

### 6.7 Portfolio
- Define entities, positions, units, books, and a portfolio‑wide period plan.
- Aggregate by book/entity/currency/tags with stable reduction order; collapse to base currency on request.
- Node aliasing allows cross‑entity statement alignment to the portfolio plan.

### 6.8 Data & Interchange
- Stable serde names and schema versioning for all public types.
- Polars DataFrame outputs for tabular results; CSV/Parquet/Arrow support via a dedicated IO layer.

### 6.9 Bindings (Python & WASM)
- Python: Pydantic v2 models mirror serde shapes; heavy compute releases the GIL; wheels for major OSes and Python 3.12+
- WASM: JSON IO parity with serde; feature flags enable tree‑shaking for minimal bundles.

### 6.10 Observability
- Tracing spans around build/run/price/analyze; optional JSON logs.
- Correlation IDs (run, scenario, book) included in logs and results.

---

## 7) Non‑Functional Requirements

- **Performance:** Meet documented targets for nodes×periods and instrument pricing with caches.
- **Stability:** Public APIs and schema are semver‑governed; unknown fields denied on inbound wire types.
- **Portability:** Rust core compiles on stable toolchains; Python wheels for Linux/macOS/Windows; WASM builds for `wasm32-unknown-unknown`.
- **Security/Safety:** No `unsafe` code; closed expression language; strict deserialization; no implicit I/O or network operations.
- **Testability:** Unit, property, golden, compile‑time, and parity tests ensure correctness and determinism.

---

## 8) User Experience Requirements

### 8.1 Python UX
- Install via `uv` (wheels); `pydantic` models validate inputs with helpful messages.
- DataFrame outputs are first‑class; users can export to CSV/Parquet with stable column names.
- Clear errors that include context (node/period/path) and link to docs/examples.

### 8.2 WASM UX
- Small bundles via features; examples demonstrate statements and cashflow previews in browser.
- Errors map to readable messages in JavaScript; JSON shapes match Rust serde.

### 8.3 Documentation & Examples
- Quickstarts for statements, valuations, scenarios, portfolio; runnable end‑to‑end in Python and WASM.
- Policy visibility examples: FX conversion choices, rounding/scale policy inspection in outputs.

---

## 9) Success Metrics

- **Determinism:** Parallel vs serial runs produce identical Decimal outputs across OSes (golden tests pass in CI).
- **Performance:** Meets or exceeds target latency for statement evaluation and instrument pricing on reference hardware.
- **Adoption:** >80% of example flows runnable by new users in <15 minutes (usability tests).
- **Stability:** No breaking schema/API changes without documented semver bump and migration notes.
- **Transparency:** All result envelopes include numeric mode, FX policies, and rounding context.

---

## 10) Release Plan (Phased)

### Phase 1 (Core + Statements)
- Periods, calendars/day‑count, Decimal numerics, rounding policy, statements engine, metrics registry, Python/WASM bindings, and basic observability.

### Phase 2 (Valuations + Scenarios)
- Core instrument pricing, cashflow tagging/aggregation, scenarios DSL with preview, and FX policy stamping.

### Phase 3 (Portfolio + IO)
- Portfolio model, aggregation and alignment, Polars‑based outputs, CSV/Parquet/Arrow interop, and extended examples.

Each phase must ship with deterministic results, docs, examples, and CI coverage matching acceptance criteria.

---

## 11) Acceptance Criteria (High‑Level)

- Statements evaluate deterministically with correct Value > Forecast > Formula semantics; metrics are namespaced.
- Currency‑preserving aggregation with explicit FX collapse; FX policy is visible in outputs.
- Python and WASM bindings round‑trip inputs/outputs with stable serde names; heavy operations release GIL in Python.
- Scenario previews are deterministic; composition rules and strict/lenient modes behave as documented.
- Portfolio rollups are deterministic with stable reduction order; base currency totals match explicit FX policies.
- All results include numeric mode and rounding context; CSV/JSON outputs are stable under the same policy.

---

## 12) Risks & Mitigations

- **Numeric drift across hosts:** Use Decimal default and stable ordering; include rounding context in outputs.
- **Hidden FX assumptions:** Require explicit conversion APIs and stamp policy metadata into results.
- **Schema creep:** Enforce strict serde, semver checks, and golden file tests.
- **Performance regressions:** Criterion benchmarks and CI performance gates on core kernels.

---

## 14) References

- Technical design: `docs/new/01_overall/01_overall_tdd.md`
- Detailed crate‑level designs in `docs/new/02_core`, `03_valuations`, `05_scenarios`, `07_portfolio`, and bindings docs under `08_bindings`.


