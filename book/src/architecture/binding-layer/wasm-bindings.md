# WASM Bindings

The `finstack-wasm` crate provides WebAssembly bindings via `wasm-bindgen`,
enabling Finstack to run in browsers and Node.js.

## Architecture

WASM bindings use a flat API (no class hierarchy) due to wasm-bindgen
limitations:

```typescript
// TypeScript
import { create_discount_curve, df, forward_rate } from 'finstack-wasm';

const curve = create_discount_curve('USD-OIS', baseDate, knots);
const discount = df(curve, 5.0);          // DF at 5Y
const fwd = forward_rate(curve, 5.0, 10.0); // 5Y-10Y forward
```

## JsValue Conversion

Complex types are passed as JSON and deserialized on the Rust side:

```rust,no_run
#[wasm_bindgen]
pub fn create_discount_curve(
    id: &str,
    base_date: &str,  // ISO 8601 date string
    knots: JsValue,   // JSON array of [time, df] pairs
) -> Result<JsValue, JsError> {
    let knots: Vec<(f64, f64)> = serde_wasm_bindgen::from_value(knots)?;
    // ...
}
```

## TypeScript Types

Type definitions are auto-generated in `finstack-wasm/types/` and bundled
with the npm package. The `index.d.ts` file provides full TypeScript
type safety.

## Error Handling

Rust errors are converted to `JsError` for JavaScript consumption:

```typescript
try {
    const result = price_bond(instrument, market, asOf);
} catch (e) {
    console.error(e.message);  // Meaningful error from Rust
}
```

## Browser vs Node.js

| Environment | Initialization | Notes |
|-------------|---------------|-------|
| Browser | `await init()` | Loads .wasm file asynchronously |
| Node.js | `require('finstack-wasm')` | Synchronous loading |
| Bundlers | Automatic via webpack/vite plugin | Zero-config with wasm-pack |

## Bundle Size

The WASM binary is optimized for size:
- `wasm-opt -Os` for size optimization
- Tree-shaking removes unused functions
- Gzip compression typically achieves ~50% reduction

## Feature Parity

WASM bindings expose the same functionality as Python bindings. The
binding layer architecture ensures that any instrument or metric available
in Python is also available in WASM.
