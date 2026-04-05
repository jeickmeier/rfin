# Quick Start — WASM

> **Note:** WASM bindings are under active development. The API shown below
> reflects the current binding design and may change as the surface stabilizes.

## Installation

```bash
npm install @finstack/wasm
```

## Example: Price a Bond

```typescript
import {
  DiscountCurve,
  MarketContext,
  Bond,
  Money,
  standardRegistry,
} from "@finstack/wasm";

// 1. Build market data
const asOf = new Date(2024, 0, 15);
const curve = new DiscountCurve("USD-OIS", asOf, [
  [0.0, 1.0],
  [1.0, 0.9524],
  [5.0, 0.7835],
  [10.0, 0.6139],
]);

const market = new MarketContext();
market.insertDiscount(curve);

// 2. Build an instrument
const bond = Bond.fixed(
  "US-TREASURY-5Y",
  new Money(1_000_000, "USD"),
  0.045,       // coupon
  asOf,        // issue
  new Date(2029, 0, 15),  // maturity
  "USD-OIS",   // discount curve
);

// 3. Price it
const registry = standardRegistry();
const result = registry.priceWithMetrics(
  bond, market, asOf,
  ["dirty_price", "ytm", "dv01"],
);

console.log(`NPV:   ${result.value}`);
console.log(`YTM:   ${(result.metric("ytm") * 100).toFixed(2)}%`);
console.log(`DV01:  ${result.metric("dv01").toFixed(2)}`);
```

## API Conventions

The WASM bindings use **camelCase** for method names (matching JavaScript
conventions) and expose a flat module structure rather than the nested Python
hierarchy:

| Python | WASM (TypeScript) |
|--------|-------------------|
| `standard_registry()` | `standardRegistry()` |
| `price_with_metrics()` | `priceWithMetrics()` |
| `DiscountCurve.builder(...)` | `new DiscountCurve(...)` |

## Browser Usage

```html
<script type="module">
  import init, { DiscountCurve, Bond } from "@finstack/wasm";
  await init();
  // ... same API as above
</script>
```

## Limitations

- Memory-constrained: no portfolio-level valuation (use Python or Rust)
- Subset of instrument types registered by default
- No file I/O — all data must be passed in

See the [WASM Bindings architecture page](../architecture/binding-layer/wasm-bindings.md)
for the full binding design.
