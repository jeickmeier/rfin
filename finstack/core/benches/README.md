# Finstack Core Benchmarks

This directory contains Criterion benchmark suites for `finstack-core`.

The benchmark sources are the ground truth. This README explains what is
measured and how to run the suites. It intentionally avoids hard latency,
allocation, or "all targets met" claims unless you have current benchmark
results to back them up.

## Running Benchmarks

```bash
# Run all core benchmarks
cargo bench --package finstack-core

# Run selected suites
cargo bench --package finstack-core --bench money_operations
cargo bench --package finstack-core --bench daycount_operations
cargo bench --package finstack-core --bench calendar_operations
cargo bench --package finstack-core --bench interpolation
cargo bench --package finstack-core --bench curve_operations
cargo bench --package finstack-core --bench expr_dag
cargo bench --package finstack-core --bench rolling
cargo bench --package finstack-core --bench solver_operations
cargo bench --package finstack-core --bench market_context
cargo bench --package finstack-core --bench vol_surface
cargo bench --package finstack-core --bench integration
cargo bench --package finstack-core --bench statistical_functions

# Compile benchmark targets without running them
cargo bench --package finstack-core --bench expr_dag --bench market_context --no-run

# Save and compare Criterion baselines
cargo bench --package finstack-core -- --save-baseline baseline_name
cargo bench --package finstack-core -- --baseline baseline_name
```

## Benchmark Coverage

### `money_operations.rs`

- Money construction and arithmetic
- FX-backed conversion paths
- Batch monetary operations
- Formatting overhead

### `daycount_operations.rs`

- Year-fraction calculations across supported day-count conventions
- Batch date-period calculations
- More complex conventions such as `ActActIsma` and `Bus252`

### `calendar_operations.rs`

- Holiday and business-day checks
- Business-day adjustments
- Composite calendar behavior
- Batch date checks and counting loops

### `interpolation.rs`

- Single-point and batch interpolation
- Interpolation style comparisons
- Extrapolation behavior

### `curve_operations.rs`

- Discount, forward, and hazard curve lookup costs
- Batch evaluation across multiple tenors
- Curve construction overhead

### `expr_dag.rs`

- Complex DAG evaluation without planning
- Planned DAG execution
- Cached DAG execution
- Row-scaling behavior for larger series

### `rolling.rs`

- Rolling mean, median, and standard deviation
- Different data sizes and window sizes
- Repeated expression-evaluation overhead for rolling operators

### `solver_operations.rs`

- Newton and Brent root finding
- IRR/XIRR solver paths
- Multi-dimensional solver scenarios where present

### `market_context.rs`

- Typed curve and surface lookups
- Batch lookups
- Context cloning
- Bump operations
- Credit-index rebinding-sensitive bump scenarios

### `vol_surface.rs`

- Vol-surface construction
- Interpolation and boundary handling
- Bump operations

### `integration.rs`

- Simpson, adaptive Simpson, trapezoidal, Gauss-Legendre, and Gauss-Hermite paths

### `statistical_functions.rs`

- Distribution helpers
- Random sampling utilities
- Basic statistics and probability primitives

## Reading Results

Criterion writes results under `target/criterion/`. Useful outputs include:

- Terminal summaries with confidence intervals
- HTML reports in `target/criterion/*/report/index.html`
- Raw measurement data under each benchmark directory

## Evidence Standard

Use current benchmark output, not this README, to make performance claims.

Recommended workflow:
1. Compile touched benchmark targets with `--no-run` during refactoring.
2. Run the relevant suites on the current branch.
3. Save a baseline before larger changes.
4. Compare against that baseline after the change.
5. Record any release-note or README performance claims only after those results exist.

## Notes

- Benchmarks run under Cargo's benchmark profile.
- Results vary by hardware, toolchain, thermal state, and background load.
- `black_box()` is used to reduce optimizer distortion.
- If you add a new benchmark suite, update this README with what it measures, not with guessed numbers or stale target values.
