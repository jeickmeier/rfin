# Finstack Python Bindings: Parity & Quality Review

**Date**: 2026-02-28
**Scope**: Full audit of `finstack-py` bindings vs Rust crate

## Executive Summary

The `finstack-py` bindings are ~90-93% at parity with the Rust crate: 447 PyClasses, 165 PyFunctions, 160 .pyi stubs, and 50 test files. The architecture is sound with clean module hierarchy, consistent builder patterns, proper error hierarchy, and strong type safety. This document catalogs every gap for triage and incremental resolution.

---

## P0: Parity-Breaking (Must Fix)

### P0-1: Create Python bindings for 4 missing instruments

These instruments exist in the Rust crate with full pricing/metrics but have **zero** Python binding code.

| Rust Instrument | Asset Class | Rust Location |
|-----------------|-------------|---------------|
| `FxForward` | FX | `instruments/fx/fx_forward/` |
| `FxDigitalOption` | FX Exotics | `instruments/fx/fx_digital_option/` |
| `FxTouchOption` | FX Exotics | `instruments/fx/fx_touch_option/` |
| `CommodityAsianOption` | Commodity | `instruments/commodity/commodity_asian_option/` |

**Work**: Create `finstack-py/src/valuations/instruments/{fx_forward,fx_digital_option,fx_touch_option,commodity_asian_option}.rs`, matching existing instrument binding patterns (PyClass + Builder + register in `mod.rs`).

---

### P0-2: Create .pyi stubs for 8 instruments already bound in Rust

These have Rust binding code but **no .pyi stub files** and are **missing from `instruments/__init__.pyi`**.

| Instrument | Rust Binding File | .pyi | In `__init__.pyi` |
|-----------|-------------------|------|--------------------|
| `BondFuture` | `bond_future.rs` | Missing | Missing |
| `EquityIndexFuture` | `equity_index_future.rs` | Missing | Missing |
| `FxVarianceSwap` | `fx_variance_swap.rs` | Missing | Missing |
| `InflationCapFloor` | `inflation_cap_floor.rs` | Missing | Missing |
| `LeveredRealEstateEquity` | `levered_real_estate_equity.rs` | Missing | Missing |
| `Ndf` | `ndf.rs` | Missing | Missing |
| `RealEstateAsset` | `real_estate.rs` | Missing | Missing |
| `CrossCurrencySwap` | `xccy_swap.rs` | Missing | Missing |

**Work**: For each, create `finstack-py/finstack/valuations/instruments/{name}.pyi` and add to `instruments/__init__.pyi` exports.

---

### P0-3: Add ~12 missing instruments to `instruments/__init__.pyi` re-exports

These instruments have both Rust bindings AND .pyi stubs but are **not re-exported from `__init__.pyi`**, so `from finstack.valuations.instruments import Swaption` fails type-checking.

| Instrument | Has .pyi | In `__init__.pyi` |
|-----------|----------|-------------------|
| `Swaption` | Yes | **Missing** |
| `InflationLinkedBond` | Yes | **Missing** |
| `InflationSwap` | Yes | **Missing** |
| `Repo` | Yes | **Missing** |
| `VarianceSwap` | Yes | **Missing** |
| `AsianOption` | Yes | **Missing** |
| `Autocallable` | Yes | **Missing** |
| `DCF` (DiscountedCashFlow) | Yes | **Missing** |
| `EquityTotalReturnSwap` | Yes | **Missing** |
| `FIIndexTotalReturnSwap` | Yes | **Missing** |
| `CDSPayReceive` | Yes | Present (via `cds`) |

**Work**: Add import lines and `__all__` entries to `instruments/__init__.pyi`.

---

### P0-4: Create `portfolio/optimization.pyi`

16 portfolio optimization classes are bound in Rust but have **no .pyi stub** -- the entire optimization DSL is invisible to IDEs.

Missing classes: `WeightingScheme`, `MissingMetricPolicy`, `Inequality`, `OptimizationStatus`, `TradeDirection`, `TradeType`, `PerPositionMetric`, `MetricExpr`, `Objective`, `PositionFilter`, `Constraint`, `TradeSpec`, `OptimizationResult`, `CandidatePosition`, `TradeUniverse`, `PortfolioOptimizationProblem`.

**Work**: Create `finstack-py/finstack/portfolio/optimization.pyi`.

---

### P0-5: Create `valuations/covenants.pyi`

10 covenant types/functions are bound in Rust but have **no .pyi stub**.

Missing: `CovenantType`, `Covenant`, `CovenantSpec`, `CovenantScope`, `SpringingCondition`, `CovenantForecastConfig`, `CovenantForecast`, `FutureBreach`, `forecast_covenant`, `forecast_breaches`.

**Work**: Create `finstack-py/finstack/valuations/covenants.pyi`.

---

### P0-6: Create `valuations/calibration/methods.pyi`

6 individual calibrator classes are bound in Rust but missing from stubs.

Missing: `DiscountCurveCalibrator`, `ForwardCurveCalibrator`, `HazardCurveCalibrator`, `InflationCurveCalibrator`, `VolSurfaceCalibrator`, `BaseCorrelationCalibrator`.

**Work**: Create `finstack-py/finstack/valuations/calibration/methods.pyi` and add to `calibration/__init__.pyi`.

---

### P0-7: Create `valuations/dataframe.pyi`

3 DataFrame export functions are bound but missing stubs.

Missing: `results_to_polars`, `results_to_pandas`, `results_to_parquet`.

**Work**: Create `finstack-py/finstack/valuations/dataframe.pyi`.

---

## P1: Should Fix (Quant Developer Experience)

### P1-1: Expose core volatility pricing functions

These are fundamental quant building blocks available in Rust but not callable from Python.

| Function | Purpose |
|----------|---------|
| `black_call`, `black_put`, `black_vega` | Black-76 pricing |
| `bachelier_call`, `bachelier_put`, `bachelier_vega` | Normal model pricing |
| `black_shifted_call`, `black_shifted_put`, `black_shifted_vega` | Shifted lognormal |
| `implied_vol_black`, `implied_vol_bachelier` | Implied vol extraction |
| `black_delta_call/put`, `bachelier_delta_call/put` | Option deltas |
| `black_gamma`, `bachelier_gamma` | Option gammas |
| `VolatilityConvention`, `convert_atm_volatility` | Convention conversion |

**Work**: Add `#[pyfunction]` wrappers in `finstack-py/src/core/volatility.rs` (or new file) and create/update .pyi stubs.

---

### P1-2: Expose Hull-White calibration

Entire Hull-White calibration is missing from Python.

Missing: `HullWhiteParams`, `SwaptionQuote`, `calibrate_hull_white_to_swaptions`.

**Work**: Add bindings in `finstack-py/src/valuations/calibration/` and .pyi stubs.

---

### P1-3: Add `__iter__`/`__len__`/`__contains__` to collection types

| Type | Missing Protocols | Expected Behavior |
|------|------------------|-------------------|
| `Portfolio` | `__iter__`, `__len__`, `__contains__` | Iterate over positions |
| `CashFlowSchedule` | `__iter__`, `__len__`, `__getitem__` | Iterate/index cashflows |
| `PathDataset` | `__iter__`, `__len__` | Iterate simulated paths |
| `FxMatrix` | `__contains__`, `__len__` | Check currency pair exists |
| `MarketContext` | `__contains__` | Check curve/surface exists |

**Work**: Add `#[pymethods]` implementations for each, update .pyi stubs.

---

### P1-4: Add `__copy__`/`__deepcopy__` to `MarketContext`

Quants frequently clone market state for scenario analysis. Currently no `copy.deepcopy()` support.

**Work**: Implement `__copy__` and `__deepcopy__` on `PyMarketContext`, update .pyi stub.

---

### P1-5: Fix `MetricId` stub -- replace `__richcmp__` with proper dunder methods

The .pyi stub exposes raw `__richcmp__(self, other, op: int)` instead of `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`, `__ge__`. Breaks IDE autocompletion.

**Work**: Update `finstack-py/finstack/valuations/metrics.pyi` to use standard comparison methods.

---

### P1-6: Fix `base_date` on curve stubs to `@property`

`DiscountCurve`, `ForwardCurve`, `HazardCurve` use `#[getter]` in Rust (property) but .pyi stubs show `def base_date(self) -> date` (method).

**Work**: Update `finstack-py/finstack/core/market_data/term_structures.pyi`.

---

### P1-7: Add missing properties to `equity_option.pyi` and `irs.pyi`

| Stub | Missing Properties |
|------|-------------------|
| `equity_option.pyi` | `notional`, `spot_id`, `div_yield_id` |
| `irs.pyi` | `notional`, `side`, `fixed_rate`, `float_spread_bp`, `start`, `end` |

**Work**: Add `@property` definitions to each .pyi file.

---

### P1-8: Add 5 missing classes to `extensions/extensions.pyi`

Missing: `AccountType`, `CorkscrewAccount`, `CorkscrewConfig`, `ScorecardMetric`, `ScorecardConfig`.

**Work**: Update `finstack-py/finstack/statements/extensions/extensions.pyi`.

---

### P1-9: Add `npv_static`, `npv_using_curve_dc` to `core/cashflow/__init__.pyi`

These discounting functions are bound but missing from stubs.

**Work**: Update `finstack-py/finstack/core/cashflow/__init__.pyi`.

---

### P1-10: Expose missing stats functions

| Function | Purpose |
|----------|---------|
| `population_variance` | Population variance (vs sample) |
| `quantile` | Percentile calculation |
| `OnlineStats` | Streaming mean/variance accumulator |
| `OnlineCovariance` | Streaming covariance accumulator |

**Work**: Add `#[pyfunction]`/`#[pyclass]` wrappers in `finstack-py/src/core/math/stats.rs`, update .pyi.

---

### P1-11: Expose advanced solver APIs

| Function | Purpose |
|----------|---------|
| `NewtonSolver::solve_with_derivative` | Analytic-derivative Newton's method |
| `LevenbergMarquardtSolver` diagnostics | `LmStats`, `LmSolution`, `LmTerminationReason` |

**Work**: Extend existing solver bindings, update .pyi stubs.

---

## P2: Nice to Have (Polish)

### P2-1: Add pickle support to commonly-serialized types

Add `__getnewargs__` or `__reduce__` to: `Currency`, `Rate`, `Bps`, `CurveId`, `InstrumentId`, `MetricId`, `Tenor`, `DayCount`.

Currently only `Money` supports pickle. This blocks `multiprocessing` workflows.

---

### P2-2: Add `__format__` protocol to `Money` and `Rate`

Enable f-string formatting: `f"PV: {money:,.2f}"`, `f"Rate: {rate:.4%}"`.

---

### P2-3: Add `@overload` signatures in .pyi files

Where `Union` types are used (e.g., `Union[DayCount, str]`), add `@overload` for more precise IDE hints.

---

### P2-4: Standardize annotation style across all .pyi stubs

Some files use `from __future__ import annotations` + PEP 604 (`str | None`), others use `Optional[str]`. Pick one and apply consistently.

---

### P2-5: Extend NumPy-style docstrings to all instrument .pyi files

Use `Bond`, `Currency`, `DiscountCurve` as templates. Every instrument .pyi should have:
- Class docstring with description, Parameters section
- Builder method docstrings
- Returns/Raises/Examples on key methods
- Sources/References for pricing models

---

### P2-6: Expose Monte Carlo building blocks

Expose processes (`HestonProcess`, `CirProcess`, `BatesProcess`, etc.), discretizations (`EulerMaruyama`, `Milstein`, `QeHeston`, etc.), and payoff types for advanced quant users building custom MC experiments.

---

### P2-7: Add academic/source references to all pricing model docs

Extend the `Sources` pattern from `Bond` and `EquityOption` to all instrument .pyi files, citing relevant papers and textbooks.

---

## Statistics

| Category | Count |
|----------|-------|
| P0 items (parity-breaking) | 7 |
| P1 items (developer experience) | 11 |
| P2 items (polish) | 7 |
| Missing instrument bindings | 4 |
| Missing .pyi stubs (instruments) | 8 |
| Missing .pyi re-exports | ~12 |
| Missing .pyi stubs (non-instrument modules) | 4 |
| Missing core math functions | ~25 |
| Missing collection protocols | 5 types |

## Suggested Triage Order

1. **Sprint 1 (stubs only)**: P0-2, P0-3, P0-4, P0-5, P0-6, P0-7 -- pure .pyi file creation, no Rust changes
2. **Sprint 2 (missing bindings)**: P0-1 -- 4 new instrument bindings
3. **Sprint 3 (core math)**: P1-1, P1-2, P1-10, P1-11 -- expose volatility/calibration/stats functions
4. **Sprint 4 (Pythonic polish)**: P1-3, P1-4, P1-5, P1-6, P1-7, P1-8, P1-9 -- collection protocols, property fixes, stub corrections
5. **Sprint 5 (documentation)**: P2-5, P2-7 -- docstring coverage
6. **Sprint 6 (advanced)**: P2-1, P2-2, P2-3, P2-4, P2-6 -- pickle, formatting, MC building blocks
