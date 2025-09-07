# Finstack v1 Refactoring Spec — **Final and Complete**

*(Analytics deferred to v2; calendars/registry live in **core::dates**; instruments split into **CDS** and **CDS Index**; cashflow spec + optionality fully defined.)*

---

## 0) Executive Summary

This specification defines a layered refactor of Finstack with clear, one‑directional dependencies:

* **Layer 0 — `finstack-core`**: foundational types, math, **dates module with calendars + registry (source of truth)**.
* **Layer 1 — `finstack-instruments`**: **pure data** instruments & quotes (includes **CashflowSpec + Call/Put optionality**, **CDS** and **CDS Index** as separate instruments, **CDS Tranche**, and **Credit Default Option**).
* **Layer 2 — `finstack-market-data`**: immutable `MarketContext`, curves, surfaces (incl. **base correlation** & **credit option vol**), indices, FX; **uses calendars from core**.
* **Layer 3 — `finstack-pricing`**: pricers, models, a single cashflow engine (consumes the unified `CashflowSpec`, emits exercise events), pricers for **CDS**, **CDS Index**, **CDS Tranche**, and **Credit Default Option**.
* **Layer 4 — `finstack-calibration`**: bootstraps & fittings (yield/multi-curve, hazard, inflation, FX, SABR/local vol, **base correlation**, optional **credit option vol**).
**Layer 5 — `finstack-analytics`** is deferred to v2 (portfolio, VaR, covenants engine, reporting).

A simple **registry‑based pricer dispatch** is used (erased‑trait approach) to keep instruments pure data while enabling open‑ended extension.

A compatibility bridge re‑exports `MarketContext` for one release:

```rust
// in finstack-core
pub use finstack_market_data::MarketContext as LegacyMarketContext;
```

---

## 1) Architecture & Dependencies

### Layered Design (v1)

```
┌─────────────────────────────────────┐
│  finstack-calibration (Layer 4)     │  ← Bootstraps & fitting (uses pricing)
├─────────────────────────────────────┤
│    finstack-pricing (Layer 3)       │  ← Pricers, models, single cashflow engine
├─────────────────────────────────────┤
│  finstack-market-data (Layer 2)     │  ← Context, curves, surfaces, indices, FX
├─────────────────────────────────────┤
│  finstack-instruments (Layer 1)     │  ← Pure instrument & quote data
├─────────────────────────────────────┤
│       finstack-core (Layer 0)       │  ← Types, math, **dates+calendars**
└─────────────────────────────────────┘
```

### Rules

* Dependencies flow **downward only**; no cycles.
* Instruments have **no** pricing/calibration logic.
* **Calendars and calendar registry live in `finstack-core::dates`**. All layers that need calendars use the core registry.
* `MarketContext` is immutable; bumping builds a **new** context.

---

## 2) Workspace & Features

**Workspace members**

* `finstack-core`
* `finstack-instruments`
* `finstack-market-data`
* `finstack-pricing`
* `finstack-calibration`
* *(v2 later: `finstack-analytics`)*

**Common features**

* `serde`
* `parallel` (deterministic parallel computations)
* `wasm` (restricted std + deterministic RNG)

---

## 3) Layer 0 — `finstack-core`

### Purpose

Foundational types, math, and the **dates subsystem with calendars & registry**.

### Module Layout

```
finstack-core/
└── src/
    ├── lib.rs
    ├── prelude.rs
    ├── ids.rs           # InstrumentId, CurveId, SurfaceId, IndexId, CreditIndexId, ScalarId, SeriesId
    ├── money.rs         # Money, Currency
    ├── time.rs          # Date, DateTime
    ├── conv.rs          # DayCount, BusinessDayConvention, Frequency
    ├── math/
    │   ├── interp.rs
    │   ├── roots.rs
    │   ├── stats.rs
    │   └── fp.rs        # stable reductions (pairwise/Kahan)
    └── dates/
        ├── mod.rs       # CalendarId, CalendarRegistry, business-day logic
        └── builtin.rs   # TARGET2, NYB, UK, etc.
```

### Key Points

* **CalendarId** and **CalendarRegistry** live here, with **builtin calendars**.
* All schedule generation / date adjustments anywhere in the stack call:

  ```rust
  use finstack_core::dates::{CalendarId, CalendarRegistry, BusinessDayConvention};
  let cal = CalendarRegistry::global().resolve(CalendarId::TARGET2()).unwrap();
  let adj_date = finstack_core::dates::adjust(date, BusinessDayConvention::ModifiedFollowing, &cal).unwrap();
  ```
* IDs:

  * Instruments: `InstrumentId`
  * Curves/Surfaces: `CurveId`, `SurfaceId`
  * Indices: `IndexId`, `CreditIndexId`
  * Scalars/Series: `ScalarId`, `SeriesId`

---

## 4) Layer 1 — `finstack-instruments`

### Purpose

**Pure data** definitions for instruments and quotes. Serializable with builders. No pricing logic.

### Layout

```
finstack-instruments/
└── src/
    ├── lib.rs
    ├── traits.rs                 # Identifiable, Attributable (data-only)
    ├── types.rs                  # Attributes bag
    ├── quotes/
    │   ├── mod.rs
    │   ├── quote_types.rs        # DepositQuote, FRAQuote, SwapQuote, BondQuote,
    │   │                         # CdsQuote, CdsIndexQuote, CdsTrancheQuote, CreditOptionQuote
    │   └── conversion.rs         # Conventions-aware *data* conversions
    ├── fixed_income/
    │   ├── mod.rs
    │   ├── cashflow/
    │   │   ├── mod.rs
    │   │   ├── spec.rs           # (see full definitions below)
    │   │   └── builder.rs        # CashflowSpecBuilder (leg/spec only)
    │   ├── deposit.rs
    │   ├── fra.rs
    │   ├── future.rs
    │   ├── swap.rs               # InterestRateSwap (+ SwapBuilder)
    │   ├── bond.rs               # Bond (+ BondBuilder)
    │   ├── cds.rs                # **Single-name CDS** (+ ProtectionSpec)
    │   ├── cds_index.rs          # **CDS Index** (basket of constituents)
    │   ├── tranche.rs            # **CDS Tranche** (on credit index)
    │   ├── loan/
    │   │   ├── mod.rs
    │   │   ├── term_loan.rs
    │   │   ├── revolver.rs
    │   │   └── ddtl.rs
    │   └── inflation/
    │       ├── mod.rs
    │       ├── inflation_swap.rs
    │       └── inflation_bond.rs
    ├── options/
    │   ├── mod.rs
    │   ├── equity_vanilla.rs         # European/American (v1 scope)
    │   └── credit_default_option.rs  # **Credit Default Option** (CDOtion)
    ├── equity/
    │   ├── mod.rs
    │   ├── stock.rs
    │   ├── index.rs
    │   └── etf.rs
    └── covenants/
        ├── mod.rs
        ├── covenant_spec.rs
        ├── breach.rs
        └── types.rs
```

### **Cashflow Spec & Related Structs (Full Definitions)**

```rust
use serde::{Serialize, Deserialize};
use finstack_core::prelude::*;
use finstack_core::dates::CalendarId;

/// Unified cashflow specification consumed by a single engine in pricing.
/// Instrument-level fields (notional, issue/maturity dates) live on instruments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashflowSpec {
    pub coupon: CouponSpec,
    pub amortization: AmortizationSpec,
    pub fees: Vec<FeeSpec>,
    pub day_count: DayCount,
    pub business_day_convention: BusinessDayConvention,
    pub calendar: CalendarId,
    pub optionality: Option<CallPutSchedule>,  // exercise logic handled by pricers
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CouponSpec {
    Fixed   { rate: F, frequency: Frequency },
    Floating{ index_id: CurveId, spread: F, frequency: Frequency },
    StepUp  { schedule: Vec<(Date, F)>, frequency: Frequency },
    Range   { floor: F, cap: F, reference: String, frequency: Frequency },
    PIK     { rate: F, frequency: Frequency },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AmortizationSpec {
    Bullet,
    Linear { target: Money },
    Custom { schedule: Vec<(Date, Money)> },
    PercentPerPeriod { percent: F },
    StepRemaining { schedule: Vec<(Date, Money)> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FeeSpec {
    Upfront    { amount: Money },
    Periodic   { bps: F, frequency: Frequency, base: FeeBase },
    Exit       { amount: Money },
    Commitment { bps: F, on_undrawn: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FeeBase { Outstanding, Original, Drawn, Undrawn }

/// Call/Put optionality data. Schedule generation emits events; pricers decide exercise.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallPutSchedule {
    pub call: Vec<CallOption>,
    pub put:  Vec<PutOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallOption {
    pub exercise_date: Date,
    pub strike_price: Money,             // clean price or price-equivalent notion
    pub make_whole_spread: Option<F>,    // optional
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PutOption {
    pub exercise_date: Date,
    pub strike_price: Money,
}
```

**Builder (leg/spec only)**

```rust
pub struct CashflowSpecBuilder {
    coupon: Option<CouponSpec>,
    amortization: AmortizationSpec,
    fees: Vec<FeeSpec>,
    day_count: Option<DayCount>,
    bdc: Option<BusinessDayConvention>,
    calendar: Option<CalendarId>,
    optionality: Option<CallPutSchedule>,
}

impl CashflowSpecBuilder {
    pub fn new() -> Self { /* defaults */ }
    pub fn coupon(mut self, c: CouponSpec) -> Self { self.coupon = Some(c); self }
    pub fn amortization(mut self, a: AmortizationSpec) -> Self { self.amortization = a; self }
    pub fn with_fees(mut self, f: Vec<FeeSpec>) -> Self { self.fees = f; self }
    pub fn day_count(mut self, dc: DayCount) -> Self { self.day_count = Some(dc); self }
    pub fn bdc(mut self, b: BusinessDayConvention) -> Self { self.bdc = Some(b); self }
    pub fn calendar(mut self, cal: CalendarId) -> Self { self.calendar = Some(cal); self }
    pub fn optionality(mut self, s: CallPutSchedule) -> Self { self.optionality = Some(s); self }
    pub fn build(self) -> Result<CashflowSpec, InstrumentError> { /* validate & return */ }
}
```

### **Key Instruments (Selected)**

**Bond** *(pure data; optionality inside its `CashflowSpec`)*

**InterestRateSwap** *(two legs with `CashflowSpec`)*

**CDS (Single-name)**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtectionSpec {
    pub accrual_on_default: bool,
    pub pay_on_default: bool,
    pub protection_start: Date,
    pub protection_end: Date,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditDefaultSwap {
    pub id: InstrumentId,
    pub reference_entity: String,
    pub effective_date: Date,
    pub maturity_date: Date,
    pub notional: Money,
    pub premium_leg: CashflowSpec,     // fixed-rate premium schedule
    pub protection_leg: ProtectionSpec,
    pub recovery_rate: F,
    pub disc_curve: CurveId,           // explicit curve wiring allowed
    pub credit_curve: CurveId,         // hazard curve id
    pub attributes: Attributes,
}
```

**CDS Index (basket of constituents)**

> Separate from single-name CDS; references the **basket** that composes the index (e.g., CDX/iTraxx).

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CdsConstituent {
    pub reference_entity: String,
    pub weight: F,                     // weight or notional share (sum to 1.0)
    pub fixed_recovery: Option<F>,     // if index uses fixed recovery per entity
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CdsIndex {
    pub id: InstrumentId,
    pub index_id: CreditIndexId,       // e.g., CDX_IG_S38 (series/version)
    pub constituents: Vec<CdsConstituent>,
    pub effective_date: Date,
    pub maturity_date: Date,
    pub notional: Money,
    pub premium_leg: CashflowSpec,     // index premium schedule
    pub disc_curve: CurveId,
    pub index_credit_curve: CurveId,   // aggregated hazard curve id
    pub attributes: Attributes,
}
```

**CDS Tranche (on index)**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CdsTranche {
    pub id: InstrumentId,
    pub index_id: CreditIndexId,
    pub attach: F,                     // e.g., 0.03
    pub detach: F,                     // e.g., 0.07
    pub effective_date: Date,
    pub maturity_date: Date,
    pub notional: Money,
    pub disc_curve: CurveId,
    pub index_credit_curve: CurveId,   // for index-level hazard
    pub base_corr_curve: CurveId,      // base correlation for tranche pricing
    pub attributes: Attributes,
}
```

**Credit Default Option (option on single-name or index CDS spread)**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnderlyingCds {
    SingleName { reference: String },
    Index { index_id: CreditIndexId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditDefaultOption {
    pub id: InstrumentId,
    pub underlying: UnderlyingCds,
    pub option_type: OptionType,       // Call/Put on spread
    pub exercise_style: ExerciseStyle, // v1: European; (American optional)
    pub strike: F,                     // strike spread (bps) or price
    pub maturity: Date,                // option expiry
    pub notional: Money,
    pub disc_curve: CurveId,
    pub credit_curve: CurveId,         // single-name or index aggregate curve
    pub vol_surface: Option<SurfaceId>,// credit option vol surface (Black)
    pub attributes: Attributes,
}
```

### Quotes & Conversions (always conventions‑explicit)

* `DepositQuote`, `FraQuote`, `SwapQuote`, `BondQuote`
* `CdsQuote` (single-name), `CdsIndexQuote`, `CdsTrancheQuote`, `CreditOptionQuote`

`conversion.rs` converts quotes to instruments using an explicit `Conventions` struct (no hard-coded calendars/BDC):

```rust
#[derive(Clone, Debug)]
pub struct Conventions {
    pub fixed_dc: DayCount,
    pub float_dc: DayCount,
    pub fixed_bdc: BusinessDayConvention,
    pub float_bdc: BusinessDayConvention,
    pub calendar: CalendarId,     // resolved through core::dates::CalendarRegistry at use sites
}
```

---

## 5) Layer 2 — `finstack-market-data`

### Purpose

Immutable `MarketContext` with all curves, surfaces, indices, FX, scalars/series. **Calendars are consumed from `finstack-core::dates`**.

### Layout

```
finstack-market-data/
└── src/
    ├── lib.rs
    ├── context.rs                 # MarketContext (immutable)
    ├── builder.rs                 # MarketContextBuilder
    ├── traits.rs                  # Discount, Forward, Hazard, Inflation, ...
    ├── primitives.rs              # MarketScalar, ScalarTimeSeries
    ├── term_structures/
    │   ├── discount_curve.rs
    │   ├── forward_curve.rs
    │   ├── hazard_curve.rs
    │   ├── inflation_curve.rs
    │   └── base_correlation.rs
    ├── surfaces/
    │   ├── vol_surface.rs         # rates/equity/fx vol
    │   ├── credit_vol_surface.rs  # credit option vol
    │   └── local_vol.rs
    ├── indices/
    │   ├── inflation_index.rs
    │   ├── equity_index.rs
    │   └── credit_index.rs        # CreditIndexData (num_constituents, recovery, etc.)
    ├── fx/
    │   ├── fx_matrix.rs
    │   └── fx_provider.rs
    ├── utils/
    │   ├── validation.rs
    │   └── forward.rs             # forward extraction traits (equity/FX/rates)
    └── bumping/
        ├── spec.rs                # BumpSpec (curves/surfaces/scalars)
        ├── bumped_curves.rs
        ├── scenarios.rs
        └── engine.rs
```

### MarketContext (uses core calendars internally where needed)

```rust
pub struct MarketContext {
    // Curves
    disc:      HashMap<CurveId, Arc<dyn Discount + Send + Sync>>,
    fwd:       HashMap<CurveId, Arc<dyn Forward  + Send + Sync>>,
    hazard:    HashMap<CurveId, Arc<HazardCurve>>,
    infl:      HashMap<CurveId, Arc<InflationCurve>>,
    base_corr: HashMap<CurveId, Arc<BaseCorrelationCurve>>,

    // Surfaces
    vols:        HashMap<SurfaceId, Arc<VolSurface>>,
    credit_vols: HashMap<SurfaceId, Arc<CreditVolSurface>>,

    // Indices
    infl_idx: HashMap<IndexId, Arc<InflationIndex>>,
    eq_idx:   HashMap<IndexId, Arc<EquityIndex>>,
    cr_idx:   HashMap<CreditIndexId, Arc<CreditIndexData>>,

    // FX
    fx: Option<Arc<FxMatrix>>,

    // Scalars/Series
    prices: HashMap<ScalarId, MarketScalar>,
    series: HashMap<SeriesId, ScalarTimeSeries>,

    // Collateral (CSA -> discount curve)
    csa_discount: HashMap<&'static str, CurveId>,

    // Metadata
    as_of_date: Date,
    market_close_time: Option<DateTime>,
}
```

### Builder & Bumping

```rust
pub struct MarketContextBuilder { /* partial maps + defaults */ }
impl MarketContextBuilder {
    pub fn new(as_of: Date) -> Self { /* ... */ }
    pub fn with_discount<T: Discount + Send + Sync + 'static>(self, id: CurveId, c: T) -> Self { /* ... */ }
    pub fn with_forward<T: Forward  + Send + Sync + 'static>(self, id: CurveId, c: T) -> Self { /* ... */ }
    pub fn with_hazard(self, id: CurveId, c: HazardCurve) -> Self { /* ... */ }
    pub fn with_base_correlation(self, id: CurveId, bc: BaseCorrelationCurve) -> Self { /* ... */ }
    pub fn with_vol_surface(self, id: SurfaceId, s: VolSurface) -> Self { /* ... */ }
    pub fn with_credit_vol_surface(self, id: SurfaceId, s: CreditVolSurface) -> Self { /* ... */ }
    pub fn with_index_credit(self, id: CreditIndexId, idx: CreditIndexData) -> Self { /* ... */ }
    pub fn with_scalar(self, id: ScalarId, s: MarketScalar) -> Self { /* ... */ }
    pub fn with_series(self, id: SeriesId, s: ScalarTimeSeries) -> Self { /* ... */ }
    pub fn with_collateral_map(self, csa: &'static str, disc_id: CurveId) -> Self { /* ... */ }
    pub fn build(self) -> MarketContext { /* validate + freeze */ }
}

pub enum BumpSpec {
    ParallelBps(i32),      // curves
    SpreadBps(i32),        // credit curves
    Multiplier(F),         // prices/vols
    InflationPct(F),
    CorrelationBps(i32),   // base correlation
    CreditVolPct(F),       // credit option vol
}
```

> **Calendars:** When schedule logic is required inside market-data utilities, they resolve calendars via `finstack_core::dates::CalendarRegistry` (no local calendar impl).

---

## 6) Layer 3 — `finstack-pricing`

### Purpose

Pricing models and instrument pricers + a **single cashflow engine** consuming `CashflowSpec` (with optionality events). Deterministic parallelism.

### Layout

```
finstack-pricing/
└── src/
    ├── lib.rs
    ├── error.rs                    # PricingError
    ├── traits.rs                   # Pricer<T>, MetricCalculator
    ├── results.rs                  # PricingResult, ResultsMeta
    ├── registry.rs                 # PricingRegistry (erased-pricer)
    ├── models/
    │   ├── black_scholes.rs
    │   ├── black.rs
    │   ├── garman_kohlhagen.rs
    │   ├── sabr.rs
    │   ├── trees/
    │   │   ├── binomial.rs
    │   │   └── trinomial.rs
    │   └── monte_carlo/
    │       ├── path_generator.rs
    │       ├── processes.rs
    │       └── random.rs           # splittable RNG
    ├── cashflow_engine/
    │   ├── generator.rs            # single engine (fees/amort/option events)
    │   └── events.rs               # CashflowEvent incl. Call/Put schedule events
    ├── fixed_income/
    │   ├── deposit.rs
    │   ├── fra.rs
    │   ├── swap.rs
    │   ├── bond.rs                 # callable/putable via trees/FD
    │   ├── cds.rs                  # **single-name CDS pricer**
    │   ├── cds_index.rs            # **CDS Index pricer**
    │   ├── tranche.rs              # **CDS Tranche pricer (base correlation)**
    │   └── loan.rs
    ├── options/
    │   ├── equity_vanilla.rs
    │   └── credit_default_option.rs # **Credit Default Option pricer (Black/structural)**
    └── metrics/
        ├── fixed_income/
        │   ├── yield.rs
        │   ├── spread.rs
        │   ├── price.rs
        │   └── duration.rs
        └── options/
            └── greeks.rs
```

### API Highlights

**PricingMethod & Results**

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PricingMethod {
    Auto,
    Analytical,
    MonteCarlo { paths: usize, seed: Option<u64> },  // default seed applied if None
    BinomialTree { steps: usize },
    TrinomialTree { steps: usize },
    FiniteDifference { grid_points: usize },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultsMeta {
    pub numeric_mode: String,
    pub parallel: bool,
    pub rng_seed: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingResult {
    pub value: Money,
    pub as_of: Date,
    pub method_used: PricingMethod,
    pub meta: ResultsMeta,
    pub calculation_time: std::time::Duration,
    pub warnings: Vec<String>,
}
```

**Pricer Trait & Registry**

```rust
pub trait Pricer<T> {
    fn price(&self, instrument: &T, ctx: &MarketContext, as_of: Date, method: PricingMethod)
        -> Result<PricingResult, PricingError>;
    fn method_hint(&self, instrument: &T) -> PricingMethod;
}

// Erased-pricer registry centralizes downcast at a single, audited point.
pub struct PricingRegistry { /* ... */ }
impl PricingRegistry {
    pub fn new() -> Self { /* ... */ }
    pub fn register<T: 'static, P>(&mut self, pricer: P)
    where P: Pricer<T> + Send + Sync + 'static { /* ... */ }
    pub fn register_defaults(&mut self) {
        // v1 coverage across core instruments
        self.register::<EquityVanillaOption, _>(EquityVanillaPricer::default());
        self.register::<Bond, _>(BondPricer::default());
        self.register::<InterestRateSwap, _>(SwapPricer::default());
        self.register::<CreditDefaultSwap, _>(CdsPricer::default());       // single-name
        self.register::<CdsIndex, _>(CdsIndexPricer::default());           // index
        self.register::<CdsTranche, _>(CdsTranchePricer::default());       // tranche
        self.register::<CreditDefaultOption, _>(CreditDefaultOptionPricer::default());
        // + deposits, FRAs, futures, loans, inflation instruments, etc.
    }
    pub fn price<T: 'static>(&self, instrument: &T, ctx: &MarketContext, as_of: Date)
        -> Result<PricingResult, PricingError> { /* Auto */ }
    pub fn price_with_method<T: 'static>(&self, instrument: &T, ctx: &MarketContext, as_of: Date, method: PricingMethod)
        -> Result<PricingResult, PricingError> { /* override */ }
}
```

**Determinism & Parallelism**

* MC defaults to `Some(0)` seed if `None`; splittable RNG yields identical serial/parallel results.
* Cashflow engine sorts events by `(date, kind)`; stable FP reductions (pairwise/Kahan).
* `ResultsMeta` captures `parallel` and `rng_seed`.

**Method Hints (examples)**

* Equity vanilla: European → Analytical; American → Binomial(≥200).
* Bond: no optionality → Analytical; callable/putable → Tree/FD.
* CDS (SN & Index): Analytical (hazard-based) default; MC fallback.
* CDS Tranche: Base‑correlation analytical/integration default; MC fallback if needed.
* Credit Default Option: Analytical (Black on spread using credit vol) default; structural/MC optional.

---

## 7) Layer 4 — `finstack-calibration`

### Purpose

Calibrate curves/surfaces using **pricing** (no duplicated formulas): yield/multi-curve, hazard (single‑name & index), inflation, FX forward, vol (SABR/local), **base correlation**, optional **credit option vol**.

### Layout

```
finstack-calibration/
└── src/
    ├── lib.rs
    ├── traits.rs                  # Calibrator<TQuote, TArtifact>
    ├── config.rs                  # CalibrationConfig
    ├── report.rs                  # CalibrationReport
    ├── error.rs
    ├── primitives.rs              # HashableFloat, constraints
    ├── bootstrap/
    │   ├── yield_curve.rs
    │   ├── multi_curve.rs
    │   ├── hazard_curve.rs        # single-name & index
    │   ├── inflation_curve.rs
    │   └── fx_curve.rs
    ├── surface/
    │   ├── vol_surface.rs
    │   ├── sabr.rs
    │   └── local_vol.rs
    ├── correlation/
    │   └── base_correlation.rs    # fit base correlation to tranche quotes
    ├── credit_vol/
    │   └── credit_opt_surface.rs  # fit credit option vol to option quotes (optional)
    ├── common/
    │   ├── grouping.rs
    │   ├── identifiers.rs
    │   └── time.rs
    └── orchestrator.rs
```

### Configuration (deterministic)

```rust
#[derive(Clone, Debug)]
pub struct CalibrationConfig {
    pub tolerance: F,             // e.g., 1e-12
    pub max_iterations: usize,    // e.g., 200
    pub use_parallel: bool,       // deterministic
    pub random_seed: Option<u64>, // default Some(0) propagated to pricing
    pub verbose: bool,
}
impl Default for CalibrationConfig {
    fn default() -> Self { Self { tolerance: 1e-12, max_iterations: 200, use_parallel: true, random_seed: Some(0), verbose: false } }
}
```

### Process

* Use builders for provisional curves/surfaces; **rebuild a fresh `MarketContext`** for each iteration.
* For **hazard**: support bootstraps from **CdsQuote** (single-name) and **CdsIndexQuote** (index par spreads).
* For **base correlation**: fit to **CdsTrancheQuote** set.
* For **credit option vol**: fit to **CreditOptionQuote** set (Black on spread).
* Reports return residuals/iterations/timings; artifacts returned for insertion into `MarketContextBuilder`.

---

## 8) Testing, Performance, Determinism

* Unit tests per crate; integration tests across layers.
* Golden tests for IRS, bonds (incl. callable), CDS (single-name & index), tranche, credit options.
* Deterministic MC (fixed seed default + splittable RNG) validated serial vs parallel.
* Benchmarks: cashflow gen, IRS/bond/CDS/tranche/credit option, bootstrap steps.
* CI enforces tolerances, no regressions.

---

## 9) Migration Plan

**Phase 1 — Instruments**

1. Add `finstack-instruments`; move instruments & quotes.
2. Implement **CashflowSpec + CallPutSchedule** and **CashflowSpecBuilder** (leg/spec only).
3. Split credit: add **CreditDefaultSwap** (single-name), **CdsIndex** (basket), **CdsTranche**, **CreditDefaultOption**.
4. Per-instrument builders; serde & validation tests.

**Phase 2 — Market Data**
5\. Add `finstack-market-data`; extract curves/surfaces/indices; **no calendars here**.
6\. Add **base correlation** & **credit option vol** surfaces.
7\. Implement `MarketContextBuilder`, bumping, forward utils.
8\. In `finstack-core`, `pub use finstack_market_data::MarketContext as LegacyMarketContext` for one release.

**Phase 3 — Pricing**
9\. Create `finstack-pricing`: models, single cashflow engine (emits call/put events), pricers for all v1 instruments **including CDS, CDS Index, Tranche, Credit Default Option**.
10\. Determinism policies; `PricingRegistry::register_defaults`.
11\. Deprecate legacy “engines”; shim to pricers with warnings.

**Phase 4 — Calibration**
12\. Move bootstraps/fittings; add index hazard, base correlation, credit option vol.
13\. Orchestrator & reports; tests/benchmarks.

**Phase 5 — Bindings**
14\. Python (PyO3 + Pydantic v2), WASM (TS types); deterministic RNG.

**Phase 6 — Docs & Deprecation**
15\. Update docs/examples; performance testing & profiling.
16\. Deprecate legacy `valuations`; publish migration guide & timeline.

---

## 10) Example Snippets (v1‑only; calendars from core)

### Callable Bond Spec using Core Calendars

```rust
use finstack_core::prelude::*;
use finstack_core::dates::{CalendarId, CalendarRegistry};
use finstack_instruments::fixed_income::cashflow::{
    builder::CashflowSpecBuilder,
    spec::{CouponSpec, AmortizationSpec, FeeSpec, CallPutSchedule, CallOption},
};
use finstack_instruments::fixed_income::bond::BondBuilder;

let call_sched = CallPutSchedule {
    call: vec![ CallOption {
        exercise_date: date(2029,1,1),
        strike_price: Money::of(102.0, Currency::USD),
        make_whole_spread: Some(0.005),
    }],
    put: vec![],
};

let spec = CashflowSpecBuilder::new()
    .coupon(CouponSpec::Fixed { rate: 0.04, frequency: Frequency::SemiAnnual })
    .amortization(AmortizationSpec::Bullet)
    .with_fees(vec![ FeeSpec::Upfront { amount: Money::of(500_000.0, Currency::USD) } ])
    .day_count(DayCount::Thirty360)
    .bdc(BusinessDayConvention::ModifiedFollowing)
    .calendar(CalendarId::TARGET2())
    .optionality(call_sched)
    .build()?;

// calendar usage (core):
let _cal = CalendarRegistry::global().resolve(CalendarId::TARGET2())?;

let bond = BondBuilder::new(InstrumentId::from_static("BOND_1"))
    .issue_date(date(2024,1,1))
    .maturity_date(date(2034,1,1))
    .notional(Money::of(100_000_000.0, Currency::USD))
    .cashflow_spec(spec)
    .issue_price(Money::of(100.0, Currency::USD))
    .build()?;
```

### CDS Index (basket) and Tranche Pricing

```rust
use finstack_core::prelude::*;
use finstack_instruments::fixed_income::cds_index::{CdsIndex, CdsConstituent};
use finstack_instruments::fixed_income::tranche::CdsTranche;
use finstack_market_data::MarketContextBuilder;
use finstack_pricing::PricingRegistry;

// Build market context (disc/hazard/base corr/vol surfaces omitted)
let ctx = MarketContextBuilder::new(date(2025,3,20)).build();

let idx = CdsIndex {
    id: InstrumentId::from_static("CDX_IG_5Y"),
    index_id: CreditIndexId::from_static("CDX_IG_S38"),
    constituents: vec![
        CdsConstituent { reference_entity: "Acme Corp".into(), weight: 1.0/125.0, fixed_recovery: Some(0.4) },
        // ... 124 more
    ],
    effective_date: date(2025,3,20),
    maturity_date: date(2030,6,20),
    notional: Money::of(125_000_000.0, Currency::USD),
    premium_leg: /* CashflowSpec for index premiums */,
    disc_curve: CurveId::from_static("USD_DISC"),
    index_credit_curve: CurveId::from_static("CDX_IG_HAZARD"),
    attributes: Default::default(),
};

let tr = CdsTranche {
    id: InstrumentId::from_static("TR_IG_3_7"),
    index_id: CreditIndexId::from_static("CDX_IG_S38"),
    attach: 0.03, detach: 0.07,
    effective_date: date(2025,3,20),
    maturity_date: date(2030,6,20),
    notional: Money::of(100_000_000.0, Currency::USD),
    disc_curve: CurveId::from_static("USD_DISC"),
    index_credit_curve: CurveId::from_static("CDX_IG_HAZARD"),
    base_corr_curve: CurveId::from_static("CDX_IG_BC"),
    attributes: Default::default(),
};

let mut reg = PricingRegistry::new(); reg.register_defaults();
let pv_idx = reg.price(&idx, &ctx, date(2025,3,20))?;
let pv_tr  = reg.price(&tr , &ctx, date(2025,3,20))?;
```

---

## 11) Policy Notes & Conventions

* **Calendars**: The only calendar implementation and registry live in **`finstack-core::dates`**. All crates use this registry. No ad‑hoc strings.
* **Quotes → Instruments**: All conversions require explicit `Conventions` (dc/bdc/calendar).
* **Instrument coupling**: Curve/surface IDs may be present for explicit wiring; pricers may support context‑policy defaults when IDs are absent (document defaults).
* **Import hygiene**: No `Priceable` on instruments; all pricing through `PricingRegistry`.

---

## 12) Rationale for the Erased‑Pricer Registry (brief)

* Keeps **instruments as pure data** (serde‑friendly).
* Enables **open‑world** extension (register pricers for new instrument types without changing central enums/visitors).
* Centralizes type erasure in one audited module with clear errors (`NoPricerRegisteredFor(type)`).

**Alternatives** (enum/visitor/Priceable trait/static generics) either close the world, push pricing into instruments, or complicate dynamic portfolios. Given goals, the registry is the best fit.

---

## 13) Deliverables & Success Criteria

* **Architecture:** One‑directional deps; immutable `MarketContext`; single cashflow engine; optionality in spec; CDS vs CDS Index split; tranche & credit option supported.
* **Pricing:** Default pricers for deposits, FRAs, futures, IRS, bonds (incl. callable/putable), CDS (SN & Index), **CDS Tranche**, **Credit Default Option**, loans, inflation instruments.
* **Calibration:** Yield, multi‑curve, hazard (SN & index), inflation, FX, SABR/local vol, **base correlation**, optional **credit option vol**.
* **Calendars:** Solely from `core::dates` across the stack.
* **Determinism:** Serial == parallel (within tolerance); MC fixed‑seed default; metadata recorded.
* **Performance:** No regressions; benches published.
* **Usability:** Builders, explicit conventions, clear examples; migration guide.
* **Docs & Tests:** READMEs, API docs, unit/integration/golden tests, CI thresholds.

---

## 14) Open Type Stubs (declared in their crates)

* **Instruments**: `ProtectionSpec`, `CdsConstituent`, `CdsIndex`, `CdsTranche`, `CreditDefaultOption`, `CallPutSchedule` and related enums shown above.
* **Quotes**: `CdsQuote`, `CdsIndexQuote`, `CdsTrancheQuote`, `CreditOptionQuote`, `SwapQuoteType`, etc.
* **Market‑data**: `CreditIndexData`, `BaseCorrelationCurve`, `CreditVolSurface`.
* **Pricing**: `PricingError`, `ResultsMeta`, `CashflowEvent`.
* **Calibration**: `CalibrationReport`, `CalibError`.

---

## 15) Conclusion

This finalized v1 spec delivers a clean, extensible architecture that:

* Uses **core calendars and registry** everywhere (single source of truth).
* Keeps **instruments pure data**, with **CashflowSpec** + **Call/Put optionality** explicitly modeled.
* Splits **CDS** and **CDS Index** instruments (index contains its **basket**), and supports **CDS Tranche** and **Credit Default Option** end‑to‑end (instruments, pricers, calibration).
* Provides deterministic, testable pricing with an ergonomic, extensible **registry**.

The migration plan, examples, and module breakdowns are complete and ready for implementation.
