# finstack-statements

`finstack-statements` is the statement-modeling crate in Finstack. It builds and evaluates period-based financial models, supports forecast methods and capital-structure-aware formulas, and provides analysis helpers such as DCF, sensitivity, variance, and Monte Carlo.

## Operational Notes

- Built-in metrics are compile-time embedded. Deployments do not need a runtime `data/metrics` directory.
- Capital-structure-aware formulas require `Evaluator::evaluate_with_market(&model, &market_ctx, as_of)`.
- DCF valuation now fails closed on missing `currency`, `debt`, or `cash` inputs unless the caller provides explicit overrides.
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

## Key Docs

- `docs/architecture.md`
- `docs/operations.md`

## Verification

Primary crate verification:

```bash
cargo test -p finstack-statements
cargo bench -p finstack-statements --bench statements_operations --no-run
```
