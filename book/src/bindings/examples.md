# Side-by-Side Examples: Python & TypeScript

This page shows common finstack workflows in both Python and TypeScript/WASM side-by-side.

## Example 1: Basic Bond Pricing

<table>
<tr>
<th>Python</th>
<th>TypeScript/WASM</th>
</tr>
<tr>
<td>

```python
from datetime import date
from finstack.core.currency import Currency
from finstack.core.money import Money
from finstack.core.market_data import (
    DiscountCurve,
    MarketContext
)
from finstack.valuations.instruments import Bond
from finstack.valuations.pricer import PricerRegistry

# Create USD currency
usd = Currency("USD")

# Create bond
bond = Bond.treasury(
    "US-TREAS-10Y",
    1_000_000,
    "USD",
    coupon_rate=0.0375,
    maturity=date(2034, 5, 15),
    issue_date=date(2024, 5, 15)
)

# Create discount curve
curve = DiscountCurve(
    "USD-OIS",
    date(2024, 1, 2),
    [(0.0, 1.0), (10.0, 0.70)],
    day_count="act_365f"
)

# Build market
market = MarketContext()
market.insert_discount(curve)

# Price bond
pricer = PricerRegistry.standard()
result = pricer.price(bond, market)

print(f"PV: ${result.present_value:,.2f}")
print(f"Duration: {result.duration:.4f}")
```

</td>
<td>

```typescript
import {
  FsDate,
  Currency,
  Money,
  DiscountCurve,
  MarketContext,
  Bond,
  createStandardRegistry
} from 'finstack-wasm';

// Create USD currency
const usd = new Currency("USD");

// Create bond
const bond = Bond.treasury(
  "US-TREAS-10Y",
  1_000_000,
  "USD",
  0.0375,  // couponRate
  new FsDate(2034, 5, 15),  // maturity
  new FsDate(2024, 5, 15)   // issueDate
);

// Create discount curve
const curve = new DiscountCurve(
  "USD-OIS",
  new FsDate(2024, 1, 2),
  [0.0, 10.0],      // times
  [1.0, 0.70],      // factors
  "act_365f"
);

// Build market
const market = new MarketContext();
market.insertDiscount(curve);

// Price bond
const pricer = createStandardRegistry();
const result = pricer.price(bond, market);

console.log(`PV: $${result.presentValue.toLocaleString()}`);
console.log(`Duration: ${result.duration.toFixed(4)}`);
```

</td>
</tr>
</table>

## Example 2: Curve Calibration

<table>
<tr>
<th>Python</th>
<th>TypeScript/WASM</th>
</tr>
<tr>
<td>

```python
from datetime import date
from finstack.valuations.calibration import (
    DiscountCurveCalibrator,
    CalibrationConfig,
    RatesQuote,
    SolverKind
)

# Configure calibrator
calibrator = DiscountCurveCalibrator(
    "USD-SOFR",
    date(2024, 1, 2),
    "USD"
)

config = (CalibrationConfig.multi_curve()
    .with_solver_kind(SolverKind.HYBRID)
    .with_max_iterations(100)
    .with_tolerance(1e-10))

calibrator = calibrator.with_config(config)

# Define market quotes
quotes = [
    RatesQuote.deposit(
        date(2024, 4, 2),
        0.0525,
        "act_360"
    ),
    RatesQuote.swap(
        date(2027, 1, 4),
        0.0485,
        "semi_annual",
        "quarterly",
        "30_360",
        "act_360",
        "USD-SOFR"
    ),
]

# Calibrate
curve, report = calibrator.calibrate(quotes, None)

print(f"Success: {report.success}")
print(f"Iterations: {report.iterations}")
print(f"Residual: {report.max_residual:.2e}")
```

</td>
<td>

```typescript
import {
  FsDate,
  DiscountCurveCalibrator,
  CalibrationConfig,
  RatesQuote,
  SolverKind,
  Frequency
} from 'finstack-wasm';

// Configure calibrator
let calibrator = new DiscountCurveCalibrator(
    "USD-SOFR",
    new FsDate(2024, 1, 2),
    "USD"
);

const config = CalibrationConfig.multiCurve()
    .withSolverKind(SolverKind.Hybrid())
    .withMaxIterations(100)
    .withTolerance(1e-10);

calibrator = calibrator.withConfig(config);

// Define market quotes
const quotes = [
    RatesQuote.deposit(
        new FsDate(2024, 4, 2),
        0.0525,
        "act_360"
    ),
    RatesQuote.swap(
        new FsDate(2027, 1, 4),
        0.0485,
        Frequency.semiAnnual(),
        Frequency.quarterly(),
        "30_360",
        "act_360",
        "USD-SOFR"
    ),
];

// Calibrate
const [curve, report] = calibrator.calibrate(quotes, null);

console.log(`Success: ${report.success}`);
console.log(`Iterations: ${report.iterations}`);
console.log(`Residual: ${report.maxResidual.toExponential(2)}`);
```

</td>
</tr>
</table>

## Example 3: Statement Modeling

<table>
<tr>
<th>Python</th>
<th>TypeScript/WASM</th>
</tr>
<tr>
<td>

```python
from finstack.statements import (
    ModelBuilder,
    Evaluator,
    ForecastSpec
)
from finstack.core.dates import PeriodId
from finstack.core.money import Money

# Build P&L model
builder = ModelBuilder.new("P&L Model")
builder.periods("2024Q1..Q4", "2024Q2")

# Add revenue actuals
builder.value("revenue", [
    (PeriodId.quarter(2024, 1), 
     Money.from_code(5_000_000, "USD")),
    (PeriodId.quarter(2024, 2),
     Money.from_code(5_500_000, "USD")),
])

# Forecast with 3% growth
builder.forecast("revenue", 
                ForecastSpec.growth(0.03))

# Add operating expenses
builder.value("opex", [
    (PeriodId.quarter(2024, 1),
     Money.from_code(3_000_000, "USD")),
    (PeriodId.quarter(2024, 2),
     Money.from_code(3_200_000, "USD")),
])
builder.forecast("opex",
                ForecastSpec.growth(0.02))

# Compute EBITDA
builder.compute("ebitda", "revenue - opex")
builder.compute("margin", "ebitda / revenue")

model = builder.build()

# Evaluate
evaluator = Evaluator.new()
results = evaluator.evaluate(model)

# Print results
for q in range(1, 5):
    period = PeriodId.quarter(2024, q)
    rev = results.get("revenue", period)
    ebitda = results.get("ebitda", period)
    margin = results.get("margin", period)
    print(f"2024Q{q}: Rev=${rev:,.0f}, "
          f"EBITDA=${ebitda:,.0f}, "
          f"Margin={margin:.1%}")
```

</td>
<td>

```typescript
import {
  ModelBuilder,
  Evaluator,
  ForecastSpec,
  PeriodId,
  Money
} from 'finstack-wasm';

// Build P&L model
const builder = ModelBuilder.new("P&L Model");
builder.periods("2024Q1..Q4", "2024Q2");

// Add revenue actuals
builder.value("revenue", [
    [PeriodId.quarter(2024, 1),
     Money.fromCode(5_000_000, "USD")],
    [PeriodId.quarter(2024, 2),
     Money.fromCode(5_500_000, "USD")],
]);

// Forecast with 3% growth
builder.forecast("revenue",
                ForecastSpec.growth(0.03));

// Add operating expenses
builder.value("opex", [
    [PeriodId.quarter(2024, 1),
     Money.fromCode(3_000_000, "USD")],
    [PeriodId.quarter(2024, 2),
     Money.fromCode(3_200_000, "USD")],
]);
builder.forecast("opex",
                ForecastSpec.growth(0.02));

// Compute EBITDA
builder.compute("ebitda", "revenue - opex");
builder.compute("margin", "ebitda / revenue");

const model = builder.build();

// Evaluate
const evaluator = Evaluator.new();
const results = evaluator.evaluate(model);

// Print results
for (let q = 1; q <= 4; q++) {
    const period = PeriodId.quarter(2024, q);
    const rev = results.get("revenue", period);
    const ebitda = results.get("ebitda", period);
    const margin = results.get("margin", period);
    console.log(`2024Q${q}: Rev=$${rev.toLocaleString()}, ` +
                `EBITDA=$${ebitda.toLocaleString()}, ` +
                `Margin=${(margin * 100).toFixed(1)}%`);
}
```

</td>
</tr>
</table>

## Example 4: Scenario Analysis

<table>
<tr>
<th>Python</th>
<th>TypeScript/WASM</th>
</tr>
<tr>
<td>

```python
from finstack.scenarios import (
    ScenarioEngine,
    ScenarioSpec,
    OperationSpec,
    ExecutionContext,
    CurveKind
)

# Define scenario operations
ops = [
    OperationSpec.curve_parallel_bp(
        CurveKind.DISCOUNT,
        "USD-OIS",
        50.0  # +50 bps
    ),
    OperationSpec.equity_shock(
        "AAPL",
        0.10  # +10%
    ),
    OperationSpec.fx_shock(
        "EUR",
        "USD",
        -0.05  # -5%
    ),
]

# Create scenario
scenario = ScenarioSpec(
    "rates_up_equity_up",
    ops,
    name="Rates +50bp, Equity +10%",
    description="Stress scenario"
)

# Apply to execution context
engine = ScenarioEngine()
ctx = ExecutionContext(
    market=market,
    as_of=date(2024, 1, 2)
)

report = engine.apply(scenario, ctx)

print(f"Applied: {report.applied_operations}")
print(f"Warnings: {len(report.warnings)}")

# Re-price with shocked market
result_shocked = pricer.price(bond, ctx.market)
pv_change = (result_shocked.present_value - 
             result_base.present_value)
print(f"PV Change: ${pv_change:,.2f}")
```

</td>
<td>

```typescript
import {
  ScenarioEngine,
  ScenarioSpec,
  OperationSpec,
  ExecutionContext,
  ScenarioCurveKind
} from 'finstack-wasm';

// Define scenario operations
const ops = [
    OperationSpec.curveParallelBp(
        ScenarioCurveKind.Discount,
        "USD-OIS",
        50.0  // +50 bps
    ),
    OperationSpec.equityShock(
        "AAPL",
        0.10  // +10%
    ),
    OperationSpec.fxShock(
        "EUR",
        "USD",
        -0.05  // -5%
    ),
];

// Create scenario
const scenario = new ScenarioSpec(
    "rates_up_equity_up",
    ops,
    "Rates +50bp, Equity +10%",
    "Stress scenario"
);

// Apply to execution context
const engine = new ScenarioEngine();
const ctx = new ExecutionContext(
    market,
    null,
    new FsDate(2024, 1, 2)
);

const report = engine.apply(scenario, ctx);

console.log(`Applied: ${report.appliedOperations}`);
console.log(`Warnings: ${report.warnings.length}`);

// Re-price with shocked market
const resultShocked = pricer.price(bond, ctx.market);
const pvChange = resultShocked.presentValue -
                 resultBase.presentValue;
console.log(`PV Change: $${pvChange.toLocaleString()}`);
```

</td>
</tr>
</table>

## Example 5: Portfolio Aggregation

<table>
<tr>
<th>Python</th>
<th>TypeScript/WASM</th>
</tr>
<tr>
<td>

```python
from finstack.portfolio import (
    PortfolioBuilder,
    Entity,
    Position,
    value_portfolio,
    aggregate_by_attribute
)

# Build portfolio
portfolio = (PortfolioBuilder("HEDGE_FUND")
    .name("Alpha Strategies Fund")
    .base_ccy("USD")
    .as_of(date(2024, 1, 1))
    .entity(Entity(
        "TECH_CO_A",
        name="Tech Company A",
        attributes={
            "sector": "Technology",
            "region": "North America"
        }
    ))
    .entity(Entity(
        "BANK_B",
        name="Bank B",
        attributes={
            "sector": "Financials",
            "region": "Europe"
        }
    ))
    .position(Position(
        "BOND_1",
        "TECH_CO_A",
        "BOND-TECH-A-2030",
        5_000_000.0
    ))
    .position(Position(
        "BOND_2",
        "BANK_B",
        "BOND-BANK-B-2028",
        3_000_000.0
    ))
    .build())

# Value entire portfolio
results = value_portfolio(portfolio, market)

print(f"Total: ${results.total_value:,.2f}")

# Aggregate by sector
by_sector = aggregate_by_attribute(
    portfolio,
    "sector",
    results
)

for sector, value in by_sector.items():
    pct = value / results.total_value * 100
    print(f"{sector}: ${value:,.0f} ({pct:.1f}%)")
```

</td>
<td>

```typescript
import {
  PortfolioBuilder,
  Entity,
  Position,
  valuePortfolio,
  aggregateByAttribute
} from 'finstack-wasm';

// Build portfolio
const portfolio = new PortfolioBuilder("HEDGE_FUND")
    .name("Alpha Strategies Fund")
    .baseCcy("USD")
    .asOf(new FsDate(2024, 1, 1))
    .entity(new Entity(
        "TECH_CO_A",
        "Tech Company A",
        {
            "sector": "Technology",
            "region": "North America"
        }
    ))
    .entity(new Entity(
        "BANK_B",
        "Bank B",
        {
            "sector": "Financials",
            "region": "Europe"
        }
    ))
    .position(new Position(
        "BOND_1",
        "TECH_CO_A",
        "BOND-TECH-A-2030",
        5_000_000.0
    ))
    .position(new Position(
        "BOND_2",
        "BANK_B",
        "BOND-BANK-B-2028",
        3_000_000.0
    ))
    .build();

// Value entire portfolio
const results = valuePortfolio(portfolio, market);

console.log(`Total: $${results.totalValue.toLocaleString()}`);

// Aggregate by sector
const bySector = aggregateByAttribute(
    portfolio,
    "sector",
    results
);

for (const [sector, value] of Object.entries(bySector)) {
    const pct = (value / results.totalValue) * 100;
    console.log(`${sector}: $${value.toLocaleString()} (${pct.toFixed(1)}%)`);
}
```

</td>
</tr>
</table>

## Key Patterns Summary

### Naming Conventions

| Python | TypeScript |
|--------|------------|
| `snake_case` methods | `camelCase` methods |
| `from_code()` | `fromCode()` |
| `insert_discount()` | `insertDiscount()` |
| `as_of()` | `asOf()` |

### Constructors

| Python | TypeScript |
|--------|------------|
| `Currency("USD")` | `new Currency("USD")` |
| `Money.from_code(100, "USD")` | `Money.fromCode(100, "USD")` |
| `date(2024, 1, 15)` | `new FsDate(2024, 1, 15)` |

### Collections

| Python | TypeScript |
|--------|------------|
| `[(a, b), (c, d)]` | `[[a, b], [c, d]]` |
| `{"key": "value"}` | `{"key": "value"}` (same) |
| `list[T]` | `Array<T>` or `T[]` |

### Iteration

| Python | TypeScript |
|--------|------------|
| `for item in items:` | `for (const item of items) {` |
| `for k, v in dict.items():` | `for (const [k, v] of Object.entries(obj)) {` |

## Related Documentation

- [API Reference](api-reference.md) - Complete API comparison
- [Migration Guide](migration-guide.md) - Detailed migration patterns
- [Naming Conventions](../../../NAMING_CONVENTIONS.md) - Comprehensive naming rules

---

**Looking for more examples?** Check the examples directories:
- Python: `finstack-py/examples/scripts/`
- TypeScript: `finstack-wasm/examples/src/`

