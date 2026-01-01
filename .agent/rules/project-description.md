---
alwaysApply: true
---
# Finstack (Rust) — Deterministic Financial Computation Library

## Overview

Finstack is a deterministic, cross‑platform financial computation engine with a Rust core and first‑class Python and WebAssembly bindings. It emphasizes accounting‑grade correctness (Decimal numerics), currency‑safety, stable wire formats, and predictable performance for statements, valuations, scenarios, and portfolio analysis.

## Project Purpose

Finstack aims to provide:

- **Determinism**: Decimal by default; serial and parallel runs produce identical results.
- **Currency‑safety**: No implicit cross‑currency math; explicit FX policies stamped in results.
- **Stable schemas**: Strict serde names for long‑lived pipelines and golden tests.
- **Performance**: Vectorized and parallel execution without changing Decimal results.
- **Parity**: Ergonomic, parity‑checked APIs for Python and WASM.

## Architecture

```
Workspace (meta‑crate: finstack)
┌────────────────────┐
│   finstack (meta)  │  -> re‑exports subcrates via features
└─────────┬──────────┘
          │
 ┌────────┴───────────────────────────────────────────────────────────────────────────────────────┐
 │ Subcrates                                                                                      │
 │                                                                                                │
 │  core              ← primitives: types, money/fx, time (periods/calendars/day‑count),          │
 │                       expression engine, validation, config, errors; Polars re‑exports         │
 │  statements        ← model graph (Value > Forecast > Formula), vectorized evaluation           │
 │  valuations        ← cashflows, pricing, risk, period aggregation (currency‑preserving)        │
 │  scenarios         ← deterministic DSL + preview; adapters for market/statements/valuations    │
 │  portfolio         ← entities/positions/books; base‑currency rollups with explicit FX          │
 │  io                ← CSV/Parquet/Arrow interop (optional; schema‑stable)                       │
 │  py                ← Python bindings (PyO3 + Pydantic v2)                                      │
 │  wasm              ← WASM bindings (wasm‑bindgen + serde_wasm_bindgen)                         │
 └────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Cross‑Cutting Invariants

- **Determinism**: Decimal mode; stable ordering; parallel ≡ serial.
- **Currency‑safety**: Arithmetic on `Amount` requires same currency; explicit FX conversions only.
- **Rounding/Scale policy**: Global policy; active `RoundingContext` stamped into results metadata.
- **FX policy visibility**: Applied conversion strategy recorded per layer (e.g., valuations, statements, portfolio).
- **Serde stability**: Strict field names; unknown fields denied on inbound types.
- **Time‑series standard**: Polars is the canonical DataFrame/Series surface (re‑exported from core).

## Core Responsibilities (by crate)

- **core**: `Amount`, `Currency`, `Rate`; FX interfaces (`FxProvider`, `FxMatrix`); periods/calendars/day‑count; expression engine (with Polars lowering); validation; config (rounding/scale); errors; Polars re‑exports.
- **statements**: Deterministic period evaluation with precedence: **Value > Forecast > Formula**; corkscrew schedules; optional balance‑sheet articulation; long/wide DataFrame exports.
- **valuations**: Instrument cashflows, pricing, risk; currency‑preserving period aggregation; explicit FX collapse with policy stamping; private‑credit and real‑estate readiness (fees, covenants, construction loans, equity waterfalls).
- **scenarios**: DSL with quoting, selectors, and globs; deterministic preview/composition; phase‑ordered execution with precise cache invalidation.
- **portfolio**: Positions/books, period alignment, and deterministic aggregation to base currency with explicit FX.

## Language Bindings

### Python (finstack‑py)

- Wheels for major OSes; Pydantic v2 models mirror serde shapes; heavy compute releases the GIL; DataFrame‑friendly outputs.

### WebAssembly (finstack‑wasm)

- Browser/Node support; JSON IO parity with serde; feature flags for tree‑shaking and small bundles.

## Key Features

### Performance

- Vectorized execution via Polars pushdown; optional Rayon parallelism; caches for hot paths.

### Safety & Standards

- Currency type safety; strict serde; ISO‑4217 currencies; ISDA day‑count conventions; no `unsafe`.

### Policy Visibility

- Results include numeric mode, parallel flag, rounding context, and any applied FX policy.

## Primary Use Cases

- **Statements modeling**: Build/evaluate models over periods with deterministic precedence.
- **Instrument pricing & risk**: Cashflows, PV/NPV, yields/spreads, DV01/CS01, options Greeks.
- **Scenario analysis**: Deterministic DSL across market/statements/valuations with preview.
- **Portfolio aggregation**: Stable rollups by book/entity/currency with explicit FX collapse.
- **Data interchange**: Stable serde names and DataFrame outputs for pipelines and notebooks.

## Development Philosophy

1. **Correctness first**; 2. **Performance second** (without changing Decimal outputs);
2. **Ergonomic APIs**; 4. **Documentation** for every public API; 5. **Testing** across unit/property/golden/parity.

## Technical Guidelines

- Follow `.cursor/rules/[rust|python|wasm]/` standards; deny `unsafe`.
- Keep cross‑currency math explicit via `FxProvider` and record policies in results.
- Prefer compile‑time validation and strict deserialization; stable serde names.
- Use Polars for time‑series; avoid ad‑hoc series types.
- Ensure serial ≡ parallel in Decimal mode; stamp `RoundingContext` in all result envelopes.
