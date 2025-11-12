# Performance Guide

This document describes build profiles, optimization strategies, and profiling techniques for the Finstack library.

## Build Profiles

Finstack provides multiple build profiles optimized for different use cases:

### Development (`dev`)
```bash
cargo build
```
- Fast compilation
- Full debug symbols
- No optimizations
- Use for: Active development, debugging

### Testing (`test`)
```bash
cargo test
```
- Fast compilation with minimal debug info
- No optimizations
- Use for: Unit tests, integration tests

### Release (`release`)
```bash
cargo build --release
# or for WASM:
cd finstack-wasm && wasm-pack build
```
- **Optimized for size** (`opt-level = "z"`)
- Full LTO for minimal bundle size
- Single codegen unit
- Use for: WASM deployments, web applications

### Performance (`release-perf`)
```bash
cargo build --profile release-perf
```
- **Optimized for speed** (`opt-level = 3`)
- Thin LTO for fast builds with good optimization
- 8 codegen units for parallel compilation
- Use for: CPU-intensive workloads, pricing engines, risk calculations

### Benchmarking (`bench`)
```bash
cargo bench
```
- Optimized for speed (`opt-level = 3`)
- Thin LTO
- Line-level debug info for profiling
- Automatically used by `cargo bench`
- Use for: Performance measurements, profiling

## When to Use Each Profile

| Use Case | Profile | Command |
|----------|---------|---------|
| Web/WASM deployment | `release` | `cargo build --release` |
| Pricing server | `release-perf` | `cargo build --profile release-perf` |
| Risk batch jobs | `release-perf` | `cargo build --profile release-perf` |
| Benchmarking | `bench` | `cargo bench` |
| Performance profiling | `bench` | `cargo flamegraph --profile bench` |
| Development | `dev` | `cargo build` |
| Testing | `test` | `cargo test` |

## Performance Characteristics

### Expected Performance Ranges

Based on benchmark results:

| Operation | Latency (p50/p95) | Notes |
|-----------|-------------------|-------|
| Bond cashflow (30Y) | ~10-20μs / ~30-50μs | Fixed coupon, semi-annual |
| Swap cashflow (30Y) | ~15-30μs / ~50-80μs | Floating leg projection |
| MC European (100k paths) | ~50-100ms / ~150-250ms | GBM, serial |
| MC Heston (50k paths) | ~100-200ms / ~300-500ms | Stochastic vol |
| Structured credit waterfall | ~200-500μs / ~1-2ms | Per payment date |

### Allocation Guidelines

Hot paths should minimize allocations:
- Cashflow generation: Pre-allocate with `Vec::with_capacity(num_periods)`
- MC simulation: Reuse buffers across paths
- Waterfall allocation: Use `HashMap::with_capacity(recipients.len())`

## Profiling Techniques

See [PROFILING.md](PROFILING.md) for detailed profiling instructions.

## Build Tips

### Faster Incremental Builds
```bash
# Use release-perf instead of release for faster compile times
cargo build --profile release-perf
```

### Cross-compilation
```bash
# For Linux target from macOS:
cargo build --profile release-perf --target x86_64-unknown-linux-gnu
```

### Parallel Builds
```bash
# Leverage all CPU cores
cargo build --profile release-perf -j $(nproc)
```

## Known Performance Characteristics

### Strengths
- **Decimal arithmetic**: Deterministic, no floating-point errors
- **Currency safety**: Zero-cost abstraction for currency types
- **Parallel MC**: Near-linear scaling with Rayon
- **Vectorized cashflows**: Efficient batch discounting

### Trade-offs
- **Size vs Speed**: `release` profile prioritizes WASM bundle size over raw CPU performance
- **Decimal vs f64**: 2-3x slower than f64 for arithmetic, but deterministic
- **Type safety**: Some runtime overhead for currency/FX checks (negligible in practice)

## Optimization Checklist

When optimizing hot paths:

1. ✅ Use `--profile release-perf` for CPU-bound workloads
2. ✅ Pre-allocate collections with `with_capacity`
3. ✅ Avoid clones in loops (use references)
4. ✅ Reuse buffers across iterations
5. ✅ Enable `parallel` feature for MC/batch pricing
6. ✅ Profile before optimizing (flamegraph, dhat)
7. ✅ Benchmark regressions with `cargo bench --baseline`

## Further Reading

- [Profiling Guide](PROFILING.md) - Detailed profiling instructions
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)

