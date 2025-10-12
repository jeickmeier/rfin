# Finstack Portfolio

Portfolio management and aggregation for the Finstack ecosystem.

## Features

- **Entity-based position tracking**: Organize positions under entities (companies, funds) with support for standalone instruments via dummy entity
- **Flat position structure**: Simple Vec-based position storage with flexible attribute-based grouping
- **Multi-instrument support**: Works with any instrument from `finstack-valuations` (deposits, bonds, swaps, options, etc.)
- **Cross-currency aggregation**: Automatic FX conversion to portfolio base currency using `FxMatrix`
- **Valuation & metrics**: Value all positions and aggregate metrics (DV01, CS01, Delta, etc.)
- **Attribute-based grouping**: Group and aggregate by any position tag (rating, sector, instrument type, etc.)
- **Scenario integration**: Apply scenarios to portfolios and re-value (requires `scenarios` feature)
- **DataFrame exports**: Export results to Polars DataFrames for analysis

## Quick Start

```rust
use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit, Entity, DUMMY_ENTITY_ID};
use finstack_portfolio::value_portfolio;
use finstack_core::prelude::*;
use finstack_valuations::instruments::deposit::Deposit;
use std::sync::Arc;
use time::macros::date;

// Create market data
let market = MarketContext::new();
let config = FinstackConfig::default();

// Create a deposit instrument
let deposit = Deposit::builder()
    .id("DEP_1M".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .start(date!(2024-01-01))
    .end(date!(2024-02-01))
    .day_count(DayCount::Act360)
    .disc_id("USD".into())
    .build()
    .unwrap();

// Create a position
let position = Position::new(
    "POS_001",
    DUMMY_ENTITY_ID, // Standalone instrument
    "DEP_1M",
    Arc::new(deposit),
    1.0,
    PositionUnit::Units,
).with_tag("rating", "AAA");

// Build portfolio
let portfolio = PortfolioBuilder::new("MY_FUND")
    .base_ccy(Currency::USD)
    .as_of(date!(2024-01-01))
    .position(position)
    .build()
    .unwrap();

// Value the portfolio
let valuation = value_portfolio(&portfolio, &market, &config).unwrap();
println!("Total value: {}", valuation.total_base_ccy);
```

## Architecture

### Entity Model

Portfolios use an entity-based structure:
- **Entity**: Represents a company, fund, or legal entity that owns positions
- **Dummy Entity**: Special entity (`DUMMY_ENTITY_ID`) for standalone instruments (derivatives, FX, etc.)
- **Position**: Links an entity to an instrument with quantity and tags

### Position Units

Positions support multiple unit types:
- `Units`: Number of shares/contracts (for equities, options)
- `Notional(Currency)`: Notional amount (for derivatives, FX)
- `FaceValue`: Face value of debt (for bonds, loans)
- `Percentage`: Ownership percentage

### Valuation Flow

1. **Price instruments**: Call `instrument.value()` for each position
2. **Scale by quantity**: Multiply unit value by position quantity
3. **Cross-currency conversion**: Convert to portfolio base currency using `FxMatrix`
4. **Aggregation**: Sum by entity and compute portfolio total

### Metrics Aggregation

Metrics are classified as:
- **Summable**: DV01, CS01, Delta, Gamma, Vega, Theta, etc. (aggregate across positions)
- **Non-summable**: YTM, Duration, Spread, etc. (store by position only)

## Scenario Support

Apply market scenarios to portfolios (requires `scenarios` feature):

```rust
use finstack_scenarios::spec::{ScenarioSpec, OperationSpec, CurveKind};

let scenario = ScenarioSpec {
    id: "stress_test".to_string(),
    operations: vec![
        OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD".to_string(),
            bp: 50.0,
        },
    ],
    ..Default::default()
};

let (stressed_valuation, report) = 
    apply_and_revalue(&portfolio, &scenario, &market, &config)?;
```

## Attribute-Based Grouping

Group positions and aggregate values by any tag:

```rust
use finstack_portfolio::aggregate_by_attribute;

// Group by rating
let by_rating = aggregate_by_attribute(
    &valuation,
    &portfolio.positions,
    "rating",
    portfolio.base_ccy,
)?;

for (rating, total) in &by_rating {
    println!("{}: {}", rating, total);
}
```

## DataFrame Exports

Export results to Polars DataFrames:

```rust
use finstack_portfolio::dataframe::{positions_to_dataframe, entities_to_dataframe};

// Position-level data
let df_positions = positions_to_dataframe(&valuation)?;
// Columns: position_id, entity_id, value_native, value_base, currency_native, currency_base

// Entity-level aggregates
let df_entities = entities_to_dataframe(&valuation)?;
// Columns: entity_id, total_value, currency
```

## Future Enhancements

The following features are planned for future releases:
- **Full metrics computation**: Integrate with `price_with_metrics` for complete risk measures
- **Statement aggregation**: Attach financial models to entities and aggregate statements
- **Book hierarchy**: Optional nested book/folder structure for organization
- **Performance optimization**: Parallel valuation and caching

## Examples

See `examples/rust/portfolio_example.rs` for a comprehensive example demonstrating:
- Entity-based and standalone positions
- Cross-currency aggregation
- Attribute-based grouping
- Scenario application
- DataFrame exports

Run with:
```bash
cargo run --example portfolio_example
```

## Testing

Run tests with:
```bash
cargo test --package finstack-portfolio
```

All tests include market data setup and demonstrate real valuation workflows.

## Dependencies

- `finstack-core`: Foundation types, dates, money, FX
- `finstack-valuations`: Instrument pricing and metrics
- `finstack-scenarios`: Scenario engine (optional, enabled by default)
- `finstack-statements`: Financial models (optional, for future statement aggregation)

## License

Licensed under Apache-2.0, consistent with the Finstack ecosystem.

