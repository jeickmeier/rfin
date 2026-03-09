# Operations Guide

## Required Inputs

### Capital Structure Evaluation

Models with `cs.*` references require:

- `market_ctx`
- `as_of`

Call:

```rust
let results = evaluator.evaluate_with_market_context(&model, Some(&market_ctx), Some(as_of))?;
```

Without both values, capital-structure cashflows are not populated and `cs.*` formulas will fail.

### Corporate DCF

DCF requires:

- `model.meta["currency"]`
- Either `net_debt_override`, or balance-sheet nodes for debt and cash

Expected balance-sheet nodes:

- `total_debt` or `debt`
- `cash` or `cash_and_equivalents`

## Extensions

- `ExtensionRegistry::execute()` validates `context.config` before invoking an extension.
- Built-in extensions can read runtime config from `ExtensionContext::with_config(...)` on targeted `execute(name, ...)` calls.
- `execute_all()` and `execute_all_safe()` intentionally clear `context.config` to avoid spraying one extension-specific blob across unrelated extensions.
- Unknown config fields are rejected for corkscrew and scorecard configs.

## Feature Flags

### `dataframes`

- Enables Polars-backed export APIs such as `StatementResult::to_polars_long()` and Monte Carlo `path_data`.
- Production consumers should enable this only when DataFrame export is part of the runtime contract.

### `parallel`

- Enables Rayon-backed Monte Carlo path evaluation.
- Improves throughput for larger path counts.
- Uses parallel fold/reduce over the Monte Carlo accumulator instead of collecting a full path-result vector first, but still retains path-level storage for percentile and breach calculations.

### Verification Matrix

```bash
cargo test -p finstack-statements
cargo test -p finstack-statements --features dataframes
cargo test -p finstack-statements --features parallel
cargo test -p finstack-statements --features "dataframes parallel"
```

## Diagnostics

Tracing spans now exist around:

- statement evaluation
- Monte Carlo evaluation
- extension execution
- capital-structure waterfall execution

These spans are intended for embedding services that already collect `tracing` output.

### Monte Carlo Diagnostics

- `MonteCarloResults.warnings` preserves warnings emitted while evaluating finite Monte Carlo paths.
- Non-finite Monte Carlo path values are rejected during aggregation and returned as hard errors instead of warning-only diagnostics.

## Result Metadata

- `StatementResult.meta.parallel` reflects only the normal evaluator path, not Monte Carlo execution.
- `StatementResult.meta.rounding_context` is currently reserved and will remain `None` until evaluator-level rounding config is wired.

## Verification Commands

```bash
cargo test -p finstack-statements
cargo test -p finstack-statements --lib
cargo bench -p finstack-statements --bench statements_operations --no-run
```
