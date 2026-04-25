# finstack-statements

`finstack-statements` is the statement-modeling crate in Finstack. It builds and evaluates period-based financial models, supports deterministic and stochastic forecast methods, exposes a formula DSL, manages reusable metric registries, and integrates capital-structure cashflows into statement formulas.

Higher-level analysis workflows such as DCF, sensitivity analysis, variance analysis, scorecards, and covenant-oriented reports live in `finstack-statements-analytics`.

## Operational Notes

- Built-in metrics are compile-time embedded. Deployments do not need a runtime `data/metrics` directory.
- Capital-structure-aware formulas require `Evaluator::evaluate_with_market(&model, &market_ctx, as_of)`.
- Monte Carlo results preserve path-evaluation warnings when the simulated paths remain finite. Non-finite Monte Carlo path values are treated as hard failures during aggregation.

## Runtime Notes

Monte Carlo path parallelism via Rayon is always enabled; the crate has no
Cargo feature flags. Parallel execution is deterministic: results match a
serial run bit-for-bit given the same seed.

Recommended verification matrix:

```bash
cargo test -p finstack-statements
cargo bench -p finstack-statements --bench statements_operations --no-run
```

## Key Module Docs

- `src/lib.rs` - crate overview, quick start, and module map.
- `src/dsl/mod.rs` - formula DSL operators, function reference, and examples.
- `src/evaluator/mod.rs` - evaluation entry points, precedence, and result conventions.
- `src/capital_structure/mod.rs` - `cs.*` formula namespace and market-context evaluation.
- `data/metrics/README.md` - built-in metric registry conventions.

## Verification

Primary crate verification:

```bash
cargo test -p finstack-statements
cargo bench -p finstack-statements --bench statements_operations --no-run
```
