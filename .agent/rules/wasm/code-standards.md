---
trigger: model_decision
description: Rust-Wasm Bindings
---

# WASM Bindings Code Standards for rfin-wasm

## Core Principles

1. **Web-first API design** - APIs should feel natural to JavaScript/TypeScript developers
2. **Small bundle size** - Minimize generated WASM size through careful feature selection
3. **Performance** - Avoid unnecessary allocations and copies between JS and WASM
4. **Error handling** - Convert Rust errors to JavaScript-friendly error messages
5. **Cross-platform** - Support both browser and Node.js environments

## Project Structure

### Organization
```
rfin-wasm/
├── src/
│   ├── lib.rs        # Module initialization and re-exports
│   ├── currency.rs   # Currency type bindings
│   ├── money.rs      # Money type bindings
│   ├── dates.rs      # Date type bindings
│   ├── calendar.rs   # Calendar bindings
│   ├── cashflow.rs   # CashFlow bindings
│   ├── schedule.rs   # Schedule generation
│   └── utils.rs      # WASM utilities (panic hook, etc.)
├── pkg/              # Generated web package
├── pkg-node/         # Generated Node.js package
└── tests/           # WASM tests
    └── web.rs
```

### Build Targets
```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]  # Required for WASM

[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"  # For complex object serialization
```

## Type Wrapping Patterns

### Basic Type Wrapper
```rust
use wasm_bindgen::prelude::*;
use rfin_core::TypeName as CoreType;

/// JavaScript-visible documentation
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct TypeName {
    inner: CoreType,
}

#[wasm_bindgen]
impl TypeName {
    /// Constructor - always use #[wasm_bindgen(constructor)]
    #[wasm_bindgen(constructor)]
    pub fn new(param: String) -> Result<TypeName, JsValue> {
        let inner = param
            .parse::<CoreType>()
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        Ok(TypeName { inner })
    }
}
```

### Property Getters
```rust
#[wasm_bindgen]
impl TypeName {
    /// Use getter for properties
    #[wasm_bindgen(getter)]
    pub fn property(&self) -> String {
        self.inner.property().to_string()
    }
    
    /// Use js_name for JavaScript naming conventions
    #[wasm_bindgen(getter, js_name = "numericCode")]
    pub fn numeric_code(&self) -> u32 {
        self.inner.code()
    }
}
```

### Methods
```rust
#[wasm_bindgen]
impl TypeName {
    /// Regular methods
    #[wasm_bindgen]
    pub fn calculate(&self, value: f64) -> f64 {
        self.inner.calculate(value)
    }
    
    /// Methods with JS-friendly names
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.inner)
    }
    
    /// Equality checks (JavaScript doesn't have operator overloading)
    #[wasm_bindgen]
    pub fn equals(&self, other: &TypeName) -> bool {
        self.inner == other.inner
    }
}
```

## Error Handling

### Convert Rust Errors to JavaScript
```rust
use rfin_core::error::{Error, InputError};

fn convert_error(err: Error) -> JsValue {
    match err {
        Error::Input(InputError::InvalidDateRange) => {
            JsValue::from_str("Invalid date range: start must be before end")
        }
        Error::CurrencyMismatch { expected, actual } => {
            JsValue::from_str(&format!(
                "Currency mismatch: expected {}, got {}", 
                expected, actual
            ))
        }
        Error::InterpOutOfBounds => {
            JsValue::from_str("Interpolation input out of bounds")
        }
        _ => JsValue::from_str(&format!("Operation failed: {}", err))
    }
}
```

### Method Error Handling
```rust
#[wasm_bindgen]
impl Money {
    #[wasm_bindgen]
    pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
        self.inner
            .checked_add(other.inner)
            .map(|result| Money { inner: result })
            .map_err(convert_error)
    }
}
```

## Enum Handling

### Simple Enums
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum Frequency {
    Annual = "Annual",
    SemiAnnual = "SemiAnnual",
    Quarterly = "Quarterly",
    Monthly = "Monthly",
}

// Conversion to core enum
impl Into<CoreFrequency> for Frequency {
    fn into(self) -> CoreFrequency {
        match self {
            Frequency::Annual => CoreFrequency::annual(),
            Frequency::SemiAnnual => CoreFrequency::semi_annual(),
            Frequency::Quarterly => CoreFrequency::quarterly(),
            Frequency::Monthly => CoreFrequency::monthly(),
        }
    }
}
```

### Static Enum Methods
```rust
#[wasm_bindgen]
impl DayCount {
    #[wasm_bindgen(js_name = "Act360")]
    pub fn act360() -> DayCount {
        DayCount { inner: CoreDayCount::Act360 }
    }
    
    #[wasm_bindgen(js_name = "Act365F")]
    pub fn act365f() -> DayCount {
        DayCount { inner: CoreDayCount::Act365F }
    }
}
```

## Complex Type Serialization

### Using Serde for Complex Objects
```rust
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::{to_value, from_value};

#[derive(Serialize, Deserialize)]
pub struct CashFlowData {
    date: String,
    amount: f64,
    currency: String,
    kind: String,
}

#[wasm_bindgen]
impl FixedRateLeg {
    /// Return cash flows as JavaScript array
    #[wasm_bindgen(js_name = "getCashFlows")]
    pub fn get_cash_flows(&self) -> Result<JsValue, JsValue> {
        let flows: Vec<CashFlowData> = self.inner
            .flows()
            .iter()
            .map(|cf| CashFlowData {
                date: cf.date.to_string(),
                amount: cf.amount.amount(),
                currency: cf.amount.currency().to_string(),
                kind: format!("{:?}", cf.kind),
            })
            .collect();
        
        to_value(&flows).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

## Module Initialization

### lib.rs Pattern
```rust
use wasm_bindgen::prelude::*;

mod utils;

/// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

// Re-export all public types
pub use currency::Currency;
pub use money::Money;
pub use dates::Date;

// Re-export functions with JS-friendly names
pub use dates::{
    third_wednesday as thirdWednesday,
    next_imm as nextImm,
    next_cds_date as nextCdsDate,
};
```

### utils.rs Pattern
```rust
pub fn set_panic_hook() {
    // Only include panic hook in debug builds
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
```

## Memory Management

### Avoid Unnecessary Clones
```rust
#[wasm_bindgen]
impl Currency {
    // Good: Return lightweight copy
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        format!("{}", self.inner)
    }
    
    // Avoid: Returning references (not supported by wasm-bindgen)
    // pub fn code(&self) -> &str { ... }
}
```

### Handle Collections Efficiently
```rust
use js_sys::Array;

#[wasm_bindgen]
pub fn generate_dates(start: &Date, end: &Date, freq: Frequency) -> Array {
    let dates = generate_schedule_internal(start, end, freq);
    
    let array = Array::new();
    for date in dates {
        array.push(&JsValue::from(Date::from_inner(date)));
    }
    array
}
```

## Testing

### WASM Test Structure
```rust
// tests/web.rs
use wasm_bindgen_test::*;
use rfin_wasm::{Currency, Money};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_currency_creation() {
    let currency = Currency::new("USD".to_string()).unwrap();
    assert_eq!(currency.code(), "USD");
    assert_eq!(currency.numeric_code(), 840);
}

#[wasm_bindgen_test]
fn test_money_arithmetic() {
    let usd = Currency::new("USD".to_string()).unwrap();
    let m1 = Money::new(100.0, usd.clone());
    let m2 = Money::new(50.0, usd);
    
    let result = m1.add(&m2).unwrap();
    assert_eq!(result.amount(), 150.0);
}
```

## JavaScript API Design

### Constructor Pattern
```rust
// Always provide constructors for main types
#[wasm_bindgen(constructor)]
pub fn new(/* params */) -> Result<Self, JsValue> {
    // Implementation
}
```

### Method Naming
```rust
// Use JavaScript conventions
#[wasm_bindgen(js_name = "toString")]
pub fn to_string_js(&self) -> String { }

#[wasm_bindgen(js_name = "valueOf")]
pub fn value_of(&self) -> f64 { }

#[wasm_bindgen(js_name = "toJSON")]
pub fn to_json(&self) -> Result<JsValue, JsValue> { }
```

### Optional Parameters
```rust
use wasm_bindgen::JsValue;

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen]
    pub fn calculate(&self, value: f64, options: Option<JsValue>) -> Result<f64, JsValue> {
        let params = if let Some(opts) = options {
            // Parse options object
            parse_options(opts)?
        } else {
            // Use defaults
            Default::default()
        };
        
        Ok(self.inner.calculate(value, params))
    }
}
```

## Documentation

### Type Documentation
```rust
/// Currency representation based on ISO 4217 standards.
/// 
/// A Currency represents a specific currency using the ISO 4217 standard.
/// 
/// @example
/// ```javascript
/// const usd = new Currency("USD");
/// console.log(usd.code); // "USD"
/// console.log(usd.numericCode); // 840
/// ```
#[wasm_bindgen]
pub struct Currency {
    // ...
}
```

### Method Documentation
```rust
/// Add two money values.
/// 
/// Both money values must have the same currency.
/// 
/// @param {Money} other - The money value to add
/// @returns {Money} The sum of the two money values
/// @throws {Error} If the currencies don't match
#[wasm_bindgen]
pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
    // ...
}
```

## Build Configuration

### wasm-pack Settings
```toml
# Cargo.toml
[package.metadata.wasm-pack]
"wasm-pack-plugin" = "0.1"

[features]
# Include panic hook only in debug builds
default = ["console_error_panic_hook"]
console_error_panic_hook = ["dep:console_error_panic_hook"]

# Pass through features to core
decimal128 = ["rfin-core/decimal128"]
```

### Build Scripts
```json
// package.json
{
  "scripts": {
    "build": "wasm-pack build --target web --out-dir pkg",
    "build:node": "wasm-pack build --target nodejs --out-dir pkg-node",
    "build:bundler": "wasm-pack build --target bundler --out-dir pkg-bundler",
    "test": "wasm-pack test --chrome --firefox --headless"
  }
}
```

## Performance Guidelines

### Minimize Boundary Crossings
```rust
// Good: Batch operations
#[wasm_bindgen]
pub fn calculate_multiple(values: &[f64]) -> Vec<f64> {
    values.iter().map(|&v| self.calculate(v)).collect()
}

// Avoid: Multiple individual calls from JavaScript
```

### Use References Where Possible
```rust
// Good: Accept references
#[wasm_bindgen]
pub fn compare(&self, other: &Money) -> bool {
    self.inner == other.inner
}

// Avoid: Unnecessary ownership transfer
pub fn compare(self, other: Money) -> bool { }
```

## Debugging Support

### Console Logging
```rust
use web_sys::console;

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen]
    pub fn debug_state(&self) {
        console::log_1(&format!("State: {:?}", self.inner).into());
    }
}
```

### Development Features
```rust
#[cfg(debug_assertions)]
#[wasm_bindgen]
impl TypeName {
    #[wasm_bindgen(js_name = "_debug")]
    pub fn debug(&self) -> String {
        format!("{:?}", self.inner)
    }
}
``` # WASM Bindings Code Standards for rfin-wasm

## Core Principles

1. **Web-first API design** - APIs should feel natural to JavaScript/TypeScript developers
2. **Small bundle size** - Minimize generated WASM size through careful feature selection
3. **Performance** - Avoid unnecessary allocations and copies between JS and WASM
4. **Error handling** - Convert Rust errors to JavaScript-friendly error messages
5. **Cross-platform** - Support both browser and Node.js environments

## Project Structure

### Organization
```
rfin-wasm/
├── src/
│   ├── lib.rs        # Module initialization and re-exports
│   ├── currency.rs   # Currency type bindings
│   ├── money.rs      # Money type bindings
│   ├── dates.rs      # Date type bindings
│   ├── calendar.rs   # Calendar bindings
│   ├── cashflow.rs   # CashFlow bindings
│   ├── schedule.rs   # Schedule generation
│   └── utils.rs      # WASM utilities (panic hook, etc.)
├── pkg/              # Generated web package
├── pkg-node/         # Generated Node.js package
└── tests/           # WASM tests
    └── web.rs
```

### Build Targets
```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]  # Required for WASM

[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"  # For complex object serialization
```

## Type Wrapping Patterns

### Basic Type Wrapper
```rust
use wasm_bindgen::prelude::*;
use rfin_core::TypeName as CoreType;

/// JavaScript-visible documentation
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct TypeName {
    inner: CoreType,
}

#[wasm_bindgen]
impl TypeName {
    /// Constructor - always use #[wasm_bindgen(constructor)]
    #[wasm_bindgen(constructor)]
    pub fn new(param: String) -> Result<TypeName, JsValue> {
        let inner = param
            .parse::<CoreType>()
            .map_err(|e| JsValue::from_str(&format!("Error: {}", e)))?;
        Ok(TypeName { inner })
    }
}
```

### Property Getters
```rust
#[wasm_bindgen]
impl TypeName {
    /// Use getter for properties
    #[wasm_bindgen(getter)]
    pub fn property(&self) -> String {
        self.inner.property().to_string()
    }
    
    /// Use js_name for JavaScript naming conventions
    #[wasm_bindgen(getter, js_name = "numericCode")]
    pub fn numeric_code(&self) -> u32 {
        self.inner.code()
    }
}
```

### Methods
```rust
#[wasm_bindgen]
impl TypeName {
    /// Regular methods
    #[wasm_bindgen]
    pub fn calculate(&self, value: f64) -> f64 {
        self.inner.calculate(value)
    }
    
    /// Methods with JS-friendly names
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{}", self.inner)
    }
    
    /// Equality checks (JavaScript doesn't have operator overloading)
    #[wasm_bindgen]
    pub fn equals(&self, other: &TypeName) -> bool {
        self.inner == other.inner
    }
}
```

## Error Handling

### Convert Rust Errors to JavaScript
```rust
use rfin_core::error::{Error, InputError};

fn convert_error(err: Error) -> JsValue {
    match err {
        Error::Input(InputError::InvalidDateRange) => {
            JsValue::from_str("Invalid date range: start must be before end")
        }
        Error::CurrencyMismatch { expected, actual } => {
            JsValue::from_str(&format!(
                "Currency mismatch: expected {}, got {}", 
                expected, actual
            ))
        }
        Error::InterpOutOfBounds => {
            JsValue::from_str("Interpolation input out of bounds")
        }
        _ => JsValue::from_str(&format!("Operation failed: {}", err))
    }
}
```

### Method Error Handling
```rust
#[wasm_bindgen]
impl Money {
    #[wasm_bindgen]
    pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
        self.inner
            .checked_add(other.inner)
            .map(|result| Money { inner: result })
            .map_err(convert_error)
    }
}
```

## Enum Handling

### Simple Enums
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub enum Frequency {
    Annual = "Annual",
    SemiAnnual = "SemiAnnual",
    Quarterly = "Quarterly",
    Monthly = "Monthly",
}

// Conversion to core enum
impl Into<CoreFrequency> for Frequency {
    fn into(self) -> CoreFrequency {
        match self {
            Frequency::Annual => CoreFrequency::annual(),
            Frequency::SemiAnnual => CoreFrequency::semi_annual(),
            Frequency::Quarterly => CoreFrequency::quarterly(),
            Frequency::Monthly => CoreFrequency::monthly(),
        }
    }
}
```

### Static Enum Methods
```rust
#[wasm_bindgen]
impl DayCount {
    #[wasm_bindgen(js_name = "Act360")]
    pub fn act360() -> DayCount {
        DayCount { inner: CoreDayCount::Act360 }
    }
    
    #[wasm_bindgen(js_name = "Act365F")]
    pub fn act365f() -> DayCount {
        DayCount { inner: CoreDayCount::Act365F }
    }
}
```

## Complex Type Serialization

### Using Serde for Complex Objects
```rust
use serde::{Serialize, Deserialize};
use serde_wasm_bindgen::{to_value, from_value};

#[derive(Serialize, Deserialize)]
pub struct CashFlowData {
    date: String,
    amount: f64,
    currency: String,
    kind: String,
}

#[wasm_bindgen]
impl FixedRateLeg {
    /// Return cash flows as JavaScript array
    #[wasm_bindgen(js_name = "getCashFlows")]
    pub fn get_cash_flows(&self) -> Result<JsValue, JsValue> {
        let flows: Vec<CashFlowData> = self.inner
            .flows()
            .iter()
            .map(|cf| CashFlowData {
                date: cf.date.to_string(),
                amount: cf.amount.amount(),
                currency: cf.amount.currency().to_string(),
                kind: format!("{:?}", cf.kind),
            })
            .collect();
        
        to_value(&flows).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

## Module Initialization

### lib.rs Pattern
```rust
use wasm_bindgen::prelude::*;

mod utils;

/// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

// Re-export all public types
pub use currency::Currency;
pub use money::Money;
pub use dates::Date;

// Re-export functions with JS-friendly names
pub use dates::{
    third_wednesday as thirdWednesday,
    next_imm as nextImm,
    next_cds_date as nextCdsDate,
};
```

### utils.rs Pattern
```rust
pub fn set_panic_hook() {
    // Only include panic hook in debug builds
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
```

## Memory Management

### Avoid Unnecessary Clones
```rust
#[wasm_bindgen]
impl Currency {
    // Good: Return lightweight copy
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        format!("{}", self.inner)
    }
    
    // Avoid: Returning references (not supported by wasm-bindgen)
    // pub fn code(&self) -> &str { ... }
}
```

### Handle Collections Efficiently
```rust
use js_sys::Array;

#[wasm_bindgen]
pub fn generate_dates(start: &Date, end: &Date, freq: Frequency) -> Array {
    let dates = generate_schedule_internal(start, end, freq);
    
    let array = Array::new();
    for date in dates {
        array.push(&JsValue::from(Date::from_inner(date)));
    }
    array
}
```

## Testing

### WASM Test Structure
```rust
// tests/web.rs
use wasm_bindgen_test::*;
use rfin_wasm::{Currency, Money};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_currency_creation() {
    let currency = Currency::new("USD".to_string()).unwrap();
    assert_eq!(currency.code(), "USD");
    assert_eq!(currency.numeric_code(), 840);
}

#[wasm_bindgen_test]
fn test_money_arithmetic() {
    let usd = Currency::new("USD".to_string()).unwrap();
    let m1 = Money::new(100.0, usd.clone());
    let m2 = Money::new(50.0, usd);
    
    let result = m1.add(&m2).unwrap();
    assert_eq!(result.amount(), 150.0);
}
```

## JavaScript API Design

### Constructor Pattern
```rust
// Always provide constructors for main types
#[wasm_bindgen(constructor)]
pub fn new(/* params */) -> Result<Self, JsValue> {
    // Implementation
}
```

### Method Naming
```rust
// Use JavaScript conventions
#[wasm_bindgen(js_name = "toString")]
pub fn to_string_js(&self) -> String { }

#[wasm_bindgen(js_name = "valueOf")]
pub fn value_of(&self) -> f64 { }

#[wasm_bindgen(js_name = "toJSON")]
pub fn to_json(&self) -> Result<JsValue, JsValue> { }
```

### Optional Parameters
```rust
use wasm_bindgen::JsValue;

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen]
    pub fn calculate(&self, value: f64, options: Option<JsValue>) -> Result<f64, JsValue> {
        let params = if let Some(opts) = options {
            // Parse options object
            parse_options(opts)?
        } else {
            // Use defaults
            Default::default()
        };
        
        Ok(self.inner.calculate(value, params))
    }
}
```

## Documentation

### Type Documentation
```rust
/// Currency representation based on ISO 4217 standards.
/// 
/// A Currency represents a specific currency using the ISO 4217 standard.
/// 
/// @example
/// ```javascript
/// const usd = new Currency("USD");
/// console.log(usd.code); // "USD"
/// console.log(usd.numericCode); // 840
/// ```
#[wasm_bindgen]
pub struct Currency {
    // ...
}
```

### Method Documentation
```rust
/// Add two money values.
/// 
/// Both money values must have the same currency.
/// 
/// @param {Money} other - The money value to add
/// @returns {Money} The sum of the two money values
/// @throws {Error} If the currencies don't match
#[wasm_bindgen]
pub fn add(&self, other: &Money) -> Result<Money, JsValue> {
    // ...
}
```

## Build Configuration

### wasm-pack Settings
```toml
# Cargo.toml
[package.metadata.wasm-pack]
"wasm-pack-plugin" = "0.1"

[features]
# Include panic hook only in debug builds
default = ["console_error_panic_hook"]
console_error_panic_hook = ["dep:console_error_panic_hook"]

# Pass through features to core
decimal128 = ["rfin-core/decimal128"]
```

### Build Scripts
```json
// package.json
{
  "scripts": {
    "build": "wasm-pack build --target web --out-dir pkg",
    "build:node": "wasm-pack build --target nodejs --out-dir pkg-node",
    "build:bundler": "wasm-pack build --target bundler --out-dir pkg-bundler",
    "test": "wasm-pack test --chrome --firefox --headless"
  }
}
```

## Performance Guidelines

### Minimize Boundary Crossings
```rust
// Good: Batch operations
#[wasm_bindgen]
pub fn calculate_multiple(values: &[f64]) -> Vec<f64> {
    values.iter().map(|&v| self.calculate(v)).collect()
}

// Avoid: Multiple individual calls from JavaScript
```

### Use References Where Possible
```rust
// Good: Accept references
#[wasm_bindgen]
pub fn compare(&self, other: &Money) -> bool {
    self.inner == other.inner
}

// Avoid: Unnecessary ownership transfer
pub fn compare(self, other: Money) -> bool { }
```

## Debugging Support

### Console Logging
```rust
use web_sys::console;

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen]
    pub fn debug_state(&self) {
        console::log_1(&format!("State: {:?}", self.inner).into());
    }
}
```

### Development Features
```rust
#[cfg(debug_assertions)]
#[wasm_bindgen]
impl TypeName {
    #[wasm_bindgen(js_name = "_debug")]
    pub fn debug(&self) -> String {
        format!("{:?}", self.inner)
    }
}
```