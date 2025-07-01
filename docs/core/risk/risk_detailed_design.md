# Risk Metrics Module – Detailed Design Document

## 0 Change-Log (2025-06-29)
* Incorporated audit feedback: C-08 linkage, clarified naming vs path, added `Order` & `FDStencil` enums, defined numeric alias `F`, refined `RiskReport` aggregation semantics, documented error handling & cache capacity, extended feature-flag matrix, added fuzz testing and timeline alignment.
* Added **cross-gamma** support to risk metrics and removed it from open questions.

## 1 Overview
The **risk-metrics** module computes first- and second-order sensitivities ("Greeks") and other key risk measures such as bucketed DV01, Vega ladders and scenario PVs for any instrument that implements the `Priced` trait. It fulfils PRD capabilities C-57 – C-60 and supersedes the earlier working name *GreekEngine*.

Key design objectives:
* 🦀 **Idiomatic Rust**, zero-`unsafe` public API.
* ⚡ **High-performance** analytic, adjoint (AAD) and finite-difference engines with parallel execution.
* 📦 **Extensible** factor taxonomy covering rates, vol, FX, inflation, credit and equity.
* ♻️ **Reusable bump-seed cache** to amortise repeated scenario shocks.
* 📊 **Report-ready outputs** for regulatory and desk risk reporting flows.

## 2 Goals & Non-Goals
### 2.1 Goals
1. Provide a **trait-based risk engine** that any `Priced` instrument can use with minimal boiler-plate (C-58, C-59).
2. Produce **bucketed risk vectors** required for regulatory reports:
   • Curve key-rate DV01 / CS01  
   • Vol-surface Vega ladders  
   • Equity Δ/Γ, **cross-gamma interactions**, FX Δ, inflation beta, etc.
3. Offer **scenario re-valuation** helpers for what-if analysis (C-60).
4. Supply a **bump-seed cache** so repeated shocks reuse pre-computed state (C-59).

### 2.2 Non-Goals
* XVA, VaR or ES metrics (handled in higher-level analytics crates).
* GUI / CLI stress-testing front-ends (rustfin-cli responsibility).
* Exotic third-order Greeks (Color, Ultima) – postponed post v1.0.

## 3 Naming Rationale
The documentation still lives under `docs/core/greeks/` while the code will reside in `src/risk/`. At GA we will **rename the doc folder to `risk`** for coherence; maintaining the original path during draft avoids broken cross-links.

## 4 High-Level API Sketch
```rust
/// Global numeric type (matches curves module)
use rustfin::curves::F; // alias f64 | Decimal via feature flag

/// Order of derivatives requested
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order { First, Second }

/// Finite-difference stencil variants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FDStencil { OneSided, TwoSided, FourPoint }

// Core engine trait
pub trait RiskEngine {
    type Output = RiskReport;             // fixed for simplicity
    fn compute<I, C>(&self, instr: &I, curves: &C, val_date: Date) -> Result<Self::Output, RiskError>
    where
        I: Priced,
        C: CurveProvider;
}

// Convenience enum for built-in engines
pub enum RiskMode {
    Analytic,
    Adjoint { order: Order },
    FiniteDiff { bump: F, stencil: FDStencil },
}

// Blanket impl selects concrete engine at runtime (pseudo-code)
impl RiskEngine for RiskMode { /* dispatch */ }

/// Aggregated sensitivity report (sparse representation)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskReport {
    pub pv: Money,
    /// Parallel vectors share index: factors[i] ↔ values[i]
    pub factors: Vec<RiskFactor>,
    pub delta:   Vec<F>,
    pub gamma:   Vec<F>,
    pub vega:    Vec<F>,
    pub theta:   Vec<F>,
    pub dv01:    Vec<F>,
    /// Sparse upper-triangular cross-gamma matrix as index pairs ↦ value.
    pub cross_gamma: Vec<(usize, usize, F)>,
}
```
A **sparse parallel-vector** layout avoids repeating `RiskFactor` in every bucket and allows fast aggregation by factor hashing.

### 4.1 RiskFactor Taxonomy
```rust
#[non_exhaustive]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RiskFactor {
    Rate      { curve: CurveId, bucket_id: BucketId },
    FxSpot    { pair: FxPair },
    Vol       { surface: CurveId, exp_idx: usize, strike_idx: usize },
    Credit    { curve: CurveId, bucket_id: BucketId },
    Inflation { curve: CurveId, bucket_id: BucketId },
    Equity    { ticker: &'static str },
}
```
`BucketId` is a `u8` index into the user-supplied tenor array, eliminating floating-point equality.

### 4.2 Bump-Seed Cache
```rust
pub struct BumpCache {
    inner: DashMap<RiskFactor, Arc<dyn Any + Send + Sync>>, // thread-safe
    capacity: usize,                                        // LRU cap
}
```
The cache evicts the least-recently-used entry once `len() == capacity` (default 10 000) using a CLOCK hand.

## 5 Module Layout
```
src/risk/
  ├─ mod.rs          // façade re-exports & prelude
  ├─ engine.rs       // RiskEngine, RiskMode, Order
  ├─ factor.rs       // RiskFactor, VolBucket helpers
  ├─ bucket.rs       // key-rate & Vega bucket generators (C-57)
  ├─ analytic.rs     // closed-form Greeks for vanillas
  ├─ adjoint.rs      // tape + reverse AAD engine
  ├─ finite_diff.rs  // fallback FD engine
  ├─ scenario.rs     // what-if re-valuation helpers (C-60)
  ├─ cache.rs        // bump-seed cache (C-59)
  ├─ aggregate.rs    // portfolio aggregation, CCY conversion
  └─ tests.rs
```

## 6 Core Algorithms
1. **Analytic Path** – instrument-specific `impl AnalyticGreeks` provides closed-form results where available.
2. **Adjoint Path (AAD)** – records a valuation tape, reverse sweep yields first- & second-order sensitivities; memory pooled via bumpalo and shared through `BumpCache`.
3. **Finite-Difference** – parallel bump-and-price using Rayon; configurable step size and stencil with Richardson extrapolation.
4. **Key-Rate Generator** – splits curve into maturity buckets to compute DV01 / CS01 vectors.
5. **Vol Bucket Generator** – builds expiry × strike (or Δ) ladders; supports SABR smile interpolation.
6. **Scenario Helper** – `MarketSnapshot` trait enables shocked re-valuation without rebuilding curves.

## 7 Input Parameters (`RiskSettings` builder)
| Parameter | Purpose | Default |
|-----------|---------|---------|
| `mode` | Select Analytic / AAD / FD engine | `Adjoint { order: First }` |
| `bump_size` | Absolute bump size (if `is_relative = false`) | `1e-4` |
| `is_relative` | Interpret `bump_size` as ×spot | `true` |
| `key_rate_tenors` | Custom DV01 bucket grid | 11-point Govt curve grid |
| `vol_buckets` | Expiry × strike/Δ spec | 10×11 standard ladder |
| `parallel` | Enable Rayon | auto via `parallel` feature |
| `order` | First or second order | `First` |
| `reuse_cache` | Share `BumpCache` across calls | `None` |

## 8 Performance Targets
| Task | Target (16 cores, f64) | Technique |
|------|------------------------|-----------|
| 1 M DV01 bumps on swap book | < 45 ms | key-rate compression, SIMD DFs |
| Vega ladder 10×11 on 10 k swaptions | < 120 ms | AAD thick-slice, cache reuse |
| Full risk vector on 50 k trades | < 400 ms | parallel aggregation |

## 9 Feature Flags (updated)
| Flag | Purpose | Notes |
|------|---------|-------|
| `parallel` | Enable Rayon in FD + aggregation | inherited global flag |
| `analytic_only` | Compile **only** analytic path | excludes AAD & FD |
| `aad_only` | Compile analytic + AAD, strip FD for latency | mutually exclusive with `analytic_only` |
| `serde` | Serde derives on `RiskReport` | additive |

`build.rs` guards illegal combos (`analytic_only` ∧ `aad_only`).

## 10 Testing Strategy (extended)
* **Golden vectors** vs QuantLib for Δ/Vega on swaps, caps, bonds.  
* **Cross-method consistency** – Analytic vs AAD vs FD within 1 bp tolerance.  
* **Property-based fuzzing** (`proptest`): random bump sizes, sign flips, extreme notionals.  
* **Cache hit-rate tests** ensure LRU eviction works.  
* **Criterion benches** – CI fails on >5 % regression.

## 11 Open Questions
1. Expose third-order Greeks behind feature flag?
2. Persist `BumpCache` across process boundary (shared memory)?

## 12 Timeline (realigned)
* **v0.1.0** – RiskFactor taxonomy, analytic Greeks for **SpotAsset & Deposit** (already available instruments).
* **v0.2.0** – Extend analytics to swaps & caps once instruments land; deliver AAD engine + bump cache.
* **v0.3.0** – DV01 & Vega helpers; scenario API.
* **v1.0.0** – Performance hardening, doc folder rename, API freeze.

---
### One-line Summary
**risk-metrics** delivers fast, extensible first- and second-order sensitivities (Greeks, DV01, Vega ladders) through analytic, adjoint and finite-difference engines, backed by a reusable bump-seed cache and parallel execution – providing the comprehensive risk reporting foundation required for RustFin v1.0. 

### 0.5 Relationship to C-08 Greeks API
This module **fulfils PRD/TDD capability C-08** (“Greeks API”) by exposing the `RiskEngine` trait as the single entry-point for sensitivity calculation. No separate API will be created. 