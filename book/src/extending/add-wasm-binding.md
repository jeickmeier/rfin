# Add a WASM Binding

> **Note:** This guide will be expanded as the WASM binding patterns mature. The
> general approach mirrors the Python binding guide: thin wrapper, type
> conversion, error mapping.

## Step 1: Create the Function

Add to the appropriate module in `finstack-wasm/src/`:

```rust,no_run
use wasm_bindgen::prelude::*;
use finstack_valuations::MyDerivative;

/// Create a new MyDerivative from JSON parameters.
#[wasm_bindgen]
pub fn create_my_derivative(
    id: &str,
    notional: f64,
    currency: &str,
    disc_id: &str,
) -> Result<JsValue, JsError> {
    let inst = MyDerivative::builder(id)
        .notional(Money::new(notional, currency.parse()?))
        .disc_id(disc_id)
        .build()
        .map_err(|e| JsError::new(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&inst)
        .map_err(|e| JsError::new(&e.to_string()))
}
```

## Step 2: TypeScript Types

Add to `finstack-wasm/types/`:

```typescript
export function create_my_derivative(
    id: string,
    notional: number,
    currency: string,
    disc_id: string,
): MyDerivative;
```

## Step 3: Test

Add a test in `finstack-wasm/tests/`:

```javascript
const { create_my_derivative, price } = require('finstack-wasm');

test('MyDerivative prices correctly', () => {
    const inst = create_my_derivative('TEST', 1000000, 'USD', 'USD-OIS');
    const result = price(inst, market, asOf);
    expect(result.npv).not.toBe(0);
});
```

## Key Differences from Python

| Aspect | Python (PyO3) | WASM (wasm-bindgen) |
|--------|--------------|--------------------|
| API style | Class-based | Function-based (flat) |
| Complex types | Native Python objects | JSON via `JsValue` |
| Errors | Python exceptions | `JsError` |
| Builder | Method chaining | Parameters in constructor |
| Type safety | `.pyi` stubs | TypeScript `.d.ts` |
