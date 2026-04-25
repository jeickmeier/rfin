# `finstack-statements` benchmark baselines

Reference timings for the production-scale benchmark suite. Use these as
regression detection points: a > 20% degradation on any row is a regression
worth investigating before merging.

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

## Baseline (2026-04-25)

- **Compiler**: `rustc 1.94.0 (stable)`
- **Profile**: `bench` (`opt-level=3`, default Criterion settings)
- **Host**: Apple Silicon (Darwin 25.4.0)

### Monte Carlo path scaling

Forecast model with one stochastic node (`Normal`, seed 42) over 9 quarterly
periods; measures evaluator-overhead-per-path.

| n_paths | wall time         | throughput     |
|---------|-------------------|----------------|
| 100     | **681 µs**        | 147 Kelem/s    |
| 1 000   | **3.80 ms**       | 263 Kelem/s    |
| 5 000   | **15.09 ms**      | 331 Kelem/s    |

Throughput rises with path count because per-run fixed costs (DAG build,
forecast cache warmup) amortise across more paths. Wall time scales roughly
linearly above 1k paths.

### Rolling-window aggregate scaling

Single value series of 24 quarterly observations referenced by *N* rolling-mean
formulas with mixed window sizes (2–7). Measures the per-call cost of the
sorted-history helpers in `formula_helpers.rs` after the per-context Rc cache
landed.

| rolling formulas | wall time | throughput     |
|------------------|-----------|----------------|
| 5                | **73 µs** | 68 Kelem/s     |
| 25               | **273 µs**| 91 Kelem/s     |
| 100              | **939 µs**| 106 Kelem/s    |

Sub-linear growth (5 → 100 = 20× formulas, 13× wall time) confirms the
`Rc<BTreeMap>` cache hits on repeated lookups against the same node.

### Large LBO-shaped model

Monthly periods, four shared drivers fanning out to *N* derived metrics that
mix arithmetic, lag, and `rolling_mean(ebitda, 3)` formulas. Throughput is
node-period evaluations per second.

| nodes × months | wall time      | throughput     |
|----------------|----------------|----------------|
| 50 × 24        | **358 µs**     | 3.35 Melem/s   |
| 100 × 60       | **1.46 ms**    | 4.12 Melem/s   |
| 200 × 60       | **2.97 ms**    | 4.04 Melem/s   |

Throughput holds roughly constant from 100×60 onward, indicating the period ×
node loop is well-behaved at production size.

## When to update this file

- After landing a deliberate optimisation that shifts a baseline > 10%.
- When hardware or compiler MSRV changes (note both at the top of the new
  baseline section).
- Never edit baselines to mask a regression — file a follow-up instead.
