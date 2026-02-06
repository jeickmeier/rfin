# Cargo Dependency Review

**Date:** 2026-02-04
**Goal:** Identify unnecessary or lightly used crates that could be removed to minimize footprint and reduce bloat.

## Summary

After comprehensive analysis of all 11 `Cargo.toml` files and codebase usage patterns, **all dependencies are actively used and necessary**. However, there are a few optimization opportunities for consistency and potential footprint reduction.

## Detailed Analysis by Crate

### finstack-core

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `parking_lot` | ✅ Active | **KEEP** | Used in 3 locations: `fx/providers.rs`, `fx.rs`, `expr/cache.rs` for thread-safe locking |
| `lru` | ✅ Active | **KEEP** | Used for LRU caching in `fx.rs` and `expr/cache.rs` - core performance feature |
| `statrs` | ✅ Active | **KEEP** | Extensively used in `math/special_functions.rs` and `math/distributions.rs` for statistical functions |
| `rand_pcg` | ✅ Active | **KEEP** | Used in `math/random.rs` for production-grade RNG (PCG64) |
| `phf` | ✅ Active | **KEEP** | Used for compile-time hash maps in generated currency code |
| `nalgebra` | ✅ Active | **KEEP** | Used for linear algebra operations (matrix math) |
| `rustc-hash` | ✅ Active | **KEEP** | Workspace dependency, used for fast integer hashing |

**Verdict:** All dependencies are essential. No removals recommended.

### finstack-statements

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `nom` | ✅ Active | **KEEP** | Used in `dsl/parser.rs` for DSL parsing - core feature |
| `log` | ✅ Active | **KEEP** | Used for warnings in 5 files (integration.rs, formula.rs, timeseries.rs, statistical.rs, deterministic.rs) |

**Optimization Opportunity:**
- `log` is only used for `log::warn!()` calls. Other crates (`valuations`, `portfolio`) use `tracing` for logging.
- **Recommendation:** Consider migrating to `tracing` for consistency, but `log` is very lightweight (~50KB) and is a standard Rust logging facade, so the refactoring effort may not be worth it unless you want unified logging infrastructure.

**Verdict:** All dependencies are essential. Optional consolidation opportunity exists.

### finstack-valuations

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `num-complex` | ✅ Active | **KEEP** | Used in Heston model implementations (`heston.rs` files) |
| `ts-rs` | ✅ Active (optional) | **KEEP** | Used for TypeScript bindings generation when `ts_export` feature is enabled |
| `tracing` | ✅ Active | **KEEP** | Extensively used for logging/warnings across 20+ files |
| `rayon` | ✅ Active (optional) | **KEEP** | Used for parallel processing when `parallel` feature is enabled |
| `nalgebra` | ✅ Active (optional) | **KEEP** | Used for Monte Carlo when `mc` feature is enabled |

**Verdict:** All dependencies are essential and properly feature-gated. No removals recommended.

### finstack-portfolio

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `good_lp` | ✅ Active | **KEEP** | Used in `optimization/lp_solver.rs` for linear programming optimization |
| `tracing` | ✅ Active | **KEEP** | Used for logging in `metrics.rs`, `position.rs`, `builder.rs` |
| `polars` | ✅ Active (optional) | **KEEP** | Used for DataFrame exports when `dataframes` feature is enabled |
| `rayon` | ✅ Active (optional) | **KEEP** | Used for parallel processing when `parallel` feature is enabled |

**Verdict:** All dependencies are essential. No removals recommended.

### finstack-io

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `sea-query` | ✅ Active | **KEEP** | Extensively used for SQL schema building across all schema files and migrations |
| `tokio-rusqlite` | ✅ Active (optional) | **KEEP** | Used for SQLite backend when `sqlite` feature is enabled |
| `deadpool-postgres` | ✅ Active (optional) | **KEEP** | Used for Postgres backend when `postgres` feature is enabled |
| `libsql` | ✅ Active (optional) | **KEEP** | Used for Turso backend when `turso` feature is enabled |

**Verdict:** All dependencies are essential and properly feature-gated. No removals recommended.

### finstack-py

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `pythonize` | ✅ Active | **KEEP** | Extensively used (30+ locations) for Python ↔ Rust serialization |
| `pyo3-polars` | ✅ Active | **KEEP** | Used for Polars DataFrame integration in 7+ files |
| `polars` | ✅ Active | **KEEP** | Required by `pyo3-polars` and used directly for DataFrame operations |

**Verdict:** All dependencies are essential. No removals recommended.

### finstack-wasm

| Dependency | Usage | Status | Notes |
|------------|-------|--------|-------|
| `console_error_panic_hook` | ✅ Active (optional) | **KEEP** | Used for better panic messages in browser console when feature is enabled |
| `ts-rs` | ✅ Active (optional) | **KEEP** | Used for TypeScript bindings when `ts_export` feature is enabled |

**Verdict:** All dependencies are essential and properly feature-gated. No removals recommended.

## Cross-Crate Dependencies

### Common Dependencies (Used in Multiple Crates)

| Dependency | Crates Using It | Status |
|------------|----------------|--------|
| `serde` | All crates | **KEEP** - Core serialization |
| `serde_json` | All crates | **KEEP** - JSON serialization |
| `thiserror` | Most crates | **KEEP** - Error handling |
| `time` | All crates | **KEEP** - Date/time handling |
| `indexmap` | Multiple crates | **KEEP** - Ordered hash maps |
| `rust_decimal` | Multiple crates | **KEEP** - Decimal arithmetic |
| `strum` | Multiple crates | **KEEP** - Enum utilities (workspace dependency) |

**Verdict:** All common dependencies are essential and properly shared via workspace dependencies where appropriate.

## Feature Gates Analysis

All optional dependencies are properly feature-gated:
- ✅ `polars` - gated behind `dataframes` feature
- ✅ `rayon` - gated behind `parallel` feature
- ✅ `nalgebra` - gated behind `mc` feature
- ✅ `ts-rs` - gated behind `ts_export` feature
- ✅ Database backends - gated behind respective features (`sqlite`, `postgres`, `turso`)
- ✅ `console_error_panic_hook` - gated behind feature flag

**Verdict:** Feature gating is well-implemented. Users can opt-out of optional functionality.

## Recommendations

### 1. **No Immediate Removals Needed**

All dependencies are actively used and serve essential purposes. The codebase is already well-optimized with proper feature gating.

### 2. **Optional: Logging Consolidation** (Low Priority)

- **Current:** `finstack-statements` uses `log`, while `finstack-valuations` and `finstack-portfolio` use `tracing`
- **Impact:** Minimal - `log` is very lightweight (~50KB)
- **Effort:** Medium - requires refactoring 5 files in `finstack-statements`
- **Benefit:** Unified logging infrastructure, but `log` is a standard Rust facade that many crates use
- **Recommendation:** Only do this if you want unified logging infrastructure. Not necessary for footprint reduction.

### 3. **Verify Build Profile Optimization**

The workspace already has excellent build profiles:
- `release-size` profile for WASM with size optimizations
- Proper `default-features = false` usage where applicable
- Feature gating for optional dependencies

### 4. **Consider Dependency Audit Tools**

For ongoing monitoring:
- `cargo-deny` - Already configured (see `deny.toml`)
- `cargo-udeps` - Can detect unused dependencies (though our analysis shows none)
- `cargo-tree` - Useful for visualizing dependency trees

## Footprint Analysis

### Estimated Binary Size Impact (Rough Estimates)

| Category | Crates | Estimated Size Impact |
|----------|--------|----------------------|
| Core dependencies | `parking_lot`, `lru`, `statrs`, `rand_pcg`, `phf` | ~500KB |
| Math libraries | `nalgebra`, `num-complex` | ~300KB |
| Logging | `log`, `tracing` | ~100KB |
| Serialization | `serde`, `serde_json` | ~200KB |
| Async runtime | `tokio` (only in io/py) | ~1MB+ (only when used) |
| Database drivers | `rusqlite`, `tokio-postgres`, `libsql` | ~500KB each (optional) |

**Note:** These are rough estimates. Actual sizes depend on features enabled and optimization level.

## Conclusion

**The dependency footprint is already well-optimized.** All dependencies are:
1. ✅ Actively used in production code
2. ✅ Properly feature-gated where appropriate
3. ✅ Using `default-features = false` where possible
4. ✅ Essential for core functionality

**No crates should be removed** - all are necessary for the library's functionality.

The only potential optimization is consolidating `log` → `tracing` in `finstack-statements` for consistency, but this is a code quality improvement rather than a footprint reduction (both crates are similarly sized).

## Next Steps

1. ✅ **No action required** - dependencies are optimal
2. Optional: Consider `log` → `tracing` migration for consistency (low priority)
3. Continue using `cargo-deny` and `cargo-udeps` for ongoing monitoring
4. Monitor dependency updates for potential bloat in future versions
