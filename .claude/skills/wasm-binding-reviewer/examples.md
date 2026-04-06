# WASM Binding Examples

## Good Patterns

### Example 1: Pure Wrapper with Error Mapping

```rust
// GOOD: Wrapper with proper error mapping and JS naming
#[wasm_bindgen(js_name = Money)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JsMoney {
    inner: Money,
}

impl JsMoney {
    pub(crate) fn from_inner(inner: Money) -> Self {
        Self { inner }
    }
    pub(crate) fn inner(&self) -> Money {
        self.inner
    }
}

#[wasm_bindgen(js_class = Money)]
impl JsMoney {
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: &JsCurrency) -> Result<JsMoney, JsValue> {
        Ok(Self::from_inner(Money::new(amount, currency.inner())))
    }

    /// Converts to target currency using FX rates
    #[wasm_bindgen(js_name = convert)]
    pub fn convert(&self, target: &JsCurrency, fx: &JsFxMatrix) -> Result<JsMoney, JsValue> {
        // Delegates to Rust, maps error - NO LOGIC HERE
        self.inner
            .convert(target.inner(), &fx.inner)
            .map(Self::from_inner)
            .map_err(core_to_js)
    }

    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()  // Just expose data
    }
}
```

### Example 2: Serde Bridge for Structured Data

```rust
// GOOD: Use serde_wasm_bindgen for complex structured types
use crate::utils::json::{to_js_value, from_js_value};

#[wasm_bindgen(js_class = ScenarioEngine)]
impl JsScenarioEngine {
    /// Accept a scenario spec as a plain JS object
    #[wasm_bindgen(js_name = runScenario)]
    pub fn run_scenario(&self, spec: JsValue) -> Result<JsValue, JsValue> {
        let spec: ScenarioSpec = from_js_value(spec)?;
        let result = self.inner.run(&spec).map_err(core_to_js)?;
        to_js_value(&result)
    }
}
```

### Example 3: Named Error Taxonomy

```rust
// GOOD: Errors with kind names for JS consumers
pub(crate) fn core_to_js(err: Error) -> JsValue {
    match err {
        Error::Input(input) => input_to_js(input),
        Error::InterpOutOfBounds => {
            js_error_with_kind(ErrorKind::Interp, "Interpolation input out of bounds")
        }
        Error::Calibration { instrument, reason } => {
            js_error_with_kind(
                ErrorKind::Calibration,
                format!("Calibration failed for {}: {}", instrument, reason),
            )
        }
        _ => js_error_with_kind(ErrorKind::Generic, err.to_string()),
    }
}

// JS consumer can match on error.name:
// try { bond.dirtyPrice(md); } catch (e) { if (e.name === "CalibrationError") ... }
```

### Example 4: Collection Conversion

```rust
// GOOD: Convert Rust collections to JS types for return values
#[wasm_bindgen(js_class = ValuationResult)]
impl JsValuationResult {
    /// Return measures as a JS Map
    #[wasm_bindgen(getter)]
    pub fn measures(&self) -> js_sys::Map {
        let map = js_sys::Map::new();
        for (k, v) in self.inner.measures() {
            map.set(&JsValue::from_str(k), &JsValue::from_f64(*v));
        }
        map
    }
}
```

### Example 5: Fluent Builder

```rust
// GOOD: Builder with fluent chaining, camelCase JS names
#[wasm_bindgen(js_name = PricingRequest)]
pub struct JsPricingRequest {
    metrics: Option<Vec<MetricId>>,
}

#[wasm_bindgen(js_class = PricingRequest)]
impl JsPricingRequest {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { metrics: None }
    }

    #[wasm_bindgen(js_name = withMetrics)]
    pub fn with_metrics(mut self, metrics: js_sys::Array) -> Self {
        self.metrics = Some(metrics_from_array(&metrics));
        self
    }
}
```

## Anti-Patterns and Fixes

### Anti-Pattern 1: Business Logic in Binding

```rust
// BAD: Yield calculation implemented in binding
#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    #[wasm_bindgen(js_name = yieldToMaturity)]
    pub fn yield_to_maturity(&self, price: f64) -> Result<f64, JsValue> {
        // Newton-Raphson solver - THIS IS BUSINESS LOGIC
        let mut y = 0.05;
        let tolerance = 1e-10;

        for _ in 0..100 {
            let pv = self.calculate_pv(y);
            let dpv = self.calculate_duration(y);
            let diff = pv - price;
            if diff.abs() < tolerance {
                return Ok(y);
            }
            y = y - diff / dpv;
        }

        Err(JsValue::from_str("Failed to converge"))
    }
}
```

**FIX**: Move to Rust core:

```rust
// In finstack_valuations/src/instruments/bond.rs
impl Bond {
    pub fn yield_to_maturity(&self, price: f64) -> Result<f64, Error> {
        solver::find_yield(self, price, SolverConfig::default())
    }
}

// In finstack-wasm/src/valuations/instruments/bond.rs
#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    #[wasm_bindgen(js_name = yieldToMaturity)]
    pub fn yield_to_maturity(&self, price: f64) -> Result<f64, JsValue> {
        self.inner.yield_to_maturity(price).map_err(core_to_js)
    }
}
```

### Anti-Pattern 2: Validation in Binding

```rust
// BAD: Validation logic in WASM binding
#[wasm_bindgen(js_class = Swap)]
impl JsSwap {
    #[wasm_bindgen(constructor)]
    pub fn new(notional: f64, fixed_rate: f64, tenor: &str) -> Result<JsSwap, JsValue> {
        // These checks should be in Rust
        if notional <= 0.0 {
            return Err(JsValue::from_str("Notional must be positive"));
        }
        if fixed_rate < -0.10 || fixed_rate > 0.50 {
            return Err(JsValue::from_str("Rate out of reasonable range"));
        }

        let inner = Swap::new(notional, fixed_rate, Tenor::parse(tenor)?);
        Ok(Self { inner })
    }
}
```

**FIX**: Move validation to Rust constructor:

```rust
// In finstack_valuations/src/instruments/swap.rs
impl Swap {
    pub fn new(notional: f64, fixed_rate: f64, tenor: Tenor) -> Result<Self, Error> {
        if notional <= 0.0 {
            return Err(Error::validation("Notional must be positive"));
        }
        // ... construct swap
    }
}

// In finstack-wasm/src/valuations/instruments/swap.rs
#[wasm_bindgen(js_class = Swap)]
impl JsSwap {
    #[wasm_bindgen(constructor)]
    pub fn new(notional: f64, fixed_rate: f64, tenor: &str) -> Result<JsSwap, JsValue> {
        let tenor = Tenor::parse(tenor).map_err(core_to_js)?;
        Swap::new(notional, fixed_rate, tenor)
            .map(Self::from_inner)
            .map_err(core_to_js)
    }
}
```

### Anti-Pattern 3: Data Transformation

```rust
// BAD: Aggregation logic in binding
#[wasm_bindgen(js_class = Portfolio)]
impl JsPortfolio {
    #[wasm_bindgen(js_name = exposureByCurrency)]
    pub fn exposure_by_currency(&self) -> js_sys::Map {
        let map = js_sys::Map::new();
        // Aggregation should be in Rust
        for position in self.inner.positions() {
            let ccy = position.currency().code();
            let value = position.market_value();
            let existing = map.get(&JsValue::from_str(ccy))
                .as_f64()
                .unwrap_or(0.0);
            map.set(
                &JsValue::from_str(ccy),
                &JsValue::from_f64(existing + value),
            );
        }
        map
    }
}
```

**FIX**: Add method to Rust core:

```rust
// In finstack_portfolio/src/portfolio.rs
impl Portfolio {
    pub fn exposure_by_currency(&self) -> HashMap<Currency, f64> {
        self.positions
            .iter()
            .fold(HashMap::new(), |mut acc, pos| {
                *acc.entry(pos.currency()).or_insert(0.0) += pos.market_value();
                acc
            })
    }
}

// In finstack-wasm/src/portfolio/portfolio.rs
#[wasm_bindgen(js_class = Portfolio)]
impl JsPortfolio {
    #[wasm_bindgen(js_name = exposureByCurrency)]
    pub fn exposure_by_currency(&self) -> js_sys::Map {
        let map = js_sys::Map::new();
        for (ccy, val) in self.inner.exposure_by_currency() {
            map.set(&JsValue::from_str(ccy.code()), &JsValue::from_f64(val));
        }
        map
    }
}
```

### Anti-Pattern 4: Raw String Errors Instead of Taxonomy

```rust
// BAD: Using raw strings instead of error taxonomy
#[wasm_bindgen(js_class = Curve)]
impl JsCurve {
    #[wasm_bindgen(js_name = rateAt)]
    pub fn rate_at(&self, tenor: f64) -> Result<f64, JsValue> {
        self.inner.rate_at(tenor)
            .map_err(|e| JsValue::from_str(&e.to_string()))  // BAD: loses error kind
    }
}
```

**FIX**: Use the error taxonomy:

```rust
// GOOD: Proper error mapping preserves error kind
#[wasm_bindgen(js_class = Curve)]
impl JsCurve {
    #[wasm_bindgen(js_name = rateAt)]
    pub fn rate_at(&self, tenor: f64) -> Result<f64, JsValue> {
        self.inner.rate_at(tenor).map_err(core_to_js)
    }
}
```

### Anti-Pattern 5: Manual JS Object Construction for DTOs

```rust
// BAD: Manual JS object construction for structured data
#[wasm_bindgen(js_class = Report)]
impl JsReport {
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"name".into(), &self.inner.name().into())?;
        js_sys::Reflect::set(&obj, &"date".into(), &self.inner.date().to_string().into())?;
        // ... 20 more fields manually set
        Ok(obj.into())
    }
}
```

**FIX**: Use serde bridge:

```rust
// GOOD: Serde bridge for DTOs
#[wasm_bindgen(js_class = Report)]
impl JsReport {
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        crate::utils::json::to_js_value(&self.inner)
    }
}
```

## Python Parity Check Examples

When reviewing, verify WASM implementations can be trivially replicated in Python:

### Good: Trivial Python Equivalent

```rust
// WASM binding
#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    #[wasm_bindgen(js_name = cleanPrice)]
    pub fn clean_price(&self, market_data: &JsMarketData) -> Result<f64, JsValue> {
        self.inner.clean_price(&market_data.inner).map_err(core_to_js)
    }
}

// Python equivalent - trivial to write
#[pymethods]
impl PyBond {
    fn clean_price(&self, market_data: &PyMarketData) -> PyResult<f64> {
        self.inner.clean_price(&market_data.inner).map_err(map_error)
    }
}
```

### Bad: Python Would Need Reimplementation

```rust
// WASM binding with logic
#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    #[wasm_bindgen(js_name = cleanPrice)]
    pub fn clean_price(&self, market_data: &JsMarketData) -> Result<f64, JsValue> {
        let dirty = self.inner.dirty_price(&market_data.inner).map_err(core_to_js)?;
        let accrued = self.calculate_accrued_interest()?;  // <-- Logic here
        Ok(dirty - accrued)
    }

    fn calculate_accrued_interest(&self) -> Result<f64, JsValue> {
        // Accrued interest calculation - should be in Rust
        let days = (today() - self.inner.last_coupon_date()).days();
        let period_days = self.inner.coupon_frequency().days();
        Ok(self.inner.coupon_rate() * self.inner.notional() * (days as f64 / period_days as f64))
    }
}

// Python would need to reimplement calculate_accrued_interest()!
```
