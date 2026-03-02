# Core Python Binding: 100% Parity Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close every remaining gap between the Rust `finstack-core` public API and the `finstack.core` Python bindings, achieving 100% parity.

**Architecture:** Six independent work streams that can run in parallel since they touch non-overlapping files:

| Stream | Module | Approach | Effort |
|---|---|---|---|
| S1 | Analytics | Add pandas DataFrame input support to `Performance` | Small |
| S2 | Market Data | Add missing curve types + MarketContext methods | Medium |
| S3 | Math | Add `Compounding`, `TimeGrid`, `SobolRng`, `moment_match()` | Small |
| S4 | FX + Types | Add FX providers + 3 missing ID types | Small |
| S5 | Dates | Add missing IMM functions, constants, enum variants | Small |
| S6 | Cashflow | Add builder methods + scalar NPV | Trivial |

**Tech Stack:** Rust (pyo3 0.28), Python (.pyi type stubs), maturin build system

---

## Current State (as of 2026-03-01)

The `finstack.core` Python bindings cover approximately 75-80% of the Rust `finstack-core` public API. The following modules have good-to-excellent coverage:

- **Currency** — complete (all ISO-4217 codes, `Currency` class)
- **Money** — complete (`Money`, arithmetic, formatting, conversion)
- **Types** — mostly complete (5/8 ID types, `Rate`, `Bps`, `Percentage`, all credit rating types)
- **Config** — complete (`FinstackConfig`, `RoundingMode`, `RoundingPolicy`, `ResultsMeta`)
- **Explain** — complete (`ExplainOpts`, `ExplanationTrace`, `TraceEntry`)
- **Expr** — complete (`Expr`, `Function`, `BinOp`, `UnaryOp`, `CompiledExpr`, `EvalOpts`)
- **Volatility Models** — complete (`HestonParams`, `SabrParams`, `SviParams`)
- **Math** — mostly complete (solvers, stats, distributions, integration, special functions)
- **Market Data** — mostly complete (5/7 curve types, `MarketContext`, FX, scalars)
- **Dates** — mostly complete (calendars, schedules, periods, tenors, day counts)
- **Cashflow** — mostly complete (all 22 `CFKind` variants, NPV, XIRR)

---

## Stream S1: Analytics — Pandas DataFrame Input Support

### Current State

The `Performance` class already has **complete method coverage** — all ~50 analytics methods are implemented in `finstack-py/src/core/analytics/performance.rs` (1069 lines), including:

- **Risk metrics:** sharpe, sortino, volatility, cagr, calmar, value_at_risk, expected_shortfall, tail_ratio, ulcer_index, risk_of_ruin, skewness, kurtosis, geometric_mean, downside_deviation, omega_ratio, gain_to_pain, martin_ratio, m_squared, modified_sharpe, parametric_var, cornish_fisher_var, recovery_factor, sterling_ratio, burke_ratio, pain_index, pain_ratio, cdar, mean_return
- **Benchmark-relative:** tracking_error, information_ratio, r_squared, beta, greeks, up_capture, down_capture, capture_ratio, batting_average, treynor
- **Series outputs:** cumulative_returns, drawdown_series, correlation, excess_returns, cumulative_returns_outperformance, drawdown_outperformance
- **Rolling metrics:** rolling_volatility, rolling_sortino, rolling_sharpe, rolling_greeks
- **Other:** multi_factor_greeks, drawdown_details, stats_during_bench_drawdowns, lookback_returns, period_stats, max_drawdown_duration, reset_date_range, reset_bench_ticker

### Gap: Pandas DataFrame Input

Currently the constructor only accepts Polars DataFrames (`pyo3_polars::PyDataFrame`). Users who work with pandas must manually convert before constructing `Performance`.

### Design

Accept **either a pandas DataFrame or a Polars DataFrame** as input. Each column represents a different asset; the index (for pandas) or first column (for Polars) contains dates.

```python
import pandas as pd
import polars as pl
from finstack.core.analytics import Performance

# Pandas input — index is dates, columns are assets
df_pd = pd.DataFrame({"AAPL": [150.0, 152.0], "MSFT": [300.0, 305.0]},
                      index=pd.to_datetime(["2024-01-01", "2024-01-02"]))
perf = Performance(df_pd, freq="daily")

# Polars input — first column is Date, remaining are assets (existing behavior)
df_pl = pl.DataFrame({"date": ["2024-01-01", "2024-01-02"],
                       "AAPL": [150.0, 152.0], "MSFT": [300.0, 305.0]})
perf = Performance(df_pl, freq="daily")
```

**Implementation approach:**
1. Change the `prices` parameter type from `PyDataFrame` to `&Bound<'_, PyAny>`
2. Detect whether the input is a pandas or Polars DataFrame
3. For pandas: convert to Polars via `polars.from_pandas(df.reset_index())` in Python, then proceed with existing extraction logic
4. For Polars: proceed with existing `PyDataFrame` extraction as-is
5. Update `.pyi` stubs to accept `polars.DataFrame | pandas.DataFrame`

### Files to Modify

- `finstack-py/src/core/analytics/performance.rs` — modify constructor to accept pandas or polars
- `finstack-py/finstack/core/analytics/__init__.pyi` — update type stub

---

## Stream S2: Market Data Missing Types

### New Types

#### PriceCurve

Wraps `finstack_core::market_data::term_structures::PriceCurve`.

```python
class PriceCurve:
    def __init__(
        self,
        id: str,
        base_date: str | date,
        knots: list[tuple[float, float]],
        interp: str | InterpStyle | None = None,
        extrapolation: str | ExtrapolationPolicy | None = None,
    ) -> None: ...

    @property
    def id(self) -> str: ...
    @property
    def base_date(self) -> date: ...
    @property
    def points(self) -> list[tuple[float, float]]: ...

    def price(self, t: float) -> float: ...
    def forward_price(self, t1: float, t2: float) -> float: ...
```

#### VolatilityIndexCurve

Wraps `finstack_core::market_data::term_structures::VolatilityIndexCurve`.

```python
class VolatilityIndexCurve:
    def __init__(
        self,
        id: str,
        base_date: str | date,
        knots: list[tuple[float, float]],
        interp: str | InterpStyle | None = None,
        extrapolation: str | ExtrapolationPolicy | None = None,
    ) -> None: ...

    @property
    def id(self) -> str: ...
    @property
    def base_date(self) -> date: ...
    @property
    def points(self) -> list[tuple[float, float]]: ...

    def vol(self, t: float) -> float: ...
```

#### FlatCurve

Convenience helper wrapping `finstack_core::market_data::term_structures::FlatCurve`.

```python
class FlatCurve:
    def __init__(
        self,
        value: float,
        base_date: str | date,
        day_count: str | DayCount,
        id: str,
    ) -> None: ...

    def as_discount_curve(self) -> DiscountCurve: ...
    def as_forward_curve(self, tenor: float) -> ForwardCurve: ...
```

#### InflationIndex

Wraps `finstack_core::market_data::scalars::InflationIndex`.

```python
class InflationIndex:
    def __init__(
        self,
        id: str,
        base_values: list[tuple[date, float]],
        seasonal_adjustments: list[float] | None = None,
    ) -> None: ...

    @property
    def id(self) -> str: ...

    def at_date(self, date: str | date) -> float: ...
    def growth_factor(self, from_date: str | date, to_date: str | date) -> float: ...
```

### MarketContext Additions

```python
class MarketContext:
    # ... existing methods ...

    # New curve type support
    def insert_price_curve(self, curve: PriceCurve) -> None: ...
    def get_price_curve(self, id: str) -> PriceCurve | None: ...
    def insert_vol_index(self, curve: VolatilityIndexCurve) -> None: ...
    def get_vol_index(self, id: str) -> VolatilityIndexCurve | None: ...
    def insert_inflation_index(self, id: str, index: InflationIndex) -> None: ...
    def get_inflation_index(self, id: str) -> InflationIndex | None: ...

    # Roll forward
    def roll(
        self,
        from_date: str | date,
        to_date: str | date,
        calendar: Calendar | None = None,
    ) -> MarketContext: ...

    # Serialization
    def to_state(self) -> str: ...
    @classmethod
    def from_state(cls, state: str) -> MarketContext: ...
```

### Files to Create/Modify

- Create: `finstack-py/src/core/market_data/price_curve.rs`
- Create: `finstack-py/src/core/market_data/vol_index_curve.rs`
- Create: `finstack-py/src/core/market_data/flat_curve.rs`
- Create: `finstack-py/src/core/market_data/inflation_index.rs`
- Modify: `finstack-py/src/core/market_data/context.rs` (add roll, serialization, new getters/setters)
- Modify: `finstack-py/src/core/market_data/mod.rs` (register new modules)
- Create/modify: corresponding `.pyi` stubs

---

## Stream S3: Math Advanced Types

### Compounding

```python
class Compounding:
    ANNUAL: Compounding       # classattr
    SEMI_ANNUAL: Compounding
    QUARTERLY: Compounding
    MONTHLY: Compounding
    CONTINUOUS: Compounding

    @classmethod
    def from_frequency(cls, frequency: int) -> Compounding: ...

    def compound_factor(self, rate: float, time: float) -> float: ...
    def discount_factor(self, rate: float, time: float) -> float: ...

    @staticmethod
    def equivalent_rate(
        from_compounding: Compounding,
        to_compounding: Compounding,
        rate: float,
    ) -> float: ...
```

### TimeGrid

```python
class TimeGrid:
    def __init__(self, times: list[float]) -> None: ...

    @classmethod
    def uniform(cls, start: float, end: float, n_steps: int) -> TimeGrid: ...

    @property
    def times(self) -> list[float]: ...
    @property
    def dt(self) -> list[float]: ...

    def map_date_to_step(self, t: float) -> int: ...
    def map_dates_to_steps(self, times: list[float]) -> list[int]: ...
```

### SobolRng

```python
MAX_SOBOL_DIMENSION: int

class SobolRng:
    def __init__(self, dimension: int, seed: int = 0) -> None: ...

    def next(self) -> list[float]: ...
    def next_batch(self, n: int) -> list[list[float]]: ...
    def skip(self, n: int) -> None: ...
```

### moment_match()

```python
# In finstack.core.math.stats
def moment_match(
    target_mean: float,
    target_variance: float,
    samples: list[float],
) -> list[float]: ...
```

### Files to Create/Modify

- Create: `finstack-py/src/core/math/compounding.rs`
- Create: `finstack-py/src/core/math/time_grid.rs`
- Modify: `finstack-py/src/core/math/random.rs` (add SobolRng)
- Modify: `finstack-py/src/core/math/stats.rs` (add moment_match)
- Modify: `finstack-py/src/core/math/mod.rs` (register new modules)
- Create/modify: corresponding `.pyi` stubs

---

## Stream S4: FX Providers + Type IDs

### FX Types

```python
class FxQuery:
    def __init__(
        self,
        from_ccy: str | Currency,
        to_ccy: str | Currency,
        on: str | date,
        policy: FxConversionPolicy | None = None,
    ) -> None: ...

    @property
    def from_currency(self) -> Currency: ...
    @property
    def to_currency(self) -> Currency: ...
    @property
    def on(self) -> date: ...
    @property
    def policy(self) -> FxConversionPolicy: ...


class SimpleFxProvider:
    def __init__(self) -> None: ...

    def set_quote(
        self,
        from_ccy: str | Currency,
        to_ccy: str | Currency,
        rate: float,
    ) -> None: ...

    def rate(self, query: FxQuery) -> float: ...
    def to_matrix(self) -> FxMatrix: ...


class BumpedFxProvider:
    def __init__(self, base: FxMatrix | SimpleFxProvider) -> None: ...

    def with_bump(
        self,
        from_ccy: str | Currency,
        to_ccy: str | Currency,
        bump_amount: float,
        bump_mode: str = "absolute",
    ) -> BumpedFxProvider: ...

    def rate(self, query: FxQuery) -> float: ...
```

### Type IDs

```python
# In finstack.core.types
class CalendarId:
    def __init__(self, value: str) -> None: ...
    def as_str(self) -> str: ...

class PoolId:
    def __init__(self, value: str) -> None: ...
    def as_str(self) -> str: ...

class DealId:
    def __init__(self, value: str) -> None: ...
    def as_str(self) -> str: ...
```

### Files to Create/Modify

- Modify: `finstack-py/src/core/market_data/fx.rs` (add FxQuery, SimpleFxProvider, BumpedFxProvider)
- Modify: `finstack-py/src/core/types.rs` (add CalendarId, PoolId, DealId)
- Create/modify: corresponding `.pyi` stubs

---

## Stream S5: Dates Utilities

### Constants

```python
# In finstack.core.dates
CALENDAR_DAYS_PER_YEAR: float  # 365.0
AVERAGE_DAYS_PER_YEAR: float   # 365.25
```

### Missing IMM Functions

```python
# In finstack.core.dates.imm
def is_imm_date(date: str | date) -> bool: ...
def third_wednesday(date: str | date) -> date: ...
def third_friday(date: str | date) -> date: ...
def sifma_settlement_date(date: str | date) -> date: ...
def next_sifma_settlement(date: str | date) -> date: ...
def next_equity_option_expiry(date: str | date) -> date: ...
```

### BusinessDayConvention Addition

Add `HalfMonthModifiedFollowing` variant to the existing enum.

### ScheduleWarning Enum

```python
class ScheduleWarning:
    @property
    def kind(self) -> str: ...
    @property
    def message(self) -> str: ...
```

Update `Schedule` to return warnings:

```python
class Schedule:
    @property
    def dates(self) -> list[date]: ...
    @property
    def warnings(self) -> list[ScheduleWarning]: ...
```

### Files to Modify

- Modify: `finstack-py/src/core/dates/imm.rs` (add missing functions)
- Modify: `finstack-py/src/core/dates/calendar.rs` (add HalfMonthModifiedFollowing)
- Modify: `finstack-py/src/core/dates/schedule.rs` (add ScheduleWarning)
- Modify: `finstack-py/src/core/dates/mod.rs` (add constants)
- Create/modify: corresponding `.pyi` stubs

---

## Stream S6: Cashflow Minor

### CashFlow Builder Methods

```python
class CashFlow:
    # ... existing ...
    def with_rate(self, rate: float) -> CashFlow: ...
    def with_reset_date(self, date: str | date) -> CashFlow: ...
```

### Scalar NPV Function

```python
# In finstack.core.cashflow
def npv_amounts(
    cashflows: list[tuple[date, float]],
    rate: float,
    base_date: str | date | None = None,
    day_count: str | DayCount | None = None,
) -> float: ...
```

### Files to Modify

- Modify: `finstack-py/src/core/cashflow/primitives.rs` (add builder methods)
- Modify: `finstack-py/src/core/cashflow/discounting.rs` (add npv_amounts)
- Create/modify: corresponding `.pyi` stubs

---

## Intentionally Excluded

The following Rust `finstack-core` modules are NOT bound by design:

| Module | Reason |
|---|---|
| `golden` | Testing framework — internal tooling, not user-facing |
| `validation` | `require()` / `require_or()` — Rust-internal assertion helpers |
| `collections` | Re-exports of `HashMap` / `HashSet` — Python has `dict` / `set` |

---

## Verification Plan

After all streams are complete:

### 1. Build

```bash
cd finstack-py && maturin develop --release
```

### 2. Parity audit script

```python
# Run: python scripts/core_parity_check.py
"""Verify all core module items are accessible from Python."""

from finstack.core.analytics import Performance
from finstack.core.market_data.term_structures import (
    DiscountCurve, ForwardCurve, HazardCurve, InflationCurve,
    BaseCorrelationCurve, PriceCurve, VolatilityIndexCurve, FlatCurve,
)
from finstack.core.market_data.scalars import InflationIndex, ScalarTimeSeries
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxQuery, SimpleFxProvider, BumpedFxProvider
from finstack.core.math.compounding import Compounding
from finstack.core.math.time_grid import TimeGrid
from finstack.core.math.random import SobolRng
from finstack.core.math.stats import moment_match
from finstack.core.types import CalendarId, PoolId, DealId
from finstack.core.dates import CALENDAR_DAYS_PER_YEAR, AVERAGE_DAYS_PER_YEAR
from finstack.core.dates.imm import (
    is_imm_date, third_wednesday, third_friday,
    sifma_settlement_date, next_equity_option_expiry,
)
from finstack.core.dates.calendar import BusinessDayConvention
from finstack.core.cashflow import npv_amounts

# Verify Performance accepts both pandas and polars DataFrames
import pandas as pd
import polars as pl

dates = pd.to_datetime(["2024-01-02", "2024-01-03", "2024-01-04"])
prices = {"AAPL": [150.0, 152.0, 151.0], "MSFT": [300.0, 305.0, 303.0]}
df_pd = pd.DataFrame(prices, index=dates)
df_pl = pl.DataFrame({"date": dates, **prices})

perf_pd = Performance(df_pd, freq="daily")
perf_pl = Performance(df_pl, freq="daily")
assert perf_pd.sharpe() is not None
assert perf_pl.sharpe() is not None

# Verify new MarketContext methods
ctx_methods = [
    'insert_price_curve', 'get_price_curve',
    'insert_vol_index', 'get_vol_index',
    'insert_inflation_index', 'get_inflation_index',
    'roll', 'to_state', 'from_state',
]
missing = [m for m in ctx_methods if not hasattr(MarketContext, m)]
assert not missing, f"Missing MarketContext methods: {missing}"

# Verify BusinessDayConvention
assert hasattr(BusinessDayConvention, 'HalfMonthModifiedFollowing')

# Verify constants
assert CALENDAR_DAYS_PER_YEAR == 365.0
assert AVERAGE_DAYS_PER_YEAR == 365.25

print("PASS: All core Python binding parity checks passed")
```

### 3. Test suite

```bash
cd finstack-py && pytest tests/ -v --tb=short
```

### 4. Stub validation

```bash
find finstack-py/finstack -name '*.pyi' -exec python -c "import ast; ast.parse(open('{}').read())" \;
echo "All stubs valid"
```
