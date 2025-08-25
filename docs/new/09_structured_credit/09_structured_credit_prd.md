### Finstack Structured Credit (`/structured_credit`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, credit analysts/investors, structurers, quants, and developers (Python/WASM)
**Purpose:** Define high‑level user‑facing requirements for the Structured Credit crate (CLO/ABS), aligned with `docs/new/09_structured_credit/09_structured_credit_tdd.md`.

---

## 1) Executive Summary

Structured Credit provides a focused, deterministic engine to model, project, and value tranche‑based securitizations such as CLOs and ABS. Users define a deal (collateral pool, tranches, fees, reserve accounts, triggers, and waterfall), run period‑by‑period projections with defaults and prepayments, and obtain transparent waterfall distributions, coverage tests, and tranche valuations. The crate reuses Core and Valuations primitives to ensure currency safety, stable schemas, and performance with predictable results.

Outcomes:
- Deterministic, auditable deal projections and valuations.
- Clear, configurable waterfall rules and trigger behavior.
- Currency‑preserving cash collection and distribution with stable reporting shapes.
- First‑class Python and WASM ergonomics for analysis, scenarios, and embedding.

---

## 2) Goals and Non‑Goals

Goals:
- Enable end‑to‑end modeling of securitizations: pool definition, tranches, fees, reserve targets, waterfalls, and triggers.
- Provide deterministic pool projections with defaults, prepayments, and recoveries under simple, explainable assumptions.
- Execute waterfalls exactly as specified (priority, amounts, conditions) and surface an auditable step‑by‑step distribution log.
- Compute coverage ratios (OC/IC and related) and apply consequences (e.g., divert cash, accelerate, stop reinvestment).
- Produce per‑tranche cashflows and valuation metrics using the shared valuations layer.
- Expose stable, serde‑backed schemas and DataFrame outputs ready for analytics and reporting stacks.

Non‑Goals:
- Build a UI or deal editor; this is a headless engine with examples and bindings.
- Provide external data connectors; pools, rates, and assumptions come from the host.
- Implement non‑deterministic Monte Carlo engines in this phase.

---

## 3) Target Users & Personas

- **Structurer/PM:** Specifies deal terms (waterfalls, triggers, fees), validates coverage, and iterates on structures.
- **Credit Analyst/Investor:** Reviews tranche cashflows, coverage metrics, and scenario outcomes; compares to reference models.
- **Quant Developer (Rust/Python):** Integrates market data, builds pricing/risk reports, automates projections and scenarios.
- **Risk/Treasury:** Monitors compliance (OC/IC), reinvestment state, and reserve balances; runs periodic surveillance.
- **Data/Platform Engineer:** Embeds the engine in pipelines and apps; relies on stable schemas and deterministic outcomes.

---

## 4) Primary Use Cases

- Define a CLO/ABS deal with pool, tranches, fees, reserves, triggers, and waterfall priority.
- Project collections and distributions across periods with defaults, prepayments, and recoveries.
- Monitor and report coverage ratios (OC/IC), trigger status, and consequences (cash diversion, acceleration, etc.).
- Produce tranche‑level cashflows and value tranches using discount curves and spreads.
- Run what‑if scenarios on assumptions (default/prepay/recovery), fees, coverage thresholds, and reinvestment rules.
- Export results to DataFrames/JSON for dashboards, filings, and investor reporting.

---

## 5) Scope (In/Out)

In‑Scope:
- Deal definition: collateral pool, tranche terms (fixed/floating, deferrable), fees, reserve accounts, waterfalls, triggers.
- Assumptions: default, prepayment, and recovery models with simple curve families (constant, vectors, PSA‑style) and lags.
- Projection and distribution: period collections, trigger evaluation, reinvestment phase behavior, and post‑acceleration rules.
- Coverage metrics: OC/IC and related ratios required by triggers and disclosures.
- Valuation: tranche pricing from projected cashflows using the shared valuations API.
- Integration: scenarios adapters for structured‑product knobs; currency‑safe accounting via Core.

Out‑of‑Scope (for now):
- Complex collateral manager behavior beyond simple reinvestment spread/eligibility settings.
- Exotic waterfalls or bespoke legal clauses not expressible in standard step/condition forms.
- Live data ingestion, warehousing, or document parsing.

---

## 6) Functional Requirements

### 6.1 Deal Definition
- Users can define a `StructuredProduct` with: collateral pool, tranches (seniority, balances, coupon spec), fees, reserve accounts, triggers, and a waterfall split into interest/principal phases (and optional post‑acceleration steps).
- Coupons support fixed and floating (index + spread) with optional caps/floors and deferrable/PIK semantics under defined conditions.
- Reserve accounts define targets (fixed, % of pool, % of tranche) and funding priority within the waterfall.

### 6.2 Pool Modeling & Assumptions
- Pool assets are references to existing cashflow providers; each asset includes par, price, credit quality, industry, obligor, and status.
- Assumptions include default, prepayment, and recovery settings with simple, explainable curves and optional lags/costs.
- Reinvestment period is optional and bounded; eligibility and concentration limits can be recorded for reporting.

### 6.3 Waterfall & Triggers
- Waterfalls execute as an ordered list of steps with recipients, amount types (e.g., current interest, principal, fixed amount, percentage of available, up‑to‑target), and conditions.
- Coverage ratios (OC/IC) are computed per period and are usable as trigger inputs.
- Triggers support thresholds, cure periods, and consequences (divert cash, accelerate, trap excess spread, stop reinvestment).
- The engine produces an auditable distribution log each period, including breached triggers and remaining balances.

### 6.4 Tranche Economics & Valuation
- Tranche cashflows are produced per period (interest, deferred/paid, principal, fees), honoring deferral/capitalization rules.
- Valuation uses the shared pricing functions (e.g., NPV) with market curves/spreads provided by the caller.
- Report standard analytics: PV, WAL/WAM where applicable, and coverage trends; deterministic across runs.

### 6.5 Results & Reporting
- Outputs include: per‑tranche cashflows, coverage ratios, trigger states, reserve balances, and remaining pool/tranche balances.
- Provide DataFrame‑friendly shapes and stable serde names for all public types.
- Include result metadata: numeric mode, currency, scenario/run identifiers, and rounding/FX policy context inherited from Core.

### 6.6 Scenarios Integration
- Scenario paths allow adjusting assumptions (defaults, prepayments, recoveries), trigger thresholds, fee rates, and reinvestment flags.
- Preview and execution behave deterministically; cache invalidation aligns with overall scenario phases.

### 6.7 Interoperability & Schema
- Reuse Core types (Amount, Currency, calendars, day‑count) and Valuations traits (CashflowProvider, Priceable) to ensure parity.
- All public types are serde‑stable and versioned; unknown fields are rejected by default.

---

## 7) Non‑Functional Requirements

- **Determinism:** Projections and waterfalls are deterministic by default; parallel execution (when enabled) does not change numeric results.
- **Currency Safety:** Collections and distributions preserve currency; any FX conversions are explicit and policy‑stamped.
- **Performance:** Efficient waterfall execution and pool projection with clear performance targets and benchmarks.
- **Portability:** Works across supported Rust toolchains with parity in Python/WASM bindings.
- **Testability:** Unit, property (cash conservation, ratio monotonicity), golden, and parity tests against reference deals.
- **Stability:** Public APIs/schemas governed by semver with documented migration notes.

---

## 8) User Experience Requirements

### 8.1 Python UX
- Pydantic v2 models mirror serde shapes; validation errors are readable and point to the field/step.
- DataFrame outputs for cashflows, distributions, and coverage metrics; simple helpers to export CSV/Parquet via IO layers.
- Long‑running projections release the GIL in underlying compute where possible (via shared bindings policy).

### 8.2 WASM UX
- JSON IO parity with serde; small bundles via feature flags; examples show running a small CLO waterfall in‑browser.
- Errors surface readable messages and include step IDs, trigger IDs, and dates.

### 8.3 Documentation & Examples
- End‑to‑end examples: define a 3‑tranche CLO, run projections under base/optimistic/stress assumptions, and price tranches.
- How‑to guides: writing waterfalls, configuring triggers, modeling deferrable coupons, and interpreting coverage outputs.

---

## 9) Success Metrics

- **Determinism:** Identical outputs across OSes and serial/parallel modes under Decimal numerics (golden tests pass in CI).
- **Parity:** Tranche PVs and coverage paths match reference models within agreed tolerances on a published test set.
- **Adoption:** Analysts can run the quickstart and produce tranche cashflows in <15 minutes.
- **Stability:** No breaking schema/API changes without semver bump and migration notes.
- **Performance:** Meets target throughput for pool size × periods on reference hardware.

---

## 10) Release Plan (Phased)

### Phase 1 — MVP Waterfall & Coverage
- Deal definition, pool assumptions, waterfall execution, OC/IC calculation, reserve funding, deterministic outputs.

### Phase 2 — Tranche Valuation & Deferral
- Per‑tranche cashflows with deferrable/PIK logic; valuation via shared pricing; reporting shapes finalized.

### Phase 3 — Scenarios & Analytics
- Scenario adapters for assumptions/thresholds; WAL/WAM reporting; audit logs and DataFrame outputs.

### Phase 4 — Performance & Parity
- Benchmarks and performance tuning; parity checks vs reference deals; finalize acceptance tests.

Each phase ships with docs, examples, and CI coverage that meet acceptance criteria.

---

## 11) Acceptance Criteria (High‑Level)

- Feature‑gated crate integrates and compiles cleanly; re‑export available from the meta crate.
- Waterfall engine is deterministic, currency‑preserving, and cash‑conserving; distribution logs are auditable.
- Coverage ratios computed correctly and used by triggers; trigger consequences apply in the next applicable step.
- Tranche valuation matches reference within tolerance; results expose PV and standard analytics.
- Public types are serde‑stable; schemas documented with examples; Python/WASM round‑trip examples succeed.

---

## 12) Risks & Mitigations

- **Bespoke legal terms vary by deal:** Provide expressive but bounded step/condition model and document patterns; include escape hatches via custom recipients/amounts only under feature flags.
- **Numeric drift across hosts:** Default to Decimal numerics and stable ordering; stamp numeric mode/rounding in outputs.
- **Waterfall complexity/performance:** Build small, testable kernels and property tests; benchmark representative deals.
- **Schema creep:** Enforce strict serde names and semver governance; add migration notes on changes.

---

## 13) References

- Technical design: `docs/new/09_structured_credit/09_structured_credit_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`
- Valuations PRD/TDD for pricing and cashflow primitives



