# Curves Module – Detailed Design Document

## 1 Overview
The **curves** module provides time-dependent term structures such as discount factors, zero rates, forward rates, credit hazard rates, inflation indices, and volatility/variance surfaces. It underpins valuation, risk, and calibration engines throughout RustFin. The design fulfils PRD/TDD capabilities C-03, C-04, C-07, C-18–C-21, C-33, C-41–C-45.

Key attributes:
* 🦀 Idiomatic Rust with zero-unsafe public API.
* ⚡ High-performance: SIMD-friendly interpolation, cache-aligned knot storage.
* 🔌 Extensible: traits for new curve types & interpolation policies.
* 📦 Serde support behind `serde` feature flag for persistence and FFI.

## 2 Goals & Non-Goals
### 2.1 Goals
1. Provide **core traits** (`Curve`, `Surface`, `CurveId`) consumed by cash-flow, instrument, and risk layers (C-03).
2. Support **multiple numeric precisions** via the optional `decimal128` feature (C-04).
3. Offer **pluggable interpolation & extrapolation policies** selectable at runtime or compile-time (C-41).
4. Implement concrete term-structure types:
   • Yield/discount curves (zero-rate & DF)
   • Forward index curves (C-18)
   • Credit-hazard curves (C-43)
   • Real & breakeven inflation curves (C-44)
   • Volatility & variance surfaces (C-42)
   • Option-pricing trees (binomial, trinomial) for equity/FX rates.
   • Short-rate and credit-spread trees for IR and credit derivatives.
5. Expose **multi-curve framework** allowing a `MarketContext` to hold discount, forecast and collateral curves (C-33, C-45).
6. Deliver **serde derives** on all public structs once stabilised (C-07).

### 2.2 Non-Goals
* Bootstrapping/calibration algorithms (lives in `bootstrap` module).
* Market data ingestion or storage (handled by application layer).
* Exotic stochastic-volatility models (beyond basic SABR helpers).

## 3 High-Level API Sketch
```rust
use rustfin::curves::{CurveId, DiscountCurve, MarketContext};
use rustfin::dates::Date;

let yc = YieldCurve::builder("USD-OIS")
    .base_date(Date::new(2025, 6, 30)?)
    .knots([ (0.5, 0.0195), (1.0, 0.0201) ])
    .monotone_convex()
    .build()?;

let df = yc.df(2.75);      // discount factor for 2.75 years

let curves = MarketContext::new()
    .with_discount(yc)
    .with_forecast("USD-LIB3M", fwd_curve)
    .with_collateral("CSA-USD", coll_curve);

use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
let pv = swapleg.npv(&curves.get::<DiscountCurve>("USD-OIS")?);
```

## 4 Module Layout
```
src/curves/
  ├─ mod.rs           // re-exports & facade
  ├─ id.rs            // CurveId, FactorKey taxonomy (C-19)
  ├─ traits.rs        // Curve, DiscountCurve, Surface traits
  ├─ interp.rs        // Interpolation & extrapolation policies (C-41)
  ├─ yield_curve.rs   // Piecewise DF/zero curves
  ├─ fwd_curve.rs     // Forward index curve (C-18)
  ├─ hazard_curve.rs  // Credit hazard rates (C-43)
  ├─ inflation.rs     // Real / breakeven curves (C-44)
  ├─ vol_surface.rs   // Vol/variance surfaces (C-42)
  ├─ option_tree.rs   // Binomial / trinomial lattices
  ├─ rate_tree.rs     // Short-rate trees (Ho-Lee, BDT, HW)
  ├─ credit_tree.rs   // Default-intensity lattices
  ├─ multicurve.rs    // MarketContext & collateral spec (C-33, C-45)
  ├─ math.rs          // helper maths (log/linear DF ↔ zero, roots)
  └─ tests.rs
```

## 5 Core Types & Traits
### 5.1 Numeric Precision Layer (C-04)
RustFin exposes a single alias `pub type F` in the `primitives` crate; every other crate must `use primitives::F` and must **not** redeclare the alias locally. All curve values therefore use `F`; helper methods such as `as_f64()` remain available for crates that require a concrete floating-point type.

### 5.2 CurveId & FactorKey (C-19)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CurveId(&'static str);

pub enum FactorKey<'a> {
    Yield(&'a CurveId),
    Hazard(&'a CurveId),
    VolSurface(&'a CurveId),
    // … ILIQ, CPI, etc.
}
```
* String literals interned at compile-time; cheap `&'static` comparisons.

### 5.3 Core Traits
```rust
pub trait Curve {
    fn id(&self) -> &CurveId;
    fn df(&self, t: F) -> F;                         // discount factor at time t (years)
    fn zero(&self, t: F) -> F { -self.df(t).ln() / t }
    fn fwd(&self, t1: F, t2: F) -> F {               // simple forward rate
        (self.zero(t1)*t1 - self.zero(t2)*t2)/(t2-t1)
    }
}

pub trait DiscountCurve: Curve {
    fn base_date(&self) -> Date;
}

pub trait Surface {
    fn id(&self) -> &CurveId;
    fn value(&self, x: F, y: F) -> F;               // e.g. vol(expiry, strike)
}
```
* Default methods avoid duplicated formulae; specialisations can override for speed.

### 5.4 Interpolator Enum (C-41)
```rust
pub enum Interpolator {
    LinearDf(LinearDf),
    LogDf(LogDf),
    MonotoneConvex(MonotoneConvex),
    CubicHermite(CubicHermite),
    FlatFwd(FlatFwd),
}

impl Interpolator {
    pub fn linear_df(knots: Box<[F]>, dfs: Box<[F]>) -> Result<Self> { /* … */ }
    // … other smart constructors omit for brevity
    pub fn interp(&self, x: F) -> F { /* dispatch */ }
}
```
Curves now receive an `Interpolator` directly via specialised builder helpers
(`.linear_df()`, `.log_df()`, `.monotone_convex()`, etc.) so the separate
`InterpPolicy` enum is no longer required.

### 5.5 YieldCurve Implementation
```rust
pub struct YieldCurve {
    id: CurveId,
    base: Date,
    knots: Box<[F]>,          // times in years (strictly increasing)
    dfs:   Box<[F]>,
    interp: Box<dyn Interpolator + Send + Sync>,
}
```
* Functions `df`, `zero`, `fwd` delegate to `interp` which sees `(knots, dfs)`.
* Memory layout contiguous for cache-line prefetch.

### 5.6 HazardCurve Implementation (Credit) {#hazard-curve}
```rust
pub struct HazardCurve {
    id: CurveId,
    base: Date,
    knots: Box<[F]>,     // times in years
    lambdas: Box<[F]>,   // hazard rates λ(t) piecewise-const or interp
    interp: Box<dyn Interpolator + Send + Sync>,
}
```
* Survival probability \(S(t) = \exp\bigl(-\int_0^t \lambda(u)\,du\bigr)\) computed via cumulative trapezoid of interpolated hazards.
* Provides helper `sp(t)` and `default_prob(t1,t2)` functions.
* Can be converted to cumulative default DF for CVA engines.

### 5.7 InflationCurve Implementation
```rust
pub struct InflationCurve {
    id: CurveId,
    base_cpi: F,         // CPI on base date
    knots: Box<[F]>,     // times in years
    cpi_levels: Box<[F]>,
    interp: Box<dyn Interpolator + Send + Sync>,
}
```
* Returns `cpi(t)` and `inflation_rate(t1,t2)` helpers.
* Supports both real (index) and breakeven curves by storing either levels or log-returns.

### 5.8 VolSurface Implementation
```rust
pub struct VolSurface {
    id: CurveId,
    expiries: Box<[F]>,          // years
    strikes: Box<[F]>,           // absolute or moneyness
    vols: ndarray::Array2<F>,    // expiry × strike grid
    interp_exp: Box<dyn Interpolator>,
    interp_strk: Box<dyn Interpolator>,
}
```
* 2-D bilinear/ bicubic interpolation for `value(exp,strike)`.
* Alternate analytic representation (e.g., SABR params) gated behind `sabr` feature.

### 5.9 SwaptionSurface3D Implementation (Expiry × Tenor × Strike)
```rust
pub struct SwaptionSurface3D {
    id: CurveId,
    expiries: Box<[F]>,        // option expiries (years)
    tenors:   Box<[F]>,        // underlying swap tenors (years)
    strikes:  Box<[F]>,        // absolute or log-moneyness
    vols: ndarray::Array3<F>,  // shape: expiries × tenors × strikes
    interp_exp:  Box<dyn Interpolator>,
    interp_tenor:Box<dyn Interpolator>,
    interp_strk: Box<dyn Interpolator>,
}
```
* Supports `value(expiry, tenor, strike)` with trilinear / tricubic interpolation.
* Memory stored in `C`-contiguous order for cache locality when looping over strikes.
* For SABR-param surfaces the 3-D grid is replaced by analytic model parameters behind the `sabr` feature flag.

### 5.10 MarketContext / Multi-Curve (C-33) _updated_
```rust
pub struct MarketContext {
    disc: HashMap<CurveId, Arc<dyn DiscountCurve + Send + Sync>>,
    fwd:  HashMap<CurveId, Arc<dyn Curve + Send + Sync>>,  // forecast curves
    hazard: HashMap<CurveId, Arc<HazardCurve>>,
    inflation: HashMap<CurveId, Arc<InflationCurve>>,
    vol2d: HashMap<CurveId, Arc<VolSurface>>,              // cap/floor, equity, etc.
    swaption3d: HashMap<CurveId, Arc<SwaptionSurface3D>>,  // new 3-D swaption surfaces
    collat: HashMap<String, CurveId>,
}
```
* Convenience getters now include `swaption(id)`.

### 5.11 ForwardIndexCurve Implementation (C-18)
```rust
pub struct ForwardIndexCurve {
    id: CurveId,
    base: Date,
    reset_lag: i32,                 // days from fixing to spot
    day_count: DayCount,            // accrual convention
    knots: Box<[F]>,                // times in years
    fwd_rates: Box<[F]>,
    interp: Box<dyn Interpolator + Send + Sync>,
}
```
* `rate(t)` returns forward rate starting at `t` with tenor implied by curve definition (e.g., 3 M).  
* `rate_period(t1,t2)` computes compounded rate using day-count basis.  
* Supports simple, compounded and continuously-compounded conventions selected via builder.

### 5.12 Additional Shared Details
| Aspect | Design Choice |
|--------|---------------|
| **FactorKey Taxonomy** (C-19) | Planned keys: `Yield`, `FwdIdx`, `Hazard`, `Inflation`, `Vol2D`, `Swaption3D`, `Liquidity`. The enum is `non_exhaustive` for forward-compatibility. |
| **Collateral Mapping** (C-45) | `type CsaCode = &'static str`; `MarketContext::collateral(csa_code)` resolves to discount curve id via the `collat` map. |
| **Serialisation** (C-07) | Serde with `#[serde(tag = "type", version = 1)]`; adding fields bumps `version`. Unit tests assert round-trip compatibility. |
| **Thread-Safety** | All concrete curves stored behind `Arc<.. + Send + Sync>`; `Clone` is O(1). |
| **Error Handling** | Constructors return `Error::Input` for empty knots, non-sorted times, negative DF; runtime eval returns `Error::InterpOutOfBounds` when extrapolation not permitted. |
| **Bootstrapping Hook** | Each builder implements `bootstrap::Bootstrappable`, enabling calibration pipelines without coupling to curve internals. |

### 5.13 OptionTree Implementation
```rust
pub struct OptionTree {
    id: CurveId,
    steps: usize,                       // depth of lattice
    dt: F,                              // time step size
    nodes: Vec<F>,                      // packed node prices length = (steps+1)*(steps+2)/2
    up: F,                              // up factor per step (or vector for local vol trees)
    down: F,                            // down factor per step
    p: F,                               // risk-neutral prob per step (or vector)
}
```
* Provides `price_option(&self, expiry_step, payoff)` using backwards induction.
* Builders accept calibration quotes invoking `calibration::tree` utilities.
* Supports Cox-Ross-Rubinstein, Jarrow-Rudd, and Trigeorgis tri-lattice via strategy enum `TreeModel`.
* Serialisable behind `serde` flag; memory footprint ~ `O(N^2)` nodes.

### 5.14 RateTree Implementation
```rust
pub struct RateTree {
    id: CurveId,
    model: RateTreeModel,            // enum HoLee, BDT, HullWhite1F
    steps: usize,
    dt: F,
    short_rates: Vec<F>,             // packed lattice of short rates
    params: RateTreeParams,          // vol, mean-reversion, etc.
}
```
* Provides `discount_factor(t)` via path averaging; `price_bond`, `price_option` helpers.
* Calibrated by `calibration::tree_rate` against cap/floor or swaption vols.

### 5.15 CreditTree Implementation
```rust
pub struct CreditTree {
    id: CurveId,
    model: CreditTreeModel,          // JarrowTurnbull, DuffieSingleton
    steps: usize,
    dt: F,
    intensities: Vec<F>,             // λ nodes
    recovery: F,
}
```
* Returns survival probability and PV of credit instruments.
* Calibrated via `calibration::tree_credit` to CDS/par spreads.

## 6 Algorithms & Performance
1. **DF ↔ zero interpolation**: choose representation based on the selected
   `Interpolator` variant to avoid per-call exponentials where possible.
2. **Vectorised evaluation**: hot loops (e.g., `df` or `hazard` over thousands of dates) get `#[inline(always)]` and can auto-vectorise (SIMD) on the scalar-`F` configuration that uses `f64`.
3. **Cache-friendly search**: knots stored in ascending years; branchless binary search; potential AVX512 gather.
4. **2-D surface interpolation**: expiry/strike bilinear interpolation uses precomputed index hints for amortised O(1) lookup.
5. **Curve hashing** for memoisation: SHA-256 of `(id, base, knots, values)` used by risk engine (C-62).

## 7 Feature Flags
* `decimal128` – use `rust_decimal::Decimal` numeric type.
* `serde` – derive `Serialize/Deserialize` for FFI and persistence.
* `parallel` – enable Rayon `par_iter()` helpers for bulk evaluations.
* `sabr` – expose SABR analytic vol surfaces.

## 8 Integration Points
* **primitives**: `Currency`, `Money`, `Error`.
* **dates**: `Date` & year-fraction helpers.
* **cashflow**: uses `DiscountCurve` in NPV.
* **bootstrap**: curve builders consume market quotes, produce concrete curves.
* **risk**: risk bump engine operates on `MarketContext`.

## 9 Testing Strategy
* Golden-vector tests: compare DF / hazard / CPI / vol outputs vs QuantLib.
* Property tests: monotonicity (yield DF ↓ with time, hazard ≥0, CPI ↗).
* Criterion benches: 10 M `df` calls < 15 ms single-thread (f64 path).
* Criterion benches: 10 M `df` calls < 15 ms single-thread (scalar-`F` / f64 path).

## 10 Open Questions
1. Expose mutable knot editing for calibration?
2. Embed currency in `CurveId` or keep separate taxonomy?
3. Surface representation: grid vs analytic (SABR or SVI)?

## 11 Timeline
* **v0.1.0** – Traits, CurveId, Interpolator, YieldCurve (linear/log DF).
* **v0.2.0** – Hazard & Inflation curves; MarketContext infrastructure.
* **v0.3.0** – VolSurface (grid), advanced interp; SABR feature.
* **v1.0.0** – API freeze after bootstrap & risk metrics integration.

## 12 Monotone–Convex Interpolator (Hagan–West, 2006)
The **monotone-convex** discount-factor scheme ensures a continuously-differentiable
curve that is arbitrage-free by construction:

* Works in log-discount space \(y(t) = -\ln P(t)\).
* Slopes at knot points are chosen using the Fritsch–Carlson harmonic mean and
  then scaled so that \(\alpha_i^2 + \beta_i^2 \le 9\) which guarantees
  convexity of \(y(t)\) on segments where the secant slopes share a sign.
* Each interval stores cubic coefficients \((a,b,c,d)\).  Evaluation is therefore
  just a binary search plus four fused-multiply-add operations – practically as
  fast as the linear scheme but with superior smoothness.
* Exposed to callers via the `.monotone_convex()` builder helper and the
  `Interpolator::MonotoneConvex` variant.

---
*Last updated: 2025-07-13*