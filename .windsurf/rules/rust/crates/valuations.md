---
trigger: model_decision
description: This is useful for learning about the valuations crate and its functionality which includes: instrument cashflows, pricing, risk (DV01/CS01/Greeks), period aggregation (currency‑preserving), explicit FX collapse with policy stamping, calibration of market curves and surfaces, metrics registry, reportable results envelopes.
globs:
---
### Finstack Valuations (Rust) — Rules, Structure, and Contribution Guide

This document governs the `finstack/valuations/` crate. It describes scope, structure, cross‑cutting invariants, crate‑specific coding standards, and practical guidance for adding instruments, pricers, risk, and aggregation while preserving determinism, currency safety, and serde stability.

### Scope and Purpose
- **Responsibilities**: instrument cashflows, pricing, risk (DV01/CS01/Greeks), period aggregation (currency‑preserving), explicit FX collapse with policy stamping, calibration of market curves and surfaces, metrics registry, reportable results envelopes.
- **Out of scope**: primitive types, calendars/day‑count, generic math, market storage — those live in `core/` and must be reused.

### Directory/Module Shape (actual structure)
- `src/instruments/*`: Instrument definitions organized by type (bonds, swaps, options, credit, structured products). Each instrument has types, pricer, metrics, and optional pricing submodules.
- `src/instruments/common/*`: Shared pricing infrastructure:
  - `analytical/*`: Closed-form/semi-analytical pricers (Black-Scholes barriers, Asian, Lookback, Quanto, Heston)
  - `mc/*`: Monte Carlo framework (processes, discretization, payoffs, variance reduction, LSMC, xVA)
  - `models/*`: Pricing models (trees, SABR, short-rate models)
  - `metrics/*`: Finite-difference Greeks, bucketed risk, helpers
  - `parameters/*`: Reusable parameter structures
  - `traits.rs`: Core `Instrument` trait
- `src/pricer.rs`: Registry-based pricing dispatch (enum-based, macro-free) mapping (InstrumentType, ModelKey) → pricers.
- `src/calibration/*`: Market data calibration (discount/forward curves, credit curves, volatility surfaces, SABR, swaption vol).
- `src/metrics/*`: Metrics registry for risk calculations (DV01, CS01, Greeks, bucketed risk).
- `src/cashflow/*`: Cashflow builders with schedule generation and aggregation utilities.
- `src/results/*`: Result envelopes, metadata stamping, DataFrame conversions.
- `src/covenants/*`: Covenant engine for structured products.
- `src/constants.rs`, `src/schema.rs`: Constants and schema definitions.

Note: The crate is large (637 .rs files). Follow existing patterns within the relevant instrument or module family when extending.

### Cross‑Cutting Invariants (inherit from core + crate‑specific)
- **Determinism**: Vectorized/parallel execution must produce identical numeric outputs to serial paths in f64 mode. Cache usage cannot affect results.
- **Currency‑safety**: All aggregation is currency‑preserving. Cross‑currency collapse requires an explicit FX policy with metadata stamped in results.
- **Explicit FX**: No implicit currency conversions. Use `FxMatrix`/`FxProvider` and propagate `FxPolicyMeta` in result envelopes.
- **Stable serde**: Public result types must maintain field names and de/serialization shapes (feature‑gated via `serde`). Unknown fields must be denied on inbound types.
- **Dates/Day‑count via core**: Always use `core/dates` types, day‑count conventions, calendars, and schedule builders; do not reimplement date logic.
- **No unsafe**: Keep the crate safe Rust only.

### Coding Standards (valuations)
- **API design**:
  - Instruments have clear builder APIs and produce schedules using `core::dates::schedule_iter` and `Calendar`/BDC rules.
  - Pricing is implemented via the pricer registry system (`pricer::PricerRegistry`) mapping (InstrumentType, ModelKey) to concrete pricers.
  - Instruments implement the `Instrument` trait providing `value()`, `id()`, `currency()`, and `as_any()` for type-safe downcasting.
  - Input validation happens up front with actionable `Error` variants from `core::error`.
- **Pricer Registry**: Use `create_standard_registry()` to get the populated registry. Register pricers with `PricerKey::new(InstrumentType, ModelKey)`. Avoid macro-based pricing systems; the registry provides compile-time type safety with enum dispatch.
- **Type safety**: Use newtype IDs (`types::id::CurveId`, `InstrumentId`) when referencing market objects and instruments.
- **Market access**: Resolve curves/indices via `market_data::context::MarketContext` with explicit IDs and trait bounds (`Discounting`, `Forward`, `Survival`).
- **Results metadata**: Stamp `config::results_meta(&cfg)` on outputs; include applied FX policy in envelopes when collapsing to a base currency.
- **Performance**: Prefer vectorized schedule evaluation and batched DF/rate queries. Avoid dynamic dispatch within inner loops where static is available.
- **Error handling**: Public APIs return `Result<T>`; avoid panics. Error messages should identify the faulty input (e.g., date ranges, calendars, missing curve IDs).
- **Testing**: Unit tests for cashflows, PV, risk, and edge cases; golden/parity tests for determinism and analytic sanity (where applicable).

### Pricing & Risk Patterns
- **Cashflow generation**: Use `cashflow::builder` for schedule-based flows with BDC and calendars from `core::dates::schedule_iter::ScheduleBuilder`.
- **Pricing methods**: Instruments support multiple pricing methods via the registry:
  - **Analytical**: Closed-form Black-Scholes variants for barriers, Asian, lookback, quanto (see `instruments/common/analytical/*`). 100-10,000x faster than MC. Default for applicable instruments.
  - **Monte Carlo**: Full MC framework with GBM, Heston, jump-diffusion processes; variance reduction (antithetic, control variates); LSMC for early exercise; xVA exposure profiles (see `instruments/common/mc/*`). Use for discrete monitoring, complex path dependencies, American exercise.
  - **Trees**: Binomial/trinomial for American options, short-rate models (see `instruments/common/models/*`).
- **Model selection**: Specify via `ModelKey` enum (e.g., `BarrierBSContinuous`, `MonteCarloGBM`, `HestonFourier`). Analytical is default where available; MC via explicit `ModelKey::MonteCarloGBM` or instrument's `npv_mc()` method.
- **NPV/discounting**: Use `core::cashflow::discounting::{Discountable, npv_static}` for dated `Money` flows; instruments expose `npv()` and optionally `npv_mc()`.
- **Risk**: DV01/CS01 via parallel/key‑rate bumps (`market_data::bumps::BumpSpec`) and re‑pricing; Greeks via finite-difference or analytical formulas. Use `metrics::registry` for standard risk calculations. Ensure perturbations are deterministic with fixed seeds for MC instruments.
- **Metrics registry**: Register metrics per instrument type with `MetricId` → calculator mapping. Bucketed risk (DV01, CS01, Vega) uses standard maturity/strike grids.
- **Aggregation**: Aggregate by currency first. FX collapse is a distinct, explicit step using an `FxPolicy` and must record the policy in results. Stamp rounding context.
- **Parallelism**: If adding Rayon paths, ensure serial ≡ parallel outputs and avoid interior mutability that could reorder numeric summations across runs.

### Adding a New Instrument (recommended steps)
1) **Model struct + builder**: Define an ergonomic builder (validated) to produce a strongly‑typed instrument definition (tenor, rate/leg spec, calendars, BDC, notional).
2) **Schedule & flows**: Build schedules with `ScheduleBuilder`, apply BDC using `HolidayCalendar`, create `CashFlow`s with explicit kinds and accrual factors (use `DayCount` from core).
3) **Instrument trait**: Implement `instruments::common::traits::Instrument` providing `value()` (delegates to pricer), `id()`, `currency()`, and `as_any()` for downcasting.
4) **Pricer implementation**: Create pricer struct(s) in `instruments/<instrument>/pricer.rs`:
   - For simple cashflow instruments: implement direct NPV calculation with `MarketContext` access.
   - For options: support both analytical (if available) and MC pricing methods.
   - Register all pricers in `pricer::create_standard_registry()` with appropriate `(InstrumentType, ModelKey)` pairs.
5) **InstrumentType enum**: Add variant to `pricer::InstrumentType` with unique numeric discriminant and implement `as_str()` mapping.
6) **ModelKey variants**: Add applicable `ModelKey` variants (e.g., `AsianGeometricBS`, `MonteCarloGBM`) if introducing new pricing methods.
7) **Metrics**: Implement metrics in `instruments/<instrument>/metrics/` directory:
   - Create `mod.rs` with metric calculator structs implementing appropriate trait.
   - Register metrics in the global metrics registry (see `metrics::registry`).
   - Standard metrics: Theta (all instruments), DV01/CS01 (rates/credit), Delta/Gamma/Vega (options).
   - Use finite-difference helpers from `instruments/common/metrics/finite_difference.rs`.
8) **Risk**: Provide DV01/CS01/Greeks helpers. Use `BumpSpec` and `MarketContext::bump` where applicable. Keep bump labeling deterministic. For MC instruments, use fixed seeds.
9) **FX and base currency**: If returning base‑currency totals, take `FxMatrix` and policy explicitly; do not silently convert. Include `FxPolicyMeta` in result.
10) **Serde**: For any serializable inputs/outputs, add serde behind the feature flag with stable names and defaults for new fields.
11) **Tests**: Add unit tests for schedule generation, PV (including known expected PVs / analytic checks), risk (finite‑difference consistency), serialization round‑trips, and determinism tests (serial vs parallel). For analytical methods, add parity tests vs MC.

### Using Market Data
- Fetch curves via `MarketContext` (e.g., `get_discount`, `get_forward`, `get_hazard`, `inflation_index`, `get_vol_surface`). Handle missing IDs as `InputError::NotFound` with the offending ID.
- For time conversions, prefer `DiscountCurve::df_on_date`/`df_on_date_curve` or compute year fractions with `DayCount::year_fraction` from core.
- For credit, use `HazardCurve` (`Survival` trait) and ensure recovery/tenor metadata are respected by the pricer. Note: CS01 bumps quote spreads, not derived hazard rates.
- For volatilities, use `VolSurface::value_clamped(t, strike)` with proper interpolation. Heston parameters are scalars in MarketContext (e.g., `HESTON_KAPPA`, `HESTON_THETA`).

### Reports & Metadata
- Construct compact result envelopes containing PV breakdown, accrued, risk vectors, config `ResultsMeta`, and optional `fx_policy_applied`.
- Always include the numeric mode (`NumericMode::F64`), rounding context snapshot, and whether parallel execution was used when relevant.

### Calibration Framework
- The `calibration/*` module provides market data construction from quotes:
  - **Interest rates**: Discount curves (OIS) and forward curves (IBOR/RFR) from deposits, FRAs, futures, swaps. Post-2008 multi-curve framework.
  - **Credit**: Survival/hazard curves from CDS spreads with ISDA 2014 standard model compliance.
  - **Inflation**: Real CPI level curves from zero-coupon inflation swaps with lag handling.
  - **Volatilities**: Implied volatility surfaces using SABR models per expiry. Supports equity (lognormal beta) and rates (normal beta).
  - **Swaption vol**: `SwaptionVolCalibrator` handles normal/lognormal quoting, ATM conventions, forward swap rate estimation.
  - **Base correlation**: Credit correlation curves from CDS tranche quotes with Gaussian Copula.
- **Orchestration**: Use `SimpleCalibration` for end-to-end market environment setup from quote lists.
- **Calibrator trait**: All calibrators implement `Calibrator` trait with `calibrate(&quotes, &base_context) -> Result<(Output, Report)>`.
- **Determinism**: All calibrations use deterministic algorithms. Record inputs and solve configurations in `CalibrationReport` for auditability.
- **Instrument pricers**: Calibrations leverage instrument pricers (not ad-hoc discounting) for consistency across pricing and calibration.

### Review & Testing Checklist (valuations)
- Instrument builder validates inputs and produces consistent schedules across calendars/BDC.
- Instrument implements `Instrument` trait and is registered in pricer registry with appropriate `InstrumentType` and `ModelKey` variants.
- Analytical methods (if applicable) are documented in `docs/ANALYTICAL_METHODS.md` with formulas, references, and use cases.
- Metrics are implemented in dedicated `metrics/` subdirectory and registered in global metrics registry.
- PV/risk outputs are deterministic and stable under serial/parallel execution. MC instruments use fixed seeds for reproducibility.
- Currency aggregation is preserved; FX collapse step is explicit and policy‑stamped.
- No reimplementation of core date/day‑count helpers; no implicit FX.
- Public result types are serde‑compatible with stable names and defaults for new optional fields.
- Adequate tests: unit, parity/golden (analytical vs MC where applicable), serialization round‑trips, and edge cases (stub periods, EoM, leap days, holiday boundaries).
- Run `make lint` and `make test` after changes; fix any errors before proceeding.

### Anti‑Patterns to Avoid
- Writing custom date, calendar, or day‑count logic; always use `core/dates`.
- Implicit cross‑currency math or mixing currencies in `Money` operations.
- Hiding FX collapse inside aggregation without recording/stamping policy.
- Using macro-based pricing systems or string-based dispatch; use the enum-based pricer registry.
- Bypassing the pricer registry for ad-hoc pricing; always register pricers and use `PricerKey` lookups.
- Duplicating finite-difference logic; use helpers from `instruments/common/metrics/finite_difference.rs`.
- Introducing nondeterministic behavior (unordered reductions, randomness without fixed seeds) in public paths.
- For MC instruments, using non-deterministic seeds; derive seeds from instrument/metric IDs.

### Key Features Summary
- **Pricer Registry**: Enum-based dispatch (no macros) with type-safe `(InstrumentType, ModelKey)` → pricer mapping.
- **Analytical Methods**: Closed-form Black-Scholes variants (barriers, Asian, lookback, quanto) providing 100-10,000x speedup vs MC. See `docs/ANALYTICAL_METHODS.md`.
- **Monte Carlo Framework**: Full-featured MC with stochastic processes, variance reduction, LSMC, xVA. See `instruments/common/mc/`.
- **Calibration**: Market-standard curve/surface construction from quotes (discount, forward, credit, inflation, volatility, correlation). See `calibration/README.md`.
- **Metrics Registry**: Comprehensive risk framework with DV01, CS01, Greeks, bucketed risk. See `metrics/METRICS.md`.
- **Covenant Engine**: Rule-based covenant evaluation for structured products. See `covenants/`.
- **Determinism**: All paths (serial/parallel, analytical/MC with fixed seeds) produce identical results.

This guide applies to the `valuations/` crate only. See `core/` rules for foundational types and policies, and bindings rules for Python/WASM parity.


