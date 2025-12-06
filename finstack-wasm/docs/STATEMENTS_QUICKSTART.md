# WASM Statements Bindings - Quick Start Guide

## Overview

The statements bindings provide a complete JavaScript/TypeScript API for building and evaluating financial statement models with formulas, forecasts, and dynamic metrics.

## Installation

### For Use in Your Project

```bash
# Copy the pkg directory from finstack-wasm/pkg to your project
cp -r finstack-wasm/pkg your-project/node_modules/finstack-wasm

# Or if using from the built package
npm install ./finstack-wasm/pkg
```

### For Development

```bash
cd finstack-wasm

# Build the WASM package
wasm-pack build --target web --out-dir pkg --release

# Run tests
wasm-pack test --headless --chrome

# Run examples
cd examples
npm install
npm run dev
```

## Running Tests

### Browser Tests

```bash
cd finstack-wasm

# Run all tests
wasm-pack test --headless --chrome

# Run tests in Firefox
wasm-pack test --headless --firefox

# Run tests with output
wasm-pack test --headless --chrome -- --nocapture
```

### Specific Test File

```bash
# Run only statements tests
wasm-pack test --headless --chrome --test statements_tests
```

## Running Examples

### Start Development Server

```bash
cd finstack-wasm/examples
npm install
npm run dev
```

Then open http://localhost:5173 in your browser.

### Navigate to Statements Example

1. Click "Examples" in the sidebar
2. Find "Statements" group
3. Click "Financial Statements Modeling"

Or navigate directly to: http://localhost:5173/example/statements-modeling

## Basic Usage

### 1. Import the Library

```typescript
import init, {
  ModelBuilder,
  Evaluator,
  ForecastSpec,
  Registry,
  AmountOrScalar,
  Currency,
} from './pkg/finstack_wasm.js';

// Initialize WASM (required)
await init();
```

### 2. Build a Simple Model

```typescript
// Create builder
const builder = new ModelBuilder('Simple P&L');

// Define periods
builder.periods('2025Q1..Q4', '2025Q1');

// Add revenue with actual value
const revenue = { '2025Q1': 1000000 };
builder.value('revenue', revenue);

// Add formulas
builder.compute('cogs', 'revenue * 0.6');
builder.compute('gross_profit', 'revenue - cogs');
builder.compute('gross_margin', 'gross_profit / revenue');

// Build the model
const model = builder.build();
```

### 3. Evaluate the Model

```typescript
// Create evaluator
const evaluator = new Evaluator();

// Evaluate model
const results = evaluator.evaluate(model);

// Access results
const q1Revenue = results.get('revenue', '2025Q1');
const q1Margin = results.get('gross_margin', '2025Q1');

console.log(`Q1 Revenue: $${q1Revenue}`);
console.log(`Q1 Margin: ${(q1Margin * 100).toFixed(1)}%`);
```

### 4. Add Forecasts

```typescript
// Growth forecast (5% annual)
const growthForecast = ForecastSpec.growth(0.05);
builder.forecast('revenue', growthForecast);

// Curve forecast (different rate per period)
const curveForecast = ForecastSpec.curve([0.02, 0.03, 0.04]);
builder.forecast('expenses', curveForecast);

// Normal distribution forecast (deterministic)
const normalForecast = ForecastSpec.normal(100000, 10000, 12345);
builder.forecast('volatility', normalForecast);
```

### 5. Use the Registry

```typescript
// Create registry
const registry = new Registry();

// Load built-in metrics
registry.loadBuiltins();

// List all metrics
const allMetrics = registry.listMetrics();
console.log(`Total metrics: ${allMetrics.length}`);

// List metrics in a namespace
const finMetrics = registry.listMetrics('fin');
console.log(`Finance metrics: ${finMetrics.length}`);

// Get metric details
const grossMargin = registry.get('fin.gross_margin');
console.log(`Formula: ${grossMargin.formula()}`);
```

## API Quick Reference

### ModelBuilder

```typescript
const builder = new ModelBuilder(id: string);

// Define periods
builder.periods(range: string, actuals_until?: string);

// Add nodes
builder.value(nodeId: string, values: { [period: string]: number });
builder.compute(nodeId: string, formula: string);
builder.forecast(nodeId: string, forecastSpec: ForecastSpec);

// Metadata
builder.withMeta(key: string, value: any);

// Build
const model = builder.build();
```

### Evaluator

```typescript
const evaluator = new Evaluator();

// Basic evaluation
const results = evaluator.evaluate(model);

// With market context
const results = evaluator.evaluateWithMarketContext(model, marketContext, asOfDate);
```

### Results

```typescript
// Get single value
const value = results.get(nodeId: string, periodId: string);

// Get all periods for a node
const nodeValues = results.getNode(nodeId: string);

// Get with default
const value = results.getOr(nodeId: string, periodId: string, default: number);

// Get all periods as array
const periods = results.allPeriods(nodeId: string);

// Get all nodes
const allNodes = results.nodes;

// Get metadata
const meta = results.meta();
console.log(meta.numNodes());
console.log(meta.numPeriods());
console.log(meta.evalTimeMs());
```

### ForecastSpec

```typescript
// Forward fill
const ff = ForecastSpec.forwardFill();

// Growth percentage
const growth = ForecastSpec.growth(rate: number);

// Curve (different rates per period)
const curve = ForecastSpec.curve(rates: number[]);

// Normal distribution
const normal = ForecastSpec.normal(mean: number, stdDev: number, seed: number);

// Log-normal distribution
const lognormal = ForecastSpec.lognormal(mean: number, stdDev: number, seed: number);
```

### Registry

```typescript
const registry = new Registry();

// Load built-ins
registry.loadBuiltins();

// Load from JSON
const metricRegistry = registry.loadFromJsonStr(jsonString);

// Get metric
const metric = registry.get(metricId: string);

// List metrics
const all = registry.listMetrics();
const namespaced = registry.listMetrics(namespace: string);

// Check existence
const exists = registry.hasMetric(metricId: string);

// Count
const count = registry.metricCount();
```

### AmountOrScalar

```typescript
// Scalar (dimensionless)
const scalar = AmountOrScalar.scalar(100.0);

// Currency amount
const currency = new Currency('USD');
const amount = AmountOrScalar.amount(100.0, currency);

// Check type
scalar.isScalar(); // true
amount.isAmount(); // true

// Get value
const value = scalar.getValue();

// Get currency (if amount)
const curr = amount.getCurrency();
```

## Formula DSL Reference

### Operators

```
+   Addition
-   Subtraction
*   Multiplication
/   Division
^   Power
()  Grouping
```

### Functions

```typescript
// Basic math
abs(x), sqrt(x), exp(x), ln(x), log10(x)
sin(x), cos(x), tan(x)
min(x, y), max(x, y)

// Time series
lag(x, n)         // Value from n periods ago
lead(x, n)        // Value from n periods ahead
diff(x)           // Difference from previous period
pct_change(x)     // Percentage change

// Rolling windows
rolling_mean(x, n)
rolling_sum(x, n)
rolling_std(x, n)
rolling_min(x, n)
rolling_max(x, n)

// Aggregates
sum(x1, x2, ...)
mean(x1, x2, ...)

// Financial
ttm(x)            // Trailing twelve months
annualize(x, n)   // Annualize based on periods
coalesce(x, y)    // First non-null value
```

### Example Formulas

```typescript
// Simple arithmetic
'revenue - expenses';

// Ratios
'gross_profit / revenue';

// Growth
'(revenue - lag(revenue, 1)) / lag(revenue, 1)';

// Trailing twelve months
'ttm(revenue)';

// Rolling average
'rolling_mean(revenue, 4)';

// Complex
'max(0, revenue * 0.4 - opex)';
```

## Common Patterns

### Building Multi-Period Models

```typescript
const builder = new ModelBuilder('Multi-Period');
builder.periods('2024Q1..2025Q4', '2024Q4');

// Historical revenue
const historical = {
  '2024Q1': 900000,
  '2024Q2': 950000,
  '2024Q3': 975000,
  '2024Q4': 1000000,
};
builder.value('revenue', historical);

// Forecast growth
const forecast = ForecastSpec.growth(0.08);
builder.forecast('revenue', forecast);
```

### Calculating Margins

```typescript
builder.compute('revenue', '...');
builder.compute('cogs', 'revenue * 0.6');
builder.compute('gross_profit', 'revenue - cogs');
builder.compute('gross_margin', 'gross_profit / revenue');

builder.compute('opex', '...');
builder.compute('ebitda', 'gross_profit - opex');
builder.compute('ebitda_margin', 'ebitda / revenue');
```

### Using Registry Metrics

```typescript
// Load registry
const registry = new Registry();
registry.loadBuiltins();

// Use metric formulas
const grossMarginMetric = registry.get('fin.gross_margin');
builder.compute('gross_margin', grossMarginMetric.formula());

const opMarginMetric = registry.get('fin.operating_margin');
builder.compute('op_margin', opMarginMetric.formula());
```

### Error Handling

```typescript
try {
  const builder = new ModelBuilder('MyModel');
  builder.periods('2025Q1..Q4');
  builder.compute('revenue', 'invalid * formula');
  const model = builder.build();
} catch (error) {
  console.error('Model build failed:', error);
  // Handle error appropriately
}

try {
  const results = evaluator.evaluate(model);
  const value = results.get('nonexistent', '2025Q1');
  if (value === null) {
    console.log('Value not found');
  }
} catch (error) {
  console.error('Evaluation failed:', error);
}
```

## Troubleshooting

### Common Issues

**Issue**: "Cannot find module 'finstack-wasm'"
**Solution**: Ensure you've built the package with `wasm-pack build`

**Issue**: "RuntimeError: unreachable executed"
**Solution**: Make sure to `await init()` before using any types

**Issue**: "Invalid period ID"
**Solution**: Use correct format: "2025Q1", "2025M01", etc.

**Issue**: "Currency mismatch"
**Solution**: Ensure all amounts use the same currency or use explicit conversion

### Debug Tips

```typescript
// Enable detailed logging
console.log('Model info:', {
  id: model.id(),
  periods: model.periodCount(),
  nodes: model.nodeCount(),
});

// Check results metadata
const meta = results.meta();
console.log('Evaluation:', {
  nodes: meta.numNodes(),
  periods: meta.numPeriods(),
  time: meta.evalTimeMs(),
});

// Inspect all node results
const allResults = results.nodes;
console.log('All results:', allResults);
```

## Performance Tips

1. **Reuse objects**: Create Currency instances once and reuse them
2. **Batch operations**: Build model with all nodes before evaluating
3. **Cache results**: Store evaluation results if evaluating multiple times
4. **Use appropriate data types**: Use scalars for dimensionless values

## Additional Resources

- **Live Examples**: http://localhost:5173/example/statements-modeling
- **TypeScript Definitions**: `pkg/finstack_wasm.d.ts`
- **Test Suite**: `tests/statements_tests.rs`
- **Documentation**: See inline JSDoc in TypeScript definitions

## Support

For issues or questions:

1. Check the test suite for usage examples
2. Review the live web examples
3. Inspect TypeScript definitions for API details
4. Refer to the Rust documentation for core concepts
