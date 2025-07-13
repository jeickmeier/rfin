# Changelog – rfin-core

All notable changes to this crate are documented in this file.

## [0.3.0] – 2025-07-11 – CashFlow MVP
### Added
* New `cashflow` module delivering fixed-rate leg generation, accrual caching, amortisation, stub detection.
* Parallel NPV helpers behind the `parallel` feature (Rayon).
* Optional floating-rate builder scaffold behind `index` feature.
* `Notional` abstraction with `AmortRule` (None, Linear, Step).
* Global accrual factor LRU cache (~2k entries).

### Changed
* Workspace version bumped to **0.3.0**.
* Added `hashbrown` and optional `rayon` dependencies.
* Documentation extended; `#![deny(missing_docs)]` enabled for cashflow sub-modules.

### Fixed
* Clippy clean with `-D warnings` across all features.

## [0.4.0] – 2025-07-12 – Interpolator API Simplification
### Removed
* `InterpPolicy` enum – redundant abstraction eliminated.

### Added
* Smart-constructor helpers on `Interpolator` (`linear_df`, `log_df`, `monotone_convex`, `cubic_hermite`, `flat_fwd`).

### Changed
* All curve builders now accept interpolation style via dedicated helper methods (e.g., `.linear_df()`) and store an `Interpolator` directly.
* `CurveSet` and all tests updated accordingly; this is a **breaking change**.

## [0.4.1] – 2025-07-13 – Monotone-Convex Interpolator
### Added
* Production-grade Hagan–West **monotone-convex** discount-factor interpolator (`MonotoneConvex`).
* Unit test `monotone_convex_basic_properties` and Criterion benchmark `benches/monotone_convex.rs`.

### Changed
* `Interpolator::monotone_convex()` now uses the new algorithm instead of a linear fallback.

--- 