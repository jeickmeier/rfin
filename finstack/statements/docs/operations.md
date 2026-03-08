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
- Built-in extensions can read runtime config from `ExtensionContext::with_config(...)`.
- Unknown config fields are rejected for corkscrew and scorecard configs.

## Diagnostics

Tracing spans now exist around:

- statement evaluation
- Monte Carlo evaluation
- extension execution
- capital-structure waterfall execution

These spans are intended for embedding services that already collect `tracing` output.

## Verification Commands

```bash
cargo test -p finstack-statements
cargo test -p finstack-statements --lib
cargo bench -p finstack-statements --bench statements_operations --no-run
```
