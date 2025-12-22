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
cargo bench --package finstack-core --bench expr_dag
cargo bench --package finstack-core --bench rolling
cargo bench --package finstack-core --bench solver_operations
cargo bench --package finstack-core --bench market_context
cargo bench --package finstack-core --bench vol_surface
cargo bench --package finstack-core --bench integration
cargo bench --package finstack-core --bench statistical_functions

# Quick mode (fewer samples)
cargo bench --package finstack-core -- --quick

# Compare against baseline (after running once)
cargo bench --package finstack-core -- --save-baseline my_baseline
cargo bench --package finstack-core -- --baseline my_baseline

# Performance-optimized profile (faster than default release)
cargo bench --package finstack-core --profile release-perf
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
- Complexity ranking: Linear < LogLinear < CubicHermite < MonotoneConvex

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

### expr_dag.rs - DAG Expression Evaluation (NEW)

Tests performance of complex expression graphs with shared sub-expressions:
- **DAG Evaluation**: Complex graphs with 10-100 interdependent nodes
- **DAG Planning**: Topological ordering and deduplication
- **Cache Enabled**: LRU caching of intermediate results
- **Arena Allocation**: Pre-allocated buffers to minimize allocations

**Test Scenarios:**
- Complex DAG with 10, 50, 100 nodes
- With and without execution planning
- With cache (10MB budget)

**Key Performance Indicators:**
- 10-node DAG: < 5μs
- 50-node DAG: < 25μs
- 100-node DAG: < 50μs
- Memory allocations: ~60% reduction with arena

### rolling.rs - Rolling Window Operations (NEW)

Tests performance of rolling window functions with optimized scratch buffers:
- **Rolling Median**: Window sizes 5, 10, 20
- **Rolling Mean**: Various data and window sizes
- **Rolling Std**: Standard deviation over windows
- **Scratch Reuse**: Optimized buffer management

**Test Scenarios:**
- Data sizes: 100, 500, 1000 points
- Window sizes: 5, 10, 20 periods
- Multiple rolling functions

**Key Performance Indicators:**
- Rolling median (1000 data, win=10): < 100μs
- Rolling mean (1000 data, win=10): < 20μs
- Rolling std (1000 data, win=10): < 30μs
- Memory allocations: ~10% reduction with truncate(0)

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
2. **Curve caching**: Frequently-used df values could be memoized
3. **Calendar compression**: Further bitset optimizations

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

- Benchmarks use **release build** with optimizations (now opt-level=3 by default)
- Use `--profile release-size` for WASM bundle size optimization (opt-level="z")
- Results may vary by hardware (M1/M2/x86)
- Criterion automatically determines sample size for statistical validity
- Use `--quick` for faster iteration during development
- All benchmarks use `black_box()` to prevent compiler optimizations
- Core types are fundamental to all higher-level operations
- Performance regressions in core directly impact statements, valuations, and portfolio layers

## Performance Optimizations (v0.3.1)

Recent optimizations have significantly improved performance:
- **Build profile**: Changed default release to opt-level=3 (~25% faster)
- **HashMap → FxHashMap**: Faster integer key lookups (~5% faster DAG eval)
- **Arena allocation**: Reduced allocations in expression evaluation (~60% fewer allocs)
- **Rolling windows**: Optimized scratch buffer reuse (~10% faster)
- **Batch operations**: Pre-allocated vectors in curve operations (~5% faster)

## Benchmark Design Principles

1. **Representative workloads**: Test realistic usage patterns
2. **Scalability testing**: Verify linear scaling assumptions
3. **Isolated measurements**: Single operations for precise profiling
4. **Batch operations**: Test cache and memory effects
5. **Comparison tests**: Direct A/B tests of algorithms
6. **Edge cases**: Extrapolation, boundary conditions

### market_context.rs - Market Context Operations

Tests market data storage and retrieval operations:
- **Curve Lookups**: Discount, forward, hazard curve retrieval by ID
- **Surface Lookups**: Volatility surface retrieval
- **Bump Operations**: Single and batch curve bumping
- **Context Cloning**: Arc-based shallow copy performance

**Test Scenarios:**
- Small context (10 curves), medium (50 curves), large (100 curves)
- Single vs batch lookups
- Parallel bumps on single and multiple curves

**Key Performance Indicators:**
- Single lookup: < 50ns (HashMap lookup + Arc clone)
- Batch 50 lookups: < 2μs
- Context clone: < 1μs (Arc-based, shallow)
- Single curve bump: < 5μs

### vol_surface.rs - Volatility Surface Operations

Tests volatility surface construction and interpolation:
- **Construction**: Surface building from grid data
- **Bilinear Interpolation**: Single and batch lookups
- **Boundary Handling**: Checked vs clamped evaluation
- **Bump Operations**: Parallel, scaled, and point bumps

**Test Scenarios:**
- Grid sizes: 5×5, 10×10, 20×20, 50×50
- At-grid vs interpolated lookups
- In-bounds vs out-of-bounds (clamped)

**Key Performance Indicators:**
- Single interpolation: 20-50ns
- Batch 100 lookups: 2-5μs
- Surface construction (10×10): < 1μs
- Parallel bump: < 5μs

### integration.rs - Numerical Integration Algorithms

Tests quadrature methods for financial computation:
- **Simpson's Rule**: Fixed interval integration
- **Adaptive Simpson**: Error-controlled integration
- **Gauss-Legendre**: High-order polynomial quadrature
- **Gauss-Hermite**: Normal distribution integrals (option pricing)
- **Trapezoidal Rule**: Simple baseline method

**Test Scenarios:**
- Polynomial, oscillatory, Gaussian functions
- Different tolerance levels (1e-4 to 1e-10)
- Quadrature orders (5, 7, 10 for Hermite; 2, 4, 8, 16 for Legendre)

**Key Performance Indicators:**
- Simpson 100 intervals: < 1μs
- Gauss-Legendre order 8: < 200ns
- Gauss-Hermite order 10: < 100ns
- Adaptive Simpson (tol=1e-8): < 5μs

### statistical_functions.rs - Statistical Functions

Tests probability distributions and basic statistics:
- **Normal Distribution**: CDF (Φ), PDF (φ), inverse CDF (Φ⁻¹)
- **Error Function**: erf(x) computation
- **Binomial Distribution**: PMF and full distribution generation
- **Beta Sampling**: Recovery rate modeling
- **Basic Statistics**: mean, variance, covariance, correlation

**Test Scenarios:**
- Single point vs batch evaluation
- Standard vs tail regions (important for VaR)
- Different distribution parameters

**Key Performance Indicators:**
- norm_cdf single: < 50ns
- norm_inv_cdf single: < 100ns
- Binomial probability: < 200ns
- Beta sample: < 500ns
- Mean/variance (1000 points): < 5μs

### solver_operations.rs - Root Finding Algorithms

Tests 1D and multi-dimensional solvers:
- **Newton-Raphson**: With analytic vs finite difference derivatives
- **Brent's Method**: Robust bracketing solver
- **XIRR/IRR**: Internal rate of return calculations
- **Levenberg-Marquardt**: Multi-dimensional least squares

**Test Scenarios:**
- Simple polynomials and transcendental equations
- Different daycount conventions for XIRR
- Dense systems of varying sizes (30×30 to 200×80)

**Key Performance Indicators:**
- Newton single solve: < 500ns
- Brent single solve: < 1μs
- XIRR 6 flows: < 5μs
- LM 100×50 system: < 1ms

## Future Additions

Potential future benchmarks:

- Market data serialization/deserialization
- Curve calibration workflows
- FX matrix operations
- Term structure bump ladders























