### Finstack Core (Rust) — Rules, Structure, and Contribution Guide

This document defines Cursor rules for the `finstack/core/` crate. It explains purpose, structure, invariants, coding standards, and how to add new features safely while preserving determinism, currency safety, and serde stability.

### Scope and Purpose
- **Core responsibilities**: currency/types, money and FX, dates/calendars/day-count, math (interp/solver/integration), expression engine, market term structures (discount/forward/hazard/inflation/surfaces), cashflow primitives/discounting, config and errors.
- **Determinism-first**: Results must be identical across serial and parallel execution; no hidden randomness.
- **Currency-safety**: Arithmetic on `Money` requires identical currencies; FX conversions are explicit via `FxProvider`/`FxMatrix` and the applied policy should be stampable into results metadata.
- **Serde stability**: Stable field names and enums; unknown fields are denied on inbound types; do not rename serialized fields for public APIs.

### Directory Structure (selected)
- `src/lib.rs`: crate facade and public modules.
- `src/config.rs`: numeric mode, rounding policy, and results metadata.
- `src/currency.rs` and `src/generated/currency_generated.rs`: ISO‑4217 currencies (generated at build).
- `src/money/` (`types.rs`, `fx.rs`, `rounding.rs`): `Money`, FX matrix/provider, rounding helpers.
- `src/dates/`: date utilities, calendars (algo/rule/types/registry), day-count, periods, schedules, IMM dates.
- `data/calendars/*.json`: calendar rule sources; build.rs generates compiled calendars.
- `src/market_data/`: term structures (`discount_curve`, `forward_curve`, `hazard_curve`, `inflation`), surfaces, context, bumps, dividends, scalars.
- `src/math/`: interpolation, solvers, integration, statistics, random, summation.
- `src/expr/`: AST, DAG planning, cache, evaluator, Polars lowering.
- `src/cashflow/`: primitives and discounting helpers.

### Cross‑Cutting Invariants
- **No `unsafe`**: Keep the crate safe Rust only.
- **Numeric mode**: `type F = f64` (see `config::NUMERIC_MODE`). Do not introduce alternate numeric engines without explicit feature gating and stability review.
- **Determinism**: Parallel implementations must reproduce serial results bit‑for‑bit where feasible; otherwise document and test for acceptable tolerances.
- **Currency safety**: `Money` add/sub enforce currency equality; FX conversions are explicit and auditable.
- **Stable serde**: Avoid changing serialized names or shapes. When adding types/fields, prefer additive changes with defaults.
- **Polars as canonical DF**: For time‑series surfaces/series, prefer Polars re‑exports from core.

### Coding Standards (Core)
- **Naming**: Functions are verbs; types are nouns. Avoid abbreviations; prefer clarity (e.g., `compute_present_value` over `pv`).
- **Type safety**: Use newtype IDs (`types::id::Id`) such as `CurveId`, `InstrumentId` instead of raw `String`.
- **APIs**: Public APIs must be documented with examples where practical. Avoid panics in public paths; return `crate::Result<T>` with `crate::error` variants.
- **Errors**: Prefer `InputError` for validation issues, `Validation` for semantic checks, and keep messages actionable. Avoid stringly‑typed errors in hot paths.
- **Serde**: Gate serialization under the `serde` feature; maintain `rename_all = "snake_case"` where applicable; add defaults for new fields to preserve backward compatibility.
- **Threading config**: Do not rely on global state. Pass `FinstackConfig` where rounding/metadata is needed.
- **Concurrency**: Respect the `parallel` feature flag (Rayon); do not change outputs when toggled.
- **Performance**: Prefer preallocation, smallvec where appropriate, and vectorized math. Avoid unnecessary boxing/trait objects in tight loops.
- **No implicit FX**: Never auto‑convert `Money` across currencies; require explicit provider and policy.
- **Tests**: Add unit, property, and golden/parity tests. Ensure serial ≡ parallel for deterministic paths and validate boundary conditions.

### Feature Design Patterns
- **Term structures**: Implement as concrete types with builders and `TermStructure` + specific traits (`Discounting`, `Forward`, `Survival`). Store knots/values with validated interpolation (`math::interp`). Provide `to_state/from_state` under `serde`.
- **Calendars**: Defined via `rule::Rule` and generated sets from `data/calendars/*.json` by `build.rs`. Use `HolidayCalendar` trait for queries and provide a registry lookup.
- **Day‑count**: Add to `dates::daycount` with clear semantics and tests (year fraction, days, edge cases, calendars when relevant).
- **Expression engine**: Keep operations deterministic; try to lower to Polars when possible and fall back to scalar with identical results.
- **Cashflows**: Keep primitives small and explicit (`CashFlow`, `CFKind`, `Notional`); avoid instrument‑specific logic here (lives in valuations crate).

### Adding New Features to `core/`

1) New Calendar or Calendar Rule
- Add a JSON file to `finstack/core/data/calendars/` following existing shape (see examples like `nyse.json`).
- Supported rule types are serialized to `rule::Rule`: `Fixed`, `NthWeekday`, `WeekdayShift`, `EasterOffset`, `Span`, `ChineseNewYear`, `QingMing`, `BuddhasBirthday`, `VernalEquinoxJP`, `AutumnalEquinoxJP`.
- Run `cargo build` – `build.rs` will generate `OUT_DIR/calendars.rs` and wire into `dates::calendar` (constants, `ALL_IDS`, `calendar_by_id`).
- Add tests under `finstack/core/tests/dates/` to validate known holidays, weekend behavior, and ID lookup.

2) New Day‑Count Convention
- Extend `dates/daycount.rs`:
  - Add enum variant to `DayCount` with docs and any supporting helpers.
  - Implement logic in `DayCount::days` / `year_fraction` with clear error handling.
  - Add tests covering equal dates, inverted ranges, leap years, calendar‑aware cases (for Bus/252‑like rules).

3) New Interpolation Method
- Add implementation under `math/interp/` and extend `InterpStyle` + `Interp` factories.
- Ensure input validation (`utils::validate_knots`, monotonicity if required) and derivative support (`InterpFn::interp_prime`).
- Add serialization support if appropriate and tests for exact‑knot and mid‑segment behavior.

4) New Term Structure Type
- Create a new module in `market_data/term_structures/` with:
  - Concrete struct + builder, enforcing invariants.
  - Trait implementation(s) (and `TermStructure`).
  - Serialization state type and `to_state/from_state` behind `serde`.
  - Unit tests (construction, evaluation, edge cases, serialization round‑trip).
- Consider integration with `market_data::context` if it must be discoverable.

5) Money/FX Enhancements
- Add policies or provider capabilities in `money/fx.rs` with explicit `FxConversionPolicy` and stamped `FxPolicyMeta` in results where relevant.
- Keep caching bounded and deterministic; avoid time‑dependent behavior in tests.

6) Expression Engine Additions
- Add new `Function` variants in `expr/ast.rs` with lowering in `expr/eval.rs` and (optionally) Polars lowering.
- Ensure scalar and Polars paths match exactly; add parity tests.
- Consider DAG cost updates if computational profile changes.

7) Cashflow Primitives
- Extend `cashflow/primitives.rs` sparingly. Preserve small struct sizes and add validation (`InputError::Invalid`).
- Do not embed pricing logic here; keep this as a foundational layer.

### Review & Testing Checklist
- Public API has doc comments and examples.
- New types implement serde (feature‑gated) with stable names and defaults.
- No panics in public code paths; return `Result`.
- Determinism maintained (serial vs parallel parity where applicable).
- Currency safety preserved; no implicit cross‑currency math.
- Benchmarks added if performance‑critical (criterion is available in dev‑deps).
- Tests: units for happy paths and edges; property tests where feasible; serialization round‑trips; calendar/date boundary tests when relevant.

### Practical Tips
- Use `types::id::Id` newtypes for identifiers, never raw `String`.
- For date math, reuse `dates::utils` and `DateExt`; do not invent ad‑hoc helpers.
- For time series, prefer Polars and `ScalarTimeSeries`/`InflationIndex` instead of custom vectors.
- Keep interpolation and solvers separated from financial layer code; make them reusable.

### Anti‑Patterns to Avoid
- Hidden global state or singletons.
- Implicit FX or silent currency conversion.
- Changing serde names/variants in released types.
- `unwrap()`/`expect()` in public code paths.
- Divergent implementations between scalar and Polars paths.

### How to Propose a Change
- Open a small, scoped PR that:
  - Explains the problem and target API.
  - Shows tests that guard behavior and stability.
  - Calls out any serde or compatibility risks explicitly.
  - Demonstrates serial ≡ parallel determinism where applicable.

This file governs only the `core/` crate. See separate rules for Python/WASM bindings and higher‑level pricing/evaluation crates.


