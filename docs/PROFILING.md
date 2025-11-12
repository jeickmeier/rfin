# Performance Profiling Guide

This guide covers tools and techniques for profiling the Finstack library to identify performance bottlenecks and optimization opportunities.

## Quick Start

```bash
# Run benchmarks with baseline comparison
make bench-baseline         # Save baseline
# ... make changes ...
cargo bench -- --baseline main

# Profile with flamegraph
make bench-flamegraph

# Profile heap allocations with dhat
cargo build --profile release-perf --features dhat-heap
# Run your scenario (requires dhat instrumentation in code)
```

## Profiling Tools

### 1. Flamegraphs (CPU Profiling)

Flamegraphs visualize where CPU time is spent, showing call stacks and hotspots.

#### Installation

```bash
# Install flamegraph tool (requires perf on Linux)
cargo install flamegraph

# On macOS, also install dtrace-based profiler
# (dtrace is built-in, no install needed)
```

#### Usage

```bash
# Profile a benchmark
cargo flamegraph --bench mc_pricing --profile bench -- --bench

# Profile a specific test
cargo flamegraph --test integration_tests --profile release-perf

# Profile with custom output
cargo flamegraph --bench bond_pricing --profile bench -o bond_flame.svg -- --bench

# Interactive profiling
# Open the generated flamegraph.svg in a browser
```

#### Reading Flamegraphs

- **Width**: Proportional to time spent in that function
- **Color**: Random (only for visual distinction)
- **Stack depth**: Call stack depth from bottom (entry) to top (leaves)
- **Click**: Zoom into a specific function

Look for:
- Wide bars (high CPU time)
- Unexpected call stacks
- Repeated clones or allocations
- Deep call chains (potential inlining opportunities)

### 2. dhat (Heap Profiling)

dhat tracks heap allocations, showing total bytes allocated, live bytes, and allocation sites.

#### Installation

dhat is already added as an optional dependency in `Cargo.toml`.

#### Usage

**Step 1**: Add dhat instrumentation to your code:

```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    
    // Your code here
    run_pricing_scenario();
}
```

**Step 2**: Build and run with dhat feature:

```bash
# Build with dhat enabled
cargo build --profile release-perf --features dhat-heap

# Run the binary
./target/release-perf/your_scenario

# This generates dhat-heap.json
```

**Step 3**: View results:

```bash
# Install dhat viewer (Python)
pip install dhat

# View results
dh_view.py dhat-heap.json

# Or use the web viewer
# Upload dhat-heap.json to: https://nnethercote.github.io/dh_view/dh_view.html
```

#### Metrics to Watch

- **Total bytes**: All allocations (even if freed)
- **Total blocks**: Number of allocation calls
- **Peak bytes**: Maximum heap usage
- **At-t-end bytes**: Live allocations at program end
- **Reads/Writes**: Memory access patterns

### 3. Criterion Benchmarks

Criterion provides statistical benchmarking with regression detection.

#### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark group
cargo bench --bench mc_pricing --features mc

# Save baseline for comparison
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main

# Generate detailed report
cargo bench -- --verbose
```

#### Benchmark Results

Criterion generates:
- `target/criterion/`: HTML reports with plots
- Console output with statistical analysis
- Regression warnings if performance degrades

Key metrics:
- **time**: Wall-clock time per iteration
- **thrpt**: Throughput (iterations/second or elements/second)
- **change**: % change vs baseline (with confidence interval)

#### Writing Effective Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_pricing(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_pricing");
    
    // Set sample size for statistical power
    group.sample_size(100);
    
    // Set significance level for regression detection
    group.significance_level(0.05);
    
    // Set noise threshold
    group.noise_threshold(0.03);
    
    // Benchmark with multiple parameter values
    for tenor in [2, 5, 10, 30] {
        group.bench_with_input(
            BenchmarkId::from_parameter(tenor),
            &tenor,
            |b, &t| {
                b.iter(|| {
                    // Use black_box to prevent compiler optimizations
                    black_box(price_bond(black_box(t)))
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_pricing);
criterion_main!(benches);
```

### 4. perf (Linux Only)

For low-level CPU profiling on Linux:

```bash
# Record
cargo build --profile bench
perf record --call-graph=dwarf ./target/bench/your_binary

# Report
perf report

# Generate flamegraph from perf data
perf script | stackcollapse-perf.pl | flamegraph.pl > perf.svg
```

### 5. Cachegrind (Cache Profiling)

Analyze cache hits/misses and memory access patterns:

```bash
# Install valgrind
# On macOS: not natively supported
# On Linux:
sudo apt install valgrind

# Run cachegrind
cargo build --profile release-perf
valgrind --tool=cachegrind ./target/release-perf/your_binary

# View results
cg_annotate cachegrind.out.<pid>
```

## Common Profiling Workflows

### Workflow 1: Find CPU Hotspots

```bash
# 1. Run baseline benchmarks
cargo bench -- --save-baseline before

# 2. Generate flamegraph
cargo flamegraph --bench mc_pricing --profile bench -- --bench

# 3. Identify hot functions (wide bars)
# 4. Optimize the hot path
# 5. Compare benchmarks
cargo bench -- --baseline before
```

### Workflow 2: Reduce Allocations

```bash
# 1. Profile heap with dhat
cargo build --profile release-perf --features dhat-heap --bin scenario
./target/release-perf/scenario

# 2. View dhat results
dh_view.py dhat-heap.json

# 3. Identify allocation hotspots
# Look for: High "total bytes", high "total blocks"

# 4. Fix allocations (use with_capacity, reuse buffers, etc.)

# 5. Re-run and compare
./target/release-perf/scenario
dh_view.py dhat-heap.json
```

### Workflow 3: Parallel Scaling

```bash
# Benchmark with varying thread counts
for threads in 1 2 4 8; do
    RAYON_NUM_THREADS=$threads cargo bench --bench mc_pricing -- parallel
done

# Check for:
# - Near-linear scaling (2x threads ≈ 2x throughput)
# - Lock contention (poor scaling)
# - Overhead from small chunks
```

## Build Profiles for Profiling

Different profiles balance debuggability and performance:

| Profile | Use Case | Command |
|---------|----------|---------|
| `bench` | Benchmarking | `cargo bench` |
| `release-perf` | Production profiling | `cargo build --profile release-perf` |
| `release` | WASM (size-optimized) | `cargo build --release` |

Key differences:
- `bench`: opt-level=3, lto=thin, debug=1 (line info for flamegraphs)
- `release-perf`: opt-level=3, lto=thin (fastest)
- `release`: opt-level=z, lto=full (smallest)

## Interpreting Results

### Good Performance Indicators

✅ Flat allocation count across path increases (good pre-allocation)
✅ CPU time scales linearly with work (no quadratic algorithms)
✅ Parallel speedup close to # threads (no lock contention)
✅ Flamegraph shows domain logic, not infrastructure

### Red Flags

⚠️ Excessive `clone()` calls in hot paths
⚠️ `Vec::new()` or `HashMap::new()` in loops (missing `with_capacity`)
⚠️ Deep call stacks from heavy monomorphization
⚠️ Lock contention (Mutex/RwLock appears in flamegraph)
⚠️ Poor parallel scaling (<50% efficiency)

## Performance Targets

Based on benchmark suite (on modern CPU, 4+ cores):

| Operation | Target p50 | Target p95 | Notes |
|-----------|-----------|-----------|-------|
| Bond cashflow (30Y) | <20μs | <50μs | Semi-annual fixed |
| IRS cashflow (30Y) | <30μs | <80μs | Floating leg |
| MC European (100k, serial) | <100ms | <250ms | GBM, 252 steps |
| MC Heston (50k, serial) | <200ms | <500ms | Stochastic vol |
| Structured credit waterfall | <500μs | <2ms | Per payment date |

## Tips and Best Practices

### Minimize Noise

- Close background apps
- Use `nice -n -20` for higher priority
- Disable CPU frequency scaling (Linux):
  ```bash
  echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
  ```

### Reproducible Benchmarks

- Use fixed seeds for stochastic models
- Run with `--sample-size 200` for better statistics
- Compare against saved baselines, not point-in-time runs

### Profiling in CI/CD

```yaml
# .github/workflows/perf-check.yml
name: Performance Check
on: [pull_request]
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo bench -- --save-baseline pr
      - run: cargo bench -- --baseline main --threshold 5
```

## Further Reading

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [dhat Documentation](https://docs.rs/dhat/)
- [Flamegraph Documentation](https://github.com/flamegraph-rs/flamegraph)

## Troubleshooting

### Flamegraph Not Generating

**macOS**: Ensure you have dtrace permissions:
```bash
sudo dtruss ls  # Test dtrace access
# If denied, grant Terminal full disk access in System Preferences
```

**Linux**: Install perf:
```bash
sudo apt install linux-tools-generic
```

### Benchmarks Too Variable

- Increase sample size: `cargo bench -- --sample-size 500`
- Check for background processes
- Use noise_threshold in criterion config

### dhat Not Producing Output

- Verify feature is enabled: `cargo tree -f "{p} {f}"`
- Check that `_profiler` is not dropped early
- Ensure program runs to completion

