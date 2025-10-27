# Feature Flags: Quick Reference

## The 4 Core Features

```
┌─────────────────────────────────────────────────────────────────┐
│                      FINSTACK FEATURE FLAGS                      │
│                   (2 Default + 2 Optional)                       │
└─────────────────────────────────────────────────────────────────┘

DEFAULT FEATURES (Included by default)

1. serde       │ Serialization & wire formats
   [DEFAULT]   │ • JSON, CBOR, MessagePack
               │ • Python/WASM bindings
               │ • Size: +150 KB
               └─→ Applied to: ALL crates

2. parallel    │ Multi-threaded computation  
   [DEFAULT]   │ • Rayon-based parallelism
               │ • 2-10x performance boost
               │ • Deterministic results
               │ • Size: +200 KB
               └─→ Applied to: core, valuations

OPTIONAL FEATURES (Opt-in)

3. dataframes  │ Data science integration
   [OPT-IN]    │ • Polars DataFrame exports
               │ • Time-series analysis
               │ • Jupyter notebooks
               │ • Size: +2-3 MB
               └─→ Applied to: statements, portfolio

4. stochastic  │ Monte Carlo & stochastic models
   [OPT-IN]    │ • Random number generation
               │ • Path-dependent pricing
               │ • Advanced risk analytics
               │ • Size: +100 KB
               └─→ Applied to: valuations
```

## Feature Combinations by Use Case

### 🔹 Basic Library Usage
```toml
finstack = "0.3"
```
**Features:** `serde` + `parallel` (defaults)  
**Use cases:** Production servers, batch processing, risk engines, general use

---

### 🔹 Minimal Build (opt-out of parallel)
```toml
finstack = { version = "0.3", default-features = false, features = ["serde"] }
```
**Features:** `serde` only  
**Use cases:** Embedded systems, minimal binary size, WASM bundles

---

### 🔹 Data Science / Analytics
```toml
finstack = { version = "0.3", features = ["dataframes"] }
```
**Features:** `serde` + `parallel` + `dataframes`  
**Use cases:** Jupyter notebooks, pandas integration, data pipelines

---

### 🔹 Quantitative Research
```toml
finstack = { version = "0.3", features = ["stochastic"] }
```
**Features:** `serde` + `parallel` + `stochastic`  
**Use cases:** Monte Carlo pricing, exotic options, CVaR analysis

---

### 🔹 Everything Enabled
```toml
finstack = { version = "0.3", features = ["dataframes", "stochastic"] }
```
**Features:** All features enabled  
**Use cases:** Research platforms, advanced analytics suites, full-stack applications

## Comparison: Before vs After

### Before (Current State)
```
Core Features:
├─ std
├─ serde  
└─ parallel

Statements Features:
└─ polars_export

Valuations Features:
├─ serde
├─ parallel
├─ index (unused!)
└─ stochastic-models

Portfolio Features:
├─ scenarios
└─ dataframe

Scenarios Features:
└─ serde

Total: 11 features across 5 crates (fragmented)
```

### After (Proposed)
```
Cross-Cutting Features:
├─ serde       → ALL crates
├─ parallel    → core, valuations
├─ dataframes  → statements, portfolio
└─ stochastic  → valuations

Total: 4 features, clearly defined
```

## Dependency Impact

| Feature      | Dependencies Added                    | Size Impact |
|--------------|---------------------------------------|-------------|
| `serde`      | serde (1.0), serde_json (1.0)         | ~150 KB     |
| `parallel`   | rayon (1.10)                          | ~200 KB     |
| `dataframes` | polars (0.44, minimal features)       | ~2-3 MB     |
| `stochastic` | rand (0.8), rand_pcg, rand_distr      | ~100 KB     |

**Total library size:**
- Minimal (serde only): ~3 MB
- Default (serde + parallel): ~3.2 MB
- + dataframes: ~5.5 MB
- + stochastic: ~3.5 MB
- All features: ~6 MB

## Testing Matrix

Required CI test combinations: **5 configurations**

1. ☐ Minimal (serde only, default-features = false)
2. ☐ Default (serde + parallel)
3. ☐ `dataframes`
4. ☐ `stochastic`
5. ☐ All features (dataframes + stochastic)

Each combination tested on:
- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

## Migration Checklist

### For Users (Non-Breaking)
- [x] No immediate action required
- [ ] Consider renaming `polars_export` → `dataframes` in Cargo.toml
- [ ] Review feature flags for optimization opportunities

### For Maintainers
- [ ] Add `dataframes` alias to statements/Cargo.toml
- [ ] Add `dataframes` alias to portfolio/Cargo.toml
- [ ] Rename `stochastic-models` → `stochastic` in valuations/Cargo.toml
- [ ] Remove unused `index` feature from valuations/Cargo.toml
- [ ] Update all examples to use new feature names
- [ ] Update README.md with new feature descriptions
- [ ] Add deprecation warnings to old feature names
- [ ] Update CI matrix to test 7 configurations
- [ ] Plan breaking change for 0.4.0 (remove deprecated aliases)

## FAQ

### Q: Why not include `std` as a feature?
**A:** The `no_std` use case is extremely rare for Finstack. Making `std` mandatory simplifies the codebase and reduces testing burden. Users needing `no_std` can request it as a future enhancement.

### Q: Can I still use `polars_export`?
**A:** Yes, during the transition period it will be an alias for `dataframes`. It will be removed in version 0.4.0.

### Q: Does `parallel` guarantee faster execution?
**A:** Generally yes (2-10x speedup on multi-core CPUs), but small datasets may not benefit. The feature guarantees deterministic results identical to sequential execution.

### Q: When will `stochastic` feature be implemented?
**A:** The dependencies are in place; implementation is planned for version 0.3.x. The feature flag is reserved to ensure stable API.

### Q: Why separate `dataframes` from core?
**A:** Polars adds ~2-3 MB to the library. Users who don't need DataFrame exports (e.g., embedded pricing engines) can keep binaries smaller.

## Decision Matrix

**When to enable each feature:**

| Feature      | Enable if you need...                                    |
|--------------|----------------------------------------------------------|
| `serde`      | ✅ Always (default, included)                             |
| `parallel`   | ✅ Usually (default, included) - opt-out for minimal size |
| `dataframes` | Pandas integration, CSV/Parquet export, Jupyter notebooks|
| `stochastic` | Monte Carlo pricing, path-dependent options, CVaR        |

## Summary

**2 default + 2 optional features. Clear purposes. Minimal decisions.**

This design prioritizes:
1. **Simplicity** - Easy to understand and choose
2. **Orthogonality** - Features don't overlap
3. **Performance** - Pay only for what you use
4. **Maintainability** - Fewer combinations to test

The proposal reduces complexity while maintaining full functionality and backward compatibility during migration.

