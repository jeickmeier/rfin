# WASM Bindings Parity Analysis

## Executive Summary

This document analyzes the parity between the Rust `finstack-valuations` crate and its WASM bindings (`finstack-wasm/src/valuations/`), using the Python bindings (`finstack-py/src/valuations/`) as a reference for what a complete binding layer should expose.

**Current Status**: ~70% parity
- ✅ **Complete**: Instruments (all 25+ types), Basic Calibration, Cashflow Builder, Pricer, Results
- ⚠️ **Partial**: Calibration (missing SABR & validation)
- ❌ **Missing**: Metrics Framework, Performance (XIRR), Covenants

---

## 1. Calibration Module Gaps

### 1.1 SABR Model Support (Missing)

**Location in Rust**: `finstack/valuations/src/calibration/derivatives/`

**Status**: ❌ Not exposed in WASM

**Types to expose**:
- `SABRModelParams` - SABR model parameters (alpha, nu, rho, beta)
- `SABRCalibrationDerivatives` - Derivatives for SABR calibration
- `SABRMarketData` - Market data for SABR calibration

**Reference**: Python bindings implement this in `finstack-py/src/valuations/calibration/sabr.rs`

**Impact**: High - SABR is industry standard for volatility surface calibration

**Effort**: Medium (150-200 lines)

```typescript
// Expected WASM API:
const params = SABRModelParams.equityStandard(0.2, 0.4, -0.3);
const derivatives = new SABRCalibrationDerivatives();
const marketData = new SABRMarketData(forward, strikes, vols, expiry);
```

### 1.2 Validation Module (Missing)

**Location in Rust**: `finstack/valuations/src/calibration/validation.rs`

**Status**: ❌ Not exposed in WASM

**Types to expose**:
- `CurveValidator` - Validates curve monotonicity, no-arbitrage
- `SurfaceValidator` - Validates surface arbitrage-free conditions
- `ValidationConfig` - Configuration for validation rules
- `ValidationError` - Structured validation error details

**Reference**: Python bindings implement this in `finstack-py/src/valuations/calibration/validation.rs`

**Impact**: High - Essential for production use to catch bad calibration

**Effort**: Medium (200-250 lines)

```typescript
// Expected WASM API:
const config = new ValidationConfig();
const validator = new CurveValidator(config);
const errors = validator.validateDiscountCurve(curve);
```

### 1.3 VolSurfaceCalibrator Enhancement

**Status**: ⚠️ Basic version exists in WASM, but may need enhancement

**Needs Review**:
- Check if SABR-specific surface fitting is fully exposed
- Verify interpolation methods (bilinear)
- Ensure all calibration reports include SABR parameters

---

## 2. Metrics Framework (Completely Missing)

**Location in Rust**: `finstack/valuations/src/metrics/`

**Status**: ❌ Not exposed in WASM

**Impact**: Critical - No way to compute risk metrics, Greeks, or sensitivities

This is the **largest gap** - the entire metrics framework is missing from WASM.

### 2.1 Core Metrics Types

**Types to expose**:
- `MetricId` - Strongly-typed metric identifiers (~150+ standard metrics)
- `MetricRegistry` - Registry of metric calculators with applicability
- `MetricCalculator` trait wrapper - For custom metric implementations (future)
- `MetricContext` - Execution context with caching

**Effort**: High (500+ lines for core framework)

```typescript
// Expected WASM API:
const registry = MetricRegistry.standard();
const metricIds = ["pv", "dv01", "duration", "convexity"];
const results = registry.computeMetrics(bond, market, metricIds);
```

### 2.2 Standard Metrics by Asset Class

**Bond Metrics** (18 metrics):
- Pricing: `pv`, `clean_price`, `dirty_price`, `accrued_interest`
- Yield: `ytm`, `ytw` (yield-to-worst)
- Duration: `duration_modified`, `duration_macaulay`
- Risk: `dv01`, `convexity`, `key_rate_dv01`
- Credit: `z_spread`, `i_spread`, `oas`, `asw`, `dm`
- Credit Risk: `cs01`

**IRS Metrics** (8 metrics):
- Legs: `fixed_leg_pv`, `float_leg_pv`
- Risk: `dv01`, `annuity`, `par_rate`
- Bucketed: `bucketed_dv01`

**Options Metrics** (15+ Greeks):
- First-order: `delta`, `vega`, `rho`, `theta`
- Second-order: `gamma`, `vanna`, `volga`, `veta`
- Third-order: `charm`, `color`, `speed`
- Other: `implied_vol`, `forward_pv01`

**Credit Metrics** (9 metrics):
- `spread`, `cs01`, `hazard_cs01`
- `survival_probability`, `default_probability`
- `recovery_01`, `jump_to_default`, `credit_dv01`

**Variance Swap Metrics** (6 metrics):
- `variance_vega`, `variance_expected`, `variance_realized`
- `variance_notional`, `variance_strike_vol`, `variance_time_to_maturity`

**Structured Credit Metrics**:
- `tranche_expected_loss`, `tranche_unexpected_loss`
- `base_correlation`, `compound_correlation`
- `average_life`, `wal` (weighted average life)

**Bucketed Risk Metrics**:
- `bucketed_dv01` - Per-tenor sensitivities
- `bucketed_cs01` - Per-tenor credit sensitivities
- Standard bucket sets (1M, 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y)

**Effort**: Very High (1000+ lines for all metric implementations)

### 2.3 Usage Pattern

The metrics framework is designed for:
1. On-demand computation with caching
2. Dependency management (e.g., duration depends on pv)
3. Instrument-specific registration
4. Parallel computation support

---

## 3. Performance Module (Missing)

**Location in Rust**: `finstack/valuations/src/performance/`

**Status**: ❌ Not exposed in WASM

**Impact**: Medium - Common in portfolio analytics

### 3.1 XIRR Calculation

**Function to expose**: `xirr(cashflows: Vec<(Date, f64)>, guess: Option<f64>) -> Result<f64>`

**Use case**: Calculate extended internal rate of return for irregular cashflows

**Effort**: Low (50-100 lines)

```typescript
// Expected WASM API:
const cashflows = [
  [new Date(2024, 0, 1), -100000],
  [new Date(2024, 6, 1), 5000],
  [new Date(2025, 0, 1), 5000],
  [new Date(2025, 6, 1), 105000]
];
const irr = xirr(cashflows);
```

---

## 4. Covenants Module (Missing)

**Location in Rust**: `finstack/valuations/src/covenants/`

**Status**: ❌ Not exposed in WASM

**Impact**: Low-Medium - Specialized use case for structured finance

### 4.1 Covenant Engine

**Types to expose**:
- `CovenantEngine` - Evaluates covenants and applies consequences
- `CovenantSpec` - Covenant definition with tests and consequences
- `CovenantTestSpec` - Individual covenant test specification
- `CovenantWindow` - Time window for covenant testing
- `CovenantBreach` - Breach event details
- `InstrumentMutator` - Applies covenant consequences to instruments
- `CovenantReport` - Evaluation results

**Use cases**: 
- CLO/CDO covenant testing (IC/OC tests)
- Revolving credit facility covenants
- Private credit covenant monitoring

**Effort**: Medium-High (400+ lines)

---

## 5. Additional Enhancements

### 5.1 Cashflow Module Enhancements

**Current Status**: ✅ Mostly complete

**Potential additions**:
- More payment split programs
- Additional amortization patterns
- Covenant-triggered cashflow modifications

### 5.2 Instrument Enhancements

**Current Status**: ✅ All 25+ instruments exposed

**Instruments covered**:
- **Rates**: Bond, Deposit, FRA, IRS, Basis Swap, Cap/Floor, Future, Swaption
- **FX**: Spot, Option, Swap
- **Credit**: CDS, CDS Index, CDS Tranche, CDS Option
- **Equity**: Equity, Option, TRS
- **Inflation**: ILB, Inflation Swap
- **Structured**: ABS, CLO, CMBS, RMBS, Basket, Convertible
- **Alternative**: Private Markets Fund, Repo, Variance Swap

**No gaps identified** - excellent parity with Python

### 5.3 Pricer Registry Enhancements

**Current Status**: ✅ Complete

All instruments have pricing methods. Metrics integration would be next natural step.

---

## Recommended Implementation Priority

### Phase 1: Critical Metrics (Weeks 1-3)
**Goal**: Enable basic risk analytics

1. **Core Metrics Framework** (Week 1)
   - `MetricId` with ~50 most common metrics
   - `MetricRegistry` with registration system
   - Basic `compute()` interface

2. **Bond & IRS Metrics** (Week 2)
   - YTM, duration, convexity, DV01
   - Par rates, annuities
   - Testing with examples

3. **Options Greeks** (Week 3)
   - Delta, gamma, vega, theta, rho
   - Testing with equity options

**Effort**: ~1000 lines, 3 weeks

### Phase 2: SABR & Validation (Week 4)
**Goal**: Production-ready calibration

1. **SABR Support**
   - `SABRModelParams`, `SABRCalibrationDerivatives`
   - Integration with existing `VolSurfaceCalibrator`

2. **Validation Module**
   - `CurveValidator`, `SurfaceValidator`
   - `ValidationError` reporting

**Effort**: ~400 lines, 1 week

### Phase 3: Extended Metrics (Week 5-6)
**Goal**: Complete risk & credit analytics

1. **Credit Metrics**
   - CS01, spread sensitivities
   - Survival probabilities

2. **Structured Credit Metrics**
   - Tranche metrics
   - Base correlation

3. **Bucketed Risk**
   - Key-rate DV01
   - Standard tenor buckets

**Effort**: ~600 lines, 2 weeks

### Phase 4: Specialized (Week 7+)
**Goal**: Complete parity

1. **Performance Module**
   - XIRR calculation

2. **Covenants Module** (Optional)
   - Full covenant engine
   - Only if needed for structured finance use cases

**Effort**: ~500 lines, 1+ weeks

---

## Testing Strategy

For each new module:

1. **Unit Tests**: Rust-side tests in `finstack-wasm/src/valuations/*/tests.rs`
2. **Integration Tests**: TypeScript tests in `finstack-wasm/examples/src/tests/`
3. **Parity Tests**: Compare WASM results with Python bindings
4. **Performance Tests**: Benchmark against Rust core

---

## API Design Patterns

### Pattern 1: Wrapper Struct
```rust
#[wasm_bindgen(js_name = MetricId)]
pub struct JsMetricId {
    inner: MetricId,
}

impl JsMetricId {
    pub(crate) fn from_inner(inner: MetricId) -> Self {
        Self { inner }
    }
    
    pub(crate) fn inner(&self) -> MetricId {
        self.inner.clone()
    }
}
```

### Pattern 2: Static Methods for Presets
```rust
#[wasm_bindgen(js_class = SABRModelParams)]
impl JsSABRModelParams {
    #[wasm_bindgen(js_name = equityStandard)]
    pub fn equity_standard(alpha: f64, nu: f64, rho: f64) -> Self {
        Self::from_inner(SABRModelParams::equity_standard(alpha, nu, rho))
    }
}
```

### Pattern 3: Builder Pattern
```rust
#[wasm_bindgen(js_class = ValidationConfig)]
impl JsValidationConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { ... }
    
    #[wasm_bindgen(js_name = withTolerance)]
    pub fn with_tolerance(&self, tol: f64) -> JsValidationConfig { ... }
}
```

### Pattern 4: Result Arrays
```rust
#[wasm_bindgen]
pub fn calibrate(...) -> Result<JsValue, JsValue> {
    let result = js_sys::Array::new();
    result.push(&curve.into());
    result.push(&report.into());
    Ok(result.into())
}
```

---

## Files to Create

### Calibration
- `finstack-wasm/src/valuations/calibration/sabr.rs` (new)
- `finstack-wasm/src/valuations/calibration/validation.rs` (new)

### Metrics
- `finstack-wasm/src/valuations/metrics/` (new directory)
  - `mod.rs` - Module exports
  - `ids.rs` - MetricId enum wrapper
  - `registry.rs` - MetricRegistry wrapper
  - `calculators/` (new directory)
    - `bond.rs` - Bond metric calculators
    - `irs.rs` - IRS metric calculators
    - `options.rs` - Options Greeks
    - `credit.rs` - Credit metrics
    - `bucketed.rs` - Bucketed risk
    - `mod.rs`

### Performance
- `finstack-wasm/src/valuations/performance.rs` (new)

### Covenants (Optional)
- `finstack-wasm/src/valuations/covenants.rs` (new)

---

## Estimated Total Effort

- **Phase 1 (Critical)**: 3 weeks, ~1000 lines
- **Phase 2 (SABR)**: 1 week, ~400 lines  
- **Phase 3 (Extended)**: 2 weeks, ~600 lines
- **Phase 4 (Specialized)**: 1+ weeks, ~500 lines

**Total**: 7-8 weeks for 100% parity, ~2500 lines of Rust

**Notes**:
- Assumes 1 experienced Rust/WASM developer
- Includes testing, documentation, and examples
- Phase 1-2 gets to ~85% parity for most use cases
- Phase 3-4 covers specialized/advanced scenarios
