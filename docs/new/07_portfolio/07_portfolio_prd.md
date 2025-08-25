### Portfolio (`/portfolio`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, portfolio managers, risk, analysts, quants, and developers (Python/WASM)
**Purpose:** Define high‑level, user‑facing requirements for the Portfolio layer that organizes entities and positions, aggregates values and statements, and exposes portfolio‑level risk and scenario workflows. This aligns with `docs/new/07_portfolio/07_portfolio_tdd.md` and the overall PRD.
s
---

## 1) Executive Summary

The Portfolio layer lets users model real‑world portfolios: entities (companies/funds), positions in instruments, and hierarchical books. It provides deterministic, currency‑safe aggregation of valuations, statements, and risk across positions and entities with clear FX policies and period alignment. Results are reproducible across runs/hosts and available as stable tables for analysis in Python/WASM.

Outcomes:
- **Deterministic rollups** by book/entity/tag with stable reduction order.
- **Currency‑safe aggregation** with explicit FX conversion to a portfolio base currency.
- **Aligned statements** across entities with different reporting cadences.
- **Scenario‑aware evaluations** with cache‑efficient reruns and transparent policies.

---

## 2) Goals and Non‑Goals

Goals:
- Model portfolios with entities, positions (units, lifecycle, tags), books, and a period plan.
- Aggregate valuations, statements, and risk at position, book, entity, and portfolio levels.
- Enforce currency safety; FX conversion is explicit and documented in results.
- Provide deterministic results with visible metadata (numeric mode, FX policy, base currency).
- Offer first‑class Python and WASM experiences with stable schemas and DataFrame outputs.

Non‑Goals:
- Real‑time trade capture, OMS/EMS, or execution management.
- Regulatory or statutory reporting packages (can be built on top).
- Market data connectivity (market inputs are supplied by the host environment).
- Accounting/GL ledger functionality.

---

## 3) Target Users & Personas

- **Portfolio Manager / Risk:** Needs reliable, reconcilable totals, rollups by strategy, and what‑if scenarios.
- **Financial Analyst:** Aggregates statements across entities, aligns periods, and compares contribution.
- **Quant / Developer:** Integrates pricing/risk, builds custom groupings, and automates pipelines.
- **Data Scientist (Python):** Consumes stable tables for analysis/visualization; expects reproducibility.
- **App Engineer (WASM):** Embeds portfolio previews with small, deterministic JSON interfaces.

---

## 4) Primary Use Cases

- **Position tracking:** Represent holdings with units (shares, notional, face, percentage) and lifecycle dates.
- **Book hierarchy:** Organize positions into nested books/folders for reporting and controls.
- **Valuation rollups:** Price instruments and aggregate values from positions to books to portfolio.
- **Statement aggregation:** Align and sum entity statements (flows, stocks, ratios) to the portfolio plan.
- **Risk aggregation:** Combine per‑position risk buckets (e.g., DV01) into portfolio‑level reports.
- **Scenario analysis:** Apply portfolio‑specific shocks and inspect deterministic outcomes.
- **Tag‑based grouping:** Roll up by sector, rating, strategy, or arbitrary user tags.

---

## 5) Scope (In / Out)

In‑Scope:
- Portfolio data model: entities, positions, books, tags, and portfolio‑wide periods/base currency.
- Aggregation engines: valuations, statements (with aliases), and risk rollups.
- Explicit FX policies for valuation and statements; policy metadata is stamped in results.
- Period alignment rules for flows/stocks/ratios; node aliasing across entities.
- Scenario integration (optional): portfolio paths and deterministic application order.
- Stable IO: Polars DataFrame outputs; Python/WASM bindings with serde‑compatible shapes.

Out‑of‑Scope (current release):
- Live market data feeds; connectors are provided externally.
- Real‑time PnL streaming; this layer is batch‑oriented by design.
- Regulatory templates and accounting ledgers.

---

## 6) Functional Requirements

### 6.1 Portfolio Modeling
- Users can define a portfolio with a unique ID, base currency, valuation date, and a period plan.
- Entities include references to statement models and capital structures (optional).
- Positions capture instrument ID, quantity, unit, lifecycle dates, cost basis (optional), and tags.
- Book hierarchy supports nested folders and leaf references to positions; traversal order is stable.

### 6.2 Currency Safety & FX Policy
- Monetary arithmetic requires matching currencies; cross‑currency aggregation requires explicit FX.
- Users choose FX conversion policies for valuations and statements; applied policy is visible in results.
- Position‑level values are preserved by currency; base‑currency totals are an explicit projection.

### 6.3 Valuation Aggregation
- All active positions can be valued using the pricing engine; results aggregate deterministically.
- Outputs include per‑position values by currency, base‑currency projections, book totals, and portfolio total.
- DataFrame exports are available for positions (currency‑preserving and base) and book totals.

### 6.4 Statement Aggregation & Period Alignment
- Portfolio provides a period plan (e.g., quarters); entities may report monthly/quarterly/yearly.
- Defaults: flows sum, stocks take last, ratios average; node‑level overrides are supported.
- Node aliasing maps entity‑specific nodes to canonical portfolio nodes for cross‑entity rollups.
- Outputs provide per‑node, per‑period values in base currency, with both long and wide table forms.

### 6.5 Risk Aggregation
- Reduce position‑level risk reports into stable portfolio‑level buckets (e.g., curve DV01).
- Expose DataFrame exports for bucketed risk.

### 6.6 Scenario Integration
- Portfolio supports deterministic, auditable scenario application across portfolio attributes and delegated paths.
- Scenario execution does not change numeric mode or ordering; results include scenario metadata.

### 6.7 Results & IO
- Every run includes metadata: numeric mode, parallel flag, base currency, FX policies, and rounding context.
- Tables are stable and schema‑versioned; CSV/Parquet/Arrow interop is supported via the IO layer.
- Python/WASM bindings round‑trip portfolio inputs and results with consistent serde names.

---

## 7) Non‑Functional Requirements

- **Determinism:** Serial and parallel runs produce identical Decimal results by default.
- **Performance:** Parallel valuation across positions; caching for statements, valuations, and risk where applicable.
- **Stability:** Backward‑compatible schemas and semver governance; unknown fields rejected by default.
- **Portability:** Rust core with Python wheels and WASM builds; no hidden host dependencies.
- **Observability:** Structured tracing and result metadata; clear, contextual error messages.

---

## 8) User Experience Requirements

### 8.1 Python
- Install via `uv`; Pydantic models validate and give precise errors for portfolio shapes.
- First‑class DataFrame outputs; consistent column names for joins and exports.

### 8.2 WASM
- Lightweight bundles through feature flags; JSON IO mirrors Rust serde.
- Errors map to readable messages; deterministic previews render quickly.

### 8.3 Documentation & Examples
- End‑to‑end examples: build a portfolio, run valuations/statements/risk, export to DataFrames.
- Clear guidance on FX policy choices and period alignment strategies.

---

## 9) Success Metrics

- **Deterministic rollups:** Serial vs parallel runs match across OSes in golden tests.
- **Currency transparency:** Results include FX policy metadata; no silent cross‑currency math.
- **Usability:** New users can build and evaluate a sample portfolio end‑to‑end in <15 minutes.
- **Performance:** Meets target throughput for 1k+ positions with caching enabled on reference hardware.
- **Stability:** No breaking schema changes within a minor version; round‑trip bindings parity maintained.

---

## 10) Acceptance Criteria (High‑Level)

- Users can define entities, positions, and books; validation surfaces missing references.
- Valuation outputs include per‑position currency‑preserving values and base‑currency totals.
- Statement aggregation aligns periods with documented defaults and supports node overrides/aliases.
- Risk reports aggregate deterministically into stable buckets.
- Results include numeric mode, base currency, FX policies, and rounding context.
- Python and WASM bindings expose equivalent capabilities and stable IO shapes.

---

## 11) Risks & Mitigations

- **Hidden FX assumptions:** Require explicit policies; stamp them in results and examples.
- **Numeric drift or nondeterminism:** Enforce Decimal mode and stable reduction order; test serial vs parallel.
- **Schema creep:** Version schemas; deny unknown fields; maintain golden files for tables.
- **Performance regressions:** Benchmark position valuation and aggregation; introduce caching with limits.

---

## 12) References

- Technical design: `docs/new/07_portfolio/07_portfolio_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`
- Core/Valuations/Scenarios PRDs and TDDs under `docs/new/02_core`, `03_valuations`, `05_scenarios`


