# Finstack Python Bindings Performance Benchmarks

This directory contains performance benchmarks for the finstack Python bindings, designed to measure FFI (Foreign Function Interface) overhead and identify optimization opportunities.

## Overview

The benchmarks measure three critical areas:

1. **Bond Pricing** (`test_bond_pricing.py`)
   - Price 1000 bonds sequentially
   - Price 100 bonds with full metrics (DV01, duration, etc.)
   - Measure bond construction overhead
   - Baseline single bond pricing

2. **Curve Calibration** (`test_curve_calibration.py`)
   - Calibrate 50-pillar discount curve from market quotes
   - Calibrate 10-pillar curve (baseline)
   - Multi-curve calibration (3 curves simultaneously)

3. **Statement Evaluation** (`test_statement_evaluation.py`)
   - Evaluate 100-node financial model over 60 periods
   - Evaluate 20-node model (baseline)
   - Measure model construction overhead
   - Single period evaluation

## Running Benchmarks

### Prerequisites

Install pytest-benchmark:

```bash
pip install pytest-benchmark
```

Or use the dev dependencies:

```bash
uv sync --dev
```

### Run All Benchmarks

```bash
# Run all benchmarks
pytest benches/ --benchmark-only -v

# Run with HTML report
pytest benches/ --benchmark-only --benchmark-autosave --benchmark-save-data

# Compare with previous runs
pytest benches/ --benchmark-only --benchmark-compare
```

### Run Specific Benchmarks

```bash
# Bond pricing only
pytest benches/test_bond_pricing.py --benchmark-only -v

# Curve calibration only
pytest benches/test_curve_calibration.py --benchmark-only -v

# Statement evaluation only
pytest benches/test_statement_evaluation.py --benchmark-only -v

# Specific test
pytest benches/test_bond_pricing.py::TestBondPricingBenchmarks::test_bench_price_1000_bonds --benchmark-only -v
```

### Benchmark Options

```bash
# Save results with custom name
pytest benches/ --benchmark-only --benchmark-autosave --benchmark-save=my_run

# Compare against baseline
pytest benches/ --benchmark-only --benchmark-compare=0001

# Sort by mean time
pytest benches/ --benchmark-only --benchmark-sort=mean

# Show only top N slowest
pytest benches/ --benchmark-only --benchmark-columns=min,max,mean,stddev,median

# Generate histogram
pytest benches/ --benchmark-only --benchmark-histogram=histogram
```

## Interpreting Results

### Key Metrics

- **Min**: Fastest execution time (best case)
- **Max**: Slowest execution time (worst case)
- **Mean**: Average execution time
- **StdDev**: Standard deviation (lower is more consistent)
- **Median**: Middle value (less affected by outliers)
- **IQR**: Interquartile range (middle 50% of data)
- **OPS**: Operations per second (higher is better)

### Performance Targets

Based on the task requirements, we aim for:

**Python overhead <10% vs pure Rust for batch operations**

To measure this, compare:

- Python batch operation time
- Equivalent Rust benchmark time (from `cargo bench`)
- Overhead = (Python time - Rust time) / Rust time

### Example Output

```
----------------------------------------------------------------------------------------------------------
Name (time in ms)                                  Min        Max       Mean     StdDev     Median      IQR
----------------------------------------------------------------------------------------------------------
test_bench_price_1000_bonds                     45.23      52.18      47.56       2.34      47.12     3.21
test_bench_calibrate_50_pillar_curve           123.45     145.67     132.11       8.92     130.45    12.34
test_bench_evaluate_100_node_model              89.12     102.34      94.23       5.67      93.01     7.89
----------------------------------------------------------------------------------------------------------
```

## Profiling

For deeper analysis, use py-spy or cProfile:

### Using py-spy (recommended for production profiling)

```bash
# Install py-spy
pip install py-spy

# Profile a specific benchmark
py-spy record -o profile.svg --native -- pytest benches/test_bond_pricing.py::TestBondPricingBenchmarks::test_bench_price_1000_bonds --benchmark-only

# View the flamegraph
open profile.svg
```

### Using cProfile

```bash
# Profile with cProfile
python -m cProfile -o profile.stats -m pytest benches/test_bond_pricing.py --benchmark-only

# View results
python -m pstats profile.stats
> sort cumulative
> stats 20
```

### Using pytest-profiling

```bash
# Install pytest-profiling
pip install pytest-profiling

# Run with profiling
pytest benches/ --profile --profile-svg

# View results
open prof/combined.svg
```

## Optimization Strategies

Based on profiling results, consider these optimization techniques:

### 1. Release GIL for Heavy Computation

In Rust code, use `py.allow_threads()`:

```rust
#[pyfunction]
fn price_bond(py: Python, bond: &PyBond, market: &PyMarketContext) -> PyResult<f64> {
    // Release GIL for heavy computation
    py.allow_threads(|| {
        // Pure Rust computation here
        let result = bond.inner.price(&market.inner);
        result
    })
}
```

### 2. Vectorize Batch Operations

Instead of:

```python
# Slow: Individual FFI calls
for bond in bonds:
    result = price_bond(bond, market)
```

Use:

```python
# Fast: Single FFI call for batch
results = price_bonds_batch(bonds, market)
```

### 3. Zero-Copy Conversions

Use PyO3's buffer protocol for NumPy arrays:

```rust
use numpy::PyArray1;

#[pyfunction]
fn process_array(py: Python, arr: &PyArray1<f64>) -> PyResult<Vec<f64>> {
    // Zero-copy access to NumPy data
    let slice = unsafe { arr.as_slice()? };
    // Process without copying
    Ok(slice.iter().map(|&x| x * 2.0).collect())
}
```

### 4. Cache Expensive Conversions

Cache Python objects that are converted to Rust frequently:

```python
# Cache market data instead of recreating
market = create_market_data()  # Once
for bond in bonds:
    result = price_bond(bond, market)  # Reuse market
```

## Continuous Integration

Add benchmarks to CI/CD to track performance over time:

```yaml
# .github/workflows/benchmarks.yml
name: Benchmarks

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Set up Python
        uses: actions/setup-python@v2
        with:
          python-version: 3.12
      - name: Install dependencies
        run: |
          pip install pytest pytest-benchmark
          pip install -e .
      - name: Run benchmarks
        run: |
          pytest benches/ --benchmark-only --benchmark-autosave
      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: .benchmarks/
```

## Benchmarking Best Practices

1. **Warmup**: pytest-benchmark automatically handles warmup rounds
2. **Consistent Environment**: Run on the same hardware, same Python version
3. **Disable CPU Scaling**: For more consistent results

   ```bash
   # Linux
   sudo cpupower frequency-set --governor performance
   ```

4. **Close Other Applications**: Minimize background processes
5. **Multiple Runs**: Use `--benchmark-min-rounds=10` for statistical significance
6. **Version Control**: Save benchmark data with git to track trends

## Troubleshooting

### Benchmarks Too Fast

If benchmarks complete in <1ms, pytest-benchmark will warn. Increase workload size.

### High Variance

If StdDev is >10% of Mean:

- Close background applications
- Increase rounds: `--benchmark-min-rounds=100`
- Check for system throttling (CPU frequency scaling)

### Out of Memory

For large benchmarks:

- Reduce batch size
- Run benchmarks individually
- Increase system memory or use swap

## References

- [pytest-benchmark documentation](https://pytest-benchmark.readthedocs.io/)
- [py-spy profiler](https://github.com/benfred/py-spy)
- [PyO3 performance guide](https://pyo3.rs/latest/performance.html)
- [Python profiling guide](https://docs.python.org/3/library/profile.html)
