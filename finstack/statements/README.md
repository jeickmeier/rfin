# finstack-statements

`finstack-statements` is the statement-modeling crate in Finstack. It builds and evaluates period-based financial models, supports forecast methods and capital-structure-aware formulas, and provides analysis helpers such as DCF, sensitivity, variance, and Monte Carlo.

## Operational Notes

- Built-in metrics are compile-time embedded. Deployments do not need a runtime `data/metrics` directory.
- Capital-structure-aware formulas require `Evaluator::evaluate_with_market_context()` with both `market_ctx` and `as_of`.
- DCF valuation now fails closed on missing `currency`, `debt`, or `cash` inputs unless the caller provides explicit overrides.
- Extension execution validates `context.config` before running and built-in extensions accept runtime-supplied config.

## Key Docs

- `docs/architecture.md`
- `docs/operations.md`

## Verification

Primary crate verification:

```bash
cargo test -p finstack-statements
cargo bench -p finstack-statements --bench statements_operations --no-run
```
