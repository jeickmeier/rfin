# Finstack Python Bindings: Full Parity Implementation Design

**Date**: 2026-02-28
**Approach**: 3-stream parallel implementation
**Source Audit**: `docs/python-bindings-parity-review.md`

---

## Approach

Three independent work streams running in parallel. Streams don't overlap on files.

| Stream | Scope | Items | Rust Changes |
|--------|-------|-------|-------------|
| 1. Stubs | .pyi creation/updates only | 15 | None |
| 2. New Bindings | New #[pyfunction]/#[pyclass] + stubs | 8+1 | Yes (new files) |
| 3. Enhancements | Add protocols to existing bindings | 4 | Yes (modify existing) |

---

## Stream 1: Pure .pyi Stubs

### New files to create

1. **`finstack-py/finstack/valuations/instruments/bond_future.pyi`** -- `BondFuture` (from `bond_future.rs`)
2. **`finstack-py/finstack/valuations/instruments/equity_index_future.pyi`** -- `EquityIndexFuture` (from `equity_index_future.rs`)
3. **`finstack-py/finstack/valuations/instruments/fx_variance_swap.pyi`** -- `FxVarianceSwap` (from `fx_variance_swap.rs`)
4. **`finstack-py/finstack/valuations/instruments/inflation_cap_floor.pyi`** -- `InflationCapFloor` (from `inflation_cap_floor.rs`)
5. **`finstack-py/finstack/valuations/instruments/levered_real_estate_equity.pyi`** -- `LeveredRealEstateEquity` (from `levered_real_estate_equity.rs`)
6. **`finstack-py/finstack/valuations/instruments/ndf.pyi`** -- `Ndf` (from `ndf.rs`)
7. **`finstack-py/finstack/valuations/instruments/real_estate.pyi`** -- `RealEstateAsset` (from `real_estate.rs`)
8. **`finstack-py/finstack/valuations/instruments/xccy_swap.pyi`** -- `CrossCurrencySwap` (from `xccy_swap.rs`)
9. **`finstack-py/finstack/portfolio/optimization.pyi`** -- 16 optimization classes (from `portfolio/optimization.rs`)
10. **`finstack-py/finstack/valuations/covenants.pyi`** -- 8 classes + 2 functions (from `valuations/covenants.rs`)
11. **`finstack-py/finstack/valuations/calibration/methods.pyi`** -- 6 calibrator classes (from `calibration/methods.rs`)
12. **`finstack-py/finstack/valuations/dataframe.pyi`** -- 3 DataFrame export functions (from `valuations/dataframe.rs`)

### Existing stubs to update

13. **`instruments/__init__.pyi`** -- Add ~21 missing re-exports:
    - Existing stubs not re-exported: Swaption, InflationLinkedBond, InflationSwap, Repo, VarianceSwap, AsianOption, Autocallable, DCF, EquityTotalReturnSwap, FIIndexTotalReturnSwap, Basket
    - New stubs from items 1-8: BondFuture, EquityIndexFuture, FxVarianceSwap, InflationCapFloor, LeveredRealEstateEquity, Ndf, RealEstateAsset, CrossCurrencySwap

14. **`calibration/__init__.pyi`** -- Add methods imports for 6 calibrator classes

15. **`metrics.pyi`** -- Replace `__richcmp__(self, other, op: int)` with `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`, `__ge__`

16. **`core/market_data/term_structures.pyi`** -- Change `def base_date(self) -> date` to `@property` on DiscountCurve, ForwardCurve, HazardCurve

17. **`instruments/equity_option.pyi`** -- Add `@property` for `notional`, `spot_id`, `div_yield_id`

18. **`instruments/irs.pyi`** -- Add `@property` for `notional`, `side`, `fixed_rate`, `float_spread_bp`, `start`, `end`

19. **`statements/extensions/extensions.pyi`** -- Add 5 missing classes: AccountType, CorkscrewAccount, CorkscrewConfig, ScorecardMetric, ScorecardConfig

20. **`core/cashflow/__init__.pyi`** -- Add `npv_static`, `npv_using_curve_dc` imports

21. **P2-4**: Standardize all .pyi files to PEP 604 style (`str | None` not `Optional[str]`)

22. **P2-3**: Add `@overload` signatures where `Union` types are used

23. **P2-5 + P2-7**: NumPy-style docstrings with academic/source references on all instrument stubs

### Pattern to follow

Use existing well-documented stubs as templates:
- `bond.pyi` for instrument stubs (class docstring, Parameters, builder methods, Returns/Raises/Examples)
- `solver.pyi` for function stubs
- `term_structures.pyi` for @property style

---

## Stream 2: New Rust Bindings + Stubs

### P0-1: Missing instrument bindings (4 instruments)

Each follows the existing binding pattern (PyClass + builder + register_module):

| Instrument | New Rust File | Rust Crate Source |
|-----------|---------------|-------------------|
| FxForward | `finstack-py/src/valuations/instruments/fx_forward.rs` | `finstack/valuations/src/instruments/fx/fx_forward/` |
| FxDigitalOption | `finstack-py/src/valuations/instruments/fx_digital_option.rs` | `finstack/valuations/src/instruments/fx/fx_digital_option/` |
| FxTouchOption | `finstack-py/src/valuations/instruments/fx_touch_option.rs` | `finstack/valuations/src/instruments/fx/fx_touch_option/` |
| CommodityAsianOption | `finstack-py/src/valuations/instruments/commodity_asian_option.rs` | `finstack/valuations/src/instruments/commodity/commodity_asian_option/` |

Each binding file needs:
- `#[pyclass]` wrapping the Rust instrument
- Builder methods matching the Rust API
- `register_module()` function
- Registration in `instruments/mod.rs`
- Corresponding .pyi stub

### P1-1: Volatility pricing functions

Extend `finstack-py/src/core/market_data/volatility.rs` (or create new file) with:

```
black_call, black_put, black_vega, black_delta_call, black_delta_put, black_gamma
bachelier_call, bachelier_put, bachelier_vega, bachelier_delta_call, bachelier_delta_put, bachelier_gamma
black_shifted_call, black_shifted_put, black_shifted_vega
implied_vol_black, implied_vol_bachelier
```

### P1-2: Hull-White calibration

Create `finstack-py/src/valuations/calibration/hull_white.rs`:
- `HullWhiteParams` class (kappa, sigma)
- `SwaptionQuote` class
- `calibrate_hull_white_to_swaptions()` function
- Source: `finstack/valuations/src/calibration/hull_white.rs`

### P1-10: Stats functions

Extend `finstack-py/src/core/math/stats.rs`:
- `population_variance(data)` function
- `quantile(data, p)` function
- `OnlineStats` class (update, merge, count, mean, variance, std_dev, etc.)
- `OnlineCovariance` class (update, merge, covariance, correlation, etc.)

### P1-11: Advanced solver APIs

Extend `finstack-py/src/core/math/solver.rs`:
- `NewtonSolver.solve_with_derivative(f, f_prime, initial_guess)`
- `BracketHint` enum

Extend `finstack-py/src/core/math/solver_multi.rs`:
- `LmStats` class
- `LmSolution` class
- `LmTerminationReason` enum

### NEW: SABR/Heston/SVI model bindings

Create `finstack-py/src/core/volatility_models.rs` (or extend volatility.rs):
- `HestonParams` class (v0, kappa, theta, sigma, rho) + `price_european()`, `satisfies_feller_condition()`
- `SabrParams` class (alpha, beta, rho, nu) + `implied_vol_lognormal()`, `implied_vol_normal()`, `atm_vol_lognormal()`
- `SviParams` class

### P2-6: Monte Carlo building blocks

Expose stochastic processes and discretizations for advanced quant users. Scope TBD based on what's in the Rust crate.

---

## Stream 3: Existing Binding Enhancements

### P1-3: Collection protocols

| Type | File | Protocols |
|------|------|-----------|
| Portfolio | `portfolio/positions.rs` | `__iter__`, `__len__`, `__contains__` |
| CashFlowSchedule | `valuations/cashflow/` | `__iter__`, `__len__`, `__getitem__` |
| PathDataset | `valuations/common/mc/paths.rs` | `__iter__` (already has `__len__`) |
| FxMatrix | `core/market_data/fx.rs` | `__contains__`, `__len__` |
| MarketContext | `core/market_data/context.rs` | `__contains__` |

### P1-4: Copy/deepcopy on MarketContext

File: `finstack-py/src/core/market_data/context.rs`
- Add `__copy__` returning a shallow clone
- Add `__deepcopy__` performing a full deep clone

### P2-1: Pickle support

Add `__getnewargs__` or `__reduce__` to: Currency, Rate, Bps, CurveId, InstrumentId, MetricId, Tenor, DayCount

### P2-2: Format protocol

Add `__format__` to Money and Rate for f-string formatting support.

---

## Audit corrections

Items the audit missed that are included above:
1. SABR/Heston/SVI model bindings (added as P1-level)
2. `basket.pyi` and `trs.pyi` not in **init**.pyi re-exports (added to P0-3 scope)

Items the audit listed that are already done:
- Calibration V2 (`execute_calibration`, `CALIBRATION_SCHEMA`) -- already in `calibration/__init__.pyi`
- `AmortizationSpec` -- already in `cashflow/__init__.pyi` via builder import
- `MetricRegistry` -- already in `metrics.pyi`
