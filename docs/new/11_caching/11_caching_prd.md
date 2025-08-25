### Caching & Hashing — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, platform engineers, and library authors
**Purpose:** Define user‑facing requirements for deterministic, content‑addressed caching and hashing across the finstack layers (curves, schedules, expressions, statements, valuations, scenarios, portfolio), aligned with `11_caching_tdd.md`.

---

## 1) Executive Summary

Caching and hashing make repeated computations fast, safe, and reproducible. We use canonical, content‑addressed keys to cache results per domain (e.g., curve grids, schedules, compiled expressions, pricing kernels, portfolio rollups). Keys change only when relevant inputs change, enabling precise invalidation and high hit rates in iterative workflows (scenario loops, preview → run). The system is concurrency‑safe (single‑flight for cache misses), deterministic across OS/toolchains, and observable: results carry stamped hashes and developers can explain any cache entry.

Key outcomes:
- **Deterministic reuse:** Identical inputs produce identical keys and values across hosts and runs.
- **Precise invalidation:** Changes only invalidate what depends on those changes; no broad, timestamp‑based busting.
- **Concurrency safety:** Parallel requests compute a missing key once, with waiters.
- **Reproducibility & auditability:** Results include hashes of key policies and plans; cache entries are explainable.

---

## 2) Goals and Non‑Goals

Goals:
- **Content‑addressed caching:** Canonical binary hashing with domain tags and schema versions (e.g., "Curve/1").
- **Domain partitioning:** Independent caches for curves, schedules, expressions, statements, valuations, scenarios, portfolio aggregation.
- **Single‑flight concurrency:** Avoid thundering herd; one computation per missing key with waiters.
- **Determinism:** Decimal mode yields byte‑identical results serial vs parallel; cross‑platform hash stability.
- **Observability:** Stamp hashes into results; provide metrics and an "explain cache entry" API.
- **Configurability:** Per‑domain capacity and policy knobs with safe defaults.
- **Testability:** Golden/property tests for hash stability, invalidation correctness, and parallel safety.

Non‑Goals:
- Building a distributed/external cache service (in‑process caches only in this scope).
- Using TTL/time‑based heuristics for correctness (eviction is for hygiene, not validity).
- Host‑dependent encodings (e.g., locale‑specific number formatting) or reliance on process‑global mutable state.

---

## 3) Target Users & Personas

- **Analyst/Quant (Python/Rust):** Expects speed‑ups in scenario and pricing loops; reproducible results between previews and final runs.
- **Portfolio/Risk (PM team):** Needs consistent rollups across many positions and scenarios without recomputation churn.
- **Platform/Library Engineer:** Implements and tunes domain caches, investigates misses, and explains entries.
- **QA/Release Engineer:** Verifies cross‑platform determinism and controls cache clearing in CI and staging.

---

## 4) Primary Use Cases

- **Scenario iterations:** Re‑evaluate models as shocks change; unchanged parts hit cached plans/compilations/pricing kernels.
- **Portfolio aggregation:** Reuse book/position rollups across scenario sweeps and report filters.
- **Market data tweaks:** Small curve edits invalidate only affected keys and downstream dependents.
- **Model editing:** Changing a statement or expression causes targeted recompilation; unrelated nodes remain cached.
- **Cross‑host parity:** Hashes and results match on Linux/macOS/Windows for the same canonical inputs.
- **Debug & audit:** Inspect why an entry hit/missed and which inputs formed its key.

---

## 5) Scope (In/Out)

In‑Scope:
- Canonical content hashing (BLAKE3) with domain separation and versioning.
- Per‑domain caches for: curves, schedules/accruals, expressions, statements blocks, valuation kernels, scenario plans, portfolio rollups.
- Precise invalidation by dependency content; conservative/full clear modes.
- Concurrency single‑flight for miss computations.
- Results metadata stamping for reproducibility (e.g., `period_plan_hash`, `market_view_hash`).
- Observability: metrics, debug logs, and an "explain cache entry" API.
- Configuration: capacity limits (count/approx bytes), invalidation policy knobs, test/audit flags.

Out‑of‑Scope (initial):
- External/distributed caches and persisted caches across processes.
- Real‑time streaming invalidation from external sources.
- Speculative precomputation and background warmers.

---

## 6) Functional Requirements

### 6.1 Canonical Content Hashing
- Use a cryptographic, fast hash (BLAKE3).
- Prefix all hashed payloads with domain tag and version (e.g., `b"Schedule/1"`).
- Binary canonicalization rules: Decimal as `(mantissa_i128, scale_u32)`; dates as epoch days; enums as discriminant + fields; ordered vs set‑like collections handled deterministically; UTF‑8 strings with length prefixes.
- Include relevant run‑level policies (e.g., `NumericMode`, rounding context, FX policy meta, `MarketData.as_of`) when they affect value.

### 6.2 Domain Keys (What goes into the hash)
- **Curves:** `(curve_id, currency, pillars[], rates[], interpolation, compounding, day_count, provenance_ops[], as_of)`.
- **Schedules/Accruals:** `(instrument_id, schedule_params, calendar_version, bdc, eom_rule, stub_type)`; accrual fractions keyed by `(start_date, end_date, day_count_variant, impl_version)`.
- **Expressions:** normalized formula text, function registry version, referenced node table, lowering flags → compiled DAG/plan.
- **Statements:** `(model_id, node_id, period_block, compiled_expr_hash, period_plan_hash, rounding_context_hash)` with stable ordering.
- **Valuations:** schedule keys + instrument params + referenced market subset + numeric/FX policy hashes.
- **Scenarios:** normalized DSL + deterministic expansion (globs/selectors) + priorities + conflict strategy + strict/lenient + includes resolved; sort where orderless.
- **Portfolio:** `(portfolio_layout_hash, period_plan_hash, market_view_hash, scenario_plan_hash?, rounding_context_hash, fx_policy_hash)`.

### 6.3 Invalidation & Eviction
- Default behavior is content‑addressed invalidation (key changes → miss). Provide targeted eviction by dependency id (e.g., curve ids, instrument ids, node ids).
- Modes: **precise**, **conservative** (phase‑level), and **full clear**.
- Eviction is bounded by capacity (count/approx bytes) per domain.

### 6.4 Concurrency & Single‑Flight
- For any missing key requested by multiple threads, compute exactly once and wake waiters.
- Sharded, bounded caches per domain.

### 6.5 Results Metadata (Reproducibility)
- Stamp into top‑level results when applicable: `period_plan_hash`, `scenario_plan_hash`, `market_view_hash`, `portfolio_layout_hash`, `rounding_context_hash`, `fx_policy_hash`, alongside `numeric_mode` and `parallel` flags.

### 6.6 Observability & Metrics
- Debug logs for cache events: domain, short key, dependency short‑hashes, `compute_time_ms`, hit/miss.
- Per‑domain counters/gauges: size, hits, misses, evictions, single‑flight waiters.
- Provide an **explain API** that returns the dependency list and canonical inputs used for a cache key.

### 6.7 Configuration
- Per‑domain capacities (entries and approximate bytes) with sensible defaults.
- Invalidation policy knobs (precise/conservative/full).
- Hash audit mode (tests only): record canonicalized input bytes alongside hashes for diffing.

---

## 7) Non‑Functional Requirements

- **Determinism:** Decimal mode yields byte‑identical values serial vs parallel; do not include the `parallel` flag in hashes.
- **Cross‑platform stability:** Hashes for identical inputs are equal on Linux/macOS/Windows.
- **Performance:** High hit rates on iterative workflows; single‑flight eliminates duplicate work; minimal overhead vs direct computation in hot paths.
- **Safety & Correctness:** No reliance on process‑global mutable state; no implicit I/O or host‑dependent formatting.
- **Stability & Versioning:** Domain tags carry version bytes; bump on canonical input shape changes.
- **Security/Privacy:** Hashes encode canonicalized inputs; no sensitive plaintext embedded in logs by default (debug gating applies).

---

## 8) User Experience Requirements

- **Transparency:** Results include stamped hashes and policy metadata; bindings expose these fields in Python/WASM.
- **Explainability:** Developers can call an explain API to see why an entry hit/missed and which inputs formed its key.
- **Operator controls:** Programmatic methods to clear per‑domain caches; optional full clear for tests.
- **Docs & Examples:** Guidance on configuring capacities, reading cache metrics, interpreting stamped hashes, and using explain APIs.

---

## 9) Success Metrics

- **Determinism:** 100% equality (values and hashes) for identical inputs across OSes and serial vs parallel runs in Decimal mode.
- **Concurrency:** Zero duplicate computations for the same missing key under parallel load (validated by counters).
- **Reuse:** Scenario/portfolio sample workloads achieve ≥ 80% cache hit rate after warmup.
- **Performance:** P95 wall‑time improvement ≥ 2× on representative scenario loops vs caching disabled.
- **Observability:** Explain API returns canonical inputs for ≥ 95% of domain entries; metrics available per domain.

---

## 10) Release Plan (Phased)

- **Phase A — Foundations:** Canonical hashing library (BLAKE3), domain/version tagging, per‑domain cache scaffolding (curves, schedules), basic metrics, and config. Stamp hashes in results metadata.
- **Phase B — Compute Surfaces:** Extend caching to expressions, statements, and valuation kernels. Implement single‑flight and targeted invalidation. Add golden/property tests.
- **Phase C — Orchestration Layers:** Scenario plan and portfolio aggregation caching; explain API; conservative/full clear modes; cross‑platform hash stability in CI.

Each phase ships with docs, examples, and acceptance tests aligned with this PRD and `11_caching_tdd.md`.

---

## 11) Acceptance Criteria (High‑Level)

- Canonical hashing produces stable keys across OS/toolchains for identical canonical inputs.
- Domain caches implemented for curves, schedules, expressions, statements, valuations, scenarios, and portfolio aggregation.
- Single‑flight ensures exactly one compute per missing key under concurrent access.
- Targeted invalidation by dependency id works as specified; conservative/full clear modes available.
- Results include `period_plan_hash`, `scenario_plan_hash`, `market_view_hash`, `portfolio_layout_hash`, `rounding_context_hash`, and `fx_policy_hash` where applicable, alongside `numeric_mode` and `parallel` flags.
- Explain API returns dependency lists and canonical inputs for a given cache entry.
- CI golden/property tests validate: scenario reordering equivalence, glob determinism, unrelated shock isolation, parallel safety, and cross‑platform hash stability.

---

## 12) Risks & Mitigations

- **Numeric drift/platform variance:** Use Decimal mode by default for determinism; stable reduction orders; stamp numeric policy; golden tests in CI.
- **Over‑invalidation or missed invalidation:** Encode precise dependencies in keys; provide targeted evictions and audit tooling; add property tests.
- **Memory pressure:** Bounded caches with capacity controls; domain sharding; metrics and alerts for evictions.
- **Scope creep (distributed cache):** Keep distributed/persistent caches out of initial scope; re‑evaluate with explicit design later.
- **Debug data exposure:** Guard detailed input bytes behind test/audit flags; redact sensitive fields in logs.

---

## 13) References

- Technical design: `docs/new/11_caching/11_caching_tdd.md`
- Overall requirements: `docs/new/01_overall/01_overall_prd.md`, `docs/new/01_overall/01_overall_tdd.md`
- Related designs: `docs/new/02_core`, `03_valuations`, `05_scenarios`, `07_portfolio`, `08_bindings`



