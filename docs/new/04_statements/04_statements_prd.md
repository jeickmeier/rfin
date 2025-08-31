### Statements (`/statements`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, data scientists, and developers
**Purpose:** Define user‑facing requirements for the Statements engine in alignment with `04_statements_tdd.md`, while remaining accessible to non‑engineers.

---

## 1) Executive Summary

The Statements engine lets users build financial statements as a graph of metrics evaluated over discrete periods (e.g., months, quarters). Users provide values, forecasts, and formulas for each metric, and the engine produces deterministic, currency‑aware results suitable for analysis, reporting, and downstream modeling. It also provides specialized real estate underwriting capabilities including property cash flow modeling, construction loan tracking, and equity waterfall allocation. Outputs are easy to consume in Python notebooks, data pipelines, and web apps.

Outcomes:
- **Deterministic numbers** across machines and runs (no drift between serial vs parallel).
- **Currency‑safe modeling** with explicit FX conversions only when requested.
- **Transparent models** with readable formulas, clear precedence (Value > Forecast > Formula), and predictable handling of missing data.
- **Real estate underwriting** with property cash flows, construction loans, and equity waterfall allocation modeling.
- **Portable results** with stable schemas and convenient DataFrame exports.

---

## 2) Goals and Non‑Goals

Goals:
- Model business metrics as nodes with values, forecasts, and formulas over a defined period plan.
- Ensure deterministic evaluation and stable ordering; parallel runs do not change results.
- Make currency behavior explicit; stamp FX policies into result metadata when used.
- Provide a small, namespaced set of built‑in metrics that users can extend.
- Support real estate underwriting workflows with property cash flows, construction loans, and equity waterfalls.
- Offer first‑class DataFrame outputs and strict, versioned wire formats for interchange.

Non‑Goals:
- Replace a general ledger/ERP system or manage journal entries.
- Price financial instruments (lives in valuations); scenarios and portfolios are separate crates.
- Ship connectors/IO formats (CSV/Parquet/Arrow live in a dedicated IO layer).

---

## 3) Target Users & Personas

- **Financial Analyst:** Builds and reviews statements, runs forecasts, and inspects standardized metrics.
- **Real Estate Analyst:** Models property cash flows, tracks construction loan performance, and analyzes equity waterfall distributions.
- **Quant/Engineer:** Integrates statements with pricing and risk models; needs determinism and performance.
- **Data Scientist (Python):** Consumes DataFrames, prototypes scenarios, and exports reports reliably.
- **Web/App Engineer (WASM):** Embeds statement previews; relies on stable JSON shapes.

---

## 4) Primary Use Cases

- **Statement modeling:** Create nodes (e.g., revenue, COGS, gross margin) with values, forecasts, and formulas.
- **Forecasting:** Apply simple deterministic methods (forward‑fill, growth %) and explicit overrides.
- **Metric standardization:** Use a namespaced registry (e.g., `fin.`) to align key metrics across models.
- **Period planning:** Define ranges (monthly/quarterly/yearly) and mark actuals versus forecast periods.
- **Real estate modeling:** Track property cash flows (rent, opex, taxes, reserves), construction loans with interest reserves, and equity waterfall allocations.
- **Analysis & reporting:** Export long/wide tables to DataFrames for dashboards, notebooks, and BI tools.

- **Balance Sheet articulation & plugs:** Automatically reconcile Assets with Liabilities + Equity using deterministic plug selection and clear metadata.
- **Corkscrew roll‑forward schedules:** Model begin→flows→end schedules (e.g., PPE, debt, inventory) with enforcement that end[t] = begin[t] + flows[t] and begin[t] = end[t‑1].

---

## 5) Scope (In/Out)

In‑Scope:
- Period planning and evaluation over periods using values/forecasts/formulas.
- Deterministic computation and stable ordering; currency‑safe arithmetic.
- Minimal built‑in forecasting methods with Decimal arithmetic.
- Metrics registry with namespacing to avoid collisions and enable convention.
- Stable serialization for model specs and results; DataFrame outputs.

- Balance Sheet articulation with deterministic plugs and explicit policies.
- Dedicated corkscrew schedule nodes for roll‑forward accounting.

Out‑of‑Scope (here):
- Instrument pricing (except real estate property valuations), scenario engines, and portfolio aggregation (separate crates).
- Implicit FX conversions; any conversion must be explicit in formulas/policies.
- File formats and connectors (handled by an optional IO layer).

---

## 6) Functional Requirements

### 6.1 Model Definition & Periods
- Users define a model ID and a period plan (e.g., `2025Q1..2026Q4`) with optional actuals marking.
- Nodes are identified by stable IDs and optional display names and tags.
- Precedence per period is enforced: **Value > Forecast > Formula**.
- Use periods from /core crate

### 6.2 Values, Forecasts, and Formulas
- Values can be currency amounts or unitless scalars; currency math requires matching currencies.
- Forecast methods include: forward‑fill, growth percentage, time series / curve growth rates, simple pick from distributions (e.g normal with mean/stdev) and explicit overrides.
- Formulas are readable expressions operating across nodes and periods; boolean `where` clauses act as masks only.
- Division by zero and missing values propagate predictably (e.g., `None`), with clear error messages.

### 6.3 Determinism & Execution
- Evaluation order is deterministic; parallel execution yields identical Decimal results.
- Content‑addressed caching may be used internally but must not alter outputs.
- Results include metadata: numeric mode, parallel flag, period plan, rounding context, and (when applicable) FX policy details.

### 6.4 Currency Safety & FX Policy
- Monetary arithmetic requires same currency; cross‑currency operations must be explicit in formulas.
- If the host requests conversion to a model currency, the applied policy (e.g., period‑end) is recorded in results metadata.

### 6.5 Metrics Registry
- Provide a small, namespaced set of built‑ins under `fin.` that can be loaded and extended.
- Namespacing avoids collisions and enables consistent naming across organizations and models.

### 6.6 Data & Interchange
- Stable serde names for wire types (model spec, nodes, forecasts, results) with unknown fields rejected.
- DataFrame exports are provided in long and wide formats with stable column names.

### 6.7 Observability & Errors
- Structured tracing around build and evaluate phases with node and period context.
- Clear, actionable error messages (parse errors, missing references, division by zero, currency mismatches).

### 6.8 Balance Sheet Articulation & Plugs
- Deterministically enforce the identity: Assets ≡ Liabilities + Equity, for each period.
- Users declare which nodes constitute each side; no implicit inclusion or FX.
- A prioritized plug list is supported (e.g., `cash`, then `other_current_assets`, then `retained_earnings`).
- The engine computes the plug per period after other nodes resolve; plug values are recorded as formulas with precedence preserved (Value > Forecast > Formula).
- If a plug node has an explicit Value for a period, the engine tries the next plug; if none remain, an error is raised with the unresolved difference.
- Results metadata records the chosen plug per period and any residuals (must be zero in Decimal mode unless a user‑set tolerance is provided).

### 6.9 Corkscrew Schedule Nodes
- Provide dedicated schedule nodes that model begin, flows (typed legs), and end for each period.
- Enforce identities: `end[t] = begin[t] + Σ flows[t]` and `begin[t] = end[t‑1]` with an explicit first‑period anchor.
- Support unitless scalars and currency `Amount`s with currency‑safety rules; no implicit FX.
- Vectorized evaluation over periods; schedules export cleanly to long/wide DataFrames (including begin/end columns if requested).
- Configurable legs (e.g., additions, disposals, depreciation, amortization, accretion) with deterministic signs.

### 6.10 Real Estate: Property Cash Flows
- Model rent rolls with step‑ups and CPI/RPI indexation (lag/interpolation, caps/floors); free‑rent windows; renewal probabilities and expected cash flows.
- Track operating expenses (fixed and % of rent or area), reimbursements/passthroughs, CAM recoveries with gross‑up policies.
- Calculate property taxes (assessed value × mill rate), exemptions and phase‑ins; optional passthroughs per lease.
- Manage capex, TI/LC and reserves: dated outflows; reserve accrual/use; policy‑driven capitalization vs expense treatment.
- Currency‑preserving period aggregation across property flows; optional explicit FX collapse stamped in metadata.

---

## 7) Non‑Functional Requirements

- **Determinism:** Decimal numerics by default; serial equals parallel.
- **Performance:** Vectorized evaluation over periods; predictable latency on reference hardware.
- **Stability:** Semver‑governed public types and schemas; snapshots gated by schema version.
- **Portability:** Rust core only; bindings (Python/WASM) consume stable wire types.
- **Security/Safety:** Closed expression set, strict deserialization, no implicit IO/network.

---

## 8) User Experience Requirements

### 8.1 Python
- Pydantic models mirror wire shapes; helpful validation errors.
- DataFrame outputs integrate with common Python workflows (pandas/Polars) and export reliably.

### 8.2 WASM
- JSON IO mirrors serde names; feature flags enable small bundles for browser demos.

### 8.3 Documentation
- Quickstarts: build a simple model, add forecasts, add formulas, export DataFrames; real estate examples for property cash flows, construction loans, and equity waterfalls.
- Policy visibility examples: FX conversion choices and rounding context in results.

---

## 9) Success Metrics

- **Determinism:** Serial and parallel runs match in Decimal mode across OS/CPU.
- **Usability:** New users can model a basic P&L and export results in <15 minutes.
- **Transparency:** Results include numeric mode, rounding, and any FX policy metadata.
- **Stability:** Schema names and column labels remain stable across minor versions.

---

## 10) Acceptance Criteria (High‑Level)

- Enforces Value > Forecast > Formula precedence per period; `where` is a mask only.
- Deterministic evaluation and stable ordering; Decimal serial equals parallel.
- Currency‑safe operations with explicit FX; applied policies recorded in outputs when used.
- Namespaced metrics registry available and extensible; collisions avoided by design.
- Long and wide DataFrame exports with stable schemas; serde unknown fields denied.

- Balance Sheet articulation enforces Assets ≡ Liabilities + Equity per period with deterministic plug selection and zero residuals (Decimal mode) unless a user‑configured tolerance is set.
- Corkscrew schedules enforce begin/end roll‑forwards per period and across periods; mismatches produce typed errors.
- Real estate underwriting: property cash flows, construction loans, and equity waterfalls model complex real estate scenarios with deterministic period‑based calculations.

---

## 11) Risks & Mitigations

- **Numeric drift:** Default to Decimal and stable ordering; parity tests for serial vs parallel.
- **Hidden FX behavior:** Require explicit conversion; stamp FX policy metadata in results.
- **Schema creep:** Enforce strict serde names, versioning, and golden tests.
- **Performance regressions:** Benchmarks on core kernels and representative models.

---

## 12) References

- Technical design: `docs/new/04_statements/04_statements_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`
- Core PRD: `docs/new/02_core/02_core_prd.md`


