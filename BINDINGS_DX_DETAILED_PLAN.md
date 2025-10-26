# Bindings & DX Improvements — Detailed Implementation Plan

## Table of Contents

1. [Overview & Principles](#overview--principles)
2. [Feature 1: Minimal Explainability](#feature-1-minimal-explainability)
3. [Feature 2: Run Metadata Stamping](#feature-2-run-metadata-stamping)
4. [Feature 3: Python Type DX](#feature-3-python-type-dx)
5. [Feature 4: Progress Callbacks](#feature-4-progress-callbacks)
6. [Feature 5: DataFrame Bridges](#feature-5-dataframe-bridges)
7. [Feature 6: Risk Ladders in Bindings](#feature-6-risk-ladders-in-bindings)
8. [Feature 7: JSON-Schema Getters](#feature-7-json-schema-getters)
9. [Feature 8: Python Error Hierarchy](#feature-8-python-error-hierarchy)
10. [Quick Wins](#quick-wins)
11. [Implementation Roadmap](#implementation-roadmap)
12. [Testing Strategy](#testing-strategy)
13. [Migration & Compatibility](#migration--compatibility)

---

## Overview & Principles

### Non-Negotiable Constraints

1. **Determinism preserved**: All features maintain Decimal-mode determinism
2. **Opt-in by default**: Explainability, progress callbacks, etc. are OFF unless requested
3. **Stable serde**: New fields use `#[serde(skip_serializing_if = "Option::is_none")]` or versioned envelopes
4. **No performance regression**: Default paths (explain=false) should have zero overhead
5. **Newtype IDs everywhere**: [[memory:8310676]] — no raw String IDs
6. **Use existing date utilities**: [[memory:8583311]] — no new date functions
7. **Instrument pricers for calibration**: [[memory:8583314]]

### Implementation Phases

- **Phase 1** (Week 1-2): Core infrastructure (ExplanationTrace, RunMetadata, error hierarchy)
- **Phase 2** (Week 3-4): Bindings & DX (py.typed, progress callbacks, DataFrame bridges)
- **Phase 3** (Week 5): Polish (risk ladders, JSON-Schema, quick wins)
- **Phase 4** (Week 6): Documentation, examples, notebook conversions

---

## Feature 1: Minimal Explainability

### 1.1 Design Goals

- **Small payloads**: Cap trace size at ~50KB per operation
- **Opt-in**: Default `explain: bool = false`
- **Stable format**: JSON-serializable structs with strict field names
- **Three domains**: Calibration, bond pricing, structured-credit waterfall

### 1.2 Core Rust Types

**Location**: `finstack/core/src/explain.rs` (new file)

```rust
use serde::{Deserialize, Serialize};
use crate::types::{Amount, CurveId, InstrumentId};

/// Opt-in flag for generating explanation traces
#[derive(Debug, Clone, Copy, Default)]
pub struct ExplainOpts {
    pub enabled: bool,
    pub max_entries: Option<usize>, // Cap trace size
}

impl ExplainOpts {
    pub fn enabled() -> Self {
        Self { enabled: true, max_entries: Some(1000) }
    }
    
    pub fn disabled() -> Self {
        Self { enabled: false, max_entries: None }
    }
}

/// Generic explanation trace container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationTrace {
    #[serde(rename = "type")]
    pub trace_type: String, // "calibration" | "pricing" | "waterfall"
    pub entries: Vec<TraceEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TraceEntry {
    #[serde(rename = "calibration_iteration")]
    CalibrationIteration {
        iteration: usize,
        residual: f64,
        knots_updated: Vec<String>,
        converged: bool,
    },
    #[serde(rename = "cashflow_pv")]
    CashflowPV {
        date: String, // ISO8601
        cashflow: Amount,
        discount_factor: f64,
        pv: Amount,
        curve_id: String,
    },
    #[serde(rename = "waterfall_step")]
    WaterfallStep {
        period: usize,
        step_name: String,
        cash_in: Amount,
        cash_out: Amount,
        shortfall: Option<Amount>,
    },
}
```

### 1.3 Calibration Integration

**Location**: `finstack/valuations/src/calibration/report.rs` (modify) and wire in `finstack/valuations/src/calibration/methods/` (e.g., `discount.rs`).

```rust
// Extend CalibrationReport to optionally carry an explanation trace
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationReport {
    // ... existing fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ExplanationTrace>,
}

// In calibrators (e.g., discount.rs): build and attach the trace if requested
let mut trace = if explain.enabled {
    Some(ExplanationTrace { trace_type: "calibration".into(), entries: Vec::new(), truncated: None })
} else { None };

for iter in 0..config.max_iterations {
    let residual = objective_iter(...);
    if let Some(ref mut t) = trace {
        if t.entries.len() < explain.max_entries.unwrap_or(usize::MAX) {
            t.entries.push(TraceEntry::CalibrationIteration { iteration: iter, residual, knots_updated: vec![], converged: residual < config.tolerance });
        } else { t.truncated = Some(true); }
    }
    if residual < config.tolerance { break; }
}

let mut report = CalibrationReport::for_type("yield_curve", residuals, total_iterations);
report.explanation = trace;
```

### 1.4 Bond Pricing Integration

**Location**: `finstack/valuations/src/results/valuation_result.rs` (modify)

```rust
// Extend ValuationResult with an optional explanation trace
#[derive(Clone, Debug)]
pub struct ValuationResult {
    // ... existing fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ExplanationTrace>,
}

// Example: inside a bond pricing path, populate a per-cashflow breakdown
let mut trace = if explain.enabled {
    Some(ExplanationTrace { trace_type: "pricing".into(), entries: Vec::new(), truncated: None })
} else { None };

// Resolve discount curve once (note: Bond field is `disc_id`)
let disc = market.get_discount_ref(bond.disc_id.as_str())?;

// For each scheduled cashflow (domain-specific), push a CashflowPV entry
for cf in scheduled_cashflows.iter() {
    let df = disc.df_on_date_curve(cf.date);
    let pv_cf = cf.amount * df;
    if let Some(ref mut t) = trace {
        if t.entries.len() < explain.max_entries.unwrap_or(usize::MAX) {
            t.entries.push(TraceEntry::CashflowPV {
                date: cf.date.to_string(),
                cashflow: cf.amount,
                discount_factor: df,
                pv: pv_cf,
                curve_id: bond.disc_id.to_string(),
            });
        }
    }
}

let mut result = ValuationResult::stamped(bond.id().as_str(), as_of, base_pv);
result.explanation = trace;
```

### 1.5 Structured-Credit Waterfall Integration

**Location**: `finstack/valuations/src/instruments/structured_credit/components/waterfall.rs` (modify)

Similar pattern: add `ExplainOpts` parameter, build trace with `WaterfallStep` entries for each tranche payment step.

### 1.6 Python Bindings

**Location**: `finstack-py/src/valuations/calibration/methods.rs`

```rust
#[pyclass]
pub struct PyCalibrationResult {
    inner: CalibrationResult,
}

#[pymethods]
impl PyCalibrationResult {
    #[getter]
    fn explanation(&self) -> Option<PyObject> {
        Python::with_gil(|py| {
            self.inner.explanation.as_ref().map(|trace| {
                pythonize(py, trace).expect("serialize trace")
            })
        })
    }
    
    fn explain_json(&self) -> Option<String> {
        self.inner.explanation.as_ref()
            .map(|t| serde_json::to_string_pretty(t).unwrap())
    }
}

// In calibrate function
#[pyfunction]
pub fn calibrate_curve(
    quotes: Vec<PyCalibrationQuote>,
    market: &PyMarketContext,
    opts: Option<PyCalibrationOpts>,
    explain: Option<bool>,
) -> PyResult<PyCalibrationResult> {
    let explain_opts = if explain.unwrap_or(false) {
        ExplainOpts::enabled()
    } else {
        ExplainOpts::disabled()
    };
    // ... call Rust calibrator with explain_opts
}
```

**Location**: `finstack-py/finstack/valuations/calibration.pyi`

```python
class CalibrationResult:
    """Result of curve calibration with optional explanation trace."""
    
    @property
    def curve(self) -> DiscountCurve: ...
    
    @property
    def diagnostics(self) -> CalibrationDiagnostics: ...
    
    @property
    def explanation(self) -> dict[str, Any] | None:
        """
        Explanation trace if explain=True was passed to calibrate().
        
        Structure:
        {
            "type": "calibration",
            "entries": [
                {
                    "kind": "calibration_iteration",
                    "iteration": 0,
                    "residual": 0.005,
                    "knots_updated": ["2025-01-15", "2026-01-15"],
                    "converged": false
                },
                ...
            ]
        }
        """
        ...
    
    def explain_json(self) -> str | None:
        """Pretty-printed JSON of the explanation trace."""
        ...

def calibrate_curve(
    quotes: list[CalibrationQuote],
    market: MarketContext,
    opts: CalibrationOpts | None = None,
    explain: bool = False,
) -> CalibrationResult:
    """
    Calibrate a discount or spread curve.
    
    Args:
        quotes: Market quotes (bonds, swaps, etc.)
        market: Market context with base curves
        opts: Solver options
        explain: If True, return detailed iteration trace
        
    Example:
        >>> result = calibrate_curve(quotes, market, explain=True)
        >>> if result.explanation:
        >>>     print(result.explain_json())
    """
    ...
```

### 1.7 WASM Bindings

**Location**: `finstack-wasm/src/valuations/calibration/methods.rs`

```rust
#[wasm_bindgen]
pub struct WasmCalibrationResult {
    inner: CalibrationResult,
}

#[wasm_bindgen]
impl WasmCalibrationResult {
    #[wasm_bindgen(getter)]
    pub fn explanation(&self) -> Option<JsValue> {
        self.inner.explanation.as_ref()
            .map(|trace| serde_wasm_bindgen::to_value(trace).unwrap())
    }
}

#[wasm_bindgen]
pub fn calibrate_curve(
    quotes: JsValue,
    market: &WasmMarketContext,
    opts: Option<JsValue>,
    explain: Option<bool>,
) -> Result<WasmCalibrationResult, JsValue> {
    let explain_opts = if explain.unwrap_or(false) {
        ExplainOpts::enabled()
    } else {
        ExplainOpts::disabled()
    };
    // ... call Rust
}
```

### 1.8 Validation & Testing

**Unit tests** (`finstack/core/tests/explain_tests.rs`):
- Trace size caps (max_entries enforcement)
- Truncation flag when limit exceeded
- Serialization roundtrip (JSON)
- Default disabled (zero overhead check via benchmarks)

**Golden tests** (`finstack/valuations/tests/calibration_explain_golden.rs`):
- Known calibration → check trace structure
- Redact iteration counts (non-deterministic) but validate schema

**Property tests**:
- `explain=true` ⇒ `explanation.is_some()`
- `explain=false` ⇒ `explanation.is_none()`

**Benchmark** (`finstack/valuations/benches/calibration_overhead.rs`):
- Ensure `explain=false` has <1% overhead vs. baseline

### 1.9 Documentation

**Notebook**: `finstack-py/examples/notebooks/explainability_demo.ipynb`

```python
# Calibration with explanation
result = calibrate_curve(quotes, market, explain=True)
print(result.explain_json())

# Bond pricing with cashflow breakdown
bond_result = pricer.price(bond, market, as_of, explain=True)
df = pd.DataFrame([
    {
        "date": e["date"],
        "cashflow": e["cashflow"]["amount"],
        "df": e["discount_factor"],
        "pv": e["pv"]["amount"],
    }
    for e in bond_result.explanation["entries"]
])
print(df)
```

---

## Feature 2: Run Metadata Stamping

### 2.1 Design Goals

- **Reproducibility**: Every result includes enough context to reconstruct it
- **Audit trail**: `run_id`, `timestamp`, version, rounding, FX policy
- **Redaction-safe**: No PII; deterministic IDs when needed

### 2.2 Core Rust Types

Use existing `ResultsMeta` in `finstack/core/src/config.rs` (numeric mode, rounding context, optional FX policy key). No separate run-id/timestamp struct.

### 2.3 Integration Pattern

**All result envelopes** already carry metadata where applicable. For valuations, stamp `ResultsMeta` when constructing `ValuationResult`.

**Constructor example** (`finstack/valuations/src/results/valuation_result.rs`):

```rust
let meta = finstack_core::config::results_meta(&finstack_core::config::FinstackConfig::default());
let result = ValuationResult::stamped_with_meta(bond.id().as_str(), as_of, pv, meta);
```

### 2.4 Python/WASM Bindings

**Automatic serialization**: Metadata is a plain struct, so `pythonize` and `serde_wasm_bindgen` handle it automatically.

**Python stub** (`finstack-py/finstack/core/metadata.pyi`):

```python
from datetime import datetime

class RunMetadata:
    """Metadata stamped on every computation result."""
    
    run_id: str
    timestamp: datetime
    version: str
    rounding_context: RoundingContext
    fx_policy_applied: FxPolicyMetadata | None
    curve_ids: list[str]
    seed: int | None

class FxPolicyMetadata:
    policy_type: str
    base_currency: str
    conversions_applied: int
```

### 2.5 Validation

**Golden tests**: Serialize result, check metadata fields present and valid (UUID format, version matches, etc.)

**Property test**: `result.metadata.timestamp <= Utc::now()`

---

## Feature 3: Python Type DX

### 3.1 Goals

- **`py.typed` marker**: Enable strict type checking
- **Rich docstrings**: Top ~20 classes/functions with examples
- **Stub validation**: CI checks with `mypy` and `pyright`

### 3.2 Implementation

**File**: `finstack-py/finstack/py.typed` (create empty marker)

```bash
touch finstack-py/finstack/py.typed
```

**Docstring template** (example for `BondPricer`):

```python
# finstack-py/finstack/valuations/bond.pyi

class BondPricer:
    """
    Prices fixed-rate and floating-rate bonds using discount curves.
    
    The pricer generates cashflows, applies discount factors, and computes
    risk metrics (DV01, convexity, etc.) in a currency-safe manner.
    
    Examples:
        >>> from finstack import Bond, MarketContext, BondPricer
        >>> from datetime import date
        >>> 
        >>> bond = Bond(
        ...     notional=1_000_000,
        ...     currency="USD",
        ...     coupon_rate=0.05,
        ...     maturity=date(2030, 1, 15),
        ...     frequency="SemiAnnual",
        ...     day_count="ACT_360",
        ...     discount_curve_id="USD_GOVT",
        ... )
        >>> pricer = BondPricer()
        >>> result = pricer.price(bond, market, as_of=date(2025, 1, 1))
        >>> print(f"PV: {result.pv}")
        PV: Amount(1_042_315.67, USD)
        >>> print(f"DV01: {result.metrics.dv01}")
        DV01: 4523.12
    
    See Also:
        - `Bond`: Instrument specification
        - `MarketContext`: Required curves and FX rates
        - `BondMetrics`: Available risk metrics
    """
    
    def price(
        self,
        bond: Bond,
        market: MarketContext,
        as_of: date,
        explain: bool = False,
    ) -> BondPricingResult:
        """
        Price a bond and compute risk metrics.
        
        Args:
            bond: Bond specification
            market: Market context with discount curve
            as_of: Valuation date
            explain: If True, include per-cashflow breakdown
            
        Returns:
            Pricing result with PV, cashflows, metrics, and metadata.
            
        Raises:
            MissingCurveError: If bond.discount_curve_id not in market
            InvalidDateError: If as_of > bond.maturity
        """
        ...
```

**Top 20 targets** (prioritize by usage frequency):

1. `Bond`, `BondPricer`, `BondPricingResult`
2. `MarketContext`, `DiscountCurve`, `FxProvider`
3. `Amount`, `Currency`, `Rate`
4. `CalibrationQuote`, `calibrate_curve`, `CalibrationResult`
5. `Portfolio`, `Position`, `PortfolioAggregation`
6. `Scenario`, `ScenarioEngine`, `ScenarioResults`
7. `StatementModel`, `StatementEngine`, `StatementResults`
8. `Date`, `Period`, `DayCountConvention`

### 3.3 CI Validation

**Makefile target**:

```makefile
.PHONY: typecheck-py
typecheck-py:
	cd finstack-py && uv run mypy finstack/
	cd finstack-py && uv run pyright finstack/
```

**Add to `.github/workflows/ci.yml`** (if exists):

```yaml
- name: Python type check
  run: make typecheck-py
```

### 3.4 Doctest (Optional for Examples)

Extract docstring examples into test files:

```python
# finstack-py/tests/test_docstrings.py
import doctest
import finstack.valuations.bond

def test_bond_pricer_docstring():
    doctest.testmod(finstack.valuations.bond, verbose=True)
```

---

## Feature 4: Progress Callbacks

### 4.1 Design Goals

- **Simple interface**: `fn(current: usize, total: usize, message: &str)`
- **Batched updates**: Report every N steps (not every loop iteration)
- **tqdm-friendly**: Python callbacks work with `tqdm` progress bars
- **WASM async-safe**: Callbacks don't block event loop

### 4.2 Core Rust Types

**Location**: `finstack/core/src/progress.rs` (new file)

```rust
use std::sync::{Arc, Mutex};

pub type ProgressFn = Arc<dyn Fn(usize, usize, &str) + Send + Sync>;

#[derive(Clone)]
pub struct ProgressReporter {
    callback: Option<ProgressFn>,
    batch_size: usize,
    last_reported: Arc<Mutex<usize>>,
}

impl ProgressReporter {
    pub fn new(callback: Option<ProgressFn>, batch_size: usize) -> Self {
        Self {
            callback,
            batch_size,
            last_reported: Arc::new(Mutex::new(0)),
        }
    }
    
    pub fn report(&self, current: usize, total: usize, message: &str) {
        if let Some(ref cb) = self.callback {
            let mut last = self.last_reported.lock().unwrap();
            if current - *last >= self.batch_size || current == total {
                cb(current, total, message);
                *last = current;
            }
        }
    }
    
    pub fn disabled() -> Self {
        Self::new(None, 0)
    }
}
```

### 4.3 Integration Example (Calibration)

```rust
impl CurveSolver {
    pub fn calibrate(
        &self,
        quotes: &[CalibrationQuote],
        market: &MarketContext,
        opts: CalibrationOpts,
        progress: ProgressReporter,
    ) -> Result<CalibrationResult> {
        let total = opts.max_iterations;
        
        for iter in 0..total {
            progress.report(iter, total, "Solving...");
            // ... solver step
        }
        
        progress.report(total, total, "Complete");
        Ok(result)
    }
}
```

### 4.4 Python Bindings

**Location**: `finstack-py/src/core/progress.rs` (new file)

```rust
use pyo3::prelude::*;
use pyo3::types::PyFunction;
use finstack_core::progress::{ProgressFn, ProgressReporter};
use std::sync::Arc;

pub fn py_to_progress_reporter(py_callback: Option<PyObject>) -> ProgressReporter {
    match py_callback {
        None => ProgressReporter::disabled(),
        Some(cb) => {
            let callback: ProgressFn = Arc::new(move |current, total, msg| {
                Python::with_gil(|py| {
                    let _ = cb.call1(py, (current, total, msg));
                });
            });
            ProgressReporter::new(Some(callback), 10) // batch every 10 steps
        }
    }
}
```

**Usage in Python binding**:

```rust
#[pyfunction]
pub fn calibrate_curve(
    quotes: Vec<PyCalibrationQuote>,
    market: &PyMarketContext,
    opts: Option<PyCalibrationOpts>,
    progress: Option<PyObject>,
) -> PyResult<PyCalibrationResult> {
    let progress_reporter = py_to_progress_reporter(progress);
    // ... pass to Rust calibrator
}
```

**Python API** (`finstack-py/finstack/valuations/calibration.pyi`):

```python
from typing import Callable

ProgressCallback = Callable[[int, int, str], None]

def calibrate_curve(
    quotes: list[CalibrationQuote],
    market: MarketContext,
    opts: CalibrationOpts | None = None,
    progress: ProgressCallback | None = None,
) -> CalibrationResult:
    """
    Calibrate curve with optional progress reporting.
    
    Example with tqdm:
        >>> from tqdm import tqdm
        >>> pbar = tqdm(total=100, desc="Calibrating")
        >>> def update(current, total, msg):
        ...     pbar.update(current - pbar.n)
        ...     pbar.set_description(msg)
        >>> result = calibrate_curve(quotes, market, progress=update)
        >>> pbar.close()
    """
    ...
```

### 4.5 WASM Bindings

```rust
// finstack-wasm/src/core/progress.rs
use wasm_bindgen::prelude::*;
use js_sys::Function;

pub fn js_to_progress_reporter(js_callback: Option<Function>) -> ProgressReporter {
    match js_callback {
        None => ProgressReporter::disabled(),
        Some(cb) => {
            let callback: ProgressFn = Arc::new(move |current, total, msg| {
                let this = JsValue::null();
                let _ = cb.call3(
                    &this,
                    &JsValue::from(current as u32),
                    &JsValue::from(total as u32),
                    &JsValue::from_str(msg),
                );
            });
            ProgressReporter::new(Some(callback), 10)
        }
    }
}
```

**TypeScript example**:

```typescript
const result = await calibrateCurve(
  quotes,
  market,
  opts,
  (current: number, total: number, message: string) => {
    console.log(`${message}: ${current}/${total}`);
  }
);
```

---

## Feature 5: DataFrame Bridges

### 5.1 Design Goals

- **No Rust Parquet writer**: Use Python's `polars`/`pandas` libraries
- **Row-shaping helpers**: Rust provides flat row iterators
- **Schema stability**: Golden tests for column names/types

### 5.2 Rust Row Helpers

**Location**: `finstack/valuations/src/results/dataframe.rs` (new file)

```rust
use serde::{Deserialize, Serialize};

/// Generic row for DataFrame export from a ValuationResult
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationRow {
    pub instrument_id: String,
    pub as_of_date: String,
    pub pv: f64,
    pub currency: String,
    // selected measures flattened as columns where desired (optional)
    pub dv01: Option<f64>,
}

impl ValuationResult {
    pub fn to_rows(&self) -> Vec<ValuationRow> {
        let dv01 = self.measures.get("dv01").copied();
        vec![ValuationRow {
            instrument_id: self.instrument_id.clone(),
            as_of_date: self.as_of.to_string(),
            pv: self.value.amount(),
            currency: self.value.currency().to_string(),
            dv01,
        }]
    }
}
```

### 5.3 Python DataFrame Builders

**Location**: `finstack-py/src/valuations/dataframe.rs` (new file)

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

#[pyfunction]
pub fn results_to_polars(results: Vec<PyBondPricingResult>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let polars = py.import("polars")?;
        
        let rows: Vec<_> = results.iter()
            .flat_map(|r| r.inner.to_rows())
            .collect();
        
        // Convert to Python dicts
        let py_rows: Vec<PyObject> = rows.iter()
            .map(|row| pythonize(py, row).unwrap())
            .collect();
        
        // Call pl.DataFrame(rows)
        polars.call_method1("DataFrame", (py_rows,))?.into()
    })
}
```

**Python API** (`finstack-py/finstack/valuations/bond.pyi`):

```python
import polars as pl
import pandas as pd

class ValuationResult:
    def to_polars(self) -> pl.DataFrame: ...
    def to_pandas(self) -> pd.DataFrame: ...
    def to_parquet(self, path: str) -> None: ...
```

**Python implementation** (`finstack-py/finstack/valuations/bond.py` — if needed):

```python
# If stubs alone aren't enough, implement Python-side helpers
def to_pandas(self) -> pd.DataFrame:
    return self.to_polars().to_pandas()

def to_parquet(self, path: str) -> None:
    self.to_polars().write_parquet(path)
```

### 5.4 Schema Golden Tests

**Location**: `finstack-py/tests/test_dataframe_schema.py`

```python
import polars as pl
from finstack import Bond, BondPricer, MarketContext
from datetime import date

def test_bond_result_polars_schema():
    # ... create bond, market, pricer
    result = pricer.price(bond, market, as_of=date(2025, 1, 1))
    df = result.to_polars()
    
    expected_schema = {
        "instrument_id": pl.Utf8,
        "as_of_date": pl.Utf8,
        "pv": pl.Float64,
        "currency": pl.Utf8,
        "dv01": pl.Float64,
        "convexity": pl.Float64,
    }
    
    assert df.schema == expected_schema
```

---

## Feature 6: Risk Ladders in Bindings

### 6.1 Design

**Rust**: KRD helpers already exist in `finstack/valuations/src/risk/krd.rs`

**Task**: Expose to Python/WASM with tidy table outputs

### 6.2 Rust API (already exists, just document)

Use the bucketed risk helpers in `finstack/valuations/src/metrics/bucketed.rs`:

```rust
pub fn standard_ir_dv01_buckets() -> Vec<f64> { /* ... */ }
pub fn compute_key_rate_dv01_series<I, RevalFn>(
    context: &mut MetricContext,
    disc_id: &CurveId,
    bucket_times_years: I,
    bump_bp: f64,
    revalue_with_disc: RevalFn,
) -> Result<f64> { /* ... */ }
```

### 6.3 Python Binding

**Location**: `finstack-py/src/valuations/risk.rs` (new file)

```rust
#[pyfunction]
pub fn krd_dv01(
    bond: &PyBond,
    market: &PyMarketContext,
    as_of: NaiveDate,
    buckets: Option<Vec<String>>, // e.g., ["3M", "1Y", "5Y"]
) -> PyResult<PyObject> {
    let buckets_parsed = buckets.map(|b| {
        b.iter().map(|s| Period::parse(s).unwrap()).collect()
    });
    
    let ladder = finstack_valuations::risk::compute_krd_dv01(
        &bond.inner,
        &market.inner,
        as_of,
        buckets_parsed,
    )?;
    
    Python::with_gil(|py| {
        let polars = py.import("polars")?;
        let rows: Vec<_> = ladder.iter()
            .map(|(period, dv01)| {
                let dict = PyDict::new(py);
                dict.set_item("bucket", period.to_string())?;
                dict.set_item("dv01", dv01)?;
                Ok::<_, PyErr>(dict.into())
            })
            .collect::<Result<Vec<_>, _>>()?;
        
        polars.call_method1("DataFrame", (rows,))
    })
}
```

**Python API** (`finstack-py/finstack/valuations/risk.pyi`):

```python
import polars as pl

def krd_dv01(
    bond: Bond,
    market: MarketContext,
    as_of: date,
    buckets: list[str] | None = None,
) -> pl.DataFrame:
    """
    Compute Key Rate Duration (KRD) DV01 ladder.
    
    Args:
        bond: Bond to analyze
        market: Market context
        as_of: Valuation date
        buckets: Optional tenor buckets (default: ["3M", "1Y", "2Y", "5Y", "10Y", "30Y"])
        
    Returns:
        DataFrame with columns: bucket, dv01
        
    Example:
        >>> ladder = krd_dv01(bond, market, date(2025, 1, 1))
        >>> print(ladder)
        shape: (6, 2)
        ┌────────┬──────────┐
        │ bucket │ dv01     │
        │ ---    │ ---      │
        │ str    │ f64      │
        ╞════════╪══════════╡
        │ 3M     │ 12.34    │
        │ 1Y     │ 45.67    │
        │ ...    │ ...      │
        └────────┴──────────┘
    """
    ...
```

### 6.4 WASM Binding

Similar pattern, return array of `{bucket: string, dv01: number}`.

---

## Feature 7: JSON-Schema Getters

### 7.1 Design

- **Leverage serde**: Use `schemars` crate to generate schemas from existing structs
- **No codegen in core**: Examples can optionally codegen TypeScript types

### 7.2 Rust Implementation

**Add dependency**: `Cargo.toml` in `finstack/core`, `finstack/valuations`, etc.

```toml
[dependencies]
schemars = "0.8"
```

**Derive schemas**:

```rust
// finstack/valuations/src/instruments/bond.rs
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Bond {
    // ... existing fields
}
```

**Schema getter** (`finstack/valuations/src/schema.rs` — new file):

```rust
use schemars::schema_for;
use serde_json::Value;

pub fn bond_schema() -> Value {
    let schema = schema_for!(Bond);
    serde_json::to_value(schema).unwrap()
}

pub fn scenario_schema() -> Value {
    let schema = schema_for!(Scenario);
    serde_json::to_value(schema).unwrap()
}
```

### 7.3 Python Binding

```rust
// finstack-py/src/valuations/schema.rs
#[pyfunction]
pub fn get_bond_schema() -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let schema = finstack_valuations::schema::bond_schema();
        pythonize(py, &schema).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    })
}
```

**Python API** (`finstack-py/finstack/valuations/schema.pyi`):

```python
def get_bond_schema() -> dict:
    """
    Get JSON-Schema for Bond configuration.
    
    Useful for validation in pipelines or UI schema-driven forms.
    
    Example:
        >>> schema = get_bond_schema()
        >>> import jsonschema
        >>> jsonschema.validate(my_bond_dict, schema)
    """
    ...
```

### 7.4 WASM Binding

```rust
#[wasm_bindgen]
pub fn get_bond_schema() -> JsValue {
    let schema = finstack_valuations::schema::bond_schema();
    serde_wasm_bindgen::to_value(&schema).unwrap()
}
```

### 7.5 Example Codegen (Optional)

**Location**: `finstack-wasm/examples/codegen/` (not in core)

```bash
# Generate TypeScript types from schema
npm install -g json-schema-to-typescript
json-schema-to-typescript bond-schema.json > Bond.d.ts
```

---

## Feature 8: Python Error Handling

### 8.1 Design Goals

- **Clear exceptions**: Use idiomatic Python built-ins (ValueError, KeyError, RuntimeError)
- **Centralized mapping**: Map core errors via a single helper
- **Docstrings**: Document common causes in API docstrings where relevant

### 8.2 Python Mapping (existing)

Use the existing mapping in `finstack-py/src/core/error.rs`:

```rust
pub(crate) fn core_to_py(err: Error) -> PyErr { /* maps to ValueError/KeyError/RuntimeError */ }
```

### 8.3 Rust Error Mapping

**Location**: `finstack-py/src/errors.rs` (new file)

```rust
use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use finstack_core::error::Error as CoreError;

pyo3::create_exception!(finstack, FinstackError, PyException);
pyo3::create_exception!(finstack, ConfigurationError, FinstackError);
pyo3::create_exception!(finstack, MissingCurveError, ConfigurationError);
pyo3::create_exception!(finstack, ConvergenceError, FinstackError);
// ... etc.

pub fn map_error(err: CoreError) -> PyErr {
    match err {
        CoreError::MissingCurve { curve_id } => {
            MissingCurveError::new_err(format!("Curve not found: {}", curve_id))
        }
        CoreError::Convergence { iterations, residual } => {
            ConvergenceError::new_err(format!(
                "Failed to converge after {} iterations (residual={:.6f})",
                iterations, residual
            ))
        }
        CoreError::CurrencyMismatch { expected, actual } => {
            CurrencyMismatchError::new_err(format!(
                "Currency mismatch: expected {}, got {}",
                expected, actual
            ))
        }
        _ => FinstackError::new_err(err.to_string()),
    }
}

// Register exceptions in module init
pub fn register_exceptions(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("FinstackError", py.get_type::<FinstackError>())?;
    m.add("ConfigurationError", py.get_type::<ConfigurationError>())?;
    m.add("MissingCurveError", py.get_type::<MissingCurveError>())?;
    m.add("ConvergenceError", py.get_type::<ConvergenceError>())?;
    // ... etc.
    Ok(())
}
```

**Update module init** (`finstack-py/src/lib.rs`):

```rust
#[pymodule]
fn finstack(py: Python, m: &PyModule) -> PyResult<()> {
    // ... existing registrations
    errors::register_exceptions(py, m)?;
    Ok(())
}
```

### 8.4 Usage in Bindings

```rust
#[pyfunction]
pub fn calibrate_curve(...) -> PyResult<PyCalibrationResult> {
    let result = solver.calibrate(...)
        .map_err(errors::map_error)?;
    Ok(PyCalibrationResult::new(result))
}
```

### 8.5 Python Usage

```python
from finstack import calibrate_curve, MissingCurveError, ConvergenceError

try:
    result = calibrate_curve(quotes, market)
except MissingCurveError as e:
    print(f"Missing curve: {e.curve_id}")
    # ... suggest available curves from market.curve_ids()
except ConvergenceError as e:
    print(f"Solver failed: {e.iterations} iterations, residual={e.residual}")
    # ... try different CalibrationOpts
```

---

## Quick Wins

### 1. Curve-ID Suggestions in Errors

**Rust** (`finstack/core/src/error.rs`):

```rust
impl Error {
    pub fn missing_curve(requested: CurveId, available: Vec<CurveId>) -> Self {
        let suggestions = available.iter()
            .filter(|id| id.to_string().contains(&requested.to_string()))
            .map(|id| id.to_string())
            .collect::<Vec<_>>();
        
        let msg = if suggestions.is_empty() {
            format!("Curve not found: {}", requested)
        } else {
            format!(
                "Curve not found: {}. Did you mean: {}?",
                requested,
                suggestions.join(", ")
            )
        };
        
        Error::MissingCurve { message: msg }
    }
}
```

### 2. CalibrationConfig Presets

**Rust** (`finstack/valuations/src/calibration/config.rs`):

```rust
impl CalibrationOpts {
    pub fn conservative() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-8,
            step_size: 0.5,
            regularization: Some(1e-4),
        }
    }
    
    pub fn aggressive() -> Self {
        Self {
            max_iterations: 1000,
            tolerance: 1e-6,
            step_size: 1.0,
            regularization: None,
        }
    }
}
```

**Python**:

```python
opts = CalibrationOpts.conservative()
result = calibrate_curve(quotes, market, opts)
```

### 3. Formatting Helpers

**Rust** (`finstack/core/src/money.rs`):

```rust
impl Amount {
    pub fn format(&self, decimals: usize, show_currency: bool) -> String {
        let value = format!("{:.*}", decimals, self.to_f64());
        if show_currency {
            format!("{} {}", value, self.currency())
        } else {
            value
        }
    }
}
```

**Python**:

```python
print(result.pv.format(decimals=2, show_currency=True))
# Output: "1,042,315.67 USD"
```

### 4. Notebook Conversions

Convert these scripts to notebooks with outputs:

- `finstack-py/examples/scripts/bond_pricing.py` → `bond_pricing.ipynb`
- `finstack-py/examples/scripts/calibration_demo.py` → `calibration_demo.ipynb`
- `finstack-py/examples/scripts/portfolio_aggregation.py` → `portfolio_aggregation.ipynb`
- `finstack-py/examples/scripts/scenario_stress.py` → `scenario_stress.ipynb`

### 5. Metric Aliases

**Rust** (`finstack/valuations/src/metrics.rs`):

```rust
impl BondMetrics {
    /// Alias for dv01 (common in credit markets)
    pub fn pv01(&self) -> f64 {
        self.dv01
    }
}
```

**Python stub**:

```python
class BondMetrics:
    dv01: float
    
    @property
    def pv01(self) -> float:
        """Alias for dv01 (credit convention)."""
        ...
```

---

## Implementation Roadmap

### Phase 1: Core Infrastructure (Weeks 1-2)

**Week 1**:
- [ ] Create `finstack/core/src/explain.rs` with `ExplanationTrace` types
- [ ] Create `finstack/core/src/metadata.rs` with `RunMetadata`
- [ ] Create `finstack/core/src/progress.rs` with `ProgressReporter`
- [ ] Add `schemars` derive to top 20 types (Bond, Scenario, etc.)
- [ ] Create Python error hierarchy in `finstack-py/finstack/errors.py`
- [ ] Wire up error mapping in `finstack-py/src/errors.rs`

**Week 2**:
- [ ] Integrate `ExplainOpts` into calibration solver
- [ ] Integrate `ExplainOpts` into bond pricer
- [ ] Integrate `ExplainOpts` into waterfall (ABS/RMBS/CMBS/CLO)
- [ ] Add `RunMetadata` to all result types (calibration, pricing, portfolio, scenarios, statements)
- [ ] Unit tests for explain (size caps, truncation, opt-in)
- [ ] Golden tests for metadata fields

### Phase 2: Bindings & DX (Weeks 3-4)

**Week 3**:
- [ ] Python bindings for `explanation` field (calibration, pricing)
- [ ] Python bindings for `metadata` field
- [ ] Python progress callbacks (tqdm-friendly)
- [ ] WASM progress callbacks (async-safe)
- [ ] Add `py.typed` marker
- [ ] Write docstrings for top 20 classes/functions

**Week 4**:
- [ ] Implement `to_polars()` / `to_pandas()` / `to_parquet()` for bond results
- [ ] Implement `to_polars()` for portfolio results
- [ ] Implement `to_polars()` for statement results
- [ ] Schema golden tests (column names, types)
- [ ] CI validation: `mypy` and `pyright` checks

### Phase 3: Polish (Week 5)

- [ ] Python/WASM bindings for KRD/CS01 ladders
- [ ] JSON-Schema getters (`get_bond_schema`, `get_scenario_schema`, etc.)
- [ ] Quick wins: curve suggestions, config presets, formatting helpers, metric aliases
- [ ] Convert 4 scripts to notebooks with outputs
- [ ] WASM TypeScript codegen example (optional, in examples/ only)

### Phase 4: Documentation & Examples (Week 6)

- [ ] Explainability demo notebook
- [ ] Progress reporting demo (tqdm + WASM)
- [ ] DataFrame export demo (Polars, Pandas, Parquet)
- [ ] Risk ladder demo (KRD, CS01)
- [ ] JSON-Schema validation demo (Python jsonschema, WASM AJV)
- [ ] Error handling guide (exception hierarchy)
- [ ] Update README with new features
- [ ] Release notes

---

## Testing Strategy

### Unit Tests

**Rust** (`finstack/*/tests/`):
- Explainability: opt-in flag, size caps, truncation, serialization
- Metadata: field presence, UUID format, version match
- Progress: callback count ≤ work units, batching
- DataFrame rows: schema stability, currency preservation
- Errors: variant mapping to Python exceptions

**Python** (`finstack-py/tests/`):
- Exception hierarchy: catch specific errors, check attributes
- DataFrame schema: column names, types, golden files
- Progress callbacks: tqdm integration, callback count
- Stubs: `mypy`/`pyright` validation

**WASM** (`finstack-wasm/tests/`):
- Schema JSON validity (Draft-07)
- Progress callbacks (no event loop blocking)

### Integration Tests

**Python notebooks** (run in CI via `nbmake` or similar):
- `explainability_demo.ipynb`: calibration + bond pricing traces
- `progress_demo.ipynb`: tqdm progress bar
- `dataframe_demo.ipynb`: to_polars(), to_parquet()
- `risk_ladder_demo.ipynb`: KRD, CS01 tables

**WASM example app** (manual QA):
- Schema validation with AJV
- Progress reporting in UI
- Error display with specific exception types

### Golden Tests

**Rust** (`finstack/*/tests/golden/`):
- Explanation trace structure (redact iteration counts)
- Metadata JSON format
- DataFrame schema (column names/types)
- Error message formats

**Python** (`finstack-py/tests/golden/`):
- Polars DataFrame output (parquet roundtrip)
- Exception messages and attributes

### Property Tests

**Rust** (using `proptest`):
- `explain=true` ⇒ `explanation.is_some()`
- `explain=false` ⇒ `explanation.is_none()`
- `metadata.timestamp <= Utc::now()`
- Progress: `callback_count <= total_work_units`

### Benchmarks

**Rust** (`finstack/*/benches/`):
- Calibration: `explain=false` overhead < 1% vs. baseline
- Pricing: `explain=false` overhead < 1%
- Progress: callback overhead with batch_size=1 vs. batch_size=100

---

## Migration & Compatibility

### Backward Compatibility

**All new fields are optional**:
- `explanation: Option<ExplanationTrace>` — skip serialization if `None`
- `metadata: RunMetadata` — always present, but old binaries won't have it in saved JSON

**Serde strategy**:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub explanation: Option<ExplanationTrace>,

#[serde(default)]
pub metadata: RunMetadata, // Deserialize old JSON → use default metadata
```

### Python Stub Rollout

1. Add `py.typed` marker
2. Run `mypy` / `pyright` on example scripts — fix errors
3. Document breaking changes (if any attribute names changed)
4. Release as minor version (0.X.0)

### WASM Bundle Size

- JSON-Schema adds `schemars` (~50KB gzipped)
- Feature-gate if needed: `features = ["schema"]`
- Default: schemas included; tree-shake if unused

---

## Success Metrics

1. **Explainability adoption**: >50% of calibration calls in examples use `explain=True`
2. **Metadata coverage**: 100% of result types include `RunMetadata`
3. **Type safety**: Zero `mypy` errors in example scripts
4. **Progress UX**: tqdm demo in at least 2 notebooks
5. **DataFrame usage**: >80% of result types support `.to_polars()`
6. **Error clarity**: Exception hierarchy used in at least 5 error cases
7. **Schema availability**: Bond, Scenario, Portfolio schemas exposed in Python/WASM
8. **Benchmark**: No regression (< 1% overhead) for default paths

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| **Payload bloat** (explain traces) | Size caps, opt-in default, truncation flag |
| **Performance regression** | Benchmarks in CI; zero-cost when disabled |
| **Serde drift** | Golden tests; deny unknown fields |
| **Stub maintenance** | Generate examples from doctests |
| **WASM bundle size** | Feature-gate schemas; tree-shake |
| **Python callback overhead** | Batched updates (every N steps) |
| **Error mapping gaps** | Centralized mapping; fallback to base error |

---

## Future Extensions (Out of Scope)

- **Interactive debugger**: Step through calibration iterations
- **Waterfall DSL**: Declarative payment rules (defer to later)
- **Async progress**: Long-running jobs with WebSockets (WASM)
- **Full Arrow/Parquet writer in Rust**: Defer to Python for now
- **Event bus**: Pluggable observers for all operations (over-engineering)
- **Auto-generated REST API**: OpenAPI from schemas (separate project)

---

## Conclusion

This plan provides:
- **Minimal explainability** for calibration, pricing, and waterfall (opt-in, capped)
- **Run metadata** for reproducibility and audit (stamped on all results)
- **Python type DX** with `py.typed`, rich docstrings, and stub validation
- **Progress reporting** (tqdm-friendly Python, async-safe WASM)
- **DataFrame bridges** (`.to_polars()`, `.to_pandas()`, `.to_parquet()`)
- **Risk ladders** (KRD, CS01) exposed in bindings
- **JSON-Schema getters** for validation
- **Python error hierarchy** for clear exceptions

All features are **opt-in**, **zero-overhead when disabled**, and **backward-compatible**. Implementation is phased over 6 weeks with clear testing and validation at each step.

