# Finstack Portfolio

`finstack-portfolio` provides portfolio construction, valuation aggregation,
grouping, selective repricing, scenario application, margin aggregation, factor
risk decomposition, and optimization on top of the wider Finstack ecosystem.

## What This Crate Covers

- Entity-aware portfolio containers with optional dummy-entity support for
  standalone instruments.
- Position quantity scaling via explicit units such as `Units`, `Notional`,
  `FaceValue`, and `Percentage`.
- Base-currency portfolio valuation and per-position drill-down.
- Portfolio-level metric aggregation, grouping, cashflow ladders, margin, and
  advanced analytics.
- Optional scenario application and Polars DataFrame exports.

## Core Conventions

- `Portfolio::base_ccy` is the reporting currency for totals and portfolio-level
  analytics.
- `Position::quantity` is interpreted by `PositionUnit`; it is not always a
  literal number of units.
- Summable risk metrics are FX-converted to the portfolio base currency before
  aggregation.
- Selective repricing relies on each instrument's declared market dependencies.
  Instruments with unresolved dependencies are repriced conservatively.

## Quick Start

```rust
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_portfolio::position::{Position, PositionUnit};
use finstack_portfolio::types::Entity;
use finstack_portfolio::valuation::value_portfolio;
use finstack_portfolio::PortfolioBuilder;
use finstack_valuations::instruments::rates::deposit::Deposit;
use std::sync::Arc;
use time::macros::date;

# fn main() -> finstack_portfolio::Result<()> {
let as_of = date!(2024-01-01);
let market = MarketContext::new();
let config = FinstackConfig::default();

let deposit = Deposit::builder()
    .id("DEP_1M".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .start_date(as_of)
    .maturity(date!(2024-02-01))
    .day_count(finstack_core::dates::DayCount::Act360)
    .discount_curve_id("USD".into())
    .build()
    .expect("example deposit should build");

let position = Position::new(
    "POS_001",
    "ACME_FUND",
    "DEP_1M",
    Arc::new(deposit),
    1.0,
    PositionUnit::Units,
)?
.with_tag("asset_class", "cash");

let portfolio = PortfolioBuilder::new("MY_FUND")
    .base_ccy(Currency::USD)
    .as_of(as_of)
    .entity(Entity::new("ACME_FUND"))
    .position(position)
    .build()?;

let valuation = value_portfolio(&portfolio, &market, &config, &Default::default())?;
println!("Portfolio total: {}", valuation.total_base_ccy);
# Ok(())
# }
```

## Main Workflows

### Valuation and metrics

Use `value_portfolio` to compute per-position and aggregate PV, then
`aggregate_metrics` to roll up summable risk in base currency.

### Grouping and reporting

Use `aggregate_by_attribute`, `aggregate_by_multiple_attributes`, and
`aggregate_by_book` to roll up results by user tags or book hierarchy.

### Selective repricing

Use `revalue_affected` when only a subset of market factors changed and the
portfolio's dependency index can identify affected positions.

### Scenarios

With the `scenarios` feature enabled, use `apply_scenario` or
`apply_and_revalue` to clone the portfolio, mutate market data, and compute
stressed results.

### Advanced analytics

The crate also exposes:

- `margin` for netting-set and SIMM-style aggregation.
- `factor_model` for factor assignment and risk decomposition.
- `optimization` for deterministic LP-based portfolio optimization.
- `cashflows` for portfolio cashflow ladders and base-currency bucketing.

## Quantity Semantics

`PositionUnit` controls how `Position::scale_value` interprets `quantity`:

- `Units`: direct scaling by number of shares or contracts.
- `Notional(Option<Currency>)`: direct scaling by notional amount. Use `1.0`
  when the instrument already returns a total PV for its configured notional.
- `FaceValue`: direct scaling by held face amount.
- `Percentage`: always interpreted in percentage points, so `50.0` means `50%`
  and is converted internally to `0.50`.

## FX and Reporting Semantics

- Position values are stored both in native currency and portfolio base
  currency.
- Portfolio-level totals and summable metrics are reported in base currency.
- Cashflow conversion helpers use spot-equivalent FX for all dates; for
  forward-sensitive reporting, derive forward FX explicitly outside this crate.
- Attribution distinguishes instrument FX risk from FX translation caused by
  reporting a non-base-currency position in the portfolio base currency.

## Serialization

Use `Portfolio::to_spec` and `Portfolio::from_spec` for JSON-friendly
serialization:

```rust
use finstack_portfolio::portfolio::PortfolioSpec;

# fn round_trip(portfolio: &finstack_portfolio::Portfolio) -> finstack_portfolio::Result<()> {
let spec = portfolio.to_spec();
let json = serde_json::to_string(&spec).expect("serialization should succeed");
let decoded: PortfolioSpec = serde_json::from_str(&json).expect("deserialization should succeed");
let rebuilt = finstack_portfolio::Portfolio::from_spec(decoded)?;
assert_eq!(rebuilt.id, portfolio.id);
# Ok(())
# }
```

Round-trip reconstruction requires each instrument to support
`to_instrument_json()`. If a position serializes with `instrument_spec: None`,
an external instrument registry is required to rebuild it.

## Feature Flags

- `scenarios`: enables scenario application helpers.
- `parallel`: enables rayon-backed portfolio valuation and metric collection in
  selected paths.

## Examples and Verification

- Optimization example:
  `cargo run -p finstack-portfolio --example portfolio_optimization`
- Crate tests:
  `cargo test -p finstack-portfolio`
- Doc tests:
  `cargo test -p finstack-portfolio --doc`

## References

Canonical quantitative and market-convention references used across Finstack
live in [`docs/REFERENCES.md`](../../docs/REFERENCES.md).
