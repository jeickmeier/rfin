# Portfolio WASM Bindings

This document describes the WebAssembly bindings for the finstack portfolio module, providing portfolio management, valuation, and aggregation capabilities for JavaScript/TypeScript applications.

## Overview

The portfolio WASM bindings provide 100% parity with the Rust and Python implementations, exposing:

- Entity and position management
- Portfolio construction and validation
- Position valuation with metrics
- Cross-currency aggregation
- Attribute-based grouping
- Scenario integration (when scenarios feature is enabled)

## Core Types

### Entity

Represents a company, fund, or legal entity that owns positions.

```javascript
import { Entity } from 'finstack-wasm';

// Create entity
const entity = new Entity('ACME_CORP')
  .withName('Acme Corporation')
  .withTag('sector', 'Technology')
  .withTag('region', 'US');

// Access properties
console.log(entity.id); // "ACME_CORP"
console.log(entity.name); // "Acme Corporation"
console.log(entity.tags); // { sector: "Technology", region: "US" }

// Dummy entity for standalone instruments
const dummy = Entity.dummy(); // Special entity with ID "_standalone"
```

### PositionUnit

Defines how position quantities should be interpreted.

```javascript
import { PositionUnit } from 'finstack-wasm';

// Available unit types
const units = PositionUnit.UNITS; // For equities, shares
const notional = PositionUnit.notional(); // For derivatives
const notionalUsd = PositionUnit.notionalWithCcy('USD');
const faceValue = PositionUnit.FACE_VALUE; // For bonds
const percentage = PositionUnit.PERCENTAGE; // Ownership percentage
```

### Position

Represents a holding of a specific instrument.

**Note:** Due to WASM limitations with trait objects, Position instances cannot be directly constructed from JavaScript. Positions are typically created through Rust-side builders or by using the fluent builder pattern on the portfolio.

```javascript
// Positions are created and managed within portfolios
// Access via portfolio methods:
const position = portfolio.getPosition('POS_001');
console.log(position.positionId); // "POS_001"
console.log(position.entityId); // "ACME_CORP"
console.log(position.instrumentId); // "BOND_001"
console.log(position.quantity); // 1000000
console.log(position.isLong()); // true
console.log(position.isShort()); // false
```

## Portfolio

Main container for entities and positions.

```javascript
import { Portfolio, Entity, Currency, FsDate } from 'finstack-wasm';

// Create empty portfolio
const asOf = new FsDate(2024, 1, 1);
const portfolio = new Portfolio('FUND_A', Currency.USD, asOf);

// Set name
portfolio.name = 'Alpha Fund';

// Access properties
console.log(portfolio.id); // "FUND_A"
console.log(portfolio.baseCcy); // Currency.USD
console.log(portfolio.asOf); // FsDate(2024, 1, 1)
console.log(portfolio.entities); // Object mapping entity IDs to entities
console.log(portfolio.positions); // Array of positions

// Query methods
const position = portfolio.getPosition('POS_001');
const entityPositions = portfolio.positionsForEntity('ACME_CORP');
const taggedPositions = portfolio.positionsWithTag('rating', 'AAA');

// Validate
portfolio.validate(); // Throws if invalid references
```

## PortfolioBuilder

Fluent API for constructing validated portfolios.

```javascript
import { PortfolioBuilder, Entity, Currency, FsDate } from 'finstack-wasm';

const asOf = new FsDate(2024, 1, 1);

const portfolio = new PortfolioBuilder('FUND_A')
  .name('Alpha Fund')
  .baseCcy(Currency.USD)
  .asOf(asOf)
  .entity(new Entity('ACME'))
  .entity(new Entity('BETA'))
  .tag('strategy', 'balanced')
  .tag('risk_profile', 'moderate')
  .build();

// Add multiple entities at once (from JSON)
const entities = [
  Entity.fromJSON({ id: 'ENTITY_1', name: 'Entity One' }),
  Entity.fromJSON({ id: 'ENTITY_2', name: 'Entity Two' }),
];
const builder = new PortfolioBuilder('FUND_B')
  .baseCcy(Currency.USD)
  .asOf(asOf)
  .entities([entities[0].toJSON(), entities[1].toJSON()]);
```

## Valuation

Value entire portfolios with automatic cross-currency conversion.

```javascript
import { valuePortfolio, FinstackConfig } from 'finstack-wasm';

// Build market data context
const market = new MarketContext();
market.insertDiscount(discountCurve);
market.insertForward(forwardCurve);

// Configure computation
const config = new FinstackConfig();

// Value the portfolio
const valuation = valuePortfolio(portfolio, market, config);

// Access results
console.log(valuation.totalBaseCcy); // Total portfolio value in base currency

// Get position values
const posValue = valuation.getPositionValue('POS_001');
console.log(posValue.valueNative); // Value in instrument's native currency
console.log(posValue.valueBase); // Value converted to portfolio base currency

// Get entity totals
const entityValue = valuation.getEntityValue('ACME_CORP');
console.log(entityValue); // Total value for all ACME_CORP positions

// All values as objects
console.log(valuation.positionValues); // Object mapping position IDs to values
console.log(valuation.byEntity); // Object mapping entity IDs to totals
```

## Metrics Aggregation

Aggregate risk metrics across the portfolio.

```javascript
import { aggregateMetrics } from 'finstack-wasm';

// Aggregate metrics from valuation
const metrics = aggregateMetrics(valuation);

// Get aggregated metric
const dv01Metric = metrics.getMetric('dv01');
console.log(dv01Metric.total); // Total DV01 across portfolio
console.log(dv01Metric.byEntity); // DV01 by entity

// Get total for a specific metric
const totalDv01 = metrics.getTotal('dv01');

// Get metrics for a position
const posMetrics = metrics.getPositionMetrics('POS_001');
console.log(posMetrics); // Object with all metrics for the position
```

## Grouping and Aggregation

Group positions and aggregate values by attributes.

```javascript
import { groupByAttribute, aggregateByAttribute } from 'finstack-wasm';

// Group positions by sector
const bySector = groupByAttribute(portfolio, 'sector');
console.log(bySector['Technology']); // Array of positions in tech sector
console.log(bySector['Finance']); // Array of positions in finance sector

// Aggregate values by rating
const byRating = aggregateByAttribute(valuation, portfolio, 'rating');
console.log(byRating['AAA']); // Total value of AAA-rated positions
console.log(byRating['AA']); // Total value of AA-rated positions
```

## Scenario Integration

Apply market scenarios to portfolios (requires scenarios feature).

```javascript
import { applyScenario, applyAndRevalue, ScenarioSpec } from 'finstack-wasm';

// Define scenario
const scenario = ScenarioSpec.fromJSON({
  id: 'stress_test',
  name: 'Rate Shock',
  operations: [
    {
      curve_parallel_bp: {
        curve_kind: 'discount',
        curve_id: 'USD',
        bp: 50.0, // +50bp parallel shift
      },
    },
  ],
});

// Apply scenario to portfolio
const transformedPortfolio = applyScenario(portfolio, scenario, market);

// Or apply and revalue in one step
const stressedValuation = applyAndRevalue(portfolio, scenario, market, config);

console.log(stressedValuation.totalBaseCcy); // Portfolio value under stress
```

## JSON Serialization

All types support JSON serialization (except positions with instruments).

```javascript
// Entity serialization
const entity = new Entity('ACME');
const entityJson = entity.toJSON();
const entityCopy = Entity.fromJSON(entityJson);

// Portfolio serialization (note: positions excluded)
const portfolioJson = portfolio.toJSON();
const portfolioCopy = Portfolio.fromJSON(portfolioJson);

// Valuation serialization
const valuationJson = valuation.toJSON();
const valuationCopy = PortfolioValuation.fromJSON(valuationJson);

// Metrics serialization
const metricsJson = metrics.toJSON();
const metricsCopy = PortfolioMetrics.fromJSON(metricsJson);
```

## Complete Example

```javascript
import {
  Entity,
  Portfolio,
  PortfolioBuilder,
  PositionUnit,
  Currency,
  FsDate,
  Money,
  MarketContext,
  DiscountCurve,
  FinstackConfig,
  valuePortfolio,
  aggregateMetrics,
  groupByAttribute,
  aggregateByAttribute,
  Bond,
  Deposit,
} from 'finstack-wasm';

async function runPortfolioExample() {
  // 1. Create entities
  const corpA = new Entity('CORP_A').withName('Corporate A').withTag('sector', 'Finance');

  const fundB = new Entity('FUND_B').withName('Fund B').withTag('sector', 'Technology');

  // 2. Create instruments
  const asOf = new FsDate(2024, 1, 2);

  const bond = Bond.fixedSemiannual(
    'BOND_001',
    new Money(5_000_000, Currency.USD),
    0.045,
    new FsDate(2024, 1, 15),
    new FsDate(2029, 1, 15),
    'USD-OIS'
  );

  const deposit = new Deposit(
    'DEPOSIT_001',
    new Money(2_000_000, Currency.USD),
    asOf,
    new FsDate(2024, 7, 2),
    DayCount.ACT_360,
    'USD-OIS'
  );

  // 3. Build portfolio
  const portfolio = new PortfolioBuilder('MULTI_ASSET_FUND')
    .name('Multi-Asset Investment Fund')
    .baseCcy(Currency.USD)
    .asOf(asOf)
    .entity(corpA)
    .entity(fundB)
    .tag('strategy', 'balanced')
    .build();

  // 4. Create market data
  const market = new MarketContext();
  const curve = new DiscountCurve('USD-OIS', asOf, [
    [0.0, 1.0],
    [1.0, 0.995],
    [5.0, 0.95],
    [10.0, 0.9],
  ]);
  market.insertDiscount(curve);

  // 5. Value portfolio
  const config = new FinstackConfig();
  const valuation = valuePortfolio(portfolio, market, config);

  console.log('Total Value:', valuation.totalBaseCcy);

  // 6. Aggregate metrics
  const metrics = aggregateMetrics(valuation);
  console.log('Total DV01:', metrics.getTotal('dv01'));

  // 7. Group by sector
  const bySector = groupByAttribute(portfolio, 'sector');
  console.log('Tech positions:', bySector['Technology']?.length || 0);

  // 8. Aggregate by sector
  const valueBySector = aggregateByAttribute(valuation, portfolio, 'sector');
  console.log('Tech value:', valueBySector['Technology']);

  return {
    portfolio,
    valuation,
    metrics,
  };
}
```

## TypeScript Types

The WASM package includes TypeScript type definitions generated by wasm-bindgen.

```typescript
import type {
  Entity,
  Portfolio,
  PortfolioBuilder,
  Position,
  PositionUnit,
  PortfolioValuation,
  PortfolioMetrics,
  AggregatedMetric,
  PositionValue,
  PortfolioResults,
} from 'finstack-wasm';

// Type-safe function
function buildPortfolio(
  id: string,
  entities: Entity[],
  baseCcy: Currency,
  asOf: FsDate
): Portfolio {
  let builder = new PortfolioBuilder(id).baseCcy(baseCcy).asOf(asOf);

  for (const entity of entities) {
    builder = builder.entity(entity);
  }

  return builder.build();
}
```

## Error Handling

All methods that can fail return `Result<T, JsValue>` which throws on error.

```javascript
try {
  // Build portfolio
  const portfolio = new PortfolioBuilder('FUND')
    .baseCcy(Currency.USD)
    // Missing as_of - will throw
    .build();
} catch (error) {
  console.error('Portfolio build failed:', error);
  // Error: "Valuation date (as_of) must be set"
}

try {
  // Invalid entity reference
  portfolio.validate();
} catch (error) {
  console.error('Validation failed:', error);
  // Error: "Position 'POS_1' references unknown entity 'UNKNOWN'"
}
```

## Architecture

### Data Flow

```
JavaScript/TypeScript Application
         ↓
    WASM Bindings (JsPortfolio, JsEntity, etc.)
         ↓
    Rust Portfolio Library (finstack-portfolio)
         ↓
    Rust Core (finstack-core, finstack-valuations)
```

### Design Principles

1. **Thin Wrappers**: All business logic remains in Rust; WASM bindings are simple pass-throughs
2. **JSON Serialization**: Types support `fromJSON`/`toJSON` for persistence and transport
3. **Builder Pattern**: Fluent APIs match Python bindings for consistency
4. **Type Safety**: TypeScript definitions ensure compile-time type checking
5. **Error Propagation**: Rust errors are converted to JavaScript exceptions

## Limitations

Due to WASM constraints:

1. **Position Construction**: Positions cannot be directly constructed in JavaScript due to `Arc<dyn Instrument>` trait objects. Use builder methods or Rust-side factories.

2. **Instrument Trait**: Only concrete instrument types can be passed to positions (Bond, Deposit, Swap, etc.), not generic instrument interfaces.

3. **Array Handling**: The `entities()` and `positions()` builder methods accept Array parameters but have limitations with type conversion. Prefer calling `.entity()` or `.position()` multiple times.

4. **Serialization**: Positions with instruments cannot be serialized to JSON due to trait objects. Only entity and portfolio metadata can be serialized.

## Performance Considerations

1. **Clone vs Reference**: WASM bindings clone data when crossing the JS/Rust boundary. Avoid unnecessary back-and-forth calls.

2. **Batch Operations**: Use builder pattern to batch multiple entities/positions rather than individual calls.

3. **Market Data**: Build complete market context before valuation rather than incrementally.

4. **Caching**: Cache frequently accessed portfolios and valuations in JavaScript to avoid re-computation.

## Feature Flags

The portfolio bindings support feature-based compilation:

- **default**: Includes `scenarios` feature
- **scenarios**: Enables scenario integration (`applyScenario`, `applyAndRevalue`)

Build without scenarios:

```bash
wasm-pack build --no-default-features
```

## Integration Examples

See `finstack-wasm/examples/src/components/PortfolioExample.tsx` for a complete React integration example.

For Node.js usage, import from the Node.js package:

```javascript
const { Portfolio, Entity } = require('finstack-wasm/pkg-node');
```

## API Reference

### Functions

- `valuePortfolio(portfolio, marketContext, config?)`: Value a portfolio
- `aggregateMetrics(valuation)`: Aggregate metrics from valuation
- `groupByAttribute(portfolio, attributeKey)`: Group positions by tag
- `aggregateByAttribute(valuation, portfolio, attributeKey)`: Aggregate values by tag
- `applyScenario(portfolio, scenario, marketContext)`: Apply scenario (requires scenarios feature)
- `applyAndRevalue(portfolio, scenario, marketContext, config?)`: Apply and revalue (requires scenarios feature)

### Classes

- `Entity`: Entity that owns positions
- `PositionUnit`: Position quantity unit enum
- `Position`: Position in an instrument
- `Portfolio`: Portfolio container
- `PortfolioBuilder`: Fluent portfolio builder
- `PositionValue`: Single position valuation result
- `PortfolioValuation`: Complete portfolio valuation
- `AggregatedMetric`: Aggregated metric across portfolio
- `PortfolioMetrics`: Complete metrics collection
- `PortfolioResults`: Combined valuation and metrics

## See Also

- [Rust Portfolio Documentation](../../finstack/portfolio/README.md)
- [Python Portfolio Bindings](../../finstack-py/finstack/portfolio.pyi)
- [WASM Scenarios Integration](./SCENARIOS_BINDINGS.md)
- [WASM Statements Quickstart](./STATEMENTS_QUICKSTART.md)
