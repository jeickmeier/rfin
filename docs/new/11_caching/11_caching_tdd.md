# Caching & Hashing — Technical Design

**Status:** Implementation-ready
**Audience:** Library authors and maintainers (Core/Statements/Valuations/Scenarios/Portfolio)

---

## 1) Goals and Non‑Goals

### 1.1 Goals

- Deterministic, content‑addressed caching across layers (curves, schedules, expressions, statements, valuations, portfolio aggregation).
- Precise invalidation by dependency content, not timestamps or pointers.
- Concurrency‑safe, single‑flight computation in parallel runs (Rayon), with stable results in Decimal mode.
- Observability: cache keys, dependency hashes, and policy hashes stamped into results for reproducibility.

### 1.2 Non‑Goals

- No reliance on process‑global mutable state for cache correctness.
- No host‑dependent string encodings in hashing (e.g., locale‑sensitive number formats).

---

## 2) Architecture Overview

Caching is domain‑partitioned. Each domain defines:

- A canonical input shape (what the computation depends on)
- A domain tag and version (e.g., `"Curve/1"`)
- A stable content hash over the canonical input
- A bounded, concurrency‑safe cache keyed by the content hash

Domains (non‑exhaustive):

- Curves: discount/forward/credit interpolation grids, survival probabilities
- Schedules: coupon/payment dates, accrual fractions
- Expressions: compiled DAGs, lowered Polars execution plans
- Statements: per‑node, per‑period vectorized blocks
- Valuations: instrument schedules and intermediate pricing kernels
- Scenarios: normalized + expanded plan (globs/selectors resolved) and composition outcome
- Portfolio: book/position rollups; aggregated per‑book/portfolio outputs

---

## 3) Canonical Content Hashing

### 3.1 Requirements

- Use a cryptographic, fast hash (BLAKE3).
- Domain separation: prefix each hashed payload with a domain tag and schema version (e.g., `b"Schedule/1"`).
- Canonical binary encoding; do not rely on textual serialization or map iteration of non‑deterministic maps.
- Stable across OS/toolchains; no floating‑point string conversions.

### 3.2 Canonicalization Rules

- Decimal: encode `(mantissa_i128, scale_u32)` in little‑endian.
- Currency: encode ISO‑4217 numeric id or canonical 3‑letter uppercase code as fixed‑width bytes.
- Date: days since Unix epoch (UTC) as `i32` little‑endian.
- Enums: one‑byte discriminant + canonical fields.
- IndexMap/Vec: if order carries semantics, hash in order; if representing sets (e.g., selector expansion), sort by canonical key first.
- Optional: presence byte then value.
- Strings: UTF‑8 bytes preceded by `u32` length.
- Include run‑level policy inputs when they change values: `NumericMode`, rounding context (see Config), FX policy meta, `MarketData.as_of`, portfolio period plan hash, etc.

### 3.3 Example (illustrative)

```rust
pub struct Hasher(blake3::Hasher);

pub trait HashCanon { fn hash_into(&self, h: &mut Hasher); }

impl HashCanon for rust_decimal::Decimal {
    fn hash_into(&self, h: &mut Hasher) {
        h.0.update(b"D/1");
        let (m, s) = (self.mantissa(), self.scale());
        h.0.update(&m.to_le_bytes());
        h.0.update(&(s as u32).to_le_bytes());
    }
}

pub fn compute_hash<T: HashCanon>(t: &T) -> [u8; 32] {
    let mut h = Hasher(blake3::Hasher::new());
    t.hash_into(&mut h);
    *h.0.finalize().as_bytes()
}
```

---

## 4) Domain Keys (What goes into the hash)

### 4.1 Curves

- Key inputs: `(curve_id, currency, pillars[], rates[], interpolation, compounding, day_count, provenance_ops[], as_of)`.
- Notes: provenance records arithmetic applied by scenarios (parallel shifts, twists) and must be part of the input.

### 4.2 Schedules & Accruals

- Key inputs: `(instrument_id, schedule_params, calendar_version, bdc, eom_rule, stub_type)`.
- Accrual fraction keys: `(start_date, end_date, day_count_variant)` with a version tag of the implementation.

### 4.3 Expressions

- Key inputs: normalized formula text, function registry version, referenced node table, lowering flags.
- Output: compiled DAG + optional Polars plan.

### 4.4 Statements

- Block evaluation keys: `(model_id, node_id, period_block, compiled_expr_hash, period_plan_hash, rounding_context_hash)`.
- Where order is semantically fixed, preserve `IndexMap` order; otherwise sort.

### 4.5 Valuations

- Schedule keys (see 4.2). Pricing kernel keys: instrument parameters, market view subset (referenced curves/indices only), numeric and FX policy hashes when relevant.

### 4.6 Scenarios

- Plan key: normalized DSL + deterministic expansion (globs/selectors) + priorities + conflict strategy + strict/lenient + includes resolved. Expansion lists for orderless selections are sorted.

### 4.7 Portfolio Aggregations

- Run key: `(portfolio_layout_hash, period_plan_hash, market_view_hash, scenario_plan_hash?, rounding_context_hash, fx_policy_hash)`.
- Layout: books tree, positions (ids, quantities, units, open/close), base currency, node alias map.

---

## 5) Invalidation Strategy

Preferred invalidation is by content address: changed content ⇒ new key ⇒ natural miss. Eviction is used for memory hygiene and targeted cleanup.

### 5.1 Precise Invalidation

- Market shocks: evict entries whose dependency lists reference affected curve ids/vol ids/FX pairs.
- Instrument edits: evict schedules and pricing kernels keyed by those instrument ids.
- Statement edits: evict compiled expressions and per‑node caches keyed by node ids.

### 5.2 Conservative and Full Modes

- Conservative: phase‑level eviction (e.g., all schedule caches) when precise targeting is ambiguous.
- Full: explicit operator/test trigger to clear all caches.

---

## 6) Concurrency & Single‑Flight

### 6.1 Requirements

- Avoid thundering herd; compute each missing key once, with waiters.
- Sharded, bounded caches per domain (LRU by count and approximate bytes).

### 6.2 Pattern (illustrative)

```rust
enum EntryState<V> { Ready(Arc<V>), Computing(Arc<once_cell::sync::OnceCell<Arc<V>>>) }

struct Cache<V> { map: dashmap::DashMap<Key, EntryState<V>> }

impl<V> Cache<V> {
    fn get_or_compute(&self, k: Key, f: impl FnOnce() -> V) -> Arc<V> {
        use dashmap::mapref::entry::Entry;
        match self.map.entry(k) {
            Entry::Occupied(o) => match o.get() {
                EntryState::Ready(v) => v.clone(),
                EntryState::Computing(cell) => cell.wait().clone(),
            },
            Entry::Vacant(v) => {
                let cell = Arc::new(once_cell::sync::OnceCell::new());
                v.insert(EntryState::Computing(cell.clone()));
                let value = Arc::new(f());
                let _ = cell.set(value.clone());
                v.insert(EntryState::Ready(value.clone()));
                value
            }
        }
    }
}
```

---

## 7) Determinism & Numeric Policy

- Decimal mode: serial ≡ parallel byte‑identical. Reduction order must be stable (pairwise/Kahan or fixed chunk merges).
- Do not include the `parallel` flag in content hashes; values must not depend on parallelism.
- Stamp `NumericMode` and `parallel` into results metadata for observability.

---

## 8) Results Metadata (Reproducibility)

All top‑level result envelopes (statements, valuations, portfolio) MUST include the following when applicable:

- `period_plan_hash`
- `scenario_plan_hash`
- `market_view_hash` (subset actually used)
- `portfolio_layout_hash`
- `rounding_context_hash`
- `fx_policy_hash`

These complement existing fields such as `numeric_mode`, `parallel`, `seed`, and `model_currency`.

---

## 9) Observability & Metrics

- Log cache events at debug level: domain, short key, dependency short‑hashes, compute_time_ms, hit/miss.
- Expose counters/gauges per domain: size, hits, misses, evictions, single‑flight waiters.
- Provide a debug “explain cache entry” API returning the dependency list and canonical inputs used to compute the key.

---

## 10) Configuration

- Per‑domain capacity (entries and approximate bytes).
- Invalidation policy knobs (precise/conservative) with sensible defaults.
- Hash audit mode (tests only): serialize canonicalized input bytes alongside the hash for diffing.

---

## 11) Testing & CI

### 11.1 Unit & Golden

- Golden tests for `ScenarioPlan` and portfolio results: compare values and stamped hashes.
- Curve and schedule hashing: minor content change (pillar rate tweak) flips curve key and dependent outputs.

### 11.2 Property Tests (examples)

- Scenario reordering equivalence: different declaration orders with equal priorities yield identical plan hash and outputs.
- Glob determinism: expanded target lists and plan hash are identical across runs; truncation flags are stable.
- Unrelated shock isolation: changing an unrelated curve does not alter keys or outputs for instruments that do not reference it.
- FX policy change: switching policy changes `fx_policy_hash` and affects aggregation as expected.

### 11.3 Parallel Safety

- Many threads requesting the same key compute exactly once (assert single‑flight counters) and match serial results.

### 11.4 Cross‑Platform Hash Stability

- CI matrix (Linux/macOS/Windows) asserts hash equality for the same canonical inputs.

---

## 12) Versioning & Migration

- Each domain tag carries a version byte; bump on canonical input shape changes.
- On bump, old entries are naturally invalidated due to domain/version prefix.

---

## 13) Adoption Checklist

- Define canonical input structs per domain and implement `HashCanon`.
- Build cache keys exclusively from canonical hashes.
- Record dependency hashes in cache metadata for explainability.
- Ensure evaluation reductions are stable; assert serial ≡ parallel in Decimal mode.
- Stamp hashes into `ResultsMeta` and surface in bindings.
- Add targeted unit/property/golden tests and enable hash stability checks in CI.

---

## Appendix A — Property Test Sketches (Illustrative)

```rust
proptest! {
    #[test]
    fn scenario_reordering_equivalence(a in arb_scenario(), b in permute_equivalent(a.clone())) {
        let pa = build_plan(&a)?; let pb = build_plan(&b)?;
        prop_assert_eq!(pa.hash, pb.hash);
        prop_assert_eq!(run(&pa)?.values, run(&pb)?.values);
    }

    #[test]
    fn unrelated_shock_isolation(ctx in arb_small_portfolio_ctx()) {
        let before = run(&ctx)?;
        shock_unrelated_curve(&ctx.market);
        let after = run(&ctx)?;
        prop_assert_eq!(before.values, after.values);
        prop_assert!(cache_stats().hits >= before_stats.hits);
    }
}
```

---

## Appendix B — ResultsMeta Fields (Additions)

- `fx_policies: IndexMap<String, FxPolicyMeta>` as described in Overall §2.7
- `rounding: RoundingContext` as described in Config
- `hashes: IndexMap<String, String>` for stamped short‑hashes (e.g., `scenario_plan`, `market_view`)

---

This document defines the caching and hashing invariants required to achieve deterministic performance with precise invalidation across the finstack workspace. It complements the Core (§2.7), Scenarios, Valuations, and Portfolio designs and is normative for implementation and CI testing.


