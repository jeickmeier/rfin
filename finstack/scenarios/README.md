# Finstack Scenarios

Lightweight, deterministic scenario capability for stress testing and what-if analysis.

## Features

- **Market Data Shocks**: FX rates, equity prices, yield curves, volatility surfaces, base correlation
- **Statement Adjustments**: Forecast percent changes and value assignments (Phase A stubs)
- **Deterministic Composition**: Stable ordering with priority-based conflict resolution
- **Serde-Stable Wire Format**: JSON interoperability for pipelines and storage
- **Minimal Dependencies**: Reuses existing `valuations` and `statements` APIs

## Quick Start

```rust
use finstack_scenarios::{ScenarioSpec, OperationSpec, CurveKind, ScenarioEngine, ExecutionContext};
use finstack_core::market_data::MarketContext;
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
- `CurveNodeBp`: Node-specific basis point shifts for curve shaping
- `BaseCorrParallelPts`: Parallel correlation point shift
- `BaseCorrBucketPts`: Bucket-specific correlation shifts
- `VolSurfaceParallelPct`: Parallel volatility percent shift
- `VolSurfaceBucketPct`: Bucket-specific volatility shifts

### Statements (Phase A: Stubs)
- `StmtForecastPercent`: Forecast percent change
- `StmtForecastAssign`: Forecast value assignment

### Attribute-Based (Phase A: Stubs)
- `InstrumentPricePctByAttr`: Price shock by exact attribute match
- `InstrumentSpreadBpByAttr`: Spread shock by exact attribute match

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

## Phase A Limitations

- **FX Shocks**: Stub (FxMatrix is immutable Arc; needs rebuild API)
- **Statement Shocks**: Stub (FinancialModelSpec is wire type; needs evaluator integration)
- **Node-Specific Curve Shocks**: Simplified (applies parallel bumps; tenor matching TODO)
- **Bucket-Specific Shocks**: Simplified (applies parallel shocks; filtering TODO)
- **Attribute Selectors**: Not implemented (no instrument registry query)

## Examples

Run the complete example:

```bash
cargo run -p finstack-scenarios --example scenarios_lite_example
```

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

## Future Enhancements (Post-Phase A)

- Full DSL with text parser and glob/selector expansion
- Complete statement forecast mutation API
- FX shock implementation with mutable FxMatrix
- Proper tenor-based node shock for curve shaping
- Bucket filtering for vol and base correlation shocks
- Instrument registry integration for attribute-based selectors
- Time-windowed operations (`@on`, `@during`)
- Python and WASM bindings

## License

MIT OR Apache-2.0

