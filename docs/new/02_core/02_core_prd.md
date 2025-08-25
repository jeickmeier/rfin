### Finstack Core (`/core`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, app engineers
**Purpose:** Define user-facing requirements for the foundational Core crate that underpins the suite. This aligns with `docs/new/02_core/02_core_tdd.md` while staying accessible to non-Rust users.

---

## 1) Executive Summary

Core is the foundation of Finstack. It provides trustworthy building blocks for financial software: currency-safe amounts, dates and calendars, day-count conventions, periods and schedules, a small but powerful expression engine, FX infrastructure, careful math routines, a consistent error model, and a single standard for time-series data. Other crates (e.g., valuations, portfolio, scenarios) build on Core to deliver end-user features. Core focuses on correctness, determinism, and portability.

Outcomes:
- Deterministic and reproducible results across machines and languages.
- Currency safety and explicit FX policies; no hidden conversions.
- Consistent time-series data model based on Polars for downstream reuse.
- Stable APIs and schemas that enable long-lived models and golden tests.

---

## 2) Goals and Non-Goals

Goals:
- Provide strong, composable primitives (Amount, Currency, Rate, Date/Timestamp, Periods).
- Make FX behavior explicit and auditable; enable multi-currency operations safely.
- Offer a small expression engine with deterministic, vectorizable functions.
- Standardize time-series around Polars for predictable performance and IO.
- Deliver numerically careful math kernels with deterministic modes.
- Expose a single, clear error type and validation framework.

Non-Goals:
- Implement cashflow logic or instrument pricing (lives in `valuations`).
- Build portfolio logic or scenario engines (separate crates).
- Ship file formats/connectors (CSV/Parquet/Arrow live in an optional IO crate).
- Provide a UI; Core is headless and library-first.

---

## 3) Target Users & Personas

- **Quant/Engineer:** Needs reliable primitives and performance to build pricing, risk, or analytics layers.
- **Financial Analyst (via higher layers):** Benefits from transparent FX rules and deterministic outcomes.
- **Data Scientist (Python):** Expects sane time-series defaults and stable schemas when exporting to DataFrames.
- **App/Platform Engineer (WASM):** Requires small, deterministic, browser-safe computation units.

---

## 4) Use Cases (Enabled by Core)

- Represent monetary amounts with explicit currency and rounding/scale policy.
- Convert currencies with a pluggable FX provider and policy metadata stamped into results.
- Plan periods (e.g., monthly/quarterly) with actual/forecast tracking; compute day-counts.
- Define business calendars and schedules for downstream instruments.
- Evaluate simple, deterministic expressions (lag, diff, rolling, ewm) across time-series.
- Run math routines (root finding, stable summation) with deterministic behavior when required.
- Build DataFrames/Series using a unified, re-exported Polars API for interoperability across crates.

---

## 5) Functional Requirements

### 5.1 Types & Currency Safety
- Monetary arithmetic requires matching currencies; cross-currency math must use explicit FX conversion.
- Rounding and scale policies are configurable and recorded in result metadata.

### 5.2 FX Infrastructure
- Pluggable FX provider interface and an efficient FX matrix for multi-currency operations.
- Policy metadata (strategy, target currency) is attached to results for transparency.
- Closure checks (A/B × B/C ≈ A/C) can be enabled to catch inconsistent feeds.

### 5.3 Time & Calendars
- Provide standard day-count conventions and accurate year-fraction calculations.
- Support common business calendars and weekend rules; allow consumer-supplied holiday sets.
- Period planning: define ranges (e.g., 2025Q1..Q4), distinguish actual vs forecast.
- Schedule helpers suitable for downstream use (e.g., coupon schedules).

### 5.4 Expression Engine
- Deterministic evaluation with a small, auditable function set (e.g., lag, lead, diff, pct_change, rolling*, ewm_mean).
- Vectorized execution backed by Polars where applicable; scalar fallback is consistent with vectorized results.
- Strict behavior for missing values and ordering; explicit time columns for time-based windows.

### 5.5 Math Kernels
- Provide numerical building blocks (root finding, stable summation, basic stats) with deterministic options.
- Avoid hidden fast-math; optional performance features never change results in deterministic mode.

### 5.6 Validation & Errors
- Single, well-scoped error enum for predictable failures.
- Composable validation trait and result type enabling domain validators in other crates.

### 5.7 Time-Series Standardization
- Re-export Polars DataFrame/Series and core expressions for consistent TS operations.
- Ensure schemas and column naming conventions remain stable across versions.

---

## 6) Non-Functional Requirements

- Determinism by default (Decimal numerics, stable ordering); parallelism does not change results.
- Performance suitable for large time-series through vectorization and optional parallel features.
- Portability: stable Rust toolchains; bindings built in higher layers maintain parity.
- Security/Safety: no implicit network/IO; strict deserialization and closed expression registry.
- Observability: structured tracing in critical paths; no noisy logging in hot loops.

---

## 7) Interoperability & Contracts

- Downstream crates (valuations, portfolio, scenarios) consume Core’s types, FX, time, and expressions.
- Public types use stable serde names; unknown fields are rejected unless versioned.
- Time-series exchanged as Polars objects to avoid ad-hoc series abstractions.

---

## 8) Documentation & Examples

- Clear guides for: amounts and rounding, FX policies, period planning, day-counts, calendars, and expression functions.
- Examples show how Core primitives are used by higher-level crates to produce end-user features.

---

## 9) Acceptance Criteria (High-Level)

- Currency-safe arithmetic with enforced currency matching and explicit FX conversions.
- FX policies are visible in outputs (strategy, target currency, notes) and are testable.
- Period planning and day-count calculations match published standards and golden tests.
- Expression engine produces identical results in vectorized and scalar modes for supported functions.
- Polars is the canonical time-series surface; examples compile and run end-to-end.
- Error messages are clear and stable; validation integrates cleanly in consumer crates.

---

## 10) Out-of-Scope Confirmation

- No cashflow/valuation algorithms in Core.
- No portfolio/scenario orchestration in Core.
- No Arrow/Parquet/CSV integrations in Core (lives in an optional IO crate).

---

## 11) References

- Technical design: `docs/new/02_core/02_core_tdd.md`
- Overall PRD: `docs/new/01_overall/01_overall_prd.md`


