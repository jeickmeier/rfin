# Changelog

All notable changes to this workspace are recorded here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this workspace follows a pre-1.0 cadence: breaking changes between minor
versions are possible but must be announced in this file.

See [`docs/SERDE_STABILITY.md`](docs/SERDE_STABILITY.md) for the per-wire-type
stability contract and schema-version policy.

## [Unreleased]

### Added
- `schema_version: u32` field on the following persisted result types, with
  `#[serde(default)]` for backward-compatible deserialization of pre-versioning
  payloads:
  - `finstack_valuations::results::ValuationResult`
    (`VALUATION_RESULT_SCHEMA_VERSION = 1`)
  - `finstack_statements::evaluator::StatementResult`
    (`STATEMENT_RESULT_SCHEMA_VERSION = 1`)
  - `finstack_portfolio::results::PortfolioResult`
    (`PORTFOLIO_RESULT_SCHEMA_VERSION = 1`)
  - `finstack_portfolio::optimization::PortfolioOptimizationResult`
    (`PORTFOLIO_OPTIMIZATION_RESULT_SCHEMA_VERSION = 1`)
- `finstack_py` binding-layer error helpers now flatten the full
  `std::error::Error::source()` chain into the Python exception message, so
  calibration / solver failures retain inner context that was previously
  dropped at the FFI boundary.
- `finstack_py::errors::error_to_py` helper for bindings that want to pass a
  `&dyn std::error::Error` and get the full source chain preserved.

### Changed
- PyO3 entry points that hold the GIL across long-running Rust work now
  release it via `py.detach(...)` (PyO3 0.28 API):
  - `finstack.valuations.calibrate`
  - `finstack.monte_carlo.McEngine.price_european_call` / `price_european_put`
  - `finstack.monte_carlo.price_european_call` / `price_european_put`
    (module-level)
  - `finstack.scenarios.apply_scenario` / `apply_scenario_to_market`
- WASM `Money.mulScalar` now routes through `Money::checked_mul_f64` for
  consistency with `divScalar`; previously it did a manual finiteness check
  and then unchecked multiplication, which could diverge in edge cases.
- WASM `SwaptionVolCube`/`VolCube` constructors and queries now use the
  structured `to_js_err()` error constructor; previously three sites used raw
  `JsValue::from_str(&e.to_string())`, breaking `err.name === "FinstackError"`
  pattern-matching for JS callers.
- Binding-layer `.map_err(|e| PyValueError::new_err(e.to_string()))` patterns
  have been replaced with centralized `crate::errors::display_to_py` /
  `core_to_py` helpers across 13 files in `finstack-py/src/bindings/`.
- `eprintln!` fallback logging in production code has been migrated to
  `tracing` (structured fields, standard subscriber routing):
  - `finstack-core/src/dates/calendar/types.rs` (bitset-range fallback)
  - `finstack-core/src/golden/loader.rs` (non-certified suite skip)
  - `finstack-valuations/src/instruments/.../revolving_credit/.../path_generator.rs`
    (CIR Feller-condition violation)
  - `finstack-valuations/src/instruments/common/models/trees/binomial_tree.rs`
    (Leisen-Reimer even-step warning)
- Python binding `finstack.core.money.Money` is now `frozen`; `__iadd__` /
  `__isub__` / `__imul__` / `__itruediv__` have been removed. Python `+=` etc.
  now fall back to the non-mutating dunders (`__add__` etc.) and rebind the
  variable. This matches the Rust `Money: Copy` semantics.
- Python binding `finstack.core.dates.CalendarMetadata.weekend_rule` now
  returns the stable snake_case serde name (`"saturday_sunday"`,
  `"friday_saturday"`, `"friday_only"`, `"none"`) instead of the `Debug` repr.

### Removed
- Unused wall-clock `last_access: std::time::Instant` field on the expression
  cache entry (`finstack_core::expr::cache`). LRU ordering is handled by the
  `lru` crate's insertion/access order — no wall-clock read was ever consumed,
  but removing it also removes a cross-run non-determinism risk.

### Fixed
- `finstack_core::credit::migration::scale::RatingScale` and
  `finstack_core::market_data::arbitrage` now use the deterministic
  `rustc_hash::FxHashMap` (via the `crate::HashMap` alias) instead of the
  hash-randomized `std::collections::HashMap`. The former sites would otherwise
  produce non-reproducible serialized iteration order across runs.

### Notes
- No public Rust signature was removed or renamed in this release. All
  breaking changes are guarded by new fields with `#[serde(default)]`. Existing
  persisted JSON payloads continue to deserialize under the new types.
- Schema versions start at `1` for each type above. Bumps must follow the
  policy in `docs/SERDE_STABILITY.md` and be recorded here.

## [0.4.1]

Baseline for this changelog.
