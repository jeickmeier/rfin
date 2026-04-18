# Finstack Statements Benchmarks

Performance benchmarks for statement modeling operations.

## Running Benchmarks

```bash
# Run all statements benchmarks
cargo bench --package finstack-statements

# Run specific benchmark group
cargo bench --package finstack-statements -- model_building
cargo bench --package finstack-statements -- model_evaluation

# Quick mode (fewer samples)
cargo bench --package finstack-statements -- --quick

# With Polars export benchmarks (requires feature flag)
cargo bench --package finstack-statements

# Compare against baseline (after running once)
cargo bench --package finstack-statements -- --save-baseline my_baseline
cargo bench --package finstack-statements -- --baseline my_baseline
```

## Benchmark Groups

### 1. Model Building (`model_building`)

Tests performance of constructing financial models:

- **simple_value_model**: Basic model with value nodes only
- **computed_nodes_model**: Model with computed formulas
- **large_model_50_nodes**: Scaling test with 50 nodes

**Use Cases:**

- Track overhead of the builder API
- Ensure model construction scales linearly
- Validate type-state transitions don't add runtime cost

### 2. Model Evaluation (`model_evaluation`)

Measures evaluation performance across different model complexities:

- **evaluate_value_only**: Simple value-only model (baseline)
- **evaluate_with_calculations**: Model with computed nodes
- **evaluate_with_timeseries**: Time-series functions (lag, rolling, etc.)
- **evaluate_50_nodes**: Large model with many dependencies
- **evaluate_24_periods**: Monthly model over 2 years

**Use Cases:**

- Track DAG construction and topological sort overhead
- Measure impact of time-series operators
- Validate linear scaling with nodes and periods

### 3. DSL Operations (`dsl_operations`)

Tests parser and compiler performance:

- **parse_simple_formula**: Basic arithmetic (`revenue * 0.6`)
- **parse_complex_formula**: Nested expressions with operators
- **parse_timeseries_formula**: Functions like `rolling_mean`, `lag`
- **compile_simple_ast**: AST to core expression compilation
- **compile_complex_ast**: Complex AST compilation

**Use Cases:**

- Track parser overhead (should be < 1 μs for simple formulas)
- Ensure compilation is fast enough for interactive UIs
- Monitor impact of adding new DSL features

### 4. Forecast Methods (`forecast_methods`)

Benchmarks different forecast algorithms:

- **forecast_forward_fill**: Simple forward fill
- **forecast_growth_rate**: Compound growth
- **forecast_seasonal**: Seasonal patterns with growth
- **forecast_lognormal**: Stochastic distribution (deterministic seed)

**Use Cases:**

- Compare performance of forecast methods
- Track overhead of statistical methods vs simple methods
- Validate determinism doesn't add significant cost

### 5. Registry Operations (`registry_operations`)

Tests dynamic metric registry performance:

- **load_empty_registry**: Registry initialization
- **add_10_metrics**: Adding custom metrics
- **resolve_metric**: Looking up metric by name

**Use Cases:**

- Track registry lookup overhead
- Ensure metric addition scales linearly
- Monitor namespace resolution performance

### 7. Results Export (`results_export`)

Benchmarks DataFrame conversion:

- **export_to_long_dataframe**: Long format (period × node rows)
- **export_to_wide_dataframe**: Wide format (periods as columns)
- **export_large_to_***: Same operations on 24-period, 20-node model

**Use Cases:**

- Track Polars integration overhead
- Compare long vs wide export performance
- Validate efficient memory usage for large models

### 8. Serialization (`serialization`)

Tests JSON serialization performance:

- **serialize_model_to_json**: Model → JSON string
- **deserialize_model_from_json**: JSON string → Model

**Use Cases:**

- Track serde overhead for persistence
- Validate reasonable performance for saving/loading models
- Monitor impact of adding new fields to specs

### 9. End-to-End (`end_to_end`)

Full workflow benchmarks combining multiple operations:

- **simple_pl_model**: Basic P&L with actuals + forecast
- **complex_financial_model**: Complete financial model with seasonal forecasts, time-series calculations, and derived metrics

**Use Cases:**

- Measure real-world workflow performance
- Track aggregate overhead across all subsystems
- Regression testing for common use cases

## Typical Performance (M1 Mac, Release Build)

| Operation | Complexity | Expected Latency | Notes |
|-----------|-----------|------------------|-------|
| **Model Building** | | | |
| Simple value model | 4 periods, 1 node | ~5-15 μs | Type-state overhead minimal |
| Computed nodes | 4 periods, 5 nodes | ~20-50 μs | Formula parsing included |
| Large model | 4 periods, 50 nodes | ~200-500 μs | Linear scaling |
| **Model Evaluation** | | | |
| Value only | 4 periods, 1 node | ~2-5 μs | Baseline |
| With calculations | 4 periods, 5 nodes | ~10-30 μs | DAG overhead |
| With time-series | 4 periods, 9 nodes | ~30-80 μs | Rolling/lag functions |
| 50 nodes | 4 periods, 50 nodes | ~150-400 μs | Topological sort cost |
| 24 periods | 24 periods, 4 nodes | ~50-150 μs | Period scaling |
| **DSL Operations** | | | |
| Parse simple | 3 tokens | ~0.5-2 μs | Fast enough for REPL |
| Parse complex | 10+ tokens | ~2-8 μs | Reasonable for UI |
| Compile simple | Small AST | ~1-5 μs | Minimal overhead |
| **Forecast Methods** | | | |
| Forward fill | 3 periods | ~5-15 μs | Simplest method |
| Growth rate | 3 periods | ~8-20 μs | Compound calc |
| Seasonal | 3 periods, 4-season | ~15-40 μs | Pattern matching |
| Log-normal | 3 periods | ~20-60 μs | RNG overhead |
| **Registry** | | | |
| Load empty | - | ~0.1-0.5 μs | Near-zero overhead |
| Add 10 metrics | 10 formulas | ~50-150 μs | Includes parsing |
| Resolve metric | 1 lookup | ~0.2-1 μs | Fast hashmap |
| **Serialization** | | | |
| Serialize | 5-node model | ~30-100 μs | JSON encoding |
| Deserialize | 5-node model | ~40-120 μs | JSON parsing |
| **End-to-End** | | | |
| Simple P&L | 6 nodes, 4 periods | ~40-120 μs | Build + eval |
| Complex model | 15 nodes, 4 periods | ~150-400 μs | Full workflow |

## Performance Characteristics

### Linear Scaling

All major operations should scale linearly:

- **Nodes**: 10 → 50 nodes ≈ 5x latency increase
- **Periods**: 4 → 24 periods ≈ 6x latency increase
- **Registry**: 1 → 100 metrics ≈ 100x for addition (one-time)

### Forecast Method Comparison

Relative performance (forward fill = 1.0x baseline):

- Forward fill: 1.0x (baseline)
- Growth rate: ~1.2-1.5x (simple math)
- Seasonal: ~2-3x (pattern matching)
- Log-normal: ~3-4x (RNG + distribution)

## Optimization Targets

Based on typical use cases:

### Interactive UI (< 100ms p99)

- **Model building**: < 5ms for 100 nodes ✅
- **Model evaluation**: < 50ms for 100 nodes × 48 periods ✅
- **DSL parsing**: < 10μs for typical formulas ✅

### Batch Processing (10k+ models/second)

- **Simple P&L**: < 100μs p50 ✅
- **Complex model**: < 1ms p50 ✅

### Real-Time Updates (< 16ms for 60fps)

- **Re-evaluation**: < 5ms for typical model ✅
- **Incremental updates**: Not yet implemented ⚠️

## Viewing Results

After running benchmarks, results are available in:

- **Terminal**: Summary statistics
- **HTML Report**: `target/criterion/*/report/index.html`
- **CSV Data**: `target/criterion/*/base/raw.csv`

Open HTML report:

```bash
open target/criterion/model_building/report/index.html
```

## Regression Tracking

To track performance over time:

1. **Establish baseline:**

   ```bash
   cargo bench --package finstack-statements -- --save-baseline initial
   ```

2. **Compare after changes:**

   ```bash
   cargo bench --package finstack-statements -- --baseline initial
   ```

3. **Results show:**
   - Performance changes (faster/slower)
   - Statistical significance
   - Confidence intervals

## Performance Regression Guidelines

Flag for investigation if:

- **> 20% slowdown** on any benchmark
- **> 10% slowdown** on end-to-end benchmarks
- **Non-linear scaling** appears (e.g., quadratic growth)
- **p99 > 2x p50** (high variance indicates unstable performance)

## Notes

- Benchmarks use **release build** (optimized)
- Results may vary by hardware
- Criterion automatically determines sample size for statistical significance
- Use `--quick` for faster iteration during development
- Results-export benchmarks now use the built-in table-envelope APIs directly.
- All benchmarks use deterministic operations (no network, no external data)

## Future Benchmarks

Planned additions:

- **Incremental evaluation**: Re-evaluating models with partial changes
- **Parallel evaluation**: Multi-threaded model evaluation
- **Capital structure**: Full integration benchmarks
- **Memory usage**: Heap allocation profiling
- **Cache effectiveness**: Hit rates for memoized operations
