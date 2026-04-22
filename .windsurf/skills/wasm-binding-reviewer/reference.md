# WASM Binding Reference

## Codebase Structure

```
finstack-wasm/
├── Cargo.toml              # Pinned wasm-bindgen version, feature flags
├── pkg/                    # wasm-pack output (JS glue, .d.ts, .wasm)
│   ├── package.json
│   ├── finstack_wasm.js
│   ├── finstack_wasm.d.ts
│   └── finstack_wasm_bg.wasm.d.ts
├── src/
│   ├── lib.rs              # Flat re-exports, wasm_bindgen(start)
│   ├── core/
│   │   ├── error.rs        # JS error mapping (core_to_js, js_error_with_kind)
│   │   ├── common/         # Shared argument types
│   │   ├── currency.rs, money.rs, cashflow.rs, config.rs, expr.rs, ...
│   │   ├── dates/          # Calendar, date, daycount, schedule, periods
│   │   ├── market_data/    # Context, curves, surfaces, fx, bumps
│   │   └── math/           # Integration, linalg, stats, solvers
│   ├── utils/
│   │   ├── json.rs         # serde_wasm_bindgen helpers (to_js_value, from_js_value)
│   │   └── decimal.rs
│   ├── valuations/
│   │   ├── pricer.rs       # PricerRegistry, PricingRequest
│   │   ├── results.rs, performance.rs, dataframe.rs, attribution.rs, risk.rs
│   │   ├── calibration/    # Curve/surface calibration
│   │   ├── instruments/    # One .rs per instrument + wrapper.rs
│   │   ├── common/         # Parameters, parse, monte_carlo
│   │   └── metrics/
│   ├── statements/         # Statement evaluation
│   ├── scenarios/          # Scenario engine
│   └── portfolio/          # Portfolio management
├── types/
│   └── generated/          # Hand-maintained or ts-rs generated TS types
└── tests/                  # Integration tests
```

## Standard Patterns

### 1. Wrapper Struct Pattern

Every Rust type exposed to JS follows this pattern:

```rust
use wasm_bindgen::prelude::*;
use finstack_core::money::Money;

#[wasm_bindgen(js_name = Money)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JsMoney {
    inner: Money,
}

impl JsMoney {
    /// Internal constructor - used by other bindings
    pub(crate) fn from_inner(inner: Money) -> Self {
        Self { inner }
    }

    /// Internal accessor
    pub(crate) fn inner(&self) -> Money {
        self.inner
    }
}

#[wasm_bindgen(js_class = Money)]
impl JsMoney {
    /// JS constructor
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: &JsCurrency) -> Result<JsMoney, JsValue> {
        Ok(Self {
            inner: Money::new(amount, currency.inner()),
        })
    }

    /// Getter - just exposes Rust data
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }

    /// Method - delegates to Rust, maps error
    #[wasm_bindgen(js_name = convert)]
    pub fn convert(&self, target: &JsCurrency, fx: &JsFxMatrix) -> Result<JsMoney, JsValue> {
        self.inner
            .convert(target.inner(), &fx.inner)
            .map(Self::from_inner)
            .map_err(crate::core::error::core_to_js)
    }
}
```

### 2. Error Mapping

Centralized error conversion in `core/error.rs`:

```rust
use js_sys;
use wasm_bindgen::JsValue;
use finstack_core::Error;

/// Error kind taxonomy for JS consumers
pub(crate) enum ErrorKind {
    Input,
    Validation,
    Calibration,
    Interp,
    Generic,
}

impl ErrorKind {
    pub(crate) fn js_name(&self) -> &'static str {
        match self {
            Self::Input => "InputError",
            Self::Validation => "ValidationError",
            Self::Calibration => "CalibrationError",
            Self::Interp => "InterpolationError",
            Self::Generic => "FinstackError",
        }
    }
}

/// Create a JS Error with a named kind for `catch (e) { e.name === "..." }`
pub(crate) fn js_error_with_kind(kind: ErrorKind, message: impl ToString) -> JsValue {
    let error = js_sys::Error::new(&message.to_string());
    let _ = js_sys::Reflect::set(&error, &"name".into(), &kind.js_name().into());
    JsValue::from(error)
}

/// Map core error to JS error with appropriate kind
pub(crate) fn core_to_js(err: Error) -> JsValue {
    match err {
        Error::Input(input) => input_to_js(input),
        Error::InterpOutOfBounds => {
            js_error_with_kind(ErrorKind::Interp, "Interpolation input out of bounds")
        }
        _ => js_error_with_kind(ErrorKind::Generic, err.to_string()),
    }
}

/// Convenience macro for formatted generic errors
macro_rules! js_err {
    ($($arg:tt)*) => {
        JsValue::from_str(&format!($($arg)*))
    };
}
```

### 3. Serde Bridge (utils/json.rs)

For structured data conversion between Rust and JS:

```rust
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::JsValue;

pub fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize: {}", e)))
}

pub fn from_js_value<T: DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize: {}", e)))
}
```

### 4. Module Registration (Flat Exports)

Unlike Python's `register()` pattern, WASM uses flat re-exports in `lib.rs`:

```rust
// lib.rs - flat re-export with JS-friendly names
pub use core::cashflow::{JsCFKind as CFKind, JsCashFlow as CashFlow};
pub use core::config::{JsFinstackConfig as FinstackConfig, JsRoundingMode as RoundingMode};
pub use core::currency::JsCurrency as Currency;
pub use core::money::JsMoney as Money;
pub use core::dates::{
    adjust, BusinessDayConvention, Calendar, DayCount, FsDate, Frequency,
    Period, Schedule, ScheduleBuilder, Tenor,
};
pub use valuations::pricer::{JsPricerRegistry as PricerRegistry, JsPricingRequest as PricingRequest};
// ... hundreds of exports
```

New types must be added to this re-export list to be visible from JS.

### 5. Builder Pattern

For complex objects with many optional parameters:

```rust
#[wasm_bindgen(js_name = BondBuilder)]
pub struct JsBondBuilder {
    notional: Option<f64>,
    coupon_rate: Option<f64>,
    // ... more optional fields
}

#[wasm_bindgen(js_class = BondBuilder)]
impl JsBondBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { notional: None, coupon_rate: None }
    }

    /// Fluent setter - returns Self for chaining
    #[wasm_bindgen(js_name = withNotional)]
    pub fn with_notional(mut self, value: f64) -> Self {
        self.notional = Some(value);
        self
    }

    #[wasm_bindgen(js_name = withCouponRate)]
    pub fn with_coupon_rate(mut self, rate: f64) -> Self {
        self.coupon_rate = Some(rate);
        self
    }

    pub fn build(&self) -> Result<JsBond, JsValue> {
        // Assemble core type from collected fields, map error
        let bond = Bond::builder()
            .notional(self.notional.unwrap_or(1_000_000.0))
            .coupon_rate(self.coupon_rate.ok_or_else(|| js_err!("coupon_rate required"))?)
            .build()
            .map_err(core_to_js)?;
        Ok(JsBond::from_inner(bond))
    }
}
```

### 6. Collection Conversion

Building JS collections from Rust data:

```rust
use js_sys;

// Return a js_sys::Map for key-value data
fn measures_to_js(measures: &HashMap<String, f64>) -> js_sys::Map {
    let map = js_sys::Map::new();
    for (k, v) in measures {
        map.set(&JsValue::from_str(k), &JsValue::from_f64(*v));
    }
    map
}

// Return a js_sys::Array for list data
fn dates_to_js(dates: &[NaiveDate]) -> js_sys::Array {
    dates.iter()
        .map(|d| JsValue::from_str(&d.to_string()))
        .collect()
}
```

### 7. Instrument Extraction (Unsafe)

The `extract_instrument` function is the one sanctioned `unsafe` block in the crate. It reads `__wbg_ptr` to borrow the inner Rust type from a JS object:

```rust
#[allow(unsafe_code)]
pub(crate) fn extract_instrument(value: &JsValue) -> Result<Box<dyn Instrument>, JsValue> {
    macro_rules! try_extract {
        ($js_type:ty, $js_name:expr) => {{
            let is_instance = Reflect::get(value, &JsValue::from_str("constructor"))
                .ok()
                .and_then(|c| Reflect::get(&c, &JsValue::from_str("name")).ok())
                .and_then(|n| n.as_string())
                .map(|n| n == $js_name)
                .unwrap_or(false);

            if is_instance {
                let ptr_val = Reflect::get(value, &JsValue::from_str("__wbg_ptr"))
                    .or_else(|_| Reflect::get(value, &JsValue::from_str("ptr")))
                    .map_err(|_| JsValue::from_str("Could not find Rust pointer"))?;
                // ... unsafe deref, borrow, clone inner
            }
        }};
    }
    // ...
}
```

This is pinned to `wasm-bindgen = "=0.2.114"` because it depends on WasmRefCell internal layout. Any `wasm-bindgen` version bump must verify this still works.

## Python Binding Comparison

Both WASM and Python bindings expose identical Rust functionality:

| Aspect | WASM (wasm-bindgen) | Python (PyO3) |
|--------|---------------------|---------------|
| Wrapper struct | `inner: T` (private) | `pub(crate) inner: T` |
| Constructor | `from_inner(inner: T)` | `from_inner(inner: T)` |
| Accessor | `pub(crate) fn inner()` | Direct field access `self.inner` |
| Error handling | `.map_err(core_to_js)` → `Result<T, JsValue>` | `.map_err(map_error)` → `PyResult<T>` |
| Naming | `js_name` camelCase | Python snake_case |
| Module structure | Flat exports in `lib.rs` | Submodules via `register()` |
| Serde | `serde_wasm_bindgen` | N/A (uses PyO3 conversions) |
| Type stubs | `pkg/*.d.ts` (generated) + `types/generated/*.ts` | `.pyi` stubs (manually maintained) |

## Rust Core Crates

Bindings wrap these core crates:

| Crate | Purpose |
|-------|---------|
| `finstack_core` | Dates, money, currency, market data, math |
| `finstack_valuations` | Instruments, pricers, metrics, Greeks |
| `finstack_portfolio` | Portfolio management, aggregation |
| `finstack_statements` | Financial statement modeling |
| `finstack_scenarios` | Scenario engine, stress testing |

All computation lives in these crates. Bindings only wrap and expose.

## WASM-Specific Concerns

### Pinned Dependencies

`wasm-bindgen = "=0.2.114"` is pinned because `extract_instrument` depends on `WasmRefCell` layout. Version bumps require testing the unsafe extraction code.

### Feature Flags

- `console_error_panic_hook`: Enables better panic messages in browser console
- `scenarios`: Gates scenario-related portfolio exports
- `ts_export`: Enables ts-rs TypeScript type generation

### TypeScript Generation

Two approaches coexist:
1. **wasm-pack generated**: `pkg/finstack_wasm.d.ts` — automatic from `#[wasm_bindgen]` annotations
2. **ts-rs generated**: `types/generated/*.ts` — opt-in via `#[derive(TS)]` on `genui/types.rs`

### Initialization

```rust
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
```
