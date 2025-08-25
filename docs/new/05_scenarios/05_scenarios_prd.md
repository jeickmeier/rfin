### Scenarios (`/scenarios`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, risk, and developers (Python/WASM)
**Purpose:** Define user-facing requirements for deterministic, auditable scenario analysis aligned with `05_scenarios_tdd.md` while remaining accessible to non-Rust users.

---

## 1) Executive Summary

Scenarios provide a simple, auditable way to run what‑if, stress, and sensitivity analyses across market data, statements, instruments, and portfolios. Users describe changes using a compact, human‑readable DSL and can preview impacts before execution. Scenarios compose deterministically, support prioritization, and integrate consistently with Valuations, Statements, and Portfolio layers.

Outcomes:
- Deterministic scenario application and repeatable previews.
- Clear, compact DSL with predictable behavior and validation.
- Composable scenarios with priority/merge controls and safe defaults.
- First‑class Python/WASM usage and stable JSON/serde wire formats.

---

## 2) Goals and Non‑Goals

Goals:
- Provide an easy‑to‑learn DSL for applying changes to market, statements, valuations, and portfolio targets.
- Offer deterministic previews that show exactly what will change and why.
- Support composition: include other scenarios, control priorities, and resolve conflicts deterministically.
- Ensure safe, reversible execution with cache invalidation and time‑window gating.
- Deliver parity in Rust, Python, and WASM with stable schemas.

Non‑Goals:
- Live market data connectivity or storage of market history.
- A full UI editor (examples provided; rich UI is out of scope).
- Non‑deterministic engines (e.g., Monte Carlo) beyond documented follow‑ups.

---

## 3) Target Users & Personas

- Quant/Risk: Defines stress templates (e.g., curve twists, credit spread shocks) and composes scenario suites.
- Financial Analyst: Adjusts statement drivers and forecasts for planning; previews and iterates quickly.
- PM/Risk Reviewer: Runs a grid of pre‑approved scenarios; compares outcomes and exports results.
- Python/WASM Developer: Embeds preview/execute flows in notebooks or the browser using stable JSON types.

---

## 4) Primary Use Cases

- Market shocks: parallel shifts, twists, FX and vol adjustments applied to pricing and risk.
- Statements adjustments: value overrides, growth rates, formula changes, and time‑scoped effects.
- Portfolio actions: position scaling/closure, book policy toggles, and exposure limits for planning.
- Composed scenarios: baseline + overlays (sector‑specific, region‑specific) with deterministic resolution.
- Preview and audit: see expanded targets, ordering, and expected impacts before commit.

---

## 5) Scope (In/Out)

In‑Scope:
- Text DSL and JSON forms with the same semantics.
- Deterministic pathing with quoting, selectors, and globs; typed modifiers (assign, percent, bp, multiply, shift, custom).
- Composition engine (priority, conflict strategies) and deterministic preview.
- Time windows (`@on`, `@during`) for effective/expiry control.
- Adapters for Market, Statements, Valuations (instruments), and Portfolio integration.
- Python/WASM parity and stable wire types.

Out‑of‑Scope:
- Real‑time data connectors and storage; inputs are provided by the host.
- UI designer; only examples and docs.

---

## 6) Functional Requirements

### 6.1 DSL & Determinism
- Paths target roots: `market`, `statements`, `valuations`, `portfolio` (with quoted segments, selectors, and globs).
- Modifiers: `:=` assign, `:+%`/`-%` percent change, `:+bp`/`-bp` basis points, `:*` multiply, `:shift` formula replace, and registered custom modifiers.
- Deterministic glob and selector expansion with configurable limits; canonical path normalization.
- Strict vs lenient modes: strict fails on unknown paths; lenient warns and skips.

### 6.2 Preview & Validation
- Preview shows: final ordering, expanded targets, truncated flags, and adapter‑level impact summaries.
- Validation reports errors and warnings with stable codes; linter offers safe canonicalizations.
- Composition visibility: preview exposes priority/conflict strategy choices applied to the plan.

### 6.3 Composition & Includes
- Include other scenarios with optional priority offsets and parameters (templates).
- Conflict handling strategies: last/first wins, merge (when meaningful), or error.
- Stable ordering by priority then declaration index; reproducible across hosts.

### 6.4 Time Gating & Rollback
- Lines may specify effective/expiry dates; engine filters by context date.
- Execution creates a checkpoint to allow rollback on failure (non‑preview modes).

### 6.5 Adapters & Phases
- Phases: MarketData → Instruments → Statements → Evaluation, with cache invalidation per phase.
- Market adapter: curve shifts (incl. twist), FX shocks, vol surface shocks.
- Statements adapter: value overrides, percent changes, formula shifts, period‑aware application.
- Portfolio adapter: position quantity scaling, closures, and basic book policy adjustments.

### 6.6 Programmatic & File‑Based Use
- Support inline strings, JSON specs, and files; stable serde names for all public types.
- Python and WASM expose identical behaviors; heavy compute is safe for Python notebook usage.

---

## 7) Non‑Functional Requirements

- Determinism: identical previews/executions for the same inputs across OSes and bindings.
- Performance: preview and execute meet documented latency targets at scale; glob expansion capped safely.
- Stability: semver‑governed wire types and error codes; unknown fields denied unless versioned.
- Security/Safety: closed expression language; strict deserialization; no implicit I/O.
- Observability: structured tracing with correlation IDs for runs and scenarios.

---

## 8) User Experience Requirements

### 8.1 Authoring UX
- Clear error messages referencing the offending path/modifier with fix hints.
- Linter suggestions for canonical quotes, currency codes, and safe glob patterns.
- Examples and templates for common stresses (parallel shift, twist, growth, close position).

### 8.2 Preview UX
- Show expanded target lists with truncation indication.
- Display ordering and conflict outcomes; explain which line “wins” and why.
- Provide concise before/after summaries per target where feasible.

### 8.3 Python/WASM UX
- Python: Pydantic models, friendly exceptions, and DataFrame‑friendly outputs.
- WASM: small bundles via features; JSON parity with serde; readable errors in JS.

---

## 9) Success Metrics

- Deterministic previews and executions across Rust, Python, and WASM (golden tests passing in CI).
- Time to first scenario: < 10 minutes using examples in Python notebooks.
- Preview at 500 operations with glob expansion returns in < 50 ms on reference hardware.
- Adoption: majority of valuation/statement examples include at least one scenario run.

---

## 10) Acceptance Criteria (High‑Level)

- DSL supports paths, selectors, and globs with canonical normalization and linter guidance.
- Preview lists expanded targets, ordering, and conflict strategy with warnings and codes.
- Composition applies deterministic priority and conflict handling; includes/parameters behave as documented.
- Time‑windowed operations filter correctly in preview and execution.
- Adapters apply supported modifiers and return impact summaries; caches invalidate per phase.
- Python/WASM parity for parsing, preview, and execution with stable JSON shapes.

---

## 11) Risks & Mitigations

- Ambiguous or overly broad globs: enforce limits, preview truncation flags, and linter warnings.
- Numeric drift: rely on Core’s Decimal defaults and stable ordering; stamp numeric mode in results.
- Schema creep: strict serde with semver and golden test coverage for wire types and error codes.
- Performance regressions: benchmarks and CI gates on preview/execute hot paths.

---

## 12) Release Plan (Phased)

- Phase A: DSL + Preview + Market/Statements adapters + basic composition.
- Phase B: Portfolio adapter + templates/parameters + richer conflict strategies.
- Phase C: Extended selectors, diff tooling, and advanced preview explanations.

---

## 13) References

- Technical design: `docs/new/05_scenarios/05_scenarios_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`
- Core PRD: `docs/new/02_core/02_core_prd.md`
