# Valuations Python Bindings: 100% Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close every remaining gap between the Rust `finstack-valuations` public API and the `finstack-py` Python bindings, achieving 100% parity.

**Architecture:** Four independent work streams: (1) Enum completeness — add missing InstrumentType/ModelKey classattrs and .pyi stubs, (2) Constants module — expose valuations constants, (3) CashFlowSchedule protocols — add **iter**/**len**, (4) Stub polish — fix stale annotations and naming. Streams can run in parallel since they touch non-overlapping files.

**Tech Stack:** Rust (pyo3 0.28), Python (.pyi type stubs), maturin build system

**Design Doc:** `docs/python-bindings-parity-review.md`

---

## Status Update vs 2026-02-28 Review

Most items from the prior review have been **completed**. This plan covers only the verified remaining gaps:

| Prior Item | Status | Notes |
|---|---|---|
| P0-1: 4 missing instrument bindings | **DONE** | FxForward, FxDigitalOption, FxTouchOption, CommodityAsianOption all bound |
| P0-2: 8 missing instrument .pyi stubs | **DONE** | All instruments have .pyi stubs |
| P0-3: ~12 missing **init**.pyi re-exports | **DONE** | instruments/**init**.pyi comprehensive |
| P0-4: portfolio/optimization.pyi | **DONE** | Stub exists |
| P0-5: covenants.pyi | **DONE** | Stub exists |
| P0-6: calibration/methods.pyi | **N/A** | API uses execute_calibration; no separate calibrator classes exposed by design |
| P0-7: dataframe.pyi | **DONE** | Stub exists |
| P1-1: Volatility pricing functions | **DONE** | black_call, bachelier_call, etc. all bound + stub |
| P1-2: Hull-White calibration | **DONE** | hull_white.rs + hull_white.pyi |
| P1-3: Collection protocols | **MOSTLY DONE** | Portfolio, FxMatrix, MarketContext, PathDataset done; CashFlowSchedule still missing |
| P1-4: MarketContext copy/deepcopy | **DONE** | **copy** and **deepcopy** implemented |
| P1-5: MetricId **richcmp** fix | **DONE** | Standard comparison methods in stub |
| P1-6: base_date @property fix | **DONE** | All curve stubs use @property |
| P1-7: Missing properties on equity_option/irs | **DONE** | notional, spot_id, side, fixed_rate all present |
| P1-8: Extensions.pyi missing classes | **DONE** | Classes present |
| P1-9: npv_static, npv_using_curve_dc | **DONE** | In binding + stub |
| P1-10: Stats functions | **DONE** | population_variance, quantile, OnlineStats, OnlineCovariance all bound |
| P1-11: Solver APIs | **DONE** | solve_with_derivative, LmStats, LmSolution in binding + stub |
| P2-1: Pickle support | **DONE** | **getnewargs** on Currency, Money, Rate, Bps, CurveId, vol models |
| P2-2: **format** protocol | **DONE** | Money and Rate both have **format** |
| SABR/Heston/SVI models | **DONE** | volatility_models.rs + .pyi |

### What Remains (this plan)

| Gap | Severity | Effort |
|---|---|---|
| InstrumentType: 23 enum variants missing from binding classattrs | **P0** | 1 hour |
| InstrumentType .pyi: ~14 classattr variants missing + TRS name mismatch | **P0** | 30 min |
| ModelKey .pyi: 11 classattr variants missing from stub | **P0** | 15 min |
| Constants module: not exposed to Python | **P1** | 2 hours |
| CashFlowSchedule: missing **iter**/**len**/**getitem** | **P1** | 1 hour |
| InstrumentType/ModelKey/PricerKey .pyi: use **richcmp** instead of standard dunders | **P1** | 15 min |
| Annotation style: some stubs may still use Optional[] | **P2** | 1 hour |

---

## Stream 1: Enum Completeness (P0)

### Task 1: Add 23 missing InstrumentType classattrs to Rust binding

**Files:**
- Modify: `finstack-py/src/valuations/common/mod.rs:38-118`

**Step 1: Add the following 23 classattr constants** after the existing REVOLVING_CREDIT entry (line 118):

```rust
    #[classattr]
    const BERMUDAN_SWAPTION: Self = Self::new(InstrumentType::BermudanSwaption);
    #[classattr]
    const XCCY_SWAP: Self = Self::new(InstrumentType::XccySwap);
    #[classattr]
    const YOY_INFLATION_SWAP: Self = Self::new(InstrumentType::YoYInflationSwap);
    #[classattr]
    const INFLATION_CAP_FLOOR: Self = Self::new(InstrumentType::InflationCapFloor);
    #[classattr]
    const FX_VARIANCE_SWAP: Self = Self::new(InstrumentType::FxVarianceSwap);
    #[classattr]
    const TERM_LOAN: Self = Self::new(InstrumentType::TermLoan);
    #[classattr]
    const DCF: Self = Self::new(InstrumentType::DCF);
    #[classattr]
    const BOND_FUTURE: Self = Self::new(InstrumentType::BondFuture);
    #[classattr]
    const COMMODITY_FORWARD: Self = Self::new(InstrumentType::CommodityForward);
    #[classattr]
    const COMMODITY_SWAP: Self = Self::new(InstrumentType::CommoditySwap);
    #[classattr]
    const COMMODITY_OPTION: Self = Self::new(InstrumentType::CommodityOption);
    #[classattr]
    const COMMODITY_ASIAN_OPTION: Self = Self::new(InstrumentType::CommodityAsianOption);
    #[classattr]
    const VOLATILITY_INDEX_FUTURE: Self = Self::new(InstrumentType::VolatilityIndexFuture);
    #[classattr]
    const VOLATILITY_INDEX_OPTION: Self = Self::new(InstrumentType::VolatilityIndexOption);
    #[classattr]
    const EQUITY_INDEX_FUTURE: Self = Self::new(InstrumentType::EquityIndexFuture);
    #[classattr]
    const FX_FORWARD: Self = Self::new(InstrumentType::FxForward);
    #[classattr]
    const NDF: Self = Self::new(InstrumentType::Ndf);
    #[classattr]
    const AGENCY_MBS_PASSTHROUGH: Self = Self::new(InstrumentType::AgencyMbsPassthrough);
    #[classattr]
    const AGENCY_TBA: Self = Self::new(InstrumentType::AgencyTba);
    #[classattr]
    const DOLLAR_ROLL: Self = Self::new(InstrumentType::DollarRoll);
    #[classattr]
    const AGENCY_CMO: Self = Self::new(InstrumentType::AgencyCmo);
    #[classattr]
    const FX_DIGITAL_OPTION: Self = Self::new(InstrumentType::FxDigitalOption);
    #[classattr]
    const FX_TOUCH_OPTION: Self = Self::new(InstrumentType::FxTouchOption);
```

**Step 2: Verify the `parse_instrument_type` and `instrument_type_label` helper functions** (also in this file) handle all 63 variants. Read the full function bodies and cross-check against the Rust `InstrumentType::Display` impl in `finstack/valuations/src/pricer.rs:152-199`. Add any missing match arms.

**Step 3: Build**

Run: `cd finstack-py && maturin develop --release`
Expected: Build succeeds without errors.

**Step 4: Smoke test**

Run: `python -c "from finstack.valuations.common import InstrumentType; print(InstrumentType.FX_FORWARD.name)"`
Expected: `fx_forward`

Run: `python -c "from finstack.valuations.common import InstrumentType; print(InstrumentType.TERM_LOAN.name)"`
Expected: `term_loan`

**Step 5: Commit**

```bash
git add finstack-py/src/valuations/common/mod.rs
git commit -m "feat: add 23 missing InstrumentType classattr variants to Python binding"
```

---

### Task 2: Update InstrumentType .pyi stub to list all 63 variants

**Files:**
- Modify: `finstack-py/finstack/valuations/common/__init__.pyi:9-46`

**Step 1: Replace the InstrumentType class attributes** section (lines 19-45) with all 63 variants. Remove the erroneous `TRS` entry and replace with `EQUITY_TOTAL_RETURN_SWAP`:

```python
class InstrumentType:
    """Enumerates instrument families supported by the valuation engines.

    Examples:
        >>> from finstack.valuations.common import InstrumentType
        >>> InstrumentType.BOND.name
        'bond'
    """

    # Fixed Income
    BOND: InstrumentType
    LOAN: InstrumentType
    CONVERTIBLE: InstrumentType
    INFLATION_LINKED_BOND: InstrumentType
    TERM_LOAN: InstrumentType
    BOND_FUTURE: InstrumentType
    STRUCTURED_CREDIT: InstrumentType
    REVOLVING_CREDIT: InstrumentType
    AGENCY_MBS_PASSTHROUGH: InstrumentType
    AGENCY_TBA: InstrumentType
    DOLLAR_ROLL: InstrumentType
    AGENCY_CMO: InstrumentType

    # Interest Rates
    DEPOSIT: InstrumentType
    FRA: InstrumentType
    IRS: InstrumentType
    BASIS_SWAP: InstrumentType
    CAP_FLOOR: InstrumentType
    SWAPTION: InstrumentType
    BERMUDAN_SWAPTION: InstrumentType
    REPO: InstrumentType
    INTEREST_RATE_FUTURE: InstrumentType
    INFLATION_SWAP: InstrumentType
    YOY_INFLATION_SWAP: InstrumentType
    INFLATION_CAP_FLOOR: InstrumentType
    XCCY_SWAP: InstrumentType
    CMS_OPTION: InstrumentType
    RANGE_ACCRUAL: InstrumentType

    # Credit Derivatives
    CDS: InstrumentType
    CDS_INDEX: InstrumentType
    CDS_TRANCHE: InstrumentType
    CDS_OPTION: InstrumentType

    # FX
    FX_SPOT: InstrumentType
    FX_SWAP: InstrumentType
    FX_FORWARD: InstrumentType
    FX_OPTION: InstrumentType
    FX_BARRIER_OPTION: InstrumentType
    FX_DIGITAL_OPTION: InstrumentType
    FX_TOUCH_OPTION: InstrumentType
    FX_VARIANCE_SWAP: InstrumentType
    NDF: InstrumentType

    # Equity
    EQUITY: InstrumentType
    EQUITY_OPTION: InstrumentType
    EQUITY_TOTAL_RETURN_SWAP: InstrumentType
    FI_INDEX_TOTAL_RETURN_SWAP: InstrumentType
    VARIANCE_SWAP: InstrumentType
    EQUITY_INDEX_FUTURE: InstrumentType
    VOLATILITY_INDEX_FUTURE: InstrumentType
    VOLATILITY_INDEX_OPTION: InstrumentType
    PRIVATE_MARKETS_FUND: InstrumentType
    REAL_ESTATE_ASSET: InstrumentType
    LEVERED_REAL_ESTATE_EQUITY: InstrumentType
    DCF: InstrumentType
    AUTOCALLABLE: InstrumentType
    CLIQUET_OPTION: InstrumentType

    # Exotics
    ASIAN_OPTION: InstrumentType
    BARRIER_OPTION: InstrumentType
    LOOKBACK_OPTION: InstrumentType
    QUANTO_OPTION: InstrumentType
    BASKET: InstrumentType

    # Commodity
    COMMODITY_FORWARD: InstrumentType
    COMMODITY_SWAP: InstrumentType
    COMMODITY_OPTION: InstrumentType
    COMMODITY_ASIAN_OPTION: InstrumentType
```

Keep the existing `from_name`, `name`, `__repr__`, `__str__`, `__hash__` methods unchanged.

**Step 2: Replace the `__richcmp__` method** with standard comparison dunders:

Replace:

```python
    def __richcmp__(self, other: object, op: int) -> object: ...
```

With:

```python
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
```

(InstrumentType only supports eq/ne comparisons in the binding, not ordering.)

**Step 3: Verify stub syntax**

Run: `python -c "import ast; ast.parse(open('finstack-py/finstack/valuations/common/__init__.pyi').read()); print('OK')"`
Expected: `OK`

**Step 4: Commit**

```bash
git add finstack-py/finstack/valuations/common/__init__.pyi
git commit -m "feat(stubs): list all 63 InstrumentType variants in .pyi stub, fix TRS→EQUITY_TOTAL_RETURN_SWAP"
```

---

### Task 3: Update ModelKey .pyi stub to list all 16 variants

**Files:**
- Modify: `finstack-py/finstack/valuations/common/__init__.pyi:81-129`

**Step 1: Replace the ModelKey class attributes** (lines 91-95) with all 16 variants:

```python
class ModelKey:
    """Enumerates pricing model categories recognized by the registry.

    Examples:
        >>> from finstack.valuations.common import ModelKey
        >>> ModelKey.DISCOUNTING.name
        'discounting'
    """

    # Analytic / closed-form
    DISCOUNTING: ModelKey
    BLACK76: ModelKey
    NORMAL: ModelKey
    HULL_WHITE_1F: ModelKey
    HAZARD_RATE: ModelKey
    HESTON_FOURIER: ModelKey

    # Lattice
    TREE: ModelKey

    # Monte Carlo
    MONTE_CARLO_GBM: ModelKey
    MONTE_CARLO_HESTON: ModelKey
    MONTE_CARLO_HULL_WHITE_1F: ModelKey

    # Exotic closed-form
    BARRIER_BS_CONTINUOUS: ModelKey
    ASIAN_GEOMETRIC_BS: ModelKey
    ASIAN_TURNBULL_WAKEMAN: ModelKey
    LOOKBACK_BS_CONTINUOUS: ModelKey
    QUANTO_BS: ModelKey
    FX_BARRIER_BS_CONTINUOUS: ModelKey
```

**Step 2: Replace `__richcmp__` with standard eq/ne** (same pattern as InstrumentType).

**Step 3: Also fix PricerKey's `__richcmp__`** in the same file (line 175) — replace with `__eq__` and `__ne__`.

**Step 4: Verify syntax, commit**

```bash
git add finstack-py/finstack/valuations/common/__init__.pyi
git commit -m "feat(stubs): list all 16 ModelKey variants and fix __richcmp__ on all enum stubs"
```

---

## Stream 2: Constants Module (P1)

### Task 4: Create Python constants binding

**Files:**
- Read: `finstack/valuations/src/constants.rs` (source of truth)
- Create: `finstack-py/src/valuations/constants.rs`
- Modify: `finstack-py/src/valuations/mod.rs` (register new module)

**Step 1: Read the Rust constants module** to extract all public constants and their values.

**Step 2: Create `finstack-py/src/valuations/constants.rs`:**

```rust
//! Expose valuation constants from finstack-valuations to Python.

use finstack_valuations::constants;
use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "constants")?;
    module.setattr("__doc__", "Shared numerical constants for pricing and risk calculations.")?;

    // Top-level
    module.setattr("ONE_BASIS_POINT", constants::ONE_BASIS_POINT)?;
    module.setattr("BASIS_POINTS_PER_UNIT", constants::BASIS_POINTS_PER_UNIT)?;
    module.setattr("PERCENT_TO_DECIMAL", constants::PERCENT_TO_DECIMAL)?;
    module.setattr("DECIMAL_TO_PERCENT", constants::DECIMAL_TO_PERCENT)?;

    // Numerical tolerances
    let numerical = PyModule::new(py, "numerical")?;
    numerical.setattr("ZERO_TOLERANCE", constants::numerical::ZERO_TOLERANCE)?;
    numerical.setattr("INTEGRATION_STEP_FACTOR", constants::numerical::INTEGRATION_STEP_FACTOR)?;
    numerical.setattr("SOLVER_TOLERANCE", constants::numerical::SOLVER_TOLERANCE)?;
    numerical.setattr("RATE_COMPARISON_TOLERANCE", constants::numerical::RATE_COMPARISON_TOLERANCE)?;
    numerical.setattr("DIVISION_EPSILON", constants::numerical::DIVISION_EPSILON)?;
    numerical.setattr("RELATIVE_TOLERANCE", constants::numerical::RELATIVE_TOLERANCE)?;
    numerical.setattr("DF_EPSILON", constants::numerical::DF_EPSILON)?;
    module.add_submodule(&numerical)?;

    // ISDA conventions
    let isda = PyModule::new(py, "isda")?;
    isda.setattr("STANDARD_RECOVERY_SENIOR", constants::isda::STANDARD_RECOVERY_SENIOR)?;
    isda.setattr("STANDARD_RECOVERY_SUB", constants::isda::STANDARD_RECOVERY_SUB)?;
    isda.setattr("STANDARD_INTEGRATION_POINTS", constants::isda::STANDARD_INTEGRATION_POINTS)?;
    isda.setattr("STANDARD_COUPON_DAY", constants::isda::STANDARD_COUPON_DAY)?;
    module.add_submodule(&isda)?;

    // Business day counts per year
    let time = PyModule::new(py, "time")?;
    time.setattr("BUSINESS_DAYS_PER_YEAR_US", constants::time::BUSINESS_DAYS_PER_YEAR_US)?;
    time.setattr("BUSINESS_DAYS_PER_YEAR_UK", constants::time::BUSINESS_DAYS_PER_YEAR_UK)?;
    time.setattr("BUSINESS_DAYS_PER_YEAR_JP", constants::time::BUSINESS_DAYS_PER_YEAR_JP)?;
    module.add_submodule(&time)?;

    parent.add_submodule(&module)?;

    Ok(vec!["constants"])
}
```

NOTE: Verify exact constant paths by reading the Rust source. Some constants may live in `finstack_core` rather than `finstack_valuations`. Adjust import paths accordingly.

**Step 3: Register in `finstack-py/src/valuations/mod.rs`**

Add `pub(crate) mod constants;` to the module declarations and call `constants::register(py, &module)?;` in the register function.

**Step 4: Build and test**

Run: `cd finstack-py && maturin develop --release`
Run: `python -c "from finstack.valuations.constants import ONE_BASIS_POINT; print(ONE_BASIS_POINT)"`
Expected: `0.0001`

Run: `python -c "from finstack.valuations.constants.isda import STANDARD_RECOVERY_SENIOR; print(STANDARD_RECOVERY_SENIOR)"`
Expected: `0.4`

**Step 5: Commit**

```bash
git add finstack-py/src/valuations/constants.rs finstack-py/src/valuations/mod.rs
git commit -m "feat: expose valuations constants module to Python"
```

---

### Task 5: Create constants .pyi stub

**Files:**
- Create: `finstack-py/finstack/valuations/constants.pyi`
- Modify: `finstack-py/finstack/valuations/__init__.pyi` (add constants import)

**Step 1: Create `finstack-py/finstack/valuations/constants.pyi`:**

```python
"""Shared numerical constants for pricing and risk calculations.

Examples
--------
    >>> from finstack.valuations.constants import ONE_BASIS_POINT
    >>> ONE_BASIS_POINT
    0.0001

    >>> from finstack.valuations.constants.isda import STANDARD_RECOVERY_SENIOR
    >>> STANDARD_RECOVERY_SENIOR
    0.4
"""

from __future__ import annotations

# Top-level conversion factors
ONE_BASIS_POINT: float
"""One basis point (0.0001)."""

BASIS_POINTS_PER_UNIT: float
"""Number of basis points in one unit (10,000)."""

PERCENT_TO_DECIMAL: float
"""Multiply a percentage by this to get a decimal (0.01)."""

DECIMAL_TO_PERCENT: float
"""Multiply a decimal by this to get a percentage (100.0)."""

class numerical:
    """Numerical tolerances used by solvers and comparisons."""
    ZERO_TOLERANCE: float
    INTEGRATION_STEP_FACTOR: float
    SOLVER_TOLERANCE: float
    RATE_COMPARISON_TOLERANCE: float
    DIVISION_EPSILON: float
    RELATIVE_TOLERANCE: float
    DF_EPSILON: float

class isda:
    """ISDA standard conventions for credit derivatives."""
    STANDARD_RECOVERY_SENIOR: float
    STANDARD_RECOVERY_SUB: float
    STANDARD_INTEGRATION_POINTS: int
    STANDARD_COUPON_DAY: int

class time:
    """Business day counts per year by market."""
    BUSINESS_DAYS_PER_YEAR_US: float
    BUSINESS_DAYS_PER_YEAR_UK: float
    BUSINESS_DAYS_PER_YEAR_JP: float
```

NOTE: The submodules `numerical`, `isda`, `time` are Python submodules (registered via `add_submodule`), not classes. In a .pyi stub, representing them as classes with class-level attributes is the pragmatic approach for IDE support. However, if pyright/mypy complains, an alternative is to create separate `constants/numerical.pyi`, `constants/isda.pyi`, `constants/time.pyi` files. Verify which approach works with your type checker.

**Step 2: Add import to `finstack-py/finstack/valuations/__init__.pyi`**

Add `from . import constants` (after line 20, with the other submodule imports) if not already present.

**Step 3: Verify syntax, commit**

```bash
git add finstack-py/finstack/valuations/constants.pyi finstack-py/finstack/valuations/__init__.pyi
git commit -m "feat(stubs): add constants module .pyi stub"
```

---

## Stream 3: CashFlowSchedule Collection Protocols (P1)

### Task 6: Add **iter**, **len**, **getitem** to CashFlowSchedule

**Files:**
- Read: `finstack-py/src/valuations/cashflow/` (find the CashFlowSchedule binding)
- Modify: the Rust file containing PyCashFlowSchedule
- Modify: the corresponding .pyi stub

**Step 1: Locate the CashFlowSchedule binding**

Run: `grep -rn "CashFlowSchedule" finstack-py/src/valuations/cashflow/`

**Step 2: Add collection protocol methods to the `#[pymethods]` impl block:**

```rust
    fn __len__(&self) -> usize {
        self.inner.flows().len()
    }

    fn __getitem__(&self, index: isize) -> PyResult<PyCashFlow> {
        let flows = self.inner.flows();
        let idx = if index < 0 {
            flows.len().checked_sub(index.unsigned_abs()).ok_or_else(|| {
                pyo3::exceptions::PyIndexError::new_err("index out of range")
            })?
        } else {
            index as usize
        };
        flows.get(idx)
            .map(|f| PyCashFlow::from(f.clone()))
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("index out of range"))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<PyCashFlowIterator>> {
        let flows: Vec<_> = slf.inner.flows().to_vec();
        Py::new(slf.py(), PyCashFlowIterator { flows, index: 0 })
    }
```

NOTE: Read the actual struct field names (may be `cashflows()`, `entries()`, etc. instead of `flows()`). Also need to create the `PyCashFlowIterator` struct with `__iter__` + `__next__` methods — follow the same pattern used for `PyPositionIterator` in `finstack-py/src/portfolio/positions.rs`.

**Step 3: Update .pyi stub** — add:

```python
    def __len__(self) -> int: ...
    def __getitem__(self, index: int) -> CashFlow: ...
    def __iter__(self) -> Iterator[CashFlow]: ...
```

**Step 4: Build and test**

Run: `cd finstack-py && maturin develop --release`
Run: `python -c "from finstack.valuations.cashflow import CashFlowSchedule; print('OK')"`

**Step 5: Commit**

```bash
git add finstack-py/src/valuations/cashflow/*.rs finstack-py/finstack/valuations/cashflow/*.pyi
git commit -m "feat: add __iter__/__len__/__getitem__ to CashFlowSchedule"
```

---

## Stream 4: Stub Polish (P2)

### Task 7: Standardize PEP 604 annotations across all .pyi stubs

**Files:**
- All .pyi files under `finstack-py/finstack/`

**Step 1: Find all files still using Optional or Union without PEP 604**

Run: `grep -rn "Optional\[" finstack-py/finstack/ --include="*.pyi" | head -20`
Run: `grep -rn "Union\[" finstack-py/finstack/ --include="*.pyi" | head -20`

**Step 2: For each file found:**
- Ensure `from __future__ import annotations` is present
- Replace `Optional[X]` with `X | None`
- Replace `Union[X, Y]` with `X | Y`
- Remove unused `Optional` / `Union` imports

**Step 3: Verify no syntax errors**

Run: `find finstack-py/finstack -name '*.pyi' -exec python -c "import ast; ast.parse(open('{}').read())" \;`

**Step 4: Commit**

```bash
git add finstack-py/finstack/**/*.pyi
git commit -m "style(stubs): standardize to PEP 604 annotation style across all .pyi files"
```

---

## Verification

After all tasks are complete, run the following verification steps:

### 1. Build

```bash
cd finstack-py && maturin develop --release
```

### 2. InstrumentType completeness check

```python
python -c "
from finstack.valuations.common import InstrumentType
expected = [
    'BOND', 'LOAN', 'CDS', 'CDS_INDEX', 'CDS_TRANCHE', 'CDS_OPTION',
    'IRS', 'CAP_FLOOR', 'SWAPTION', 'BERMUDAN_SWAPTION', 'BASIS_SWAP',
    'BASKET', 'CONVERTIBLE', 'DEPOSIT', 'EQUITY_OPTION', 'FX_OPTION',
    'FX_SPOT', 'FX_SWAP', 'XCCY_SWAP', 'INFLATION_LINKED_BOND',
    'INFLATION_SWAP', 'YOY_INFLATION_SWAP', 'INFLATION_CAP_FLOOR',
    'INTEREST_RATE_FUTURE', 'VARIANCE_SWAP', 'FX_VARIANCE_SWAP',
    'EQUITY', 'REPO', 'FRA', 'STRUCTURED_CREDIT', 'PRIVATE_MARKETS_FUND',
    'REVOLVING_CREDIT', 'ASIAN_OPTION', 'BARRIER_OPTION', 'LOOKBACK_OPTION',
    'QUANTO_OPTION', 'AUTOCALLABLE', 'CMS_OPTION', 'CLIQUET_OPTION',
    'RANGE_ACCRUAL', 'FX_BARRIER_OPTION', 'TERM_LOAN', 'DCF',
    'REAL_ESTATE_ASSET', 'LEVERED_REAL_ESTATE_EQUITY',
    'EQUITY_TOTAL_RETURN_SWAP', 'FI_INDEX_TOTAL_RETURN_SWAP',
    'BOND_FUTURE', 'COMMODITY_FORWARD', 'COMMODITY_SWAP',
    'COMMODITY_OPTION', 'COMMODITY_ASIAN_OPTION',
    'VOLATILITY_INDEX_FUTURE', 'VOLATILITY_INDEX_OPTION',
    'EQUITY_INDEX_FUTURE', 'FX_FORWARD', 'NDF',
    'AGENCY_MBS_PASSTHROUGH', 'AGENCY_TBA', 'DOLLAR_ROLL',
    'AGENCY_CMO', 'FX_DIGITAL_OPTION', 'FX_TOUCH_OPTION',
]
missing = [n for n in expected if not hasattr(InstrumentType, n)]
if missing:
    print(f'FAIL: Missing InstrumentType variants: {missing}')
else:
    print(f'PASS: All {len(expected)} InstrumentType variants accessible')
"
```

### 3. ModelKey completeness check

```python
python -c "
from finstack.valuations.common import ModelKey
expected = [
    'DISCOUNTING', 'TREE', 'BLACK76', 'HULL_WHITE_1F', 'HAZARD_RATE',
    'NORMAL', 'MONTE_CARLO_GBM', 'MONTE_CARLO_HESTON',
    'MONTE_CARLO_HULL_WHITE_1F', 'BARRIER_BS_CONTINUOUS',
    'ASIAN_GEOMETRIC_BS', 'ASIAN_TURNBULL_WAKEMAN',
    'LOOKBACK_BS_CONTINUOUS', 'QUANTO_BS', 'FX_BARRIER_BS_CONTINUOUS',
    'HESTON_FOURIER',
]
missing = [n for n in expected if not hasattr(ModelKey, n)]
if missing:
    print(f'FAIL: Missing ModelKey variants: {missing}')
else:
    print(f'PASS: All {len(expected)} ModelKey variants accessible')
"
```

### 4. Constants module check

```python
python -c "
from finstack.valuations.constants import ONE_BASIS_POINT, BASIS_POINTS_PER_UNIT
assert ONE_BASIS_POINT == 0.0001
assert BASIS_POINTS_PER_UNIT == 10_000.0
print('PASS: Constants module accessible')
"
```

### 5. Test suite

```bash
cd finstack-py && pytest tests/ -v --tb=short
```

### 6. Stub syntax validation

```bash
find finstack-py/finstack -name '*.pyi' -exec python -c "import ast; ast.parse(open('{}').read())" \;
echo "All stubs valid"
```
