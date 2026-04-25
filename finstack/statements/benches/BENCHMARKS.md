# `finstack-statements` Benchmarks

Production-scale benchmark entry points for evaluator, Monte Carlo, aggregate,
and capital-structure hot paths.

## How to reproduce

```bash
cargo bench -p finstack-statements --bench statements_scale
```

Re-run after meaningful changes to the evaluator hot path
(`evaluator/{engine,formula,formula_dispatch,formula_aggregates,formula_helpers}`),
the historical cache (`evaluator/context.rs`), the Monte Carlo loop
(`evaluator/monte_carlo.rs`), or the capital-structure waterfall.

The smaller `statements_operations` bench remains the place to track
correctness-sized models (4–24 periods × ≤50 nodes).

Keep machine-specific timing baselines in Criterion output or CI artifacts.
