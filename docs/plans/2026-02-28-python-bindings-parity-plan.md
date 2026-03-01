# Python Bindings Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Achieve full parity between finstack Rust crate and finstack-py Python bindings -- all P0, P1, P2 items plus SABR/Heston/SVI model bindings.

**Architecture:** Three independent work streams that don't overlap on files: (1) pure .pyi stub creation/updates, (2) new Rust bindings + stubs, (3) existing binding enhancements. Streams can run in parallel.

**Tech Stack:** Rust (pyo3), Python (.pyi type stubs), maturin build system

**Design Doc:** `docs/plans/2026-02-28-python-bindings-parity-design.md`

---

## Stream 1: Pure .pyi Stubs (no Rust changes)

### Task 1: Create 8 missing instrument .pyi stubs (P0-2)

**Files:**
- Read: `finstack-py/src/valuations/instruments/bond_future.rs`
- Read: `finstack-py/src/valuations/instruments/equity_index_future.rs`
- Read: `finstack-py/src/valuations/instruments/fx_variance_swap.rs`
- Read: `finstack-py/src/valuations/instruments/inflation_cap_floor.rs`
- Read: `finstack-py/src/valuations/instruments/levered_real_estate_equity.rs`
- Read: `finstack-py/src/valuations/instruments/ndf.rs`
- Read: `finstack-py/src/valuations/instruments/real_estate.rs`
- Read: `finstack-py/src/valuations/instruments/xccy_swap.rs`
- Template: `finstack-py/finstack/valuations/instruments/bond.pyi` (docstring pattern)
- Template: `finstack-py/finstack/valuations/instruments/commodity_option.pyi` (builder pattern)
- Create: `finstack-py/finstack/valuations/instruments/bond_future.pyi`
- Create: `finstack-py/finstack/valuations/instruments/equity_index_future.pyi`
- Create: `finstack-py/finstack/valuations/instruments/fx_variance_swap.pyi`
- Create: `finstack-py/finstack/valuations/instruments/inflation_cap_floor.pyi`
- Create: `finstack-py/finstack/valuations/instruments/levered_real_estate_equity.pyi`
- Create: `finstack-py/finstack/valuations/instruments/ndf.pyi`
- Create: `finstack-py/finstack/valuations/instruments/real_estate.pyi`
- Create: `finstack-py/finstack/valuations/instruments/xccy_swap.pyi`

**Step 1: For each of the 8 Rust binding files, read the full file and extract:**
- All `#[pyclass]` structs -> become classes in .pyi
- All `#[pymethods]` blocks -> become method signatures
- All `#[new]` methods -> become `__init__`
- All `#[getter]` methods -> become `@property`
- All builder methods returning `PyRefMut<'_, Self>` -> return `Self` in .pyi
- All `#[classmethod]` methods -> `@classmethod`
- All `#[classattr]` -> class-level constants
- Docstrings from `///` comments -> Python docstrings

**Step 2: Write each .pyi following this pattern:**

```python
"""<Module docstring from Rust file's //! comments>."""

from __future__ import annotations

from datetime import date
from typing import overload

class InstrumentBuilder:
    """Builder for Instrument.

    Parameters
    ----------
    <from builder's required fields>

    Examples
    --------
        >>> builder = InstrumentBuilder()
        >>> instrument = builder.field1("value").field2(123).build()
    """

    def __init__(self) -> None: ...
    def field1(self, value: str) -> InstrumentBuilder: ...
    def build(self) -> Instrument: ...

class Instrument:
    """<Comprehensive docstring with Parameters, Examples, Notes, Sources>."""

    @property
    def field1(self) -> str: ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
```

Use PEP 604 style (`str | None` not `Optional[str]`) and `from __future__ import annotations`.

**Step 3: Verify stubs are syntactically valid**

Run: `python -c "import ast; ast.parse(open('finstack-py/finstack/valuations/instruments/bond_future.pyi').read())"`

Repeat for all 8 files.

**Step 4: Commit**

```bash
git add finstack-py/finstack/valuations/instruments/{bond_future,equity_index_future,fx_variance_swap,inflation_cap_floor,levered_real_estate_equity,ndf,real_estate,xccy_swap}.pyi
git commit -m "feat(stubs): add 8 missing instrument .pyi stubs (P0-2)"
```

---

### Task 2: Add ~21 missing re-exports to instruments/**init**.pyi (P0-3)

**Files:**
- Modify: `finstack-py/finstack/valuations/instruments/__init__.pyi`

**Step 1: Add imports for existing stubs not yet re-exported:**

Add these import lines after the existing imports (before `__all__`):

```python
from .swaption import Swaption as Swaption
from .inflation_linked_bond import InflationLinkedBond as InflationLinkedBond
from .inflation_swap import InflationSwap as InflationSwap
from .repo import Repo as Repo
from .variance_swap import VarianceSwap as VarianceSwap
from .asian_option import AsianOption as AsianOption
from .autocallable import Autocallable as Autocallable
from .dcf import DiscountedCashFlow as DiscountedCashFlow
from .trs import EquityTotalReturnSwap as EquityTotalReturnSwap, FIIndexTotalReturnSwap as FIIndexTotalReturnSwap
from .basket import Basket as Basket
from .bond_future import BondFuture as BondFuture
from .equity_index_future import EquityIndexFuture as EquityIndexFuture
from .fx_variance_swap import FxVarianceSwap as FxVarianceSwap
from .inflation_cap_floor import InflationCapFloor as InflationCapFloor
from .levered_real_estate_equity import LeveredRealEstateEquity as LeveredRealEstateEquity
from .ndf import Ndf as Ndf
from .real_estate import RealEstateAsset as RealEstateAsset
from .xccy_swap import CrossCurrencySwap as CrossCurrencySwap
```

NOTE: Read each .pyi stub first to verify the exact class names exported. The names above are based on the Rust binding `#[pyclass(name = "...")]` attributes but must be confirmed.

**Step 2: Add corresponding entries to `__all__`:**

Add to the `__all__` list:

```python
    # Additional instruments
    "Swaption",
    "InflationLinkedBond",
    "InflationSwap",
    "Repo",
    "VarianceSwap",
    "AsianOption",
    "Autocallable",
    "DiscountedCashFlow",
    "EquityTotalReturnSwap",
    "FIIndexTotalReturnSwap",
    "Basket",
    "BondFuture",
    "EquityIndexFuture",
    "FxVarianceSwap",
    "InflationCapFloor",
    "LeveredRealEstateEquity",
    "Ndf",
    "RealEstateAsset",
    "CrossCurrencySwap",
```

**Step 3: Commit**

```bash
git add finstack-py/finstack/valuations/instruments/__init__.pyi
git commit -m "feat(stubs): add missing instrument re-exports to __init__.pyi (P0-3)"
```

---

### Task 3: Create portfolio/optimization.pyi (P0-4)

**Files:**
- Read: `finstack-py/src/portfolio/optimization.rs` (1245 lines, 16 classes)
- Read: `finstack-py/finstack/portfolio/__init__.pyi` (to see existing pattern)
- Create: `finstack-py/finstack/portfolio/optimization.pyi`
- Modify: `finstack-py/finstack/portfolio/__init__.pyi` (add optimization imports)

**Step 1: Read the Rust binding file and create .pyi with these 16 classes:**

Enums (with `#[classattr]` constants):
- `WeightingScheme` -- VALUE_WEIGHT, NOTIONAL_WEIGHT, UNIT_SCALING
- `MissingMetricPolicy` -- ZERO, EXCLUDE, STRICT
- `Inequality` -- LE, GE, EQ
- `OptimizationStatus` -- (read classattrs from Rust)
- `TradeDirection` -- BUY, SELL, HOLD
- `TradeType` -- EXISTING, NEW_POSITION, CLOSE_OUT

Builder/Value classes:
- `PerPositionMetric` -- static methods: metric(), custom_key(), pv_base(), pv_native(), tag_equals(), constant()
- `MetricExpr` -- weighted_sum(), value_weighted_average(), tag_exposure_share()
- `Objective` -- maximize(), minimize()
- `PositionFilter` -- all(), by_entity_id(), by_tag(), by_position_ids(), not_()
- `Constraint` -- metric_bound(), tag_exposure_limit(), etc.
- `TradeSpec` -- read-only fields via properties
- `OptimizationResult` -- getters + to_rebalanced_portfolio(), to_trade_list(), binding_constraints()
- `CandidatePosition` -- new(), with_tag(), with_max_weight(), with_min_weight()
- `TradeUniverse` -- all_positions(), filtered(), with_candidate(), etc.
- `PortfolioOptimizationProblem` -- new(), with_trade_universe(), with_constraint(), etc.

**Step 2: Add imports to portfolio/**init**.pyi**

```python
from .optimization import (
    WeightingScheme as WeightingScheme,
    MissingMetricPolicy as MissingMetricPolicy,
    # ... all 16 classes
)
```

**Step 3: Verify syntax and commit**

```bash
git add finstack-py/finstack/portfolio/optimization.pyi finstack-py/finstack/portfolio/__init__.pyi
git commit -m "feat(stubs): add portfolio optimization .pyi stubs (P0-4)"
```

---

### Task 4: Create valuations/covenants.pyi (P0-5)

**Files:**
- Read: `finstack-py/src/valuations/covenants.rs` (553 lines)
- Create: `finstack-py/finstack/valuations/covenants.pyi`
- Modify: `finstack-py/finstack/valuations/__init__.pyi` (add covenants import if not present)

**Step 1: Read the Rust binding and create .pyi with:**

Classes:
- `CovenantType` -- classattrs + factory methods (max_debt_to_ebitda, min_interest_coverage, etc.)
- `Covenant` -- with_scope(), with_springing_condition()
- `CovenantSpec` -- metric specifications
- `CovenantScope` -- MAINTENANCE, INCURRENCE classattrs
- `SpringingCondition` -- conditional covenant activation
- `CovenantForecastConfig` -- stochastic forecasting config
- `CovenantForecast` -- getters for test_dates, projected_values, thresholds, headroom, breach_probability, etc. + explain(), to_polars()
- `FutureBreach` -- breach data

Functions:
- `forecast_covenant(...)` -- single covenant forecast
- `forecast_breaches(...)` -- breach forecasting

**Step 2: Verify syntax and commit**

```bash
git add finstack-py/finstack/valuations/covenants.pyi
git commit -m "feat(stubs): add covenant system .pyi stubs (P0-5)"
```

---

### Task 5: Create calibration/methods.pyi (P0-6)

**Files:**
- Read: `finstack-py/src/valuations/calibration/methods.rs` (full file)
- Create: `finstack-py/finstack/valuations/calibration/methods.pyi`
- Modify: `finstack-py/finstack/valuations/calibration/__init__.pyi` (add methods imports)

**Step 1: Read the Rust binding and create .pyi with 6 calibrator classes:**

- `DiscountCurveCalibrator` -- calibrate() method returning DiscountCurve
- `ForwardCurveCalibrator` -- calibrate() method returning ForwardCurve
- `HazardCurveCalibrator` -- calibrate() method returning HazardCurve
- `InflationCurveCalibrator` -- calibrate() method returning InflationCurve
- `VolSurfaceCalibrator` -- calibrate() method returning VolSurface
- `BaseCorrelationCalibrator` -- calibrate() method returning BaseCorrelationCurve

Each calibrator has constructor params and a `calibrate()` method. Read the exact signatures from the Rust file.

**Step 2: Add imports to calibration/**init**.pyi**

Add to the imports and **all**:

```python
from .methods import (
    DiscountCurveCalibrator as DiscountCurveCalibrator,
    ForwardCurveCalibrator as ForwardCurveCalibrator,
    HazardCurveCalibrator as HazardCurveCalibrator,
    InflationCurveCalibrator as InflationCurveCalibrator,
    VolSurfaceCalibrator as VolSurfaceCalibrator,
    BaseCorrelationCalibrator as BaseCorrelationCalibrator,
)
```

**Step 3: Verify syntax and commit**

```bash
git add finstack-py/finstack/valuations/calibration/methods.pyi finstack-py/finstack/valuations/calibration/__init__.pyi
git commit -m "feat(stubs): add calibration methods .pyi stubs (P0-6)"
```

---

### Task 6: Create valuations/dataframe.pyi (P0-7)

**Files:**
- Read: `finstack-py/src/valuations/dataframe.rs` (155 lines)
- Create: `finstack-py/finstack/valuations/dataframe.pyi`

**Step 1: Read the Rust binding and create .pyi with 3 functions:**

```python
"""DataFrame export utilities for valuation results."""

from __future__ import annotations

from typing import Any
from .results import ValuationResult

def results_to_polars(results: list[ValuationResult]) -> Any:
    """Convert valuation results to a Polars DataFrame.

    Parameters
    ----------
    results : list[ValuationResult]
        List of valuation results from pricing operations.

    Returns
    -------
    polars.DataFrame
        DataFrame with one row per result, columns for all metrics.
    """
    ...

def results_to_pandas(results: list[ValuationResult]) -> Any:
    """Convert valuation results to a Pandas DataFrame.

    Parameters
    ----------
    results : list[ValuationResult]
        List of valuation results from pricing operations.

    Returns
    -------
    pandas.DataFrame
        DataFrame with one row per result.
    """
    ...

def results_to_parquet(results: list[ValuationResult], path: str) -> None:
    """Export valuation results to a Parquet file.

    Parameters
    ----------
    results : list[ValuationResult]
        List of valuation results.
    path : str
        Output file path.
    """
    ...
```

Verify exact signatures against the Rust binding before writing.

**Step 2: Verify syntax and commit**

```bash
git add finstack-py/finstack/valuations/dataframe.pyi
git commit -m "feat(stubs): add dataframe export .pyi stubs (P0-7)"
```

---

### Task 7: Fix MetricId comparison methods in metrics.pyi (P1-5)

**Files:**
- Modify: `finstack-py/finstack/valuations/metrics.pyi:165`

**Step 1: Replace **richcmp** with standard dunder methods**

Replace:

```python
    def __richcmp__(self, other: object, op: int) -> object: ...
```

With:

```python
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __lt__(self, other: MetricId) -> bool: ...
    def __le__(self, other: MetricId) -> bool: ...
    def __gt__(self, other: MetricId) -> bool: ...
    def __ge__(self, other: MetricId) -> bool: ...
```

**Step 2: Commit**

```bash
git add finstack-py/finstack/valuations/metrics.pyi
git commit -m "fix(stubs): replace __richcmp__ with standard comparison methods on MetricId (P1-5)"
```

---

### Task 8: Fix base_date to @property on curve stubs (P1-6)

**Files:**
- Modify: `finstack-py/finstack/core/market_data/term_structures.pyi`
- Read: `finstack-py/src/core/market_data/term_structures.rs` (verify #[getter] usage)

**Step 1: Read the Rust binding to confirm `base_date` uses `#[getter]`**

**Step 2: For each of DiscountCurve, ForwardCurve, HazardCurve -- change:**

From:

```python
    def base_date(self) -> date:
```

To:

```python
    @property
    def base_date(self) -> date:
```

Search for ALL `def base_date(self)` in the file and add `@property` decorator above each.

**Step 3: Commit**

```bash
git add finstack-py/finstack/core/market_data/term_structures.pyi
git commit -m "fix(stubs): mark base_date as @property on curve types (P1-6)"
```

---

### Task 9: Add missing properties to equity_option.pyi and irs.pyi (P1-7)

**Files:**
- Read: `finstack-py/src/valuations/instruments/equity_option.rs` (find #[getter] methods)
- Read: `finstack-py/src/valuations/instruments/irs.rs` (find #[getter] methods)
- Modify: `finstack-py/finstack/valuations/instruments/equity_option.pyi`
- Modify: `finstack-py/finstack/valuations/instruments/irs.pyi`

**Step 1: Read Rust binding files and identify all #[getter] methods not in stubs**

Expected missing from equity_option.pyi: `notional`, `spot_id`, `div_yield_id`
Expected missing from irs.pyi: `notional`, `side`, `fixed_rate`, `float_spread_bp`, `start`, `end`

**Step 2: Add @property definitions to each .pyi**

For equity_option.pyi:

```python
    @property
    def notional(self) -> float: ...

    @property
    def spot_id(self) -> str: ...

    @property
    def div_yield_id(self) -> str | None: ...
```

For irs.pyi -- add similar @property stubs for each missing getter.

**Step 3: Commit**

```bash
git add finstack-py/finstack/valuations/instruments/equity_option.pyi finstack-py/finstack/valuations/instruments/irs.pyi
git commit -m "fix(stubs): add missing properties to equity_option and irs stubs (P1-7)"
```

---

### Task 10: Add 5 missing classes to extensions.pyi (P1-8)

**Files:**
- Read: `finstack-py/src/statements/extensions/mod.rs` (or extensions.rs)
- Read: `finstack-py/finstack/statements/extensions/extensions.pyi`
- Modify: `finstack-py/finstack/statements/extensions/extensions.pyi`

**Step 1: Read the Rust binding to find AccountType, CorkscrewAccount, CorkscrewConfig, ScorecardMetric, ScorecardConfig**

**Step 2: Add class stubs with @property getters and builder methods matching the Rust API**

**Step 3: Commit**

```bash
git add finstack-py/finstack/statements/extensions/extensions.pyi
git commit -m "feat(stubs): add 5 missing extension classes (P1-8)"
```

---

### Task 11: Add npv_static, npv_using_curve_dc to cashflow stubs (P1-9)

**Files:**
- Read: `finstack-py/src/core/cashflow/discounting.rs`
- Modify: `finstack-py/finstack/core/cashflow/__init__.pyi`

**Step 1: Add imports and function stubs:**

```python
from datetime import date
from finstack.core.money import Money
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.dates.daycount import DayCount

def npv_static(
    curve: DiscountCurve,
    base_date: date,
    day_count: DayCount | str,
    cash_flows: list[tuple[date, Money]],
) -> Money:
    """Calculate NPV using explicit day-count convention.

    Parameters
    ----------
    curve : DiscountCurve
        Discount curve for present value calculation.
    base_date : date
        Valuation date.
    day_count : DayCount or str
        Day count convention for time calculation.
    cash_flows : list[tuple[date, Money]]
        Future cashflows as (date, amount) pairs.

    Returns
    -------
    Money
        Net present value in the cashflow currency.
    """
    ...

def npv_using_curve_dc(
    curve: DiscountCurve,
    base_date: date,
    cash_flows: list[tuple[date, Money]],
) -> Money:
    """Calculate NPV using the curve's internal day-count convention.

    Parameters
    ----------
    curve : DiscountCurve
        Discount curve (provides both discounting and day-count).
    base_date : date
        Valuation date.
    cash_flows : list[tuple[date, Money]]
        Future cashflows as (date, amount) pairs.

    Returns
    -------
    Money
        Net present value.
    """
    ...
```

**Step 2: Add to **all****

**Step 3: Commit**

```bash
git add finstack-py/finstack/core/cashflow/__init__.pyi
git commit -m "feat(stubs): add npv_static and npv_using_curve_dc to cashflow stubs (P1-9)"
```

---

### Task 12: Standardize annotation style across all .pyi stubs (P2-4)

**Files:**
- All .pyi files under `finstack-py/finstack/`

**Step 1: Find all .pyi files using Optional or Union without `from __future__ import annotations`**

Search for: `Optional[` and `Union[` in .pyi files. Replace with PEP 604 syntax.

**Step 2: For each file:**
- Ensure `from __future__ import annotations` is at the top (after module docstring)
- Replace `Optional[X]` with `X | None`
- Replace `Union[X, Y]` with `X | Y`
- Remove `from typing import Optional, Union` if no longer needed

**Step 3: Commit**

```bash
git add finstack-py/finstack/**/*.pyi
git commit -m "style(stubs): standardize to PEP 604 annotation style (P2-4)"
```

---

### Task 13: Add @overload signatures where Union types are used (P2-3)

**Files:**
- All .pyi files that accept `Union[DayCount, str]` or similar unions

**Step 1: Find methods that accept union types where overloads would help**

Common pattern:

```python
# Before
def from_rates(cls, rates: list[tuple[float, float]], day_count: DayCount | str = ...) -> DiscountCurve: ...

# After
@overload
@classmethod
def from_rates(cls, rates: list[tuple[float, float]], day_count: DayCount = ...) -> DiscountCurve: ...
@overload
@classmethod
def from_rates(cls, rates: list[tuple[float, float]], day_count: str = ...) -> DiscountCurve: ...
```

Only add overloads where it genuinely helps IDE inference. Don't add overloads to every method -- focus on constructors and factory methods.

**Step 2: Commit**

```bash
git add finstack-py/finstack/**/*.pyi
git commit -m "feat(stubs): add @overload signatures for union types (P2-3)"
```

---

### Task 14: Add NumPy-style docstrings with academic references to all instrument stubs (P2-5 + P2-7)

**Files:**
- All .pyi files under `finstack-py/finstack/valuations/instruments/`
- Template: `finstack-py/finstack/valuations/instruments/bond.pyi`

**Step 1: For each instrument .pyi that lacks comprehensive docstrings, add:**

- Class docstring with description and Parameters section
- Builder method docstrings with Parameters/Returns
- `Sources` section with academic references for the pricing model
- `Examples` section showing basic usage

Reference the Rust crate source for academic citations (the `//!` doc comments often cite papers).

**Step 2: Commit per batch (group related instruments)**

```bash
git commit -m "docs(stubs): add NumPy docstrings and references to instrument stubs (P2-5, P2-7)"
```

---

## Stream 2: New Rust Bindings + Stubs

### Task 15: Create FxForward Python binding (P0-1a)

**Files:**
- Read: `finstack/valuations/src/instruments/fx/fx_forward/types.rs` (Rust struct)
- Read: `finstack/valuations/src/instruments/fx/fx_forward/pricer.rs` (pricing logic)
- Template: `finstack-py/src/valuations/instruments/ndf.rs` (similar FX instrument)
- Create: `finstack-py/src/valuations/instruments/fx_forward.rs`
- Create: `finstack-py/finstack/valuations/instruments/fx_forward.pyi`
- Modify: `finstack-py/src/valuations/instruments/mod.rs`

**Step 1: Read the Rust crate FxForward struct to identify all fields:**
- id: InstrumentId
- base_currency, quote_currency: Currency
- maturity: Date
- notional: Money
- contract_rate: Option<f64>
- domestic_discount_curve_id, foreign_discount_curve_id: CurveId
- spot_rate_override: Option<f64>
- base_calendar_id, quote_calendar_id: Option<String>
- pricing_overrides: PricingOverrides
- attributes: Attributes

**Step 2: Write the binding file following the ndf.rs pattern:**

Create `finstack-py/src/valuations/instruments/fx_forward.rs`:

```rust
use crate::core::common::args::{CurrencyArg, DayCountArg};
use crate::core::dates::utils::py_to_date;
use crate::core::money::PyMoney;
use crate::errors::core_to_py;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_forward::FxForward;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, Py, PyAny, PyRefMut};
use std::sync::Arc;

#[pyclass(
    module = "finstack.valuations.instruments.fx_forward",
    name = "FxForward",
    frozen,
)]
#[derive(Clone)]
pub struct PyFxForward {
    pub(crate) inner: Arc<FxForward>,
}

// ... builder pattern and methods following ndf.rs structure
// Read ndf.rs for exact pattern to follow
```

Key fields to expose as builder methods:
- `id(value: str)` -> InstrumentId
- `base_currency(value: str)` -> Currency
- `quote_currency(value: str)` -> Currency
- `maturity(value: PyAny)` -> Date
- `notional(value: PyMoney)` -> Money
- `contract_rate(value: f64)` -> Optional rate
- `domestic_discount_curve_id(value: str)` -> CurveId
- `foreign_discount_curve_id(value: str)` -> CurveId

Key properties to expose as getters:
- All of the above fields
- `instrument_type` -> InstrumentType

**Step 3: Register in mod.rs -- add these lines:**

At the top of mod.rs:

```rust
mod fx_forward;
```

In the use block:

```rust
use fx_forward::PyFxForward;
```

In `extract_instrument()`:

```rust
try_extract_arc!(value, PyFxForward, InstrumentType::FxForward);
```

In `register()`:

```rust
let fx_forward_exports = fx_forward::register(py, &module)?;
exports.extend(fx_forward_exports.iter().copied());
```

**Step 4: Create .pyi stub**

**Step 5: Build and test**

Run: `cd finstack-py && maturin develop --release`
Expected: Build succeeds

Run: `python -c "from finstack.valuations.instruments import FxForward; print(FxForward)"`
Expected: `<class 'finstack.valuations.instruments.fx_forward.FxForward'>`

**Step 6: Commit**

```bash
git add finstack-py/src/valuations/instruments/fx_forward.rs finstack-py/src/valuations/instruments/mod.rs finstack-py/finstack/valuations/instruments/fx_forward.pyi
git commit -m "feat: add FxForward Python binding (P0-1)"
```

---

### Task 16: Create FxDigitalOption Python binding (P0-1b)

**Files:**
- Read: `finstack/valuations/src/instruments/fx/fx_digital_option/types.rs`
- Template: `finstack-py/src/valuations/instruments/fx_barrier_option.rs` (similar exotic)
- Create: `finstack-py/src/valuations/instruments/fx_digital_option.rs`
- Create: `finstack-py/finstack/valuations/instruments/fx_digital_option.pyi`
- Modify: `finstack-py/src/valuations/instruments/mod.rs`

Same pattern as Task 15. Key struct fields:
- DigitalPayoutType enum (CashOrNothing, AssetOrNothing)
- payout_amount, strike, option_type
- Standard FX option fields (base/quote ccy, expiry, discount curves, vol surface)

Follow same registration pattern. Build and test.

```bash
git commit -m "feat: add FxDigitalOption Python binding (P0-1)"
```

---

### Task 17: Create FxTouchOption Python binding (P0-1c)

**Files:**
- Read: `finstack/valuations/src/instruments/fx/fx_touch_option/types.rs`
- Create: `finstack-py/src/valuations/instruments/fx_touch_option.rs`
- Create: `finstack-py/finstack/valuations/instruments/fx_touch_option.pyi`
- Modify: `finstack-py/src/valuations/instruments/mod.rs`

Key enums to expose:
- `TouchType` (OneTouch, NoTouch)
- `BarrierDirection` (Up, Down)
- `PayoutTiming` (AtHit, AtExpiry)

Follow same registration pattern. Build and test.

```bash
git commit -m "feat: add FxTouchOption Python binding (P0-1)"
```

---

### Task 18: Create CommodityAsianOption Python binding (P0-1d)

**Files:**
- Read: `finstack/valuations/src/instruments/commodity/commodity_asian_option/types.rs`
- Template: `finstack-py/src/valuations/instruments/asian_option.rs` (similar Asian)
- Create: `finstack-py/src/valuations/instruments/commodity_asian_option.rs`
- Create: `finstack-py/finstack/valuations/instruments/commodity_asian_option.pyi`
- Modify: `finstack-py/src/valuations/instruments/mod.rs`

Key fields:
- fixing_dates: Vec<Date>
- realized_fixings: Vec<(Date, f64)>
- averaging_type: AveragingType (Arithmetic/Geometric)
- Standard commodity option fields

Follow same registration pattern. Build and test.

```bash
git commit -m "feat: add CommodityAsianOption Python binding (P0-1)"
```

---

### Task 19: Add update instruments/**init**.pyi for 4 new instruments

**Files:**
- Modify: `finstack-py/finstack/valuations/instruments/__init__.pyi`

Add imports and **all** entries for FxForward, FxDigitalOption, FxTouchOption, CommodityAsianOption. Also add to instruments/**init**.pyi re-exports.

```bash
git commit -m "feat(stubs): add 4 new instruments to __init__.pyi re-exports"
```

---

### Task 20: Expose core volatility pricing functions (P1-1)

**Files:**
- Read: `finstack/core/src/math/volatility/pricing.rs` (source functions)
- Read: `finstack/core/src/math/volatility/implied.rs` (implied vol solvers)
- Modify: `finstack-py/src/core/market_data/volatility.rs` (extend with new functions)
- Modify: `finstack-py/finstack/core/volatility.pyi` (extend stubs)

**Step 1: Add #[pyfunction] wrappers for each pricing function:**

```rust
#[pyfunction(name = "black_call", signature = (forward, strike, sigma, t))]
fn py_black_call(forward: f64, strike: f64, sigma: f64, t: f64) -> PyResult<f64> {
    Ok(finstack_core::math::volatility::pricing::black_call(forward, strike, sigma, t))
}

#[pyfunction(name = "black_put", signature = (forward, strike, sigma, t))]
fn py_black_put(forward: f64, strike: f64, sigma: f64, t: f64) -> PyResult<f64> {
    Ok(finstack_core::math::volatility::pricing::black_put(forward, strike, sigma, t))
}
```

Add ALL functions listed:
- black_call, black_put, black_vega, black_delta_call, black_delta_put, black_gamma
- bachelier_call, bachelier_put, bachelier_vega, bachelier_delta_call, bachelier_delta_put, bachelier_gamma
- black_shifted_call, black_shifted_put, black_shifted_vega
- implied_vol_black (wraps Result, map err to PyValueError)
- implied_vol_bachelier (wraps Result, map err to PyValueError)

**Step 2: Register all new functions in the module's register() function**

**Step 3: Update .pyi stubs with all new function signatures**

**Step 4: Build and test**

Run: `cd finstack-py && maturin develop --release`
Run: `python -c "from finstack.core.volatility import black_call; print(black_call(100, 100, 0.2, 1.0))"`
Expected: ~7.97 (Black-76 ATM call price)

**Step 5: Commit**

```bash
git commit -m "feat: expose Black/Bachelier/shifted pricing and implied vol functions (P1-1)"
```

---

### Task 21: Expose Hull-White calibration (P1-2)

**Files:**
- Read: `finstack/valuations/src/calibration/hull_white.rs` (source)
- Create: `finstack-py/src/valuations/calibration/hull_white.rs`
- Create: `finstack-py/finstack/valuations/calibration/hull_white.pyi`
- Modify: `finstack-py/src/valuations/calibration/mod.rs` (register)
- Modify: `finstack-py/finstack/valuations/calibration/__init__.pyi` (import)

**Step 1: Create binding with:**

```rust
#[pyclass(name = "HullWhiteParams", frozen)]
pub struct PyHullWhiteParams {
    pub(crate) inner: HullWhiteParams,
}

#[pymethods]
impl PyHullWhiteParams {
    #[new]
    fn new(kappa: f64, sigma: f64) -> PyResult<Self> { ... }

    #[getter]
    fn kappa(&self) -> f64 { self.inner.kappa }

    #[getter]
    fn sigma(&self) -> f64 { self.inner.sigma }
}

#[pyclass(name = "SwaptionQuote", frozen)]
pub struct PySwaptionQuote { ... }

#[pyfunction(name = "calibrate_hull_white_to_swaptions")]
fn py_calibrate_hull_white_to_swaptions(...) -> PyResult<PyHullWhiteParams> { ... }
```

**Step 2: Build and test**

**Step 3: Commit**

```bash
git commit -m "feat: expose Hull-White calibration (P1-2)"
```

---

### Task 22: Expose missing stats functions (P1-10)

**Files:**
- Read: `finstack/core/src/math/stats.rs` (OnlineStats, OnlineCovariance, quantile, population_variance)
- Modify: `finstack-py/src/core/math/stats.rs`
- Modify: `finstack-py/finstack/core/math/stats.pyi`

**Step 1: Add population_variance and quantile functions:**

```rust
#[pyfunction(name = "population_variance", signature = (data))]
fn population_variance_py(data: Vec<f64>) -> PyResult<f64> {
    if data.is_empty() {
        return Err(PyValueError::new_err("data must not be empty"));
    }
    Ok(finstack_core::math::stats::population_variance(&data))
}

#[pyfunction(name = "quantile", signature = (data, p))]
fn quantile_py(data: Vec<f64>, p: f64) -> PyResult<f64> {
    if data.is_empty() {
        return Err(PyValueError::new_err("data must not be empty"));
    }
    if !(0.0..=1.0).contains(&p) {
        return Err(PyValueError::new_err("p must be in [0, 1]"));
    }
    Ok(finstack_core::math::stats::quantile(&data, p))
}
```

**Step 2: Add OnlineStats class:**

```rust
#[pyclass(name = "OnlineStats", module = "finstack.core.math.stats")]
pub struct PyOnlineStats {
    inner: finstack_core::math::stats::OnlineStats,
}

#[pymethods]
impl PyOnlineStats {
    #[new]
    fn new() -> Self { Self { inner: OnlineStats::new() } }

    fn update(&mut self, value: f64) { self.inner.update(value); }
    fn merge(&mut self, other: &PyOnlineStats) { self.inner.merge(&other.inner); }
    fn count(&self) -> usize { self.inner.count() }
    fn mean(&self) -> f64 { self.inner.mean() }
    fn variance(&self) -> f64 { self.inner.variance() }
    fn std_dev(&self) -> f64 { self.inner.std_dev() }
    fn stderr(&self) -> f64 { self.inner.stderr() }
    fn reset(&mut self) { self.inner.reset(); }
}
```

**Step 3: Add OnlineCovariance class (similar pattern)**

**Step 4: Register, update stubs, build, test**

**Step 5: Commit**

```bash
git commit -m "feat: expose population_variance, quantile, OnlineStats, OnlineCovariance (P1-10)"
```

---

### Task 23: Expose advanced solver APIs (P1-11)

**Files:**
- Read: `finstack/core/src/math/solver.rs` (solve_with_derivative, BracketHint)
- Read: `finstack/core/src/math/solver_multi.rs` (LmStats, LmSolution, LmTerminationReason)
- Modify: `finstack-py/src/core/math/solver.rs`
- Modify: `finstack-py/src/core/math/solver_multi.rs`
- Modify: `finstack-py/finstack/core/math/solver.pyi`
- Modify: `finstack-py/finstack/core/math/solver_multi.pyi`

**Step 1: Add solve_with_derivative to NewtonSolver**

**Step 2: Add LmStats, LmSolution, LmTerminationReason classes**

**Step 3: Build, test, commit**

```bash
git commit -m "feat: expose solve_with_derivative, LM diagnostics (P1-11)"
```

---

### Task 24: Add SABR/Heston/SVI model bindings (NEW)

**Files:**
- Read: `finstack/core/src/math/volatility/heston.rs`
- Read: `finstack/core/src/math/volatility/sabr.rs`
- Read: `finstack/core/src/math/volatility/svi.rs`
- Create: `finstack-py/src/core/volatility_models.rs`
- Create: `finstack-py/finstack/core/volatility_models.pyi`
- Modify: `finstack-py/src/core/mod.rs` (register new module)

**Step 1: Create binding file with 3 classes:**

```rust
#[pyclass(name = "HestonParams", frozen)]
pub struct PyHestonParams {
    inner: HestonParams,
}

#[pymethods]
impl PyHestonParams {
    #[new]
    fn new(v0: f64, kappa: f64, theta: f64, sigma: f64, rho: f64) -> PyResult<Self> {
        HestonParams::new(v0, kappa, theta, sigma, rho)
            .map(|p| Self { inner: p })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn satisfies_feller_condition(&self) -> bool {
        self.inner.satisfies_feller_condition()
    }

    fn price_european(&self, spot: f64, strike: f64, r: f64, q: f64, t: f64, is_call: bool) -> f64 {
        self.inner.price_european(spot, strike, r, q, t, is_call)
    }

    // getters for v0, kappa, theta, sigma, rho
}
```

Similar for SabrParams (with implied_vol_lognormal, implied_vol_normal, atm_vol_lognormal) and SviParams.

**Step 2: Register, create stubs, build, test**

**Step 3: Commit**

```bash
git commit -m "feat: expose Heston, SABR, SVI volatility model bindings"
```

---

### Task 25: Expose Monte Carlo building blocks (P2-6)

**Files:**
- Read: existing MC bindings at `finstack-py/src/valuations/common/mc/`
- Check: what processes/discretizations exist in the Rust crate
- Create/extend: MC process bindings as needed

This task's scope depends on what's available in the Rust crate. Start by auditing:
1. What MC types are already exposed in Python (PathPoint, SimulatedPath, PathDataset, etc.)
2. What process types exist in the Rust crate but are NOT exposed
3. Prioritize the most commonly used ones (HestonProcess, GBM)

```bash
git commit -m "feat: expose Monte Carlo process and discretization building blocks (P2-6)"
```

---

## Stream 3: Existing Binding Enhancements

### Task 26: Add collection protocols to 5 types (P1-3)

**Files:**
- Modify: `finstack-py/src/portfolio/positions.rs`
- Modify: `finstack-py/src/valuations/cashflow/builder.rs` (or wherever CashFlowSchedule is)
- Modify: `finstack-py/src/valuations/common/mc/paths.rs`
- Modify: `finstack-py/src/core/market_data/fx.rs`
- Modify: `finstack-py/src/core/market_data/context.rs`
- Update: corresponding .pyi stubs

**Step 1: Add **iter** and **len** to PyPortfolio:**

```rust
#[pymethods]
impl PyPortfolio {
    fn __len__(&self) -> usize {
        self.inner.positions().len()
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyPositionIterator>> {
        let positions: Vec<_> = slf.inner.positions().iter().cloned().collect();
        Py::new(slf.py(), PyPositionIterator { positions, index: 0 })
    }

    fn __contains__(&self, position_id: &str) -> bool {
        self.inner.get_position(&position_id.into()).is_some()
    }
}

#[pyclass]
struct PyPositionIterator {
    positions: Vec<Position>,
    index: usize,
}

#[pymethods]
impl PyPositionIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }
    fn __next__(&mut self) -> Option<PyPosition> {
        if self.index < self.positions.len() {
            let pos = self.positions[self.index].clone();
            self.index += 1;
            Some(PyPosition::new(pos))
        } else {
            None
        }
    }
}
```

**Step 2: Similarly for CashFlowSchedule (**iter**, **len**, **getitem**)**

**Step 3: PathDataset -- add **iter** (already has **len**)**

**Step 4: FxMatrix -- add **contains** and **len****

**Step 5: MarketContext -- add **contains** (check if curve/surface ID exists)**

**Step 6: Update all corresponding .pyi stubs**

**Step 7: Build and test**

**Step 8: Commit**

```bash
git commit -m "feat: add __iter__/__len__/__contains__ to collection types (P1-3)"
```

---

### Task 27: Add **copy**/**deepcopy** to MarketContext (P1-4)

**Files:**
- Modify: `finstack-py/src/core/market_data/context.rs`
- Modify: `finstack-py/finstack/core/market_data/context.pyi`

**Step 1: Add copy and deepcopy methods:**

```rust
#[pymethods]
impl PyMarketContext {
    fn __copy__(&self) -> Self {
        Self { inner: self.inner.clone() }
    }

    fn __deepcopy__(&self, _memo: Bound<'_, PyAny>) -> Self {
        Self { inner: self.inner.clone() }
    }
}
```

Note: Since MarketContext likely uses Arc internally, **copy** and **deepcopy** may behave the same. Check if MarketContext has a true deep clone method.

**Step 2: Update .pyi stub**

**Step 3: Build and test**

Run: `python -c "import copy; from finstack.core.market_data import MarketContext; ctx = MarketContext(); ctx2 = copy.deepcopy(ctx); print(type(ctx2))"`

**Step 4: Commit**

```bash
git commit -m "feat: add copy/deepcopy support to MarketContext (P1-4)"
```

---

### Task 28: Add pickle support to commonly-serialized types (P2-1)

**Files:**
- Modify: `finstack-py/src/core/currency.rs`
- Modify: `finstack-py/src/core/types.rs` (Rate, Bps, CurveId, InstrumentId, Tenor, DayCount)
- Modify: `finstack-py/src/core/money.rs`
- Modify: `finstack-py/finstack/valuations/metrics.pyi` (MetricId)
- Update: corresponding .pyi stubs

**Step 1: For each type, add **reduce** or **getnewargs**:**

Example for Currency:

```rust
#[pymethods]
impl PyCurrency {
    fn __getnewargs__(&self) -> (String,) {
        (self.inner.to_string(),)
    }
}
```

Example for Money:

```rust
#[pymethods]
impl PyMoney {
    fn __reduce__(&self) -> PyResult<(PyObject, (f64, String))> {
        Python::with_gil(|py| {
            let cls = py.get_type::<PyMoney>();
            Ok((cls.into(), (self.inner.amount(), self.inner.currency().to_string())))
        })
    }
}
```

**Step 2: Build and test**

Run: `python -c "import pickle; from finstack.core import Currency; c = Currency('USD'); c2 = pickle.loads(pickle.dumps(c)); print(c2)"`

**Step 3: Commit**

```bash
git commit -m "feat: add pickle support to Currency, Rate, Bps, Money, and other core types (P2-1)"
```

---

### Task 29: Add **format** protocol to Money and Rate (P2-2)

**Files:**
- Modify: `finstack-py/src/core/money.rs`
- Modify: `finstack-py/src/core/types.rs`
- Update: corresponding .pyi stubs

**Step 1: Add **format** to Money:**

```rust
#[pymethods]
impl PyMoney {
    fn __format__(&self, spec: &str) -> PyResult<String> {
        if spec.is_empty() {
            return Ok(format!("{}", self.inner));
        }
        // Parse format spec: e.g., ",.2f" -> comma separator, 2 decimal places
        let amount = self.inner.amount();
        let ccy = self.inner.currency();
        // Use Rust's format machinery or manual formatting
        let formatted = format!("{amount:.prec$}", prec = parse_precision(spec));
        Ok(format!("{ccy} {formatted}"))
    }
}
```

**Step 2: Add **format** to Rate (similar pattern)**

**Step 3: Build and test**

Run: `python -c "from finstack.core import Money; m = Money(1234.5, 'USD'); print(f'{m:,.2f}')"`

**Step 4: Commit**

```bash
git commit -m "feat: add f-string formatting support to Money and Rate (P2-2)"
```

---

## Verification

After all tasks are complete:

### Final verification steps

1. **Build**: `cd finstack-py && maturin develop --release`
2. **Test suite**: `cd finstack-py && pytest tests/ -v`
3. **Type check**: `cd finstack-py && python -m pyright finstack/` (or mypy)
4. **Import check**: `python -c "from finstack.valuations.instruments import *; print('All imports OK')"`
5. **Stub syntax check**: `find finstack-py/finstack -name '*.pyi' -exec python -c "import ast; ast.parse(open('{}').read())" \;`
