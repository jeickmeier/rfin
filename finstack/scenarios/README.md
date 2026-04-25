# Finstack Scenarios

`finstack-scenarios` provides deterministic scenario composition and scenario
application for market data, statement models, instrument-level shocks, and
time roll-forward workflows.

## What This Crate Covers

The crate centers on a small set of core types:

- `ScenarioSpec`: a named scenario with metadata and ordered operations.
- `OperationSpec`: the supported shock and roll-forward operations.
- `ScenarioEngine`: composition and application engine.
- `ExecutionContext`: mutable application context for market data, statement
  models, rate bindings, and the as-of date.
- `ApplicationReport`: warnings and execution details returned by application.
- `RateBindingSpec`: optional bridge between statement nodes and market curves,
  applied through `OperationSpec::RateBinding`.

## Supported Operation Families

### Market data shocks

- FX shocks
- equity price shocks
- discount, forward, hazard, and inflation curve shifts
- base-correlation shifts
- volatility-surface shifts

### Statement operations

- forecast percentage changes
- forecast value assignment
- curve-linked rate bindings for statement workflows

### Instrument operations

- price shocks by instrument type
- spread shocks by instrument type
- attribute-based selectors where the underlying workflow provides the needed
  instrument metadata

### Time operations

- horizon roll-forward with carry and theta-aware workflows

## Composition Semantics

- Scenarios compose deterministically.
- Operation ordering is stable.
- Priority values control merge behavior.
- The wire format is serde-friendly for storage and pipelines.
- Market, statement, and time operations share one application model instead of
  separate ad hoc engines.

## Where It Fits

`finstack-scenarios` sits between the lower-level crates and the binding
surfaces:

- It uses `finstack-core` for market data and dates.
- It integrates with `finstack-statements` for statement-model workflows.
- It integrates with `finstack-valuations` for pricing-aware scenario and
  roll-forward flows.
- It is exposed through both `finstack-py` and `finstack-wasm`.

## Typical Usage

Most applications follow this pattern:

1. Build a `ScenarioSpec` from one or more `OperationSpec` values.
2. Construct an `ExecutionContext` with market data, a statement model, and an
   `as_of` date.
3. Apply the scenario with `ScenarioEngine`.
4. Consume the resulting `ApplicationReport` alongside the mutated context.

## Verification

```bash
cargo fmt -p finstack-scenarios
cargo clippy -p finstack-scenarios --all-targets -- -D warnings
cargo test -p finstack-scenarios
```

## License

MIT OR Apache-2.0
