# Scenarios WASM Bindings - Quick Start

## Installation

The scenarios bindings are included in the standard finstack-wasm package:

```bash
npm install @finstack/wasm
# or
yarn add @finstack/wasm
```

## Basic Usage

```typescript
import init, {
  JsScenarioEngine,
  JsScenarioSpec,
  JsOperationSpec,
  JsExecutionContext,
  JsCurveKind,
  MarketContext,
  FinancialModelSpec,
  FsDate,
} from '@finstack/wasm';

// Initialize WASM module
await init();

// Create market context with a curve
const market = new MarketContext();
const baseDate = new FsDate(2025, 1, 1);

const curve = new DiscountCurve(
  "USD_SOFR",
  baseDate,
  new Float64Array([0.0, 1.0, 5.0]),
  new Float64Array([1.0, 0.98, 0.90]),
  "act_365f",
  "monotone_convex",
  "flat_forward",
  true
);
market.insertDiscount(curve);

// Create a financial model
const model = FinancialModelSpec.fromJSON({
  id: "test_model",
  periods: [...],
  nodes: [...]
});

// Create execution context
const context = new JsExecutionContext(market, model, baseDate);

// Define operations
const operations = [
  JsOperationSpec.curveParallelBp(
    JsCurveKind.DISCOUNT,
    "USD_SOFR",
    50.0  // +50bp shock
  ).toJSON(),
  JsOperationSpec.equityPricePct(["SPY"], -10.0).toJSON(),
];

// Create scenario
const scenario = JsScenarioSpec.fromJSON({
  id: "stress_test",
  name: "Q1 Stress Test",
  operations,
  priority: 0
});

// Apply scenario
const engine = new JsScenarioEngine();
const report = engine.apply(scenario, context);

console.log(`Applied ${report.operationsApplied} operations`);
console.log(`Warnings: ${report.warnings.length}`);
```

## Operation Types

### Market Data Shocks

```typescript
// FX Rate Shock
JsOperationSpec.marketFxPct(Currency.EUR, Currency.USD, 5.0);

// Equity Price Shock
JsOperationSpec.equityPricePct(['SPY', 'QQQ'], -10.0);

// Curve Parallel Shift
JsOperationSpec.curveParallelBp(JsCurveKind.DISCOUNT, 'USD_SOFR', 50.0);

// Curve Node Shock
JsOperationSpec.curveNodeBp(
  JsCurveKind.DISCOUNT,
  'USD_SOFR',
  [
    ['2Y', 25.0],
    ['10Y', -10.0],
  ],
  JsTenorMatchMode.INTERPOLATE
);

// Volatility Surface Shock (Parallel)
JsOperationSpec.volSurfaceParallelPct(JsVolSurfaceKind.EQUITY, 'SPX', 15.0);

// Volatility Surface Shock (Bucketed)
JsOperationSpec.volSurfaceBucketPct(
  JsVolSurfaceKind.EQUITY,
  'SPX',
  ['1M', '3M'], // Optional tenor filter
  new Float64Array([90.0, 100.0]), // Optional strike filter
  20.0
);

// Base Correlation Shock (Parallel)
JsOperationSpec.baseCorrParallelPts('CDX_IG', 0.05);

// Base Correlation Shock (Bucketed)
JsOperationSpec.baseCorrBucketPts(
  'CDX_IG',
  [300, 700], // Detachment points in bps
  null, // Optional maturities
  0.03
);
```

### Statement Shocks

```typescript
// Forecast Percent Change
JsOperationSpec.stmtForecastPercent('Revenue', -10.0);

// Forecast Value Assignment
JsOperationSpec.stmtForecastAssign('Capex', 1_000_000.0);
```

### Time Operations

```typescript
// Roll Forward with Carry
JsOperationSpec.timeRollForward(
  '1M', // Period: 1D, 1W, 1M, 1Y
  true // Apply shocks after roll
);
```

## Scenario Composition

Combine multiple scenarios with priority-based ordering:

```typescript
const s1 = JsScenarioSpec.fromJSON({
  id: 'base',
  operations: [JsOperationSpec.curveParallelBp(JsCurveKind.DISCOUNT, 'USD_SOFR', 25.0).toJSON()],
  priority: 0, // Runs first
});

const s2 = JsScenarioSpec.fromJSON({
  id: 'overlay',
  operations: [JsOperationSpec.equityPricePct(['SPY'], -10.0).toJSON()],
  priority: 1, // Runs second
});

const engine = new JsScenarioEngine();
const composed = engine.compose([s1.toJSON(), s2.toJSON()]);
const report = engine.apply(composed, context);
```

## JSON Serialization

All scenarios can be serialized for storage or transmission:

```typescript
// Create scenario
const scenario = JsScenarioSpec.fromJSON({
  id: 'my_scenario',
  name: 'Custom Scenario',
  operations: [
    {
      kind: 'curve_parallel_bp',
      curve_kind: 'discount',
      curve_id: 'USD_SOFR',
      bp: 50.0,
    },
  ],
  priority: 0,
});

// Export to JSON
const json = scenario.toJSON();

// Store or transmit
localStorage.setItem('scenario', JSON.stringify(json));

// Load later
const stored = JSON.parse(localStorage.getItem('scenario'));
const loaded = JsScenarioSpec.fromJSON(stored);
```

## Error Handling

```typescript
try {
  const report = engine.apply(scenario, context);
  console.log('Success:', report.operationsApplied);
} catch (error) {
  console.error('Scenario failed:', error.message);
}
```

## Live Example

See the interactive example at:

```
http://localhost:5173/examples/scenarios-stress-testing
```

Run the example app:

```bash
cd finstack-wasm/examples
npm install
npm run dev
```

## TypeScript Support

Full TypeScript definitions are auto-generated:

```typescript
import type {
  JsScenarioSpec,
  JsOperationSpec,
  JsApplicationReport,
  JsExecutionContext,
  JsScenarioEngine,
} from '@finstack/wasm';
```

## API Parity

The WASM bindings provide 100% API parity with the Rust implementation:

✅ All 14 operation types  
✅ Scenario composition  
✅ Priority-based ordering  
✅ JSON serialization  
✅ Error handling  
✅ Reproducible execution

## Further Documentation

- [Complete Bindings Guide](docs/SCENARIOS_BINDINGS.md)
- [Implementation Summary](docs/SCENARIOS_IMPLEMENTATION_SUMMARY.md)
- [Rust Documentation](../finstack/scenarios/README.md)
