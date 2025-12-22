# Changelog

All notable changes to the `finstack-core` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-12-22 (v1 Beta)

### Added

#### Core Types & Currency
- ISO-4217 currency support with 180+ currencies generated from official data
- `Money` type with currency-safe arithmetic (explicit FX conversions required)
- `Rate`, `Bps`, `Percentage` newtype wrappers for type-safe rate handling
- `CreditRating` with Moody's/S&P mapping and WARF factors
- Phantom-typed `Id<T>` wrappers (`CurveId`, `InstrumentId`, etc.)

#### Date & Calendar System
- 19 pre-configured holiday calendars (NYSE, TARGET2, ASX, JPX, HKEX, etc.)
- Generated at build-time from JSON rule definitions
- Day count conventions: Act/360, Act/365F, Act/Act (ISDA/ISMA), 30/360, Bus/252
- Business day adjustment conventions (Following, ModifiedFollowing, Preceding)
- Schedule generation with stub handling
- IMM date utilities for derivatives roll dates
- Tenor parsing and arithmetic

#### Market Data Infrastructure
- `DiscountCurve`: Risk-free discount factor term structure
- `ForwardCurve`: Forward rate term structure
- `HazardCurve`: Credit hazard/survival term structure
- `InflationCurve`: CPI expectation term structure
- `BaseCorrelationCurve`: CDO tranche correlation
- `CreditIndexData`: Credit index aggregates
- `VolSurface`: 2D implied volatility surface
- `MarketScalar`, `ScalarTimeSeries`: Scalar market data and time series
- `InflationIndex`: CPI/RPI index series
- `DividendSchedule`: Equity dividend schedules
- `MarketContext`: Unified market data container with Arc-based storage

#### Math & Numerical Methods
- Interpolation: Linear, LogLinear, CubicHermite, MonotoneConvex, FlatForward
- Root finders: Newton-Raphson, Brent's method with derivative support
- Multi-dimensional solver: Levenberg-Marquardt
- Integration: Simpson, Trapezoidal, Gauss-Legendre (adaptive)
- Gauss-Hermite quadrature for normal integrals
- Special functions: erf, norm_cdf, norm_pdf, inverse normal CDF
- Student-t distribution (CDF, inverse CDF)
- Statistics: mean, variance, covariance, correlation
- Stable summation: Kahan, pairwise algorithms
- Linear algebra: Cholesky decomposition, correlation matrix construction
- Random number generation: Box-Muller transform, deterministic test RNG

#### Expression Engine
- AST-based expression representation
- DAG optimization with shared sub-expression detection
- Scalar evaluation with caching
- Rolling window functions (mean, sum, std, var, min, max, count)
- Lag/lead, diff, pct_change operations
- Cumulative operations (cumsum, cumprod, cummin, cummax)
- EWM (exponentially weighted) functions

#### Cashflow Primitives
- `CashFlow` type with date, amount, currency, and kind
- NPV calculation with discount curves
- XIRR/IRR computation via Newton-Raphson

#### FX System
- `FxProvider` trait for custom FX sources
- `FxMatrix` with LRU caching and triangulation
- `SimpleFxProvider` for testing
- `FxConversionPolicy` for conversion strategy hints
- Audit trail via `FxPolicyMeta`

#### Configuration & Errors
- `FinstackConfig` for rounding/tolerance settings
- `ToleranceConfig` for numerical precision control
- Comprehensive `Error` enum with actionable messages
- Fuzzy suggestions for missing curves (Levenshtein distance)
- `InputError` for validation failures

#### Explainability
- `ExplainOpts` for computation tracing
- `ExplainEntry` for structured attribution output

### Changed
- Uses `time` crate (not `chrono`) for all date operations
- All public types feature-gated serde support
- Strict serde field naming with `deny_unknown_fields`

### Safety & Quality
- `#![forbid(unsafe_code)]` enforced at crate level
- `#![deny(clippy::unwrap_used)]` prevents panics in public code
- `#![warn(missing_docs)]` enforces documentation
- 85%+ test coverage with 1,400+ tests
- Comprehensive benchmarks (12 suites)

---

[0.4.0]: https://github.com/finstack/finstack/releases/tag/v0.4.0
[
