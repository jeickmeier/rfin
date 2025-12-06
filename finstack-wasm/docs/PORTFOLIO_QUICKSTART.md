# Portfolio WASM Bindings - Quick Start

## Overview

The portfolio WASM bindings provide full JavaScript/TypeScript access to Finstack's portfolio management capabilities, including entity management, position tracking, valuation aggregation, and scenario analysis.

## Installation

The portfolio bindings are included in the main `finstack-wasm` package:

```typescript
import * as finstack from 'finstack-wasm';
```

## Key Classes

### Core Types

- `JsEntity` - Represents an entity (company, fund, etc.) that holds positions
- `JsPosition` - Represents a position in an instrument
- `JsPositionUnit` - Enum for position measurement (Units, Notional, FaceValue, Percentage)

### Portfolio Management

- `JsPortfolio` - Main portfolio class holding entities and positions
- `JsPortfolioBuilder` - Fluent builder for constructing portfolios

### Results

- `JsPortfolioValuation` - Valuation results for a portfolio
- `JsPortfolioMetrics` - Aggregated risk metrics across positions
- `JsPortfolioResults` - Combined valuation and metrics
- `JsPositionValue` - Per-position valuation results

### Aggregation Functions

- `valuePortfolio()` - Value all positions in a portfolio
- `aggregateMetrics()` - Aggregate risk metrics across positions
- `groupByAttribute()` - Group positions by tag attributes
- `aggregateByAttribute()` - Aggregate values by tag attributes

### Scenario Integration (Feature-Gated)

- `applyScenario()` - Apply a scenario to a portfolio
- `applyAndRevalue()` - Apply scenario and re-value in one step

## Important Notes

### Currency and DayCount

**Currency** is a class, not an enum. Always construct it:

```typescript
const usd = new finstack.Currency('USD'); // ✓ Correct
// NOT: finstack.Currency.USD  // ✗ Wrong
```

**DayCount** uses static factory methods:

```typescript
const dayCount = finstack.DayCount.act360(); // ✓ Correct
// NOT: finstack.DayCount.ACT_360  // ✗ Wrong
```

**FinstackConfig** is a class with a constructor:

```typescript
const config = new finstack.FinstackConfig(); // ✓ Correct
// NOT: finstack.FinstackConfig.default()  // ✗ Wrong
```

**Money** properties are accessed directly (not as methods):

```typescript
const money = new finstack.Money(100, usd);
console.log(money.amount); // ✓ Correct - property access
console.log(money.currency); // ✓ Correct - property access
// NOT: money.amount() or money.currency()  // ✗ Wrong
```

## Basic Usage

### Creating Entities

```typescript
// Create entity with fluent builder pattern
const entity = new finstack.JsEntity('ACME_CORP')
  .withName('Acme Corporation')
  .withTag('sector', 'Technology')
  .withTag('rating', 'AAA');

// Access properties
console.log(entity.id); // "ACME_CORP"
console.log(entity.name); // "Acme Corporation"
console.log(entity.tags); // { sector: "Technology", rating: "AAA" }

// Create dummy entity for standalone instruments
const dummy = finstack.JsEntity.dummy();
```

### Building Portfolios

```typescript
// Create date and currency
const asOf = new finstack.FsDate(2024, 1, 1);
const baseCcy = new finstack.Currency('USD');

// Build portfolio
const portfolio = new finstack.JsPortfolioBuilder('MY_FUND')
  .name('My Investment Fund')
  .baseCcy(baseCcy)
  .asOf(asOf)
  .entity(entity)
  .tag('strategy', 'balanced')
  .build();

// Access portfolio properties
console.log(portfolio.id); // "MY_FUND"
console.log(portfolio.name); // "My Investment Fund"
console.log(portfolio.baseCcy); // Currency.USD
console.log(portfolio.asOf); // FsDate(2024, 1, 1)

// Validate portfolio structure
portfolio.validate(); // Throws if invalid
```

### Position Creation

Positions can be created from concrete instrument types using factory functions:

```typescript
// Create a deposit instrument
const deposit = new finstack.Deposit(
  'DEP_001',
  new finstack.Money(1_000_000, usd),
  startDate,
  endDate,
  finstack.DayCount.act360(),
  'USD-OIS',
  0.0525 // quote rate
);

// Create a position from the deposit
const position = finstack.createPositionFromDeposit(
  'POS_001', // position_id
  'ENTITY_A', // entity_id
  deposit, // deposit instrument
  1.0, // quantity (positive=long, negative=short)
  finstack.JsPositionUnit.UNITS // position unit
);

// Create a bond instrument
const bond = finstack.Bond.fixedSemiannual(
  'BOND_001',
  new finstack.Money(5_000_000, usd),
  0.045, // 4.5% coupon
  issueDate,
  maturityDate,
  'USD-OIS'
);

// Create a position from the bond
const bondPosition = finstack.createPositionFromBond(
  'POS_002',
  'ENTITY_B',
  bond,
  1.0,
  finstack.JsPositionUnit.UNITS
);

// Add tags for grouping
const taggedPosition = position.withTag('rating', 'AAA').withTag('sector', 'Banking');

// Check position properties
console.log(position.positionId); // "POS_001"
console.log(position.entityId); // "ENTITY_A"
console.log(position.instrumentId); // "DEP_001"
console.log(position.quantity); // 1.0
console.log(position.isLong()); // true
console.log(position.isShort()); // false
```

**Available Factory Functions:**

- `createPositionFromDeposit(positionId, entityId, deposit, quantity, unit)`
- `createPositionFromBond(positionId, entityId, bond, quantity, unit)`

Additional instrument types can be added as needed (Swaps, Options, FX, etc.).

### Valuing Portfolios

```typescript
// Create market context
const market = new finstack.MarketContext();
// ... add curves and market data ...

// Create configuration
const config = new finstack.FinstackConfig();

// Value the portfolio
const valuation = finstack.valuePortfolio(portfolio, market, config);

// Access results
console.log(valuation.totalBaseCcy); // Total portfolio value (Money)
console.log(valuation.positionValues); // Values by position
console.log(valuation.byEntity); // Values by entity

// Get specific position value
const posValue = valuation.getPositionValue('POS_001');
if (posValue) {
  console.log(posValue.valueNative.amount); // Number (property, not method)
  console.log(posValue.valueNative.currency.code); // String
  console.log(posValue.valueBase.amount); // Value in base currency
}

// Get entity total
const entityValue = valuation.getEntityValue('ACME_CORP');
if (entityValue) {
  console.log(`${entityValue.amount} ${entityValue.currency.code}`);
}
```

### Aggregating Metrics

```typescript
// Aggregate metrics from valuation
const metrics = finstack.aggregateMetrics(valuation);

// Get portfolio-level metric
const dv01 = metrics.getTotal('dv01');
console.log('Portfolio DV01:', dv01);

// Get position metrics
const posMetrics = metrics.getPositionMetrics('POS_001');
console.log('Position metrics:', posMetrics);

// Get specific metric for a position
const aggMetric = metrics.getMetric('cs01');
if (aggMetric) {
  console.log('Total CS01:', aggMetric.total);
  console.log('By entity:', aggMetric.byEntity);
}
```

### Grouping and Aggregation

```typescript
// Group positions by attribute
const byRating = finstack.groupByAttribute(portfolio, 'rating');
// Returns: Map of rating -> array of positions

// Aggregate values by attribute
const valuesByRating = finstack.aggregateByAttribute(valuation, portfolio, 'rating');
// Returns: Map of rating -> Money (total value)

for (const [rating, value] of Object.entries(valuesByRating)) {
  console.log(`${rating}: ${value.amount()} ${value.currency()}`);
}
```

### Scenario Analysis

```typescript
// Create scenario
const scenario = new finstack.ScenarioSpec({
  id: 'stress_test',
  operations: [
    {
      CurveParallelBp: {
        curve_kind: { Discount: null },
        curve_id: 'USD',
        bp: 50.0,
      },
    },
  ],
});

// Apply scenario
const [modifiedPortfolio, modifiedMarket, report] = finstack.applyScenario(
  portfolio,
  scenario,
  market
);

console.log('Operations applied:', report.operationsApplied);

// Or apply and revalue in one step
const [stressedValuation, report2] = finstack.applyAndRevalue(portfolio, scenario, market, config);

console.log('Stressed value:', stressedValuation.totalBaseCcy);
```

## JSON Serialization

Most portfolio types support JSON serialization for data interchange:

```typescript
// Serialize entity
const entityJson = entity.toJSON();
const entityCopy = finstack.JsEntity.fromJSON(entityJson);

// Serialize portfolio (without positions)
const portfolioJson = portfolio.toJSON();
const portfolioCopy = finstack.JsPortfolio.fromJSON(portfolioJson);

// Serialize valuation
const valuationJson = valuation.toJSON();
const valuationCopy = finstack.JsPortfolioValuation.fromJSON(valuationJson);
```

**Note**: Positions cannot be fully serialized due to trait object instruments. Use alternative methods for position persistence.

## TypeScript Types

All classes are fully typed in the generated `.d.ts` file:

```typescript
import type {
  JsEntity,
  JsPortfolio,
  JsPortfolioBuilder,
  JsPortfolioValuation,
  JsPortfolioMetrics,
  JsPosition,
  JsPositionUnit,
} from 'finstack-wasm';
```

## Common Patterns

### Builder Pattern for Entities

```typescript
const entity = new finstack.JsEntity('ID')
  .withName('Name')
  .withTag('key1', 'value1')
  .withTag('key2', 'value2');
```

### Builder Pattern for Portfolios

```typescript
const portfolio = new finstack.JsPortfolioBuilder('ID')
  .name('Name')
  .baseCcy(finstack.Currency.USD)
  .asOf(asOf)
  .entity(entity1)
  .entity(entity2)
  .tag('key', 'value')
  .build();
```

### Error Handling

```typescript
try {
  const portfolio = builder.build();
  portfolio.validate();
} catch (error) {
  console.error('Portfolio error:', error);
  // Handle validation or construction errors
}
```

## Performance Considerations

1. **Memory Management**: WASM objects are automatically garbage collected by JavaScript, but explicitly calling `.free()` on large objects can help with memory management.

2. **Batch Operations**: Group operations when possible rather than making many small calls across the WASM boundary.

3. **Serialization**: JSON serialization has overhead. Use direct object references when working entirely in WASM.

## Limitations

1. **Position Construction**: Positions are created using instrument-specific factory functions (`createPositionFromDeposit`, `createPositionFromBond`, etc.). A generic constructor is not possible due to trait object limitations.

2. **Instrument Access**: While positions hold references to instruments, the underlying instrument cannot be re-extracted from a Position in JavaScript (trait object limitation). Store instrument references separately if needed.

3. **DataFrame Exports**: The `dataframe` feature is not available in WASM builds to minimize bundle size and avoid polars dependencies. Use JSON export for data interchange.

4. **Factory Function Coverage**: Only common instrument types have position factory functions. Additional factory functions can be added as needed for Swaps, Options, CDS, etc.

## Examples

See `finstack-wasm/examples/src/components/PortfolioExample.tsx` for a complete working example demonstrating:

- Entity creation
- Portfolio building
- Market data setup
- Portfolio validation
- Error handling

## Next Steps

- Review the full TypeScript definitions in `pkg/finstack_wasm.d.ts`
- Explore the Python portfolio examples for more complex use cases
- Check the Rust documentation for detailed algorithm descriptions
- See `PORTFOLIO_BINDINGS.md` for implementation details

## Support

For issues or questions:

- Check the main Finstack documentation
- Review the Rust portfolio module README
- Look at test files for additional examples
