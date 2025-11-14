## Core Serialization & Wire-Type Overview

This document captures the current serialization surface of `finstack-core`.
It classifies each category of public types, lists the canonical DTO/state
shapes we persist across Rust/Python/WASM, and records the handful of
intentional exceptions (builders, iterators, progress callbacks).

### 1. Value types (derive `Serialize`/`Deserialize`)

These structs/enums do not hold lifetimes or trait objects, so they derive
serde directly behind the `serde` feature:

- Configuration: `FinstackConfig`, `RoundingPolicy`, `RoundingContext`,
  `ResultsMeta`, `ExplainOpts`.
- Identifiers and numerics: `Currency`, `Money`, `Id<T>` (e.g., `CurveId`),
  `Rate`, `Bps`, `Percentage`.
- Dates: `Frequency`, `StubKind`, `PeriodId`, `Period`, `Schedule`,
  `BusinessDayConvention`, `CalendarId`, `CalendarMetadata`, `Thirty360Convention`.
- Errors: unified `Error` enum (plus `InputError`), enabling
  `Result<T, Error>` to be serialized across bindings.
- Markets: enums such as `BumpSpec`, `FxConversionPolicy`, `FxQuery`,
  `MarketScalar`, `InflationInterpolation`, `InflationLag`.
- Explainability & math helpers: `ExplanationTrace`, `TraceEntry`, solvers,
  RNGs, interpolation style/enums.

### 2. Runtime/context types with DTO state wrappers

Types holding references, trait objects, or caches use explicit state DTOs.
These DTOs are the canonical wire formats for storage and bindings.

| Runtime type                          | State / Spec DTO                | Notes                                      |
|--------------------------------------|---------------------------------|--------------------------------------------|
| `DiscountCurve`, `ForwardCurve`      | `DiscountCurveState`, `ForwardCurveState` | `impl Serialize/Deserialize for curve via state`. |
| `HazardCurve`, `InflationCurve`      | `HazardCurveState`, `InflationCurveState` | `InflationCurve` also derives serde for convenience. |
| `BaseCorrelationCurve`, `VolSurface` | `BaseCorrelationCurve` (direct) / `VolSurfaceState` | Surfaces use flattened row-major arrays. |
| `ScalarTimeSeries`                   | `ScalarTimeSeriesState`         | Stores (date, value) points + interpolation. |
| `InflationIndex`                     | `InflationIndexState`           | Includes lag/interp/seasonality metadata. |
| `FxMatrix`                           | `FxMatrixState`                 | Captures config + cached quotes (providers are runtime only). |
| `MarketContext`                      | `MarketContextState`            | Contains vectors of the curve/surface/scalar states above. |
| `DayCountCtx<'a>`                    | `DayCountCtxState`              | Stores optional `calendar_id`, `frequency`, `bus_basis`. |
| `ScheduleBuilder<'a>` configuration  | `ScheduleSpec`                  | Persisted start/end/frequency/stub/BD adjustments. |

Conversion helpers (`to_state`, `from_state`, `ScheduleSpec::build`,
`DayCountCtxState::to_ctx`) live alongside their runtime counterparts so
every binding can leverage the same logic.

### 3. Non-serializable helpers (deliberate)

Some public types intentionally stay runtime-only:

- Builders/iterators: `ScheduleBuilder<'a>`, `BusinessDayIter`, `CompositeCalendar<'a>`.
- Progress plumbing: `ProgressFn`, `ProgressReporter`.
- Calendar registry facilities (`CalendarRegistry`) and trait objects
  (`&dyn HolidayCalendar`).

For these, callers should persist the relevant DTOs (`ScheduleSpec`,
`DayCountCtxState`, calendar IDs, etc.) and reconstruct runtime instances on demand.

### 4. Cross-language guidance

- **Rust** uses the DTOs listed above as the stable JSON/Bincode surface.
- **Python** and **WASM** mirror these DTOs (via Pydantic models / TypeScript
  interfaces) to ensure lossless roundtrips.
- **Databases** should store DTO shapes (e.g., `MarketContextState`,
  `DayCountCtxState`, `ScheduleSpec`) instead of raw runtime structs.

### 5. Standards for new public types

1. If a new value type has no lifetimes or trait objects, derive
   `Serialize`/`Deserialize` behind the `serde` feature.
2. If it carries references or dynamic data, define a `*State`/`*Spec`
   DTO plus conversion helpers.
3. Record the type and its serialization strategy in this document (or
   the relevant module docs) and add roundtrip tests under
   `finstack/core/tests`.

Following these rules keeps the serialization story consistent for
financial quants working across Rust, Python, and WASM while giving us a
single source of truth for persisted data shapes.

