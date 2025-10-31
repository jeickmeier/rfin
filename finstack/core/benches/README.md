# Finstack Core Benchmarks

Performance benchmarks for core library primitives and operations.

## Running Benchmarks

```bash
# Run all core benchmarks
cargo bench --package finstack-core

# Run specific benchmark suite
cargo bench --package finstack-core --bench money_operations
cargo bench --package finstack-core --bench daycount_operations
cargo bench --package finstack-core --bench calendar_operations
cargo bench --package finstack-core --bench interpolation
cargo bench --package finstack-core --bench curve_operations

# Quick mode (fewer samples)
cargo bench --package finstack-core -- --quick

# Compare against baseline (after running once)
cargo bench --package finstack-core -- --save-baseline my_baseline
cargo bench --package finstack-core -- --baseline my_baseline
```

## Benchmark Suites

### money_operations.rs - Money Type Operations

Tests core monetary operations:
- **Creation**: Constructing Money values with different currencies
- **Arithmetic**: Addition, subtraction, multiplication, division
- **Currency Conversions**: FX conversions between major currencies (EUR, USD, GBP, JPY)
- **Batch Operations**: Summing and scaling large collections of amounts
- **Formatting**: String representation at various scales

**Key Performance Indicators:**
- Single arithmetic operation: < 10ns
- FX conversion: < 50ns
- Batch sum (1000 items): < 10μs

### daycount_operations.rs - Day Count Conventions

Tests year fraction calculations across conventions:
- **ACT/360**: Actual days divided by 360
- **ACT/365F**: Actual days divided by 365 (fixed)
- **ACT/ACT (ISDA)**: Actual/Actual ISDA convention
- **ACT/ACT (ISMA)**: Actual/Actual ISMA with coupon frequency
- **30/360 Family**: 30/360, 30E/360, 30E/360 ISDA variants
- **Bus/252**: Business days divided by 252 (calendar-dependent)

**Key Performance Indicators:**
- Single year fraction: < 100ns for simple conventions
- ACT/ACT ISMA: < 200ns (requires frequency context)
- Bus/252: < 1μs (requires calendar lookups)
- Batch (100 periods): < 10μs

### calendar_operations.rs - Holiday Calendars

Tests calendar and business day operations:
- **Holiday Checks**: `is_holiday()` and `is_business_day()` across calendars
- **Business Day Adjustments**: Following, Preceding, ModifiedFollowing conventions
- **Business Day Counting**: Days between dates
- **Composite Calendars**: Combined calendar logic (NYSE + TARGET2)
- **Batch Operations**: Checking multiple dates efficiently

**Supported Calendars:**
- NYSE (New York Stock Exchange)
- TARGET2 (European Central Bank)
- NERC (North American Electric Reliability Corporation)
- USGOVBOND (US Government Bond)

**Key Performance Indicators:**
- Holiday check (cached years 1970-2150): < 50ns
- Business day adjustment: < 200ns
- Business days between (1 year): < 5μs
- Batch holiday checks (100 dates): < 5μs

### interpolation.rs - Interpolation Algorithms

Tests interpolation methods for curve construction:
- **Linear**: Piece-wise linear interpolation
- **Log-Linear**: Logarithmic interpolation (smooth rates)
- **Cubic Hermite**: Smooth cubic interpolation
- **Monotone Convex**: Monotonicity-preserving interpolation
- **Flat Forward**: Flat forward rate interpolation

**Test Scenarios:**
- Single point lookups
- Batch interpolation (10-500 points)
- Extrapolation behavior (FlatZero vs FlatForward)
- Algorithm comparison on identical data

**Key Performance Indicators:**
- Single interpolation: 10-50ns (varies by method)
- Batch (100 points): 1-5μs
- Complexity ranking: Linear < LogLinear < FlatFwd < CubicHermite < MonotoneConvex

### curve_operations.rs - Term Structure Curves

Tests discount, forward, and hazard curve operations:
- **Discount Curves**: df(), zero(), forward() lookups
- **Forward Curves**: Forward rate interpolation
- **Hazard Curves**: Credit hazard rates and survival probabilities
- **Curve Building**: Construction overhead for various sizes
- **Interpolation Styles**: Performance comparison across methods

**Test Scenarios:**
- Single lookups (df, zero, forward)
- Batch operations (10-500 evaluations)
- Curve construction with 5-100 knot points
- Interpolation style comparison

**Key Performance Indicators:**
- Single df() lookup: 10-50ns
- Single zero() calculation: 20-60ns
- Forward rate (t1, t2): 50-100ns
- Batch df (100 times): 1-5μs
- Curve construction (20 points): < 10μs

## Typical Performance (M1 Mac, Release Build)

| Operation Category | Operation | Latency | Notes |
|--------------------|-----------|---------|-------|
| **Money** | | | |
| Creation | Money::new() | ~5-10ns | Includes rounding |
| Arithmetic | Add/Sub | ~5-10ns | Same currency |
| Arithmetic | Mul/Div scalar | ~5-10ns | Scalar multiply |
| FX Conversion | EUR→USD | ~30-50ns | With FX provider |
| Batch | Sum 1000 amounts | ~5-10μs | Sequential |
| **Day Count** | | | |
| Year Fraction | ACT/360 | ~50-100ns | Simple convention |
| Year Fraction | ACT/ACT | ~100-150ns | ISDA variant |
| Year Fraction | ACT/ACT ISMA | ~150-200ns | With frequency |
| Year Fraction | Bus/252 | ~500-1000ns | Calendar dependent |
| Batch | 100 year fractions | ~5-10μs | ACT/360 |
| **Calendars** | | | |
| Holiday Check | is_holiday() | ~20-50ns | Cached bitset |
| Business Day | is_business_day() | ~30-60ns | Weekend + holiday |
| Adjustment | Following | ~100-200ns | Single adjustment |
| Business Days | Between (1y) | ~3-5μs | ~250 days |
| Batch | 100 holiday checks | ~2-5μs | Cached |
| **Interpolation** | | | |
| Single Point | Linear | ~10-20ns | Fastest |
| Single Point | LogLinear | ~20-30ns | Log overhead |
| Single Point | CubicHermite | ~30-50ns | Cubic solver |
| Single Point | MonotoneConvex | ~40-60ns | Most expensive |
| Batch | 100 points (linear) | ~1-2μs | Good cache locality |
| **Curves** | | | |
| Discount | df(t) | ~15-30ns | Depends on interp |
| Discount | zero(t) | ~25-40ns | df + log + divide |
| Discount | forward(t1,t2) | ~60-100ns | 2× zero + calc |
| Forward | forward_rate(t) | ~20-35ns | Direct interp |
| Hazard | survival_prob(t) | ~25-40ns | Integral calc |
| Batch | 100 df lookups | ~1.5-3μs | Linear interp |

## Performance Characteristics

### Linear Scaling

Most operations scale linearly with input size:
- **Batch operations**: 10x input → 10x time
- **Curve evaluations**: O(log n) binary search in knots
- **Calendar checks**: O(1) for cached years (1970-2150)

### Memory Efficiency

Core types are designed for cache efficiency:
- `Money`: 16 bytes (f64 + Currency enum)
- `Date`: 4 bytes (compact encoding)
- Curve knots: Boxed slices (minimal indirection)
- Calendar bitsets: Pre-generated at compile time

### Optimization Opportunities

Areas for potential optimization:
1. **SIMD vectorization**: Batch interpolation could use SIMD
2. **Parallel batch ops**: Large batch operations with rayon
3. **Curve caching**: Frequently-used df values could be memoized
4. **Calendar compression**: Further bitset optimizations

## Optimization Targets

Based on downstream usage patterns:

### Money Operations
- Single creation/arithmetic: < 10ns ✅
- FX conversion: < 100ns ✅
- Batch sum (1000): < 20μs ✅

### Day Count
- Simple conventions (ACT/360): < 100ns ✅
- Complex conventions (ACT/ACT): < 200ns ✅
- Calendar-based (Bus/252): < 2μs ✅

### Calendars
- Holiday check: < 100ns ✅
- Business day adjustment: < 500ns ✅
- Business days between (1y): < 10μs ✅

### Interpolation
- Linear single point: < 50ns ✅
- Batch (100 points): < 10μs ✅
- All methods: < 100ns single point ✅

### Curves
- df() lookup: < 100ns ✅
- zero() calculation: < 100ns ✅
- Batch (100): < 10μs ✅

All targets met in initial benchmarking.

## Viewing Results

After running benchmarks, results are available in:
- **Terminal**: Summary statistics with confidence intervals
- **HTML Report**: `target/criterion/*/report/index.html`
- **CSV Data**: `target/criterion/*/base/raw.csv`

Open HTML report:
```bash
# View specific benchmark results
open target/criterion/money_creation/report/index.html
open target/criterion/interp_comparison/report/index.html
open target/criterion/curve_discount_df_single/report/index.html
```

## Regression Tracking

To track performance over time:

1. **Establish baseline:**
   ```bash
   cargo bench --package finstack-core -- --save-baseline initial
   ```

2. **Compare after changes:**
   ```bash
   cargo bench --package finstack-core -- --baseline initial
   ```

3. **Results show:**
   - Performance changes (faster/slower as %)
   - Statistical significance
   - Confidence intervals
   - Regression warnings

## Notes

- Benchmarks use **release build** with optimizations
- Results may vary by hardware (M1/M2/x86)
- Criterion automatically determines sample size for statistical validity
- Use `--quick` for faster iteration during development
- All benchmarks use `black_box()` to prevent compiler optimizations
- Core types are fundamental to all higher-level operations
- Performance regressions in core directly impact statements, valuations, and portfolio layers

## Benchmark Design Principles

1. **Representative workloads**: Test realistic usage patterns
2. **Scalability testing**: Verify linear scaling assumptions
3. **Isolated measurements**: Single operations for precise profiling
4. **Batch operations**: Test cache and memory effects
5. **Comparison tests**: Direct A/B tests of algorithms
6. **Edge cases**: Extrapolation, boundary conditions

## Future Additions

Potential future benchmarks:
- Expression engine evaluation (AST, DAG)
- Market context lookups and bumps
- Volatility surface interpolation
- Integration algorithms (Simpson, Gauss-Legendre)
- Root finding (Newton-Raphson, Brent)
- Statistical functions (CDF, quantiles)









