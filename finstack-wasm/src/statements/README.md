# Statements WASM Bindings

## Overview

This module provides JavaScript/TypeScript bindings for the `finstack-statements` crate, enabling financial statement modeling with deterministic evaluation, currency-safe arithmetic, and support for forecasting methods.

## Module Structure

```
statements/
├── mod.rs              # Module exports
├── types/
│   ├── node.rs        # NodeSpec, NodeType
│   ├── forecast.rs    # ForecastSpec, ForecastMethod, SeasonalMode
│   ├── value.rs       # AmountOrScalar
│   └── model.rs       # FinancialModelSpec, CapitalStructureSpec
├── builder.rs         # ModelBuilder (fluent API)
├── evaluator.rs       # Evaluator, Results, ResultsMeta
├── extensions.rs      # Extension system
└── registry.rs        # Dynamic metric registry
```

## Exposed JavaScript API

### Builder (JsModelBuilder)
- `new(id: string)` - Create builder
- `periods(range: string, actualsUntil?: string)` - Define periods
- `value(nodeId: string, values: object)` - Add value node
- `compute(nodeId: string, formula: string)` - Add formula node
- `forecast(nodeId: string, spec: ForecastSpec)` - Add forecast
- `withMeta(key: string, value: any)` - Add metadata
- `build()` - Build model

### Evaluator (JsEvaluator)
- `new()` - Create evaluator
- `evaluate(model: FinancialModelSpec)` - Evaluate model
- `evaluateWithMarketContext(model, ctx, date)` - Evaluate with pricing

### Results (JsResults)
- `get(nodeId: string, periodId: string)` - Get single value
- `getNode(nodeId: string)` - Get all periods for node
- `getOr(nodeId, periodId, default)` - Get with default
- `allPeriods(nodeId)` - Get all periods as array
- `nodes` - All node results
- `meta()` - Evaluation metadata

### Forecast Specifications (JsForecastSpec)
- `forwardFill()` - Carry forward last value
- `growth(rate: number)` - Compound growth
- `curve(rates: number[])` - Period-specific rates
- `normal(mean, stdDev, seed)` - Normal distribution
- `lognormal(mean, stdDev, seed)` - Log-normal distribution

### Registry (JsRegistry)
- `new()` - Create registry
- `loadBuiltins()` - Load 22 built-in metrics
- `loadFromJsonStr(json)` - Load custom metrics
- `get(metricId)` - Get metric definition
- `listMetrics(namespace?)` - List metrics
- `hasMetric(metricId)` - Check existence

### Extensions
- `CorkscrewExtension` - Balance sheet validation
- `CreditScorecardExtension` - Credit rating assignment
- `ExtensionRegistry` - Extension management

## TypeScript Usage

```typescript
import {
  ModelBuilder,
  Evaluator,
  ForecastSpec,
  Registry,
  AmountOrScalar,
} from 'finstack-wasm';

// Build model
const builder = new ModelBuilder('P&L');
builder.periods('2025Q1..Q4', null);
builder.value('revenue', { '2025Q1': 1000000 });
builder.compute('margin', 'profit / revenue');
const model = builder.build();

// Evaluate
const evaluator = new Evaluator();
const results = evaluator.evaluate(model);
console.log(results.get('margin', '2025Q1'));
```

## Implementation Notes

### Type State Pattern
The Rust builder uses a type-state pattern (`NeedPeriods` → `Ready`), but WASM exposes a runtime-checked version since wasm-bindgen doesn't support type-state patterns cleanly.

### Period Parsing
Periods are parsed from strings using `PeriodId::from_str()`. Supported formats:
- Quarterly: `2025Q1`, `2025Q2`, etc.
- Monthly: `2025M01`, `2025M02`, etc.
- Annual: `2025`, `2026`, etc.

### Value Mapping
Values are passed as JavaScript objects mapping period IDs to numbers or `AmountOrScalar` instances:

```javascript
// Simple numbers (treated as scalars)
{ '2025Q1': 1000000, '2025Q2': 1100000 }

// Or explicit AmountOrScalar
{
  '2025Q1': AmountOrScalar.scalar(1000000),
  '2025Q2': AmountOrScalar.amount(1100000, currency)
}
```

### Error Handling
All Rust errors are converted to JavaScript exceptions with descriptive messages:

```javascript
try {
  const model = builder.build();
} catch (error) {
  console.error(`Build failed: ${error}`);
}
```

## Testing

Tests are located in `finstack-wasm/tests/statements_tests.rs`.

Run with:
```bash
cd finstack-wasm
wasm-pack test --headless --chrome --test statements_tests
```

## Examples

Interactive examples are in `finstack-wasm/examples/src/components/StatementsModeling.tsx`.

Run with:
```bash
cd finstack-wasm/examples
npm run dev
```

Then navigate to http://localhost:5173/example/statements-modeling

## See Also

- **Quick Start**: `finstack-wasm/STATEMENTS_QUICKSTART.md`
- **Implementation Summary**: `WASM_STATEMENTS_IMPLEMENTATION_COMPLETE.md`
- **Python Bindings**: `STATEMENTS_PYTHON_BINDINGS_COMPLETE.md`
- **Rust Core**: `finstack/statements/src/lib.rs`

