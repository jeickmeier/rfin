# Migration Guide: Python ↔ TypeScript/WASM

This guide helps developers migrate code between the Python and TypeScript/WASM bindings.

## Overview

Both bindings provide access to the same finstack core functionality with idiomatic APIs for each language:

- **Python bindings:** PyO3-based extension module with Pythonic conventions
- **WASM bindings:** wasm-bindgen-based library with JavaScript/TypeScript conventions

**Migration effort:** Most code can be translated mechanically with naming convention changes.

## Quick Migration Checklist

When migrating from Python to TypeScript/WASM:

- [ ] Convert `snake_case` to `camelCase` for functions and methods
- [ ] Add `new` keyword for class constructors
- [ ] Change `date()` (stdlib) to `new FsDate()` or JavaScript `Date`
- [ ] Replace operator overloads with method calls
- [ ] Update module imports (nested → flat)
- [ ] Convert exception handling patterns
- [ ] Update type hints to TypeScript types

When migrating from TypeScript/WASM to Python:

- [ ] Convert `camelCase` to `snake_case` for functions and methods
- [ ] Remove `new` keyword from constructors
- [ ] Change `FsDate` to stdlib `date`
- [ ] Add operator overload support where available
- [ ] Update module imports (flat → nested)
- [ ] Convert error handling to Python exceptions
- [ ] Add type hints using Python syntax

## Common Migration Patterns

### 1. Creating a Bond and Pricing It

**Python:**
```python
from datetime import date
from finstack.core.currency import Currency
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.valuations.instruments import Bond
from finstack.valuations.pricer import PricerRegistry

# Create bond
bond = Bond.treasury(
    "US-BOND-1",
    1_000_000,
    "USD",
    coupon_rate=0.0375,
    maturity=date(2030, 6, 15),
    issue_date=date(2020, 6, 15)
)

# Create market data
discount_curve = DiscountCurve(
    "USD-OIS",
    date(2024, 1, 2),
    [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.75)],
    day_count="act_365f"
)

market = MarketContext()
market.insert_discount(discount_curve)

# Price bond
pricer = PricerRegistry.standard()
result = pricer.price(bond, market)

print(f"PV: ${result.present_value:,.2f}")
```

**TypeScript:**
```typescript
import {
  FsDate,
  Currency,
  DiscountCurve,
  MarketContext,
  Bond,
  createStandardRegistry
} from 'finstack-wasm';

// Create bond
const bond = Bond.treasury(
  "US-BOND-1",
  1_000_000,
  "USD",
  0.0375,  // couponRate (camelCase!)
  new FsDate(2030, 6, 15),  // maturity
  new FsDate(2020, 6, 15)   // issueDate
);

// Create market data
const discountCurve = new DiscountCurve(
  "USD-OIS",
  new FsDate(2024, 1, 2),
  [0.0, 1.0, 5.0, 10.0],        // times
  [1.0, 0.97, 0.85, 0.75],      // discount factors
  "act_365f"                     // dayCount
);

const market = new MarketContext();
market.insertDiscount(discountCurve);  // camelCase!

// Price bond
const pricer = createStandardRegistry();  // Different function name!
const result = pricer.price(bond, market);

console.log(`PV: $${result.presentValue.toLocaleString()}`);
```

**Key Differences:**
- Python uses nested imports, TypeScript uses flat imports
- TypeScript needs `new` keyword for constructors
- Python uses `date()`, TypeScript uses `FsDate()`
- Method names: `insert_discount()` vs `insertDiscount()`
- Registry creation: `PricerRegistry.standard()` vs `createStandardRegistry()`
- Result property: `present_value` vs `presentValue`

### 2. Curve Calibration

**Python:**
```python
from datetime import date
from finstack.core.market_data import DiscountCurve
from finstack.valuations.calibration import (
    DiscountCurveCalibrator,
    CalibrationConfig,
    RatesQuote,
    SolverKind
)

# Create calibrator
calibrator = DiscountCurveCalibrator("USD-OIS", date(2024, 1, 2), "USD")
config = (CalibrationConfig.multi_curve()
    .with_solver_kind(SolverKind.HYBRID)
    .with_max_iterations(50)
    .with_tolerance(1e-8))

calibrator = calibrator.with_config(config)

# Add quotes
quotes = [
    RatesQuote.deposit(date(2024, 4, 2), 0.045, "act_360"),
    RatesQuote.swap(
        date(2026, 1, 2),
        0.047,
        "annual",  # Fixed frequency
        "quarterly",  # Float frequency
        "30_360",  # Fixed day count
        "act_360",  # Float day count
        "USD-SOFR"
    ),
]

# Calibrate
try:
    curve, report = calibrator.calibrate(quotes, None)
    print(f"Success: {report.success}")
    print(f"Iterations: {report.iterations}")
    print(f"Max residual: {report.max_residual}")
except ValueError as e:
    print(f"Calibration failed: {e}")
```

**TypeScript:**
```typescript
import {
  FsDate,
  DiscountCurveCalibrator,
  CalibrationConfig,
  RatesQuote,
  SolverKind,
  Frequency
} from 'finstack-wasm';

// Create calibrator
let calibrator = new DiscountCurveCalibrator("USD-OIS", new FsDate(2024, 1, 2), "USD");
const config = CalibrationConfig.multiCurve()
    .withSolverKind(SolverKind.Hybrid())  // Note: method call!
    .withMaxIterations(50)
    .withTolerance(1e-8);

calibrator = calibrator.withConfig(config);

// Add quotes
const quotes = [
    RatesQuote.deposit(new FsDate(2024, 4, 2), 0.045, "act_360"),
    RatesQuote.swap(
        new FsDate(2026, 1, 2),
        0.047,
        Frequency.annual(),  // Fixed frequency (Note: method call!)
        Frequency.quarterly(),  // Float frequency
        "30_360",
        "act_360",
        "USD-SOFR"
    ),
];

// Calibrate
try {
    const [curve, report] = calibrator.calibrate(quotes, null);
    console.log(`Success: ${report.success}`);
    console.log(`Iterations: ${report.iterations}`);
    console.log(`Max residual: ${report.maxResidual}`);
} catch (e) {
    console.error(`Calibration failed: ${e}`);
}
```

**Key Differences:**
- Python uses snake_case methods, TypeScript uses camelCase
- Python uses string frequency, TypeScript uses `Frequency.annual()` enum methods
- Python unpacks tuple `curve, report = ...`, TypeScript uses destructuring `[curve, report] = ...`
- Python has typed exceptions, TypeScript catches generic `Error`
- SolverKind: `SolverKind.HYBRID` (Python) vs `SolverKind.Hybrid()` (TypeScript)

### 3. Statement Modeling with Forecasting

**Python:**
```python
from finstack.statements import ModelBuilder, Evaluator, ForecastSpec
from finstack.core.dates import PeriodId
from finstack.core.money import Money

# Build model
builder = ModelBuilder.new("Revenue Model")
builder.periods("2024Q1..Q4", "2024Q2")  # Q1-Q2 actual, Q3-Q4 forecast

# Add actuals
builder.value("revenue", [
    (PeriodId.quarter(2024, 1), Money.from_code(1_000_000, "USD")),
    (PeriodId.quarter(2024, 2), Money.from_code(1_100_000, "USD")),
])

# Add forecast
builder.forecast("revenue", ForecastSpec.growth(0.05))

# Add computed metrics
builder.compute("cogs", "revenue * 0.6")
builder.compute("gross_profit", "revenue - cogs")
builder.compute("margin", "gross_profit / revenue")

model = builder.build()

# Evaluate
evaluator = Evaluator.new()
results = evaluator.evaluate(model)

# Access results
q1 = PeriodId.quarter(2024, 1)
q3 = PeriodId.quarter(2024, 3)

print(f"Q1 Revenue: {results.get('revenue', q1)}")
print(f"Q3 Revenue (forecast): {results.get('revenue', q3)}")
print(f"Q3 Margin: {results.get('margin', q3):.1%}")
```

**TypeScript:**
```typescript
import {
  ModelBuilder,
  Evaluator,
  ForecastSpec,
  PeriodId,
  Money
} from 'finstack-wasm';

// Build model
const builder = ModelBuilder.new("Revenue Model");
builder.periods("2024Q1..Q4", "2024Q2");

// Add actuals
builder.value("revenue", [
    [PeriodId.quarter(2024, 1), Money.fromCode(1_000_000, "USD")],
    [PeriodId.quarter(2024, 2), Money.fromCode(1_100_000, "USD")],
]);

// Add forecast
builder.forecast("revenue", ForecastSpec.growth(0.05));

// Add computed metrics
builder.compute("cogs", "revenue * 0.6");
builder.compute("gross_profit", "revenue - cogs");
builder.compute("margin", "gross_profit / revenue");

const model = builder.build();

// Evaluate
const evaluator = Evaluator.new();
const results = evaluator.evaluate(model);

// Access results
const q1 = PeriodId.quarter(2024, 1);
const q3 = PeriodId.quarter(2024, 3);

console.log(`Q1 Revenue: ${results.get('revenue', q1)}`);
console.log(`Q3 Revenue (forecast): ${results.get('revenue', q3)}`);
console.log(`Q3 Margin: ${results.get('margin', q3) * 100}%`);
```

**Key Differences:**
- Python list of tuples `[(key, value)]`, TypeScript array of arrays `[[key, value]]`
- Percentage formatting: Python `:.1%`, TypeScript manual multiplication `* 100`

### 4. Scenario Analysis

**Python:**
```python
from finstack.scenarios import (
    ScenarioEngine,
    ScenarioSpec,
    OperationSpec,
    ExecutionContext,
    CurveKind
)

# Create scenario
ops = [
    OperationSpec.curve_parallel_bp(CurveKind.DISCOUNT, "USD-OIS", 50.0),
    OperationSpec.equity_shock("AAPL", 0.10),
    OperationSpec.fx_shock("EUR", "USD", 0.05),
]

scenario = ScenarioSpec(
    "stress_test",
    ops,
    name="Rates Up 50bp",
    description="Parallel rate shift with equity shock"
)

# Apply scenario
engine = ScenarioEngine()
ctx = ExecutionContext(market=market, as_of=date(2024, 1, 2))
report = engine.apply(scenario, ctx)

print(f"Applied {report.applied_operations} operations")
for warning in report.warnings:
    print(f"Warning: {warning}")
```

**TypeScript:**
```typescript
import {
  ScenarioEngine,
  ScenarioSpec,
  OperationSpec,
  ExecutionContext,
  ScenarioCurveKind  // Note: different enum name!
} from 'finstack-wasm';

// Create scenario
const ops = [
    OperationSpec.curveParallelBp(ScenarioCurveKind.Discount, "USD-OIS", 50.0),
    OperationSpec.equityShock("AAPL", 0.10),
    OperationSpec.fxShock("EUR", "USD", 0.05),
];

const scenario = new ScenarioSpec(
    "stress_test",
    ops,
    "Rates Up 50bp",  // name (positional, not named!)
    "Parallel rate shift with equity shock"  // description
);

// Apply scenario
const engine = new ScenarioEngine();
const ctx = new ExecutionContext(market, null, new FsDate(2024, 1, 2));
const report = engine.apply(scenario, ctx);

console.log(`Applied ${report.appliedOperations} operations`);
for (const warning of report.warnings) {
    console.log(`Warning: ${warning}`);
}
```

**Key Differences:**
- Enum name: `CurveKind` (Python) vs `ScenarioCurveKind` (TypeScript)
- Named parameters: Python uses `name=`, TypeScript uses positional
- Properties: `applied_operations` vs `appliedOperations`
- Iteration: `for warning in` (Python) vs `for (const warning of)` (TypeScript)

### 5. Portfolio Aggregation

**Python:**
```python
from finstack.portfolio import PortfolioBuilder, Entity, Position
from finstack.portfolio import value_portfolio, aggregate_by_attribute

# Build portfolio
portfolio = (PortfolioBuilder("FUND_A")
    .name("Alpha Fund")
    .base_ccy("USD")
    .as_of(date(2024, 1, 1))
    .entity(Entity("ACME", name="Acme Corp", attributes={"sector": "Tech"}))
    .entity(Entity("BETA", name="Beta Inc", attributes={"sector": "Finance"}))
    .position(Position("POS_1", "ACME", "BOND_1", 1_000_000.0))
    .position(Position("POS_2", "BETA", "BOND_2", 500_000.0))
    .build())

# Value portfolio
results = value_portfolio(portfolio, market)
print(f"Total value: ${results.total_value:,.2f}")

# Aggregate by sector
by_sector = aggregate_by_attribute(portfolio, "sector", results)
for sector, value in by_sector.items():
    print(f"{sector}: ${value:,.2f}")
```

**TypeScript:**
```typescript
import {
  PortfolioBuilder,
  Entity,
  Position,
  valuePortfolio,  // Note: exported as function!
  aggregateByAttribute
} from 'finstack-wasm';

// Build portfolio
const portfolio = new PortfolioBuilder("FUND_A")
    .name("Alpha Fund")
    .baseCcy("USD")  // camelCase!
    .asOf(new FsDate(2024, 1, 1))
    .entity(new Entity("ACME", "Acme Corp", {"sector": "Tech"}))
    .entity(new Entity("BETA", "Beta Inc", {"sector": "Finance"}))
    .position(new Position("POS_1", "ACME", "BOND_1", 1_000_000.0))
    .position(new Position("POS_2", "BETA", "BOND_2", 500_000.0))
    .build();

// Value portfolio
const results = valuePortfolio(portfolio, market);
console.log(`Total value: $${results.totalValue.toLocaleString()}`);

// Aggregate by sector
const bySector = aggregateByAttribute(portfolio, "sector", results);
for (const [sector, value] of Object.entries(bySector)) {
    console.log(`${sector}: $${value.toLocaleString()}`);
}
```

**Key Differences:**
- Builder methods: `base_ccy()` vs `baseCcy()`, `as_of()` vs `asOf()`
- Entity constructor: named params vs positional
- Iteration: `dict.items()` (Python) vs `Object.entries()` (TypeScript)
- Number formatting: `f"{value:,.2f}"` (Python) vs `value.toLocaleString()` (TypeScript)

## Type System Mapping

| Python Type | TypeScript Type | Notes |
|-------------|----------------|-------|
| `int` | `number` | JavaScript doesn't distinguish int/float |
| `float` | `number` | Same as int |
| `str` | `string` | Direct mapping |
| `bool` | `boolean` | Direct mapping |
| `date` | `FsDate` or `Date` | Use FsDate for precision |
| `list[T]` | `Array<T>` or `T[]` | TypeScript arrays |
| `dict[K, V]` | `Map<K, V>` or `{[key: K]: V}` | Maps or objects |
| `tuple[A, B]` | `[A, B]` | TypeScript tuples |
| `Optional[T]` | `T \| null` or `T \| undefined` | Nullable types |
| `Currency` | `Currency` | Same class |
| `Money` | `Money` | Same class |

## Error Handling Patterns

### Python
```python
try:
    result = pricer.price(bond, market)
except ValueError as e:
    # Validation or argument error
    print(f"Invalid input: {e}")
except RuntimeError as e:
    # Computation error
    print(f"Pricing failed: {e}")
except KeyError as e:
    # Missing market data
    print(f"Missing data: {e}")
```

### TypeScript
```typescript
try {
    const result = pricer.price(bond, market);
} catch (e) {
    // All errors are Error objects
    if (e instanceof Error) {
        console.error(`Error: ${e.message}`);
    }
}
```

**Note:** TypeScript/WASM uses generic `Error` objects. Check error messages for specifics.

## Performance Considerations

### Python
- GIL released for compute-heavy operations
- Use NumPy/Polars for bulk data
- DataFrame exports available

### TypeScript/WASM
- No GIL concerns
- Minimize JS↔WASM boundary crossings
- Batch operations when possible
- Consider Web Workers for parallelism

## Testing Patterns

### Python
```python
import pytest
from finstack import Currency, Money

def test_money_addition():
    usd = Currency("USD")
    m1 = Money(100, usd)
    m2 = Money(50, usd)
    result = m1 + m2  # Operator overload
    assert result.amount == 150

def test_money_addition_error():
    usd = Currency("USD")
    eur = Currency("EUR")
    m1 = Money(100, usd)
    m2 = Money(50, eur)
    with pytest.raises(ValueError):
        m1 + m2
```

### TypeScript/Jest
```typescript
import { Currency, Money } from 'finstack-wasm';

test('money addition', () => {
    const usd = new Currency("USD");
    const m1 = new Money(100, usd);
    const m2 = new Money(50, usd);
    const result = m1.add(m2);  // Method call
    expect(result.amount).toBe(150);
});

test('money addition error', () => {
    const usd = new Currency("USD");
    const eur = new Currency("EUR");
    const m1 = new Money(100, usd);
    const m2 = new Money(50, eur);
    expect(() => m1.add(m2)).toThrow();
});
```

## Best Practices

### When Migrating to TypeScript/WASM

1. **Use TypeScript** (not plain JavaScript) for type safety
2. **Enable strict mode** in `tsconfig.json`
3. **Install type definitions**: `@types/node` if using Node.js
4. **Memory management**: Call `.free()` on WASM objects when done (if applicable)
5. **Bundle size**: Use tree-shaking to minimize bundle size

### When Migrating to Python

1. **Use type hints** (`typing` module) for type safety
2. **Use linters**: `mypy`, `ruff`, `pylint`
3. **Virtual environments**: Use `uv` or `venv`
4. **Package management**: Prefer `uv` or `pip` with `requirements.txt`
5. **Testing**: Use `pytest` with coverage

## Gotchas & Common Mistakes

### 1. Date Object Confusion

❌ **Wrong:**
```typescript
// Using JavaScript Date directly everywhere
const date = new Date("2024-01-15");
bond.maturity = date;  // May lose precision!
```

✅ **Correct:**
```typescript
// Use FsDate for finstack dates
import { FsDate } from 'finstack-wasm';
const date = new FsDate(2024, 1, 15);
bond.maturity = date;
```

### 2. Forgetting Naming Convention Changes

❌ **Wrong:**
```typescript
// Copy-pasted from Python
const curve = market.get_discount("USD-OIS");  // ❌
```

✅ **Correct:**
```typescript
// camelCase in TypeScript
const curve = market.getDiscount("USD-OIS");  // ✅
```

### 3. Module Import Differences

❌ **Wrong:**
```typescript
// Trying to use nested imports like Python
import { Currency } from 'finstack-wasm/core/currency';  // ❌
```

✅ **Correct:**
```typescript
// Flat imports in WASM
import { Currency } from 'finstack-wasm';  // ✅
```

### 4. Operator Overloads Not Available in TypeScript

❌ **Wrong:**
```typescript
const result = money1 + money2;  // ❌ Won't work!
```

✅ **Correct:**
```typescript
const result = money1.add(money2);  // ✅ Use method
```

## Migration Tools

### Automated Conversion

Use search-and-replace patterns for mechanical conversions:

```bash
# Python → TypeScript naming
sed 's/\.([a-z_]+)\(/.\1(/g' | \
sed 's/_\([a-z]\)/\u\1/g'

# TypeScript → Python naming
sed 's/\.\([a-z][a-zA-Z]*\)(/._\1(/g' | \
sed 's/\([a-z]\)\([A-Z]\)/\1_\l\2/g'
```

**Note:** These are starting points. Manual review required!

### IDE Support

- **VS Code**: Install Python and TypeScript extensions
- **PyCharm**: Supports both Python and TypeScript
- **Type checking**: Use `mypy` (Python) and `tsc` (TypeScript)

## Next Steps

- Review the [API Reference](api-reference.md) for complete API comparison
- Check [NAMING_CONVENTIONS.md](../../../NAMING_CONVENTIONS.md) for detailed naming patterns
- See [examples.md](examples.md) for more side-by-side code examples
- Consult the [Python README](../../../finstack-py/README.md) and [WASM README](../../../finstack-wasm/README.md)

---

**Need help?** Open an issue on GitHub with your migration question!

