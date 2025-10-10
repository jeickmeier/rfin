# WASM Metrics Framework Implementation Guide

## Overview

This guide provides step-by-step instructions for implementing the metrics framework in the WASM bindings. The metrics framework is the **highest priority gap** identified in the parity analysis.

---

## Phase 1: Core Metrics Infrastructure

### Step 1: Create MetricId Wrapper

**File**: `finstack-wasm/src/valuations/metrics/ids.rs`

```rust
//! Metric identifier bindings for WASM.

use finstack_valuations::metrics::MetricId;
use wasm_bindgen::prelude::*;
use js_sys::Array;

/// Strongly-typed metric identifier.
///
/// Represents financial metrics like present value, DV01, duration, etc.
#[wasm_bindgen(js_name = MetricId)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

#[wasm_bindgen(js_class = MetricId)]
impl JsMetricId {
    /// Parse a metric ID from a string name.
    ///
    /// @param {string} name - Metric name like "pv", "dv01", "duration"
    /// @returns {MetricId} Parsed metric identifier
    /// @throws {Error} If metric name is unknown
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsMetricId, JsValue> {
        name.parse()
            .map(JsMetricId::from_inner)
            .map_err(|e: String| JsValue::from_str(&format!("Unknown metric: {}", e)))
    }
    
    /// Get the string name of the metric.
    ///
    /// @returns {string} Metric name in snake_case
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.as_str().to_string()
    }
    
    /// Get all standard metric names.
    ///
    /// @returns {Array<string>} Array of all built-in metric identifiers
    #[wasm_bindgen(js_name = standardNames)]
    pub fn standard_names() -> Array {
        let names = Array::new();
        for metric in MetricId::ALL_STANDARD {
            names.push(&JsValue::from_str(metric.as_str()));
        }
        names
    }
    
    /// Present value metric.
    #[wasm_bindgen(js_name = PV)]
    pub fn pv() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Pv)
    }
    
    /// DV01 (dollar value of 1 basis point) metric.
    #[wasm_bindgen(js_name = DV01)]
    pub fn dv01() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Dv01)
    }
    
    /// Modified duration metric.
    #[wasm_bindgen(js_name = DurationModified)]
    pub fn duration_modified() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DurationModified)
    }
    
    /// Macaulay duration metric.
    #[wasm_bindgen(js_name = DurationMacaulay)]
    pub fn duration_macaulay() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DurationMacaulay)
    }
    
    /// Convexity metric.
    #[wasm_bindgen(js_name = Convexity)]
    pub fn convexity() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Convexity)
    }
    
    /// Yield to maturity metric.
    #[wasm_bindgen(js_name = YTM)]
    pub fn ytm() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Ytm)
    }
    
    /// Accrued interest metric.
    #[wasm_bindgen(js_name = AccruedInterest)]
    pub fn accrued_interest() -> JsMetricId {
        JsMetricId::from_inner(MetricId::AccruedInterest)
    }
    
    /// Clean price metric.
    #[wasm_bindgen(js_name = CleanPrice)]
    pub fn clean_price() -> JsMetricId {
        JsMetricId::from_inner(MetricId::CleanPrice)
    }
    
    /// Dirty price metric.
    #[wasm_bindgen(js_name = DirtyPrice)]
    pub fn dirty_price() -> JsMetricId {
        JsMetricId::from_inner(MetricId::DirtyPrice)
    }
    
    // Options Greeks
    
    /// Delta (sensitivity to underlying price).
    #[wasm_bindgen(js_name = Delta)]
    pub fn delta() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Delta)
    }
    
    /// Gamma (sensitivity of delta to underlying).
    #[wasm_bindgen(js_name = Gamma)]
    pub fn gamma() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Gamma)
    }
    
    /// Vega (sensitivity to volatility).
    #[wasm_bindgen(js_name = Vega)]
    pub fn vega() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Vega)
    }
    
    /// Theta (time decay).
    #[wasm_bindgen(js_name = Theta)]
    pub fn theta() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Theta)
    }
    
    /// Rho (sensitivity to interest rates).
    #[wasm_bindgen(js_name = Rho)]
    pub fn rho() -> JsMetricId {
        JsMetricId::from_inner(MetricId::Rho)
    }
    
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.name()
    }
}
```

### Step 2: Create MetricRegistry Wrapper

**File**: `finstack-wasm/src/valuations/metrics/registry.rs`

```rust
//! Metric registry bindings for WASM.

use super::ids::JsMetricId;
use crate::core::market_data::context::JsMarketContext;
use crate::valuations::instruments::InstrumentWrapper;
use crate::valuations::results::JsValuationResult;
use finstack_valuations::metrics::{standard_registry, MetricId, MetricRegistry};
use finstack_valuations::results::ValuationResult;
use js_sys::{Array, Map, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Registry of metric calculators.
///
/// Manages metric computation with dependency resolution and caching.
#[wasm_bindgen(js_name = MetricRegistry)]
pub struct JsMetricRegistry {
    inner: MetricRegistry,
}

impl JsMetricRegistry {
    pub(crate) fn from_inner(inner: MetricRegistry) -> Self {
        Self { inner }
    }
    
    pub(crate) fn inner(&self) -> &MetricRegistry {
        &self.inner
    }
}

#[wasm_bindgen(js_class = MetricRegistry)]
impl JsMetricRegistry {
    /// Create a new empty metric registry.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsMetricRegistry {
        JsMetricRegistry {
            inner: MetricRegistry::new(),
        }
    }
    
    /// Create a standard registry with all built-in metrics.
    ///
    /// @returns {MetricRegistry} Registry with bond, IRS, deposit, and risk metrics
    #[wasm_bindgen(js_name = standard)]
    pub fn standard() -> JsMetricRegistry {
        JsMetricRegistry {
            inner: standard_registry(),
        }
    }
    
    /// Check if a metric is registered.
    ///
    /// @param {MetricId | string} metricId - Metric to check
    /// @returns {boolean} True if metric is registered
    #[wasm_bindgen(js_name = hasMetric)]
    pub fn has_metric(&self, metric_id: JsValue) -> Result<bool, JsValue> {
        let id = parse_metric_id(metric_id)?;
        Ok(self.inner.has_metric(&id))
    }
    
    /// List all registered metrics.
    ///
    /// @returns {Array<MetricId>} Array of registered metric IDs
    #[wasm_bindgen(js_name = registeredMetrics)]
    pub fn registered_metrics(&self) -> Array {
        let result = Array::new();
        for id in self.inner.registered_metrics() {
            result.push(&JsMetricId::from_inner(id.clone()).into());
        }
        result
    }
    
    /// Compute a single metric for an instrument.
    ///
    /// @param {Instrument} instrument - Instrument to price
    /// @param {MarketContext} market - Market data context
    /// @param {MetricId | string} metricId - Metric to compute
    /// @returns {number} Computed metric value
    /// @throws {Error} If metric computation fails
    #[wasm_bindgen(js_name = computeMetric)]
    pub fn compute_metric(
        &self,
        instrument: JsValue,
        market: &JsMarketContext,
        metric_id: JsValue,
    ) -> Result<f64, JsValue> {
        let inst = extract_instrument(&instrument)?;
        let id = parse_metric_id(metric_id)?;
        
        self.inner
            .compute_metric(inst.as_ref(), market.inner(), &id)
            .map_err(|e| JsValue::from_str(&format!("Metric computation failed: {}", e)))
    }
    
    /// Compute multiple metrics for an instrument.
    ///
    /// @param {Instrument} instrument - Instrument to analyze
    /// @param {MarketContext} market - Market data context
    /// @param {Array<MetricId | string>} metricIds - Metrics to compute
    /// @returns {Map<string, number>} Map of metric names to values
    /// @throws {Error} If any metric computation fails
    #[wasm_bindgen(js_name = computeMetrics)]
    pub fn compute_metrics(
        &self,
        instrument: JsValue,
        market: &JsMarketContext,
        metric_ids: Array,
    ) -> Result<Map, JsValue> {
        let inst = extract_instrument(&instrument)?;
        let mut ids = Vec::new();
        
        for item in metric_ids.iter() {
            ids.push(parse_metric_id(item)?);
        }
        
        let results = self
            .inner
            .compute_metrics(inst.as_ref(), market.inner(), &ids)
            .map_err(|e| JsValue::from_str(&format!("Metrics computation failed: {}", e)))?;
        
        let map = Map::new();
        for (id, value) in results {
            map.set(&JsValue::from_str(id.as_str()), &JsValue::from_f64(value));
        }
        
        Ok(map)
    }
}

impl Default for JsMetricRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Helper to parse MetricId from JsValue (either JsMetricId or string)
fn parse_metric_id(value: JsValue) -> Result<MetricId, JsValue> {
    // Try to extract JsMetricId
    if let Ok(wrapper) = value.dyn_into::<JsMetricId>() {
        return Ok(wrapper.inner());
    }
    
    // Try to extract string
    if let Some(name) = value.as_string() {
        return name
            .parse()
            .map_err(|e: String| JsValue::from_str(&format!("Unknown metric: {}", e)));
    }
    
    Err(JsValue::from_str(
        "Expected MetricId or string metric name",
    ))
}

// Helper to extract instrument from JsValue
// This will need to be implemented to handle all instrument types
fn extract_instrument(value: &JsValue) -> Result<Box<dyn finstack_valuations::instruments::common::traits::Instrument>, JsValue> {
    // Try each instrument type
    use crate::valuations::instruments::*;
    
    if let Ok(bond) = value.clone().dyn_into::<JsBond>() {
        return Ok(Box::new(bond.inner()));
    }
    
    if let Ok(deposit) = value.clone().dyn_into::<JsDeposit>() {
        return Ok(Box::new(deposit.inner()));
    }
    
    if let Ok(swap) = value.clone().dyn_into::<JsInterestRateSwap>() {
        return Ok(Box::new(swap.inner()));
    }
    
    // Add more instrument types...
    // TODO: Complete this for all instruments
    
    Err(JsValue::from_str("Unsupported instrument type for metrics"))
}
```

### Step 3: Create Module Structure

**File**: `finstack-wasm/src/valuations/metrics/mod.rs`

```rust
//! Metrics framework bindings for WASM.
//!
//! Provides access to financial metrics computation including bond metrics,
//! IRS metrics, options Greeks, and bucketed risk sensitivities.

pub mod ids;
pub mod registry;

pub use ids::JsMetricId;
pub use registry::JsMetricRegistry;
```

### Step 4: Update Main Valuations Module

**File**: `finstack-wasm/src/valuations/mod.rs`

```rust
pub mod calibration;
pub mod cashflow;
pub mod common;
pub mod instruments;
pub mod metrics;  // ADD THIS LINE
pub mod pricer;
pub mod results;
```

### Step 5: Update Lib.rs Exports

**File**: `finstack-wasm/src/lib.rs`

Add to the valuations section:

```rust
// Metrics
use valuations::metrics::{JsMetricId, JsMetricRegistry};

#[wasm_bindgen]
extern "C" {
    // ... existing exports
}

// Add exports for metrics
pub use valuations::metrics::{
    JsMetricId as MetricId,
    JsMetricRegistry as MetricRegistry,
};
```

---

## Phase 2: Enhance ValuationResult with Metrics

### Step 1: Update Results Module

**File**: `finstack-wasm/src/valuations/results.rs`

Add methods to compute metrics on results:

```rust
use super::metrics::{JsMetricId, JsMetricRegistry};

#[wasm_bindgen(js_class = ValuationResult)]
impl JsValuationResult {
    // ... existing methods ...
    
    /// Compute an additional metric on this result.
    ///
    /// @param {MetricRegistry} registry - Metric registry to use
    /// @param {MetricId | string} metricId - Metric to compute
    /// @param {MarketContext} market - Market data context
    /// @returns {number} Computed metric value
    #[wasm_bindgen(js_name = computeMetric)]
    pub fn compute_metric(
        &self,
        registry: &JsMetricRegistry,
        metric_id: JsValue,
        market: &JsMarketContext,
    ) -> Result<f64, JsValue> {
        // Implementation that uses stored instrument reference
        // This requires storing the instrument in ValuationResult
        // or accepting it as a parameter
        todo!("Implement metric computation on results")
    }
}
```

---

## Testing Strategy

### Unit Tests

**File**: `finstack-wasm/tests/metrics_test.rs`

```rust
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_metric_id_creation() {
    let pv = JsMetricId::pv();
    assert_eq!(pv.name(), "pv");
    
    let dv01 = JsMetricId::from_name("dv01").unwrap();
    assert_eq!(dv01.name(), "dv01");
}

#[wasm_bindgen_test]
fn test_standard_registry() {
    let registry = JsMetricRegistry::standard();
    assert!(registry.has_metric(JsValue::from_str("pv")).unwrap());
    assert!(registry.has_metric(JsValue::from_str("dv01")).unwrap());
}

#[wasm_bindgen_test]
fn test_bond_metrics() {
    // Create test bond
    let bond = JsBond::treasury(
        "TEST",
        &JsMoney::usd(1000000.0),
        0.05,
        &JsDate::from_iso("2024-01-15").unwrap(),
        &JsDate::from_iso("2026-01-15").unwrap(),
        None,
    );
    
    // Create market
    let market = JsMarketContext::new();
    
    // Compute metrics
    let registry = JsMetricRegistry::standard();
    let pv = registry
        .compute_metric(bond.into(), &market, JsValue::from_str("pv"))
        .unwrap();
    
    assert!(pv > 0.0);
}
```

### Integration Tests

**File**: `finstack-wasm/examples/src/metrics-example.tsx`

```typescript
import { MetricId, MetricRegistry, Bond, Money, Date, MarketContext } from 'finstack-wasm';

// Example 1: Single metric computation
function exampleSingleMetric() {
  const registry = MetricRegistry.standard();
  
  const bond = Bond.treasury(
    "US-TREASURY-2Y",
    Money.usd(1_000_000),
    0.0450,
    new Date(2024, 0, 15),
    new Date(2026, 0, 15),
    null
  );
  
  const market = new MarketContext();
  // ... populate market with curves
  
  const pv = registry.computeMetric(bond, market, "pv");
  console.log(`Present Value: ${pv}`);
  
  const dv01 = registry.computeMetric(bond, market, MetricId.DV01());
  console.log(`DV01: ${dv01}`);
}

// Example 2: Multiple metrics
function exampleMultipleMetrics() {
  const registry = MetricRegistry.standard();
  
  const bond = Bond.fixedSemiannual(
    "CORP-BOND",
    Money.usd(5_000_000),
    0.0550,
    new Date(2024, 0, 1),
    new Date(2029, 0, 1),
    "USD-OIS",
    null
  );
  
  const market = new MarketContext();
  // ... populate market
  
  const metrics = registry.computeMetrics(bond, market, [
    "pv",
    "dv01",
    "duration_modified",
    "convexity",
    "ytm",
    "accrued_interest"
  ]);
  
  console.log("Bond Metrics:");
  metrics.forEach((value, key) => {
    console.log(`  ${key}: ${value}`);
  });
}

// Example 3: Options Greeks
function exampleOptionsGreeks() {
  const registry = MetricRegistry.standard();
  
  const option = EquityOption.europeanCall(
    "SPY-CALL",
    "SPY",
    450.0,
    new Date(2024, 6, 21),
    Money.usd(100_000),
    100.0
  );
  
  const market = new MarketContext();
  // ... populate market with spot, vol surface
  
  const greeks = registry.computeMetrics(option, market, [
    MetricId.Delta(),
    MetricId.Gamma(),
    MetricId.Vega(),
    MetricId.Theta(),
    MetricId.Rho()
  ]);
  
  console.log("Option Greeks:");
  console.log(`  Delta: ${greeks.get("delta")}`);
  console.log(`  Gamma: ${greeks.get("gamma")}`);
  console.log(`  Vega: ${greeks.get("vega")}`);
  console.log(`  Theta: ${greeks.get("theta")}`);
  console.log(`  Rho: ${greeks.get("rho")}`);
}
```

---

## Performance Considerations

### 1. Caching Strategy

The Rust metrics framework already includes caching. Ensure WASM bindings preserve this:

- Cache computed metrics in `MetricContext`
- Reuse cashflow computations across metrics
- Share discount factor calculations

### 2. Bulk Computation

Prefer `computeMetrics()` over multiple `computeMetric()` calls:

```typescript
// ❌ Inefficient
const pv = registry.computeMetric(bond, market, "pv");
const dv01 = registry.computeMetric(bond, market, "dv01");
const duration = registry.computeMetric(bond, market, "duration_modified");

// ✅ Efficient (shares computation)
const metrics = registry.computeMetrics(bond, market, ["pv", "dv01", "duration_modified"]);
```

### 3. Parallel Computation (Future)

Consider exposing parallel metric computation for portfolios:

```typescript
// Future API
const results = registry.computeMetricsParallel(
  bonds, // Array of instruments
  market,
  ["pv", "dv01", "duration"]
);
```

---

## Documentation

### JSDoc Comments

All public methods should have comprehensive JSDoc:

```typescript
/**
 * Compute multiple metrics for an instrument.
 *
 * Computes the requested metrics efficiently by sharing intermediate
 * calculations (e.g., cashflows, discount factors). Dependencies between
 * metrics are automatically resolved.
 *
 * @param {Instrument} instrument - Financial instrument to analyze
 * @param {MarketContext} market - Market data including curves and surfaces
 * @param {Array<MetricId | string>} metricIds - Metrics to compute
 * @returns {Map<string, number>} Map of metric names to computed values
 * @throws {Error} If metric computation fails or market data is missing
 *
 * @example
 * ```typescript
 * const registry = MetricRegistry.standard();
 * const metrics = registry.computeMetrics(bond, market, [
 *   "pv", "dv01", "duration_modified", "convexity"
 * ]);
 * console.log(`PV: ${metrics.get("pv")}`);
 * console.log(`DV01: ${metrics.get("dv01")}`);
 * ```
 */
computeMetrics(instrument: Instrument, market: MarketContext, metricIds: Array<MetricId | string>): Map<string, number>;
```

### README Section

Add to `finstack-wasm/README.md`:

```markdown
## Computing Metrics

The metrics framework provides risk analytics and financial measures:

### Quick Start

```typescript
import { MetricRegistry, Bond, Money, Date } from 'finstack-wasm';

const registry = MetricRegistry.standard();
const bond = Bond.treasury(...);
const market = new MarketContext();

// Single metric
const pv = registry.computeMetric(bond, market, "pv");

// Multiple metrics (more efficient)
const metrics = registry.computeMetrics(bond, market, [
  "pv", "dv01", "duration_modified", "convexity"
]);
```

### Available Metrics

**Bond Metrics**:
- Present value: `pv`
- Pricing: `clean_price`, `dirty_price`, `accrued_interest`
- Yield: `ytm`, `ytw`
- Duration: `duration_modified`, `duration_macaulay`
- Risk: `dv01`, `convexity`
- Credit: `z_spread`, `i_spread`, `oas`, `asw`, `cs01`

**Options Metrics (Greeks)**:
- First-order: `delta`, `vega`, `theta`, `rho`
- Second-order: `gamma`, `vanna`, `volga`
- Other: `implied_vol`

See [Metrics Guide](./METRICS.md) for complete list.
```

---

## Next Steps

After completing Phase 1:

1. **Validate** against Python bindings for numerical accuracy
2. **Benchmark** performance vs pure Rust
3. **Document** all public APIs with examples
4. **Test** in browser and Node.js environments
5. **Begin Phase 2**: SABR & Validation modules

---

## Questions/Decisions

1. **Instrument Extraction**: Should we store instrument reference in `ValuationResult` to enable `result.computeMetric()`?

2. **Error Handling**: Use Result or throw JS errors? Current pattern uses Result for calibration but JS errors might be more idiomatic for WASM.

3. **Memory Management**: How to handle large portfolios? Consider streaming API for bulk computations.

4. **Type Definitions**: Generate `.d.ts` files automatically or maintain manually?

---

## References

- Python bindings: `finstack-py/src/valuations/metrics.rs`
- Rust core: `finstack/valuations/src/metrics/`
- Existing WASM patterns: `finstack-wasm/src/valuations/calibration/`
