### Finstack Valuations (Rust) — Rules, Structure, and Contribution Guide

This document governs the `finstack/valuations/` crate. It describes scope, structure, cross‑cutting invariants, crate‑specific coding standards, and practical guidance for adding instruments, pricers, risk, and aggregation while preserving determinism, currency safety, and serde stability.

### Scope and Purpose
- **Responsibilities**: instrument cashflows, pricing, risk (DV01/CS01/Greeks), period aggregation (currency‑preserving), explicit FX collapse with policy stamping, adapters to scenarios/statements/portfolio, reportable results envelopes.
- **Out of scope**: primitive types, calendars/day‑count, generic math, market storage — those live in `core/` and must be reused.

### Directory/Module Shape (typical)
- `src/instr/*`: instrument definitions and pricers (e.g., bonds, swaps, FRNs, credit, inflation, options).
- `src/legs/*`: reusable cashflow legs (fixed, float, fees, amortization, equity dividends where applicable).
- `src/pricing/*`: pricing/risk engines and utilities shared across instruments.
- `src/aggregation/*`: currency‑preserving period aggregation, rollups, and PV decomposition.
- `src/scenario/*`: adapters for the scenarios DSL and preview/glide paths.
- `src/reports/*`: result envelopes, metadata stamping, explain tables.
- `src/utils/*`: small helpers that orchestrate core APIs (never duplicate core logic).

Note: The actual crate is large; module names can differ. Follow the nearest existing patterns when extending.

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
  - Pricing is implemented by pricers local to instruments (avoid generic, catch‑all pricing helpers that mix concerns).
  - Input validation happens up front with actionable `Error` variants from `core::error`.
- **Type safety**: Use newtype IDs (`types::id::CurveId`, `InstrumentId`) when referencing market objects and instruments.
- **Market access**: Resolve curves/indices via `market_data::context::MarketContext` with explicit IDs and trait bounds (`Discounting`, `Forward`, `Survival`).
- **Results metadata**: Stamp `config::results_meta(&cfg)` on outputs; include applied FX policy in envelopes when collapsing to a base currency.
- **Performance**: Prefer vectorized schedule evaluation and batched DF/rate queries. Avoid dynamic dispatch within inner loops where static is available.
- **Error handling**: Public APIs return `Result<T>`; avoid panics. Error messages should identify the faulty input (e.g., date ranges, calendars, missing curve IDs).
- **Testing**: Unit tests for cashflows, PV, risk, and edge cases; golden/parity tests for determinism and analytic sanity (where applicable).

### Pricing & Risk Patterns
- **Cashflow generation**: Use `core::cashflow::primitives` (`CashFlow`, `CFKind`, `Notional`) and `dates::schedule_iter::ScheduleBuilder` to generate flows with BDC and calendars.
- **NPV/discounting**: Use `core::cashflow::discounting::{Discountable, npv_static}` for dated `Money` flows when appropriate; for instruments, expose a pricer method that accepts a `Discounting` curve and day‑count.
- **Risk**: DV01/CS01 via parallel/key‑rate bumps (`market_data::bumps::BumpSpec`) and re‑pricing; Greeks for options via perturbation or analytic where implemented. Ensure perturbations are deterministic and small enough for stable differencing.
- **Aggregation**: Aggregate by currency first. FX collapse is a distinct, explicit step using an `FxPolicy` and must record the policy in results. Stamp rounding context.
- **Parallelism**: If adding Rayon paths, ensure serial ≡ parallel outputs and avoid interior mutability that could reorder numeric summations across runs.

### Adding a New Instrument (recommended steps)
1) **Model struct + builder**: Define an ergonomic builder (validated) to produce a strongly‑typed instrument definition (tenor, rate/leg spec, calendars, BDC, notional).
2) **Schedule & flows**: Build schedules with `ScheduleBuilder`, apply BDC using `HolidayCalendar`, create `CashFlow`s with explicit kinds and accrual factors (use `DayCount` from core).
3) **Pricer**: Implement an instrument‑specific pricer struct or trait impl (PV, clean/dirty price, yield). Accept `MarketContext` or specific curve traits via references.
4) **Risk**: Provide DV01/CS01/Greeks helpers. Use `BumpSpec` and `MarketContext::bump` where applicable. Keep bump labeling deterministic.
5) **FX and base currency**: If returning base‑currency totals, take `FxMatrix` and policy explicitly; do not silently convert. Include `FxPolicyMeta` in result.
6) **Serde**: For any serializable inputs/outputs, add serde behind the feature flag with stable names and defaults for new fields.
7) **Tests**: Add unit tests for schedule generation, PV (including known expected PVs / analytic checks), risk (finite‑difference consistency), serialization round‑trips, and determinism tests (serial vs parallel).

### Using Market Data
- Fetch curves via `MarketContext` (e.g., `get_discount`, `get_forward`, `get_hazard`, `inflation_index`). Handle missing IDs as `InputError::NotFound` with the offending ID.
- For time conversions, prefer `DiscountCurve::df_on_date`/`df_on_date_curve` or compute year fractions with `DayCount::year_fraction` from core.
- For credit, use `HazardCurve` (`Survival` trait) and ensure recovery/tenor metadata are respected by the pricer.

### Reports & Metadata
- Construct compact result envelopes containing PV breakdown, accrued, risk vectors, config `ResultsMeta`, and optional `fx_policy_applied`.
- Always include the numeric mode (`NumericMode::F64`), rounding context snapshot, and whether parallel execution was used when relevant.

### Calibration Notes
- Calibrations should leverage instrument pricers rather than ad‑hoc discounting routines, to ensure model consistency and explainability across pricing and calibration results.
- Keep calibration code deterministic (fixed seeds if random starts are used, or avoid randomness altogether). Record inputs and solve configurations for auditability.

### Review & Testing Checklist (valuations)
- Instrument builder validates inputs and produces consistent schedules across calendars/BDC.
- PV/risk outputs are deterministic and stable under serial/parallel execution.
- Currency aggregation is preserved; FX collapse step is explicit and policy‑stamped.
- No reimplementation of core date/day‑count helpers; no implicit FX.
- Public result types are serde‑compatible with stable names and defaults for new optional fields.
- Adequate tests: unit, parity/golden, serialization round‑trips, and edge cases (stub periods, EoM, leap days, holiday boundaries).

### Anti‑Patterns to Avoid
- Writing custom date, calendar, or day‑count logic; always use `core/dates`.
- Implicit cross‑currency math or mixing currencies in `Money` operations.
- Hiding FX collapse inside aggregation without recording/stamping policy.
- Pricing via shared global helpers that bypass instrument pricers.
- Introducing nondeterministic behavior (unordered reductions, randomness without fixed seeds) in public paths.

### How to Propose a Change
- Open a focused PR that:
  - Explains the instrument/feature, inputs, outputs, and market data dependencies.
  - Demonstrates PV/risk with tests and determinism (serial ≡ parallel).
  - Details any serde exposure and compatibility considerations.
  - Includes documentation examples for public APIs.

This guide applies to the `valuations/` crate only. See `core/` rules for foundational types and policies, and bindings rules for Python/WASM parity.


