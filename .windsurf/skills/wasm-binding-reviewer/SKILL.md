---
name: wasm-binding-reviewer
description: Review WASM binding code for architecture compliance, logic leakage, type conversion correctness, and pattern consistency. Use when reviewing WASM bindings in finstack-wasm/, examining wasm-bindgen code, checking WASM/JS interop, or when the user asks to review WASM binding code.
---

# WASM Binding Code Reviewer

## Quick Start

When reviewing WASM binding code in `finstack-wasm/`, produce a review that ensures:

1. **Summary**: What changed, risk of logic leaking into WASM bindings.
2. **Binding concerns**: 3–7 bullets on architecture violations, type conversion issues, and pattern consistency.
3. **Findings**: Grouped by severity with concrete fixes.
4. **Action items**: Checklist with specific improvements.

After each review cycle, re-check and update. Continue until no action items remain.

## Core Principle

**All logic stays in Rust core crates. WASM bindings do only:**
- Type conversion (JS → Rust, Rust → JS)
- Wrapper construction and accessor methods
- Error mapping to `JsValue` errors with `Error.name` taxonomy
- Serde bridges for structured data (`serde_wasm_bindgen`)

**Why?** The Python bindings expose identical Rust functionality. Any logic in WASM must be reimplemented for Python, causing drift and bugs.

## Severity Rubric

| Severity | Definition |
|----------|------------|
| **Blocker** | Business logic in WASM, computation outside Rust, algorithm implementation in bindings |
| **Major** | Validation logic that should be in Rust, inconsistent patterns, missing error mapping, unsafe code without justification |
| **Minor** | Suboptimal type conversion, missing TypeScript types, inconsistent naming |
| **Nit** | Style preference, minor documentation improvements |

## What Belongs in WASM Bindings

### Acceptable (GOOD)

```rust
// Type wrapper with inner field
#[wasm_bindgen(js_name = Money)]
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

// Accessor methods - just expose Rust data
#[wasm_bindgen(js_class = Money)]
impl JsMoney {
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.inner.amount()
    }
}

// Error mapping to JsValue with named errors
pub(crate) fn core_to_js(err: Error) -> JsValue {
    let error = js_sys::Error::new(&err.to_string());
    let _ = js_sys::Reflect::set(&error, &"name".into(), &kind.js_name().into());
    JsValue::from(error)
}

// Serde bridge for structured data
pub fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize: {}", e)))
}
```

### NOT Acceptable (BAD)

```rust
// BAD: Business logic in binding
#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    pub fn calculate_yield(&self, price: f64) -> Result<f64, JsValue> {
        // Newton-Raphson iteration - THIS SHOULD BE IN RUST CORE
        let mut y = 0.05;
        for _ in 0..100 {
            let f = self.price_at_yield(y) - price;
            let df = self.price_sensitivity(y);
            y = y - f / df;
        }
        Ok(y)
    }
}

// BAD: Validation logic that should be in Rust
#[wasm_bindgen(js_class = Swap)]
impl JsSwap {
    #[wasm_bindgen(constructor)]
    pub fn new(notional: f64, rate: f64) -> Result<JsSwap, JsValue> {
        if notional <= 0.0 {
            return Err(JsValue::from_str("Notional must be positive"));
        }
        // ...
    }
}

// BAD: Data transformation logic
#[wasm_bindgen(js_class = Portfolio)]
impl JsPortfolio {
    pub fn aggregate_by_currency(&self) -> js_sys::Map {
        let map = js_sys::Map::new();
        for pos in &self.positions {
            // Aggregation logic should be in Rust
            // ...
        }
        map
    }
}
```

## Review Checklist

### Architecture Compliance

- [ ] No algorithms or computations implemented in bindings
- [ ] No business logic or financial calculations
- [ ] No validation logic (beyond type conversion)
- [ ] All computation delegated to Rust core crates
- [ ] Pattern consistency with Python bindings

### Wrapper Pattern

- [ ] Uses `inner: RustType` field (not `pub`, typically private or `pub(crate)`)
- [ ] Provides `pub(crate) fn from_inner()` constructor
- [ ] Provides `pub(crate) fn inner()` accessor
- [ ] Uses `#[wasm_bindgen(js_name = ...)]` with camelCase JS names
- [ ] Uses `#[wasm_bindgen(js_class = ...)]` on impl blocks
- [ ] Accessor methods use `#[wasm_bindgen(getter)]`

### Type Conversion

- [ ] Primitives handled by wasm-bindgen (`f64`, `String`, `bool`, `Option`)
- [ ] Structured data uses `serde_wasm_bindgen::to_value`/`from_value`
- [ ] Collections mapped via `js_sys::Array`, `js_sys::Map` as appropriate
- [ ] Uses centralized `utils/json.rs` helpers for serde bridges
- [ ] Clear error messages on deserialization failure

### Error Handling

- [ ] Returns `Result<T, JsValue>` for fallible operations
- [ ] Uses `core_to_js()` from `core/error.rs` for core errors
- [ ] Sets `Error.name` for JS error taxonomy (`InputError`, `ValidationError`, etc.)
- [ ] No raw `JsValue::from_str()` for errors that should use the taxonomy
- [ ] No `.unwrap()` or `.expect()` in non-test binding code

### Unsafe Code

- [ ] `#![deny(unsafe_code)]` at crate root
- [ ] Any `#[allow(unsafe_code)]` is narrowly scoped and documented
- [ ] `extract_instrument` usage tied to pinned `wasm-bindgen` version
- [ ] No new `unsafe` blocks without clear justification

### Module Organization

- [ ] Types exported via `pub use` in `lib.rs`
- [ ] JS names use camelCase convention
- [ ] Mirrors Rust core crate structure in `src/` subdirectories

### TypeScript Types

- [ ] `pkg/*.d.ts` accurately reflects the API
- [ ] Complex types have `ts-rs` derive where appropriate
- [ ] `types/generated/*.ts` kept current for wire types

## Red Flags

| Issue | Symptom | Fix |
|-------|---------|-----|
| Logic in binding | Loop/iteration computing values | Move to Rust core, call from binding |
| Validation in binding | `if value < X` checks with custom errors | Add validation to Rust constructor |
| Data transformation | Map/filter/reduce on collections | Create Rust method, wrap result |
| Algorithm implementation | Math operations, financial formulas | Must be in Rust core crate |
| Raw string errors | `JsValue::from_str("...")` for domain errors | Use `js_error_with_kind()` taxonomy |
| Unnecessary unsafe | `unsafe` block without clear necessity | Remove or justify with documentation |
| Missing serde bridge | Manual JS object construction for DTOs | Use `serde_wasm_bindgen::to_value` |
| Inconsistent naming | Mix of camelCase/snake_case in JS API | Use `js_name` for camelCase |

## Python Parity Check

When reviewing WASM bindings, verify the functionality mirrors the Python bindings:

```rust
// WASM binding
#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    #[wasm_bindgen(js_name = dirtyPrice)]
    pub fn dirty_price(&self, market_data: &JsMarketData) -> Result<f64, JsValue> {
        self.inner.dirty_price(&market_data.inner).map_err(core_to_js)
    }
}

// Python equivalent should be straightforward:
#[pymethods]
impl PyBond {
    fn dirty_price(&self, market_data: &PyMarketData) -> PyResult<f64> {
        self.inner.dirty_price(&market_data.inner).map_err(map_error)
    }
}
```

If the WASM implementation can't be trivially replicated in Python, it's a sign that logic has leaked into the binding layer.

## Review Output Template

```markdown
## Summary
<1–3 bullets: what changed, risk of logic leakage>

## Binding Concerns
- <architecture violation or pattern issue>
- <type conversion concern>

## Findings

### Blockers
- <logic in WASM> (move to Rust: specific location suggestion)

### Majors
- <pattern inconsistency> (align with: specific example file)

### Minors / Nits
- <documentation improvement>

## Action Items
- [ ] <specific code to move to Rust>
- [ ] <pattern to align>
- [ ] <test to add>
```

## Additional Resources

- For detailed patterns and examples, see [reference.md](reference.md)
- For common issues and fixes, see [examples.md](examples.md)
