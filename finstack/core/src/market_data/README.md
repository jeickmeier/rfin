## Market Data Module (core)

The `market_data` module in `finstack-core` provides the **core infrastructure for yield curves, credit curves, volatility surfaces, FX, and scalar market data** used across valuations, scenarios, and portfolios.

- **Term structures**: one-dimensional curves for discount factors, forward rates, credit hazard rates, and inflation.
- **Surfaces**: two-dimensional volatility surfaces indexed by expiry and strike.
- **Scalars & time series**: spot prices, FX rates, indices, and generic scalar time series.
- **Market context**: `MarketContext` as the central, thread-safe container for all market data.
- **Scenario/risk utilities**: bumping APIs (`bumps.rs`) and shift measurement utilities (`diff.rs`).
- **Dividends**: shared dividend schedules for equity and ETF valuations.

The module is designed to be **deterministic**, **type-safe**, and **serde-stable** (under the `serde` feature), forming the backbone for the higher-level `valuations`, `scenarios`, and `portfolio` crates.

---

## Module Structure

- **`mod.rs`**
  - Public entrypoint for the market data module.
  - Documents high-level concepts (discount/forward/hazard/inflation curves, vol surfaces, scalars, `MarketContext`).
  - Re-exports:
    - Submodules: `bumps`, `context`, `diff`, `dividends`, `scalars`, `surfaces`, `term_structures`, `traits`.
    - Helpers: `math::interp::utils::validate_knots`.
    - Dividend schedule types for ergonomic access.

- **`context.rs`**
  - Defines:
    - `MarketContext`: central registry for market data; cheap to clone (Arc-based), builder-style insert APIs (`insert_discount`, `insert_forward`, `insert_surface`, `insert_price`, `insert_fx`, etc.), and type-safe getters (`get_discount`, `surface`, `price`, `series`, `collateral`, …).
    - `CurveStorage`: enum wrapper for heterogeneous curve storage (`Discount`, `Forward`, `Hazard`, `Inflation`, `BaseCorrelation`) with helpers like `curve_type()` and type filters.
    - `ContextStats`: lightweight statistics struct returned by `MarketContext::stats()`.
  - Scenario helpers:
    - `MarketContext::bump` and `MarketContext::apply_bumps` integrate with `BumpSpec` / `MarketBump` to build shocked contexts.
    - `MarketContext::roll_forward` implements constant-curve roll-down scenarios.
    - `MarketContext::bump_fx_spot` and `MarketContext::apply_bumps` provide FX bump support via `FxMatrix`.
  - Serialization (behind `serde` feature):
    - `CurveState`: tagged enum for serializing any curve type.
    - `CreditIndexState` and `MarketContextState`: canonical DTOs for persisting complete context snapshots.
    - `Serialize`/`Deserialize` implementations for `CurveStorage` and `MarketContext` round-trip through the `*State` DTOs.

- **`term_structures/`**
  - `mod.rs`: documentation and re-exports for all curve types.
  - `discount_curve.rs`: discount factor curves (`DiscountCurve`) implementing:
    - `TermStructure` + `Discounting` traits.
    - Builder pattern with `base_date`, `day_count`, `knots`, `set_interp`, and extrapolation controls.
  - `forward_curve.rs`: forward-rate curves (`ForwardCurve`) with tenor-aware builders (e.g., 3M forward) and knot-based interpolation.
  - `hazard_curve.rs`: credit hazard/survival curves (`HazardCurve`) with survival/probability helpers; used for credit pricing.
  - `inflation.rs`: real/breakeven inflation term structures (`InflationCurve`) built from CPI levels.
  - `credit_index.rs`: credit index aggregates (`CreditIndexData`) referencing component hazard and base correlation curves.
  - `base_correlation.rs`: base correlation curves for tranche pricing (`BaseCorrelationCurve`).
  - All curve types:
    - Use validated knot sets (via `validate_knots`) and pluggable interpolation (`InterpStyle`).
    - Implement `TermStructure` and domain-specific traits from `traits.rs` where appropriate.
    - Support serde via `*State` DTOs when the `serde` feature is enabled.

- **`surfaces/`**
  - `mod.rs`: documentation and re-export of `VolSurface`.
  - `vol_surface.rs`: bilinear volatility surface implementation:
    - `VolSurface::builder` with `expiries`, `strikes`, and per-row volatility grids.
    - Evaluation helpers (e.g., `value(expiry, strike)`) and bucket bumping (`apply_bucket_bump`) for scenario work.

- **`scalars/`**
  - `mod.rs`: documentation and re-exports.
  - `primitives.rs`:
    - `MarketScalar`: enum for single-value market observables (e.g., equity spot, FX rate, generic scalar).
    - `ScalarTimeSeries`: generic `(Date, f64)` time series with optional interpolation (`SeriesInterpolation`) and metadata.
  - `inflation_index.rs`:
    - `InflationIndex`: CPI/RPI time series with lag/interpolation support.
  - `storage.rs`:
    - Internal storage for time series; not typically used directly by consumers.

- **`bumps.rs`**
  - Scenario bump specification types:
    - `BumpMode` (additive vs multiplicative).
    - `BumpUnits` (basis points, percent, fraction, factor).
    - `BumpType` (`Parallel`, `TriangularKeyRate` with explicit bucket neighbors).
    - `BumpSpec`: unified bump description (mode, units, value, type) with helpers like:
      - `BumpSpec::parallel_bp`, `BumpSpec::triangular_key_rate_bp`.
      - Domain-specific helpers (`inflation_shift_pct`, `correlation_shift_pct`, `multiplier`).
    - `MarketBump`: heterogeneous bump enum for curves, FX, volatility buckets, and base correlation buckets.
  - Integrates with curve/surface/scalar types via internal `Bumpable` traits.

- **`diff.rs`**
  - Market shift measurement helpers between two `MarketContext` instances:
    - `TenorSamplingMethod` (`Standard`, `Dynamic`, `Custom`) controls sampling points along a curve.
    - `measure_discount_curve_shift` and `measure_bucketed_discount_shift` for rate shifts in basis points.
    - Additional helpers for hazard spreads and volatility surfaces (P&L attribution and risk reporting).
  - Used primarily for P&L attribution, risk reports (DV01/CS01-style metrics), and calibration diagnostics.

- **`dividends.rs`**
  - Shared dividend schedule types (`DividendSchedule`, cash/yield/stock events) keyed by `CurveId`.
  - Integrated with `MarketContext` via `insert_dividends` / `dividend_schedule` helpers.

- **`traits.rs`**
  - Minimal trait surface for polymorphism:
    - `TermStructure`: base trait with `id() -> &CurveId`.
    - `Discounting`: discount curve abstraction with `base_date`, `df(t)`, and a default `day_count`.
    - `Forward`: forward-rate abstraction with `rate(t)` and `rate_period(t1, t2)`.
    - `Survival`: hazard/survival abstraction with `sp(t)` for survival probabilities.
  - The traits are intentionally small; most functionality lives on concrete curve types for discoverability and performance.

---

## Core Concepts and Types

### Term Structures and Surfaces

- **Discount curves (`DiscountCurve`)**
  - Map year fractions from a base date to discount factors.
  - Provide helpers for zero rates, forwards, and rolling.
  - Implement `TermStructure` + `Discounting` traits.
- **Forward curves (`ForwardCurve`)**
  - Represent simple or period-averaged forward rates over a tenor.
  - Builder specifies base date, tenor, day count, and knot values.
- **Hazard curves (`HazardCurve`)**
  - Encode credit hazard rates and survival probabilities.
  - Used by credit pricers in `valuations`.
- **Inflation curves (`InflationCurve`)**
  - Built from CPI levels and base CPI, enabling real/nominal conversions.
- **Base correlation and credit index data**
  - `BaseCorrelationCurve` plus `CreditIndexData` model tranche correlation and index-level credit data.
- **Volatility surfaces (`VolSurface`)**
  - Two-dimensional matrices of implied vols by expiry and strike.
  - Builder validates grid dimensions and supports bilinear interpolation and bucket-level bumps.

All curve and surface types:

- Use year-fraction time coordinates backed by `dates::DayCount`.
- Validate knots and grid structure up-front.
- Support serde under the `serde` feature via `*State` DTOs or direct derives.

### Scalars and Time Series

- **`MarketScalar`**
  - Enum wrapper for single-value market observables (e.g., equity spot, FX rate, index level).
  - Designed to integrate with `Money` and `Currency` for currency-safe arithmetic.
- **`ScalarTimeSeries`**
  - Generic `(Date, f64)` time series with interpolation (`SeriesInterpolation`) and optional metadata.
  - Used for things like historical vol, macro series, and generic market history.
- **`InflationIndex`**
  - CPI/RPI-style index with:
    - Observations as `(Date, level)` pairs.
    - Configurable interpolation (e.g., linear).
    - Currency tagging and lag/seasonality support.

These types are stored inside `MarketContext` under `prices`, `series`, and `inflation_indices`.

### MarketContext

`MarketContext` is the **central container for all market data** used in a valuation run:

- **Builder-style inserts**
  - Curves: `insert_discount`, `insert_forward`, `insert_hazard`, `insert_inflation`, `insert_base_correlation`.
  - Surfaces: `insert_surface`.
  - Scalars & time series: `insert_price`, `insert_series`.
  - Inflation indices: `insert_inflation_index`.
  - Credit indices: `insert_credit_index`.
  - FX: `insert_fx`.
  - Collateral: `map_collateral` (CSA code → discount curve ID).
  - Dividends and market history: `insert_dividends`, `insert_market_history`.
- **Type-safe getters**
  - Curves: `get_discount`, `get_forward`, `get_hazard`, `get_inflation`, `get_base_correlation` (and `_ref` borrowing variants).
  - Surfaces and indices: `surface`, `surface_ref`, `price`, `series`, `inflation_index`, `inflation_index_ref`, `dividend_schedule`, `credit_index`, `credit_index_ref`, `collateral`, `collateral_ref`.
  - Introspection: `curve_ids`, `curves_of_type`, `count_by_type`, `stats`, `is_empty`, `total_objects`.
- **Scenario support**
  - `bump` for curve/surface/price/time-series bumps keyed by `CurveId`.
  - `apply_bumps` for heterogeneous `MarketBump` lists (including FX and bucket-level shifts).
  - `roll_forward(days)` for constant-curve roll-down scenarios.
  - `bump_fx_spot` for FX-specific percentage bumps (via `FxMatrix`).
- **Serialization**
  - Under the `serde` feature, `MarketContext` serializes via `MarketContextState` with stable field names:
    - `curves`, `surfaces`, `prices`, `series`, `inflation_indices`, `credit_indices`, `collateral`.
  - `MarketContextState` is the canonical wire shape for Python/WASM bindings and long-lived storage.

---

## Usage Examples

### Build a Simple MarketContext with Curves

```rust
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
};
use finstack_core::math::interp::InterpStyle;
use time::macros::date;

let base = date!(2025 - 01 - 01);

let disc = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (5.0, 0.88)])
    .set_interp(InterpStyle::MonotoneConvex)
    .build()
    ?;

let fwd3m = ForwardCurve::builder("USD-SOFR3M", 0.25)
    .base_date(base)
    .knots([(0.0, 0.03), (5.0, 0.04)])
    .set_interp(InterpStyle::Linear)
    .build()
    ?;

let hazard = HazardCurve::builder("USD-CRED")
    .base_date(base)
    .knots([(0.0, 0.01), (10.0, 0.015)])
    .build()
    ?;

let ctx = MarketContext::new()
    .insert_discount(disc)
    .insert_forward(fwd3m)
    .insert_hazard(hazard);

assert!(ctx.get_discount("USD-OIS").is_ok());
assert!(ctx.get_forward("USD-SOFR3M").is_ok());
assert!(ctx.get_hazard("USD-CRED").is_ok());
# Ok::<(), finstack_core::Error>(())
```

### Add Scalars, Time Series, and Inflation Indices

```rust
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::{
    MarketScalar,
    ScalarTimeSeries,
    SeriesInterpolation,
    InflationIndex,
};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use time::macros::date;

let spot = MarketScalar::Price(Money::new(101.5, Currency::USD));

let ts = ScalarTimeSeries::new(
    "US-CPI-TS",
    vec![
        (date!(2024 - 01 - 31), 100.0),
        (date!(2024 - 02 - 29), 101.0),
    ],
    None,
)?
.with_interpolation(SeriesInterpolation::Linear);

let index = InflationIndex::new(
    "US-CPI",
    vec![
        (date!(2024 - 01 - 31), 100.0),
        (date!(2024 - 02 - 29), 101.0),
    ],
    Currency::USD,
)?
// configure interpolation/lag as needed
;

let ctx = MarketContext::new()
    .insert_price("AAPL", spot)
    .insert_series(ts)
    .insert_inflation_index("US-CPI", index);

// Lookups are type-safe and validated
let price = ctx.price("AAPL")?;
let series = ctx.series("US-CPI-TS")?;
let cpi = ctx.inflation_index("US-CPI").expect("Inflation index present");

assert!(matches!(price, MarketScalar::Price(_)));
assert_eq!(series.id().as_str(), "US-CPI-TS");
assert_eq!(cpi.id, "US-CPI");
# Ok::<(), finstack_core::Error>(())
```

### Apply Parallel and Key-Rate Bumps

```rust
use finstack_core::collections::HashMap;
use finstack_core::market_data::context::{MarketContext, BumpSpec};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::types::CurveId;
use time::macros::date;

let base = date!(2025 - 01 - 01);
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (5.0, 0.9)])
    .build()
    ?;

let ctx = MarketContext::new().insert_discount(curve);

// 100bp parallel bump
let mut bumps = HashMap::default();
bumps.insert(CurveId::from("USD-OIS"), BumpSpec::parallel_bp(100.0));

let bumped = ctx.bump(bumps)?;
let bumped_curve = bumped.get_discount("USD-OIS")?;

assert_eq!(bumped_curve.id(), &CurveId::from("USD-OIS"));
# Ok::<(), finstack_core::Error>(())
```

For heterogeneous scenarios (curves, FX, vol buckets, base correlation), build a list of `MarketBump` and call `MarketContext::apply_bumps`.

### Measure Market Shifts Between Contexts

```rust
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::diff::{measure_discount_curve_shift, TenorSamplingMethod};
use finstack_core::types::CurveId;

fn measure_shift(market_t0: MarketContext, market_t1: MarketContext) -> finstack_core::Result<f64> {
    let shift_bp = measure_discount_curve_shift(
        &CurveId::from("USD-OIS"),
        &market_t0,
        &market_t1,
        TenorSamplingMethod::Standard,
    )?;

    println!("USD-OIS moved {shift_bp} basis points");
    Ok(shift_bp)
}
```

Use `TenorSamplingMethod::Dynamic` or `Custom` when you need knot-aware or instrument-specific bucket definitions.

### Serialize and Deserialize a MarketContext (serde feature)

```rust
use finstack_core::market_data::context::MarketContext;
use serde_json;

// Build or obtain a MarketContext
let ctx = MarketContext::new();

// Serialize to JSON (using MarketContextState under the hood)
let json = serde_json::to_string_pretty(&ctx)?;

// Deserialize back
let round_tripped: MarketContext = serde_json::from_str(&json)?;

assert_eq!(ctx.stats().total_curves, round_tripped.stats().total_curves);
```

This requires the `serde` feature on `finstack-core` (enabled by default).

---

## Adding New Features

The `market_data` module is **core infrastructure** and must remain deterministic, currency-safe, and serde-stable. When extending it, follow the patterns in `.cursor/rules/rust/crates/core.mdc` and keep changes small and composable.

### Adding a New Term Structure Type

1. **Create a new file** under `term_structures/` (e.g., `cds_curve.rs`):
   - Define a concrete struct and a builder with:
     - `id: CurveId`.
     - `base_date: Date`, `day_count: DayCount` (where applicable).
     - Validated knot vectors (use `validate_knots` where appropriate).
   - Implement `TermStructure` and any domain-specific traits (e.g., `Discounting`, `Forward`, `Survival`) that make sense.
2. **Integrate interpolation**
   - Use `math::interp::InterpStyle` and wire it through the builder.
   - Validate monotonicity or positivity invariants as required by the domain.
3. **Serde and state**
   - Under the `serde` feature, add a `*State` DTO if the runtime type cannot cleanly derive serde.
   - Implement `to_state` / `from_state` if necessary and integrate with `CurveState` / `MarketContextState`.
4. **Wire into `MarketContext`**
   - Consider extending `CurveStorage` and `MarketContext` insert/get helpers if the new curve should be first-class.
   - Update tests in `core/tests` to cover serialization and integration.

### Adding a New Surface Type

1. **Implement the surface**
   - Add a module under `surfaces/` (e.g., `dividend_surface.rs`) with:
     - Builder for the 2D grid (axes, values, validation).
     - Interpolation and extrapolation helpers.
2. **Re-export and integrate**
   - Re-export from `surfaces/mod.rs`.
   - If you want it addressable via `CurveId` and bumps, add:
     - Storage in `MarketContext` (similar to `VolSurface`).
     - `Bumpable` implementation and, if needed, `MarketBump` variants.
3. **Serde**
   - Add `Serialize`/`Deserialize` (or a `*State` DTO) under the `serde` feature.
   - Extend `MarketContextState` if the surface is stored there.

### Adding New Scalars or Time-Series Types

1. **Extend `MarketScalar` or add new helpers** in `scalars/primitives.rs` only when there is a broadly useful new scalar concept.
2. **Preserve serde stability**
   - Use `serde(rename_all = "snake_case")` and defaults for new variants/fields.
   - Do not rename existing variants or fields.
3. **Integrate with `MarketContext`**
   - Use `insert_price`, `insert_series`, or new helpers if the type warrants its own map.
   - Add tests that ensure round-trip via `MarketContextState`.

### Extending Bumps and Diff Utilities

- **New bump behavior**
  - Prefer modeling domain-specific bumping inside curve/surface implementations via the internal `Bumpable` traits.
  - Add new helpers on `BumpSpec` (e.g., new unit styles) only when there is a clear market-standard need.
  - Keep `MarketBump` variants orthogonal and stable; avoid breaking existing variants or serde names.
- **New diff metrics**
  - Implement new shift-measurement helpers in `diff.rs` when:
    - They map to concrete risk metrics (e.g., bucketed CS01, vol skews).
    - They operate on `MarketContext` and return deterministic, well-documented values.
  - Add tests for symmetry, sign conventions, and edge cases (missing curves/surfaces).

### Extending MarketContext

- When adding new stored objects:
  - Add fields to `MarketContext` with clear doc comments.
  - Update `MarketContextState` and its `From<&MarketContext>` / `TryFrom<MarketContextState>` implementations under the `serde` feature.
  - Extend `stats`, `total_objects`, and any relevant iterators.
- Avoid:
  - Introducing hidden global state or singletons.
  - Implicit FX conversions (all FX flows should go through `FxMatrix` and explicit policies).
  - Renaming existing serialized fields or variants in `MarketContextState`.

By following these patterns, new market data types remain **composable**, **deterministic**, and **compatible** across Rust, Python, and WASM bindings.


