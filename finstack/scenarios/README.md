# Finstack Scenarios

Lightweight, deterministic scenario capability for stress testing and what-if analysis.

## Features

- **Market Data Shocks**: FX, equities, yield curves, vol surfaces, base correlation
- **Statement Adjustments**: Forecast percent changes and value assignments
- **Rate Bindings**: Curve-to-statement links with tenor, compounding, and day-count awareness
- **Attribute/Type Instrument Shocks**: Price/spread shocks by instrument type or metadata filters
- **Deterministic Composition**: Stable ordering with priority-based conflict resolution
- **Serde-Stable Wire Format**: JSON interoperability for pipelines and storage
- **Minimal Dependencies**: Reuses existing `valuations` and `statements` APIs

## Quick Start

```rust
use finstack_scenarios::{ScenarioSpec, OperationSpec, CurveKind, ScenarioEngine, ExecutionContext};
use finstack_core::market_data::context::MarketContext;
use finstack_statements::FinancialModelSpec;

let mut market = MarketContext::new(); // with curves, prices, etc.
let mut model = FinancialModelSpec::new("model_id", vec![]);

let scenario = ScenarioSpec {
    id: "stress_test".into(),
    name: Some("Q1 Stress Test".into()),
    description: None,
    operations: vec![
        OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD_SOFR".into(),
            bp: 50.0, // +50bp parallel shift
        },
        OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: -10.0, // -10% equity shock
        },
    ],
    priority: 0,
};

let engine = ScenarioEngine::new();
let mut ctx = ExecutionContext {
    market: &mut market,
    model: &mut model,
    rate_bindings: None,
    as_of: time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
};

let report = engine.apply(&scenario, &mut ctx)?;
println!("Applied {} operations", report.operations_applied);
```

## Supported Operations

### Market Data
- `MarketFxPct`: FX rate percent shift
- `EquityPricePct`: Equity price percent shock
- `CurveParallelBp`: Parallel basis point shift (discount/forecast/hazard/inflation curves)
- `CurveNodeBp`: Node-specific basis point shifts with tenor matching (exact or interpolate)
- `BaseCorrParallelPts`: Parallel correlation point shift
- `BaseCorrBucketPts`: Bucket-specific correlation shifts (filters by detachment points)
- `VolSurfaceParallelPct`: Parallel volatility percent shift
- `VolSurfaceBucketPct`: Bucket-specific volatility shifts (filters by tenor and strike)

### Statements
- `StmtForecastPercent`: Forecast percent change
- `StmtForecastAssign`: Forecast value assignment
- `RateBinding` (via context): Bind statement nodes to curves with tenor/compounding/day-count

### Instrument-Based
- `InstrumentPricePctByAttr`: Price shock by attribute match (case-insensitive AND on metadata)
- `InstrumentSpreadBpByAttr`: Spread shock by attribute match (case-insensitive AND on metadata)
- `InstrumentPricePctByType`: Price shock by instrument type (Bond, CDS, Swap, etc.)
- `InstrumentSpreadBpByType`: Spread shock by instrument type

### Time Operations
- `TimeRollForward`: Roll forward horizon by period with carry/theta calculation
  - Modes: `business_days` (default), `calendar_days`, `approximate`

## Architecture

```
ScenarioEngine
  ├─ compose(scenarios) → deterministic merge
  └─ apply(scenario, ctx) → ApplicationReport
       ├─ Phase 1: Market data (FX, equities, vols, curves)
       ├─ Phase 2: Rate bindings (optional)
       ├─ Phase 3: Statement operations
       └─ Phase 4: Re-evaluation
```

## Implementation Status

### Fully Implemented
- ✅ **Tenor-Based Curve Shocks**: Exact pillar matching and interpolated key-rate bumps
- ✅ **Bucket Filtering**: Vol surfaces filter by tenor/strike; base-corr by detachment
- ✅ **Instrument Type Shocks**: Type-safe shocks using InstrumentType enum
- ✅ **Time Roll-Forward**: Date advancement with carry/theta from valuations crate
- ✅ **FX Shocks**: Full implementation via SimpleFxProvider replacement
- ✅ **Statement Shocks**: Percent and assign operations on node values

### Phase A Limitations
- **Attribute Selectors**: Not implemented (no instrument registry query)

## Examples

Run the examples:

```bash
# Lite example - Basic usage with horizon scenarios
cargo run -p finstack-scenarios --example scenarios_lite_example

# Comprehensive example - All shock types including horizon analysis
cargo run -p finstack-scenarios --example scenarios_comprehensive_example
```

Both examples now demonstrate:
- Market data shocks (curves, equity, vol, FX)
- Statement adjustments
- **Horizon scenarios**: 1W, 1M, 3M time roll-forward with theta/carry calculations
- Combined scenarios: Horizon + market shocks

## Testing

```bash
# Unit and integration tests
cargo test -p finstack-scenarios --all-features

# Linting
cargo clippy -p finstack-scenarios --all-features -- -D warnings
```

## Design Goals

1. **Determinism**: Identical results across runs and platforms
2. **Composability**: Merge scenarios with stable priority resolution
3. **Simplicity**: Programmatic API first; DSL deferred to future phases
4. **Reusability**: Leverage existing `core`, `valuations`, and `statements` features
5. **Stability**: Serde-stable wire types for long-lived pipelines

## Future Enhancements

- Full DSL with text parser and glob/selector expansion
- Instrument registry integration for attribute-based selectors
- Time-windowed operations (`@on`, `@during`)
- Curve rolling with proper knot expiry/adjustment
- Python and WASM bindings

## License

MIT OR Apache-2.0
