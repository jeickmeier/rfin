# Binding Parity Inventory

This document tracks the remaining actionable Rust public API parity work for `finstack-py`.
Already-satisfied renames/relocations are intentionally omitted, and Rust-only helper surfaces are
called out explicitly so they do not get re-added as false-positive binding gaps.

## Core

### Missing Binding

- `finstack/core/src/math/linalg.rs`: `CholeskyError`, `SINGULAR_THRESHOLD`, `DIAGONAL_TOLERANCE`, `SYMMETRY_TOLERANCE`, `cholesky_solve`

### Intentional Python Surface

- `finstack/core/src/lib.rs`: `error`, `prelude`, `validation`, `Error`, `InputError`, `NonFiniteKind`, `Result` remain intentionally adapted in Python as root-level exception exports plus the curated `finstack.core` module tree; no additional core root-module/runtime parity binding is currently required

### Python-Only Drift

- `finstack-py/src/core/market_data/volatility.rs`: `convert_volatility`
- `finstack-py/finstack/core/expr_helpers.py`: `ExprWrapper`, `col`, `lit`, `lag`, `lead`, `diff`, `pct_change`, `growth_rate`, `cumsum`, `cumprod`, `cummin`, `cummax`, `rolling_mean`, `rolling_sum`, `rolling_std`, `rolling_var`, `rolling_min`, `rolling_max`, `if_then_else`

## Statements

### Python-Only Drift

- `finstack-py/src/statements/templates.rs`: `LeaseSpec`, `LeaseSpecV2`, and renewal-window constructors still validate `occupancy` / `probability` in the Python binding layer instead of delegating all validation to Rust

## Valuations

### Missing Binding

- `finstack/valuations/src/instruments/mod.rs`: `CmsSwap`, `IrFutureOption`, `CommoditySpreadOption`, `CommoditySwaption`, `CollateralType`, `BarrierDirection`, `DigitalPayoutType`, `PayoutTiming`, `TouchType`
- `finstack/valuations/src/instruments/equity/mod.rs`: `DiscountedCashFlow`
- `finstack/valuations/src/calibration/mod.rs`: `QuoteQuality`, `CalibrationDiagnostics`

### Internal / Helper-Only Rust Exports

- `finstack/valuations/src/calibration/mod.rs`: `CurveValidator`, `SurfaceValidator` are Rust validation traits; Python already exposes concrete `validate_*` helpers plus `ValidationConfig`
- `finstack/valuations/src/attribution/mod.rs`: `JsonEnvelope` is a Rust serialization trait, and `AttributionInput` is an internal execution struct used by the attribution implementations

## Portfolio

### Missing Binding

- `finstack/portfolio/src/optimization/mod.rs`: `ConstraintValidationError`

### Intentional Python Surface

- `finstack/portfolio/src/types.rs`: `EntityId`, `PositionId` stay represented as Python `str` values rather than separate wrapper classes
- `finstack/portfolio/src/optimization/mod.rs`: `PortfolioOptimizer` stays a Rust trait while Python uses `DefaultLpOptimizer`
