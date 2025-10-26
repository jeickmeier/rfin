# Finstack Valuations Benchmarks

Performance benchmarks for critical pricing operations.

**Market Standards Review (Week 5)** - Added to track regression in pricing latency.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench --package finstack-valuations

# Run specific benchmark
cargo bench --package finstack-valuations --bench bond_pricing
cargo bench --package finstack-valuations --bench swap_pricing
cargo bench --package finstack-valuations --bench option_pricing
cargo bench --package finstack-valuations --bench cds_pricing
cargo bench --package finstack-valuations --bench cds_option_pricing
cargo bench --package finstack-valuations --bench cds_tranche_pricing
cargo bench --package finstack-valuations --bench swaption_pricing
cargo bench --package finstack-valuations --bench cds_index_pricing
cargo bench --package finstack-valuations --bench structured_credit_pricing
cargo bench --package finstack-valuations --bench convertible_pricing
cargo bench --package finstack-valuations --bench calibration

# Quick mode (fewer samples)
cargo bench --package finstack-valuations -- --quick

# Compare against baseline (after running once)
cargo bench --package finstack-valuations --bench bond_pricing -- --save-baseline my_baseline
cargo bench --package finstack-valuations --bench bond_pricing -- --baseline my_baseline
```

## Benchmark Suite

### bond_pricing.rs - Bond Instruments (16 scenarios)
- **bond_pv**: Present value calculation (2Y, 5Y, 10Y, 30Y)
- **bond_ytm_solve**: YTM solver with Newton-Raphson + Brent (2Y, 5Y, 10Y, 30Y)
- **bond_duration**: Modified duration and convexity (2Y, 5Y, 10Y, 30Y)
- **bond_dv01**: Dollar value of 01 (2Y, 5Y, 10Y, 30Y)

### swap_pricing.rs - Interest Rate Swaps (12 scenarios)
- **swap_pv**: Present value (2Y, 5Y, 10Y, 30Y)
- **swap_dv01**: DV01 calculation (2Y, 5Y, 10Y, 30Y)
- **swap_par_rate**: Par rate and annuity (2Y, 5Y, 10Y, 30Y)

### option_pricing.rs - Equity Options (11 scenarios)
- **option_pv**: Black-Scholes PV (3M, 6M, 12M, 24M)
- **option_greeks**: Full Greeks set (delta, gamma, vega, theta, rho) (3M, 6M, 12M)

### cds_pricing.rs - Credit Default Swaps (12 scenarios)
- **cds_pv**: NPV with ISDA integration (1Y, 3Y, 5Y, 10Y)
- **cds_cs01**: Credit spread 01 (1Y, 3Y, 5Y, 10Y)
- **cds_par_spread**: Par spread calculation (1Y, 3Y, 5Y, 10Y)

### cashflow_generation.rs - Cashflow Building (22 scenarios)
- **bond_cashflow_generation**: Bond schedule generation (2Y, 5Y, 10Y, 30Y)
- **swap_cashflow_generation**: Swap schedule generation (2Y, 5Y, 10Y, 30Y)
- **schedule_builder_fixed**: Fixed coupon schedule building (2Y, 5Y, 10Y, 30Y)
- **kahan_summation**: Precision-preserving aggregation (10, 20, 50, 100, 200, 500 flows)

### cds_option_pricing.rs - CDS Options (19 scenarios)
- **cds_option_npv**: Black76 on spreads NPV (call/put, 3M-1Y expiry, 5Y-10Y CDS)
- **cds_option_greeks**: Individual Greeks (delta, gamma, vega, theta)
- **cds_option_all_greeks**: Sequential Greeks calculation
- **cds_option_implied_vol**: Implied volatility solver (Newton-Raphson + Brent)

### cds_tranche_pricing.rs - CDS Tranches (35+ scenarios)
- **cds_tranche_npv**: Gaussian Copula pricing (equity, junior mezz, senior mezz, senior)
- **cds_tranche_cs01**: Credit spread sensitivity
- **cds_tranche_correlation_delta**: Correlation sensitivity
- **cds_tranche_jump_to_default**: Immediate default impact
- **cds_tranche_par_spread**: Fair spread calculation
- **cds_tranche_all_metrics**: Full metrics suite
- **cds_tranche_heterogeneous**: Heterogeneous portfolios (10, 25, 50, 125 issuers)

### swaption_pricing.rs - Swaptions (20+ scenarios)
- **swaption_pv**: Black76 present value (3Mx5Y, 6Mx5Y, 12Mx5Y, 12Mx10Y)
- **swaption_sabr**: SABR-implied volatility pricing (3Mx5Y, 6Mx5Y, 12Mx5Y, 12Mx10Y)
- **swaption_greeks**: Greeks calculation (delta, gamma, vega, theta) (3Mx5Y, 6Mx5Y, 12Mx10Y)
- **swaption_forward_rate**: Forward swap rate calculation (3Mx5Y, 6Mx5Y, 12Mx5Y, 12Mx10Y)
- **swaption_annuity**: Swap annuity factor calculation (3Mx5Y, 6Mx5Y, 12Mx5Y, 12Mx10Y)

### cds_index_pricing.rs - CDS Indices (23+ scenarios)
- **cds_index_pv_single**: NPV with single curve (1Y, 3Y, 5Y, 10Y)
- **cds_index_pv_constituents**: NPV with constituents (10, 25, 50, 125 names)
- **cds_index_par_spread**: Par spread calculation (1Y, 3Y, 5Y, 10Y)
- **cds_index_cs01**: Credit spread 01 (1Y, 3Y, 5Y, 10Y)
- **cds_index_risky_pv01**: Risky PV01 (1Y, 3Y, 5Y, 10Y)
- **cds_index_metrics**: All metrics suite (3Y, 5Y, 10Y)

### structured_credit_pricing.rs - Structured Credit (50+ scenarios)
- **structured_credit_npv**: NPV by deal type (ABS, CLO, CMBS, RMBS)
- **structured_credit_cashflows**: Cashflow generation with waterfall (10, 25, 50, 100 assets)
- **structured_credit_wal**: Weighted average life (CLO, RMBS)
- **structured_credit_duration**: Modified duration and spread duration
- **structured_credit_cs01**: Credit spread 01 (10, 25, 50 assets)
- **structured_credit_pool_metrics**: Pool metrics (WAC, WAS, WAM, diversity) (10-200 assets)
- **structured_credit_warf**: Weighted average rating factor (25-200 assets)
- **structured_credit_prices**: Price suite (dirty, clean, accrued)
- **structured_credit_full_metrics**: All metrics combined (50 assets)
- **structured_credit_scaling**: Scaling with pool size (10-500 assets)

### convertible_pricing.rs - Convertible Bonds (40+ scenarios)
- **convertible_npv_binomial**: Binomial tree pricing (25, 50, 100, 200 steps)
- **convertible_npv_trinomial**: Trinomial tree pricing (25, 50, 100, 200 steps)
- **convertible_npv_moneyness**: NPV by moneyness (OTM, ATM, ITM)
- **convertible_npv_features**: Standard, callable, zero-coupon
- **convertible_npv_volatility**: Volatility sensitivity (low, std, high)
- **convertible_greeks**: Full Greeks suite (25, 50, 100 steps)
- **convertible_greeks_moneyness**: Greeks by moneyness (OTM, ATM, ITM)
- **convertible_metrics**: Price with metrics (delta, gamma, vega, rho, theta)
- **convertible_parity**: Parity calculation by moneyness
- **convertible_convergence**: Tree convergence (10-500 steps)

### calibration.rs - Market Data Calibration (45+ scenarios)
- **discount_curve_small**: Discount curve bootstrap (8 instruments)
- **discount_curve_medium**: Discount curve bootstrap (16 instruments)
- **discount_curve_large**: Discount curve bootstrap (22 instruments)
- **discount_curve_interp**: Interpolation methods (Linear, MonotoneConvex, CubicHermite)
- **forward_curve**: Forward curve calibration (4, 8, 16 FRAs)
- **hazard_curve**: Credit curve calibration (3, 6 tenors)
- **simple_calibration_small**: End-to-end minimal market calibration
- **simple_calibration_medium**: Rates + credit calibration
- **simple_calibration_full**: Complete market calibration with vol surfaces
- **calibration_solver**: Solver comparison (Newton, Brent, Hybrid)
- **base_correlation_small**: Base correlation curve (3 tranches)
- **base_correlation_full**: Base correlation curve (5 tranches)
- **inflation_curve**: CPI curve calibration (3, 5, 10 tenors)
- **sabr_surface_small**: SABR vol surface (2 expiries × 5 strikes)
- **sabr_surface_medium**: SABR vol surface (4 expiries × 7 strikes)
- **swaption_vol_small**: Swaption vol calibration (2 exp × 2 ten)
- **swaption_vol_medium**: Swaption vol calibration (3 exp × 3 ten)

## Typical Performance (M1 Mac, Release Build)

| Operation | Tenor | Latency | Note |
|-----------|-------|---------|------|
| **Bonds** | | | |
| Bond PV | 5Y | ~5-10 μs | Fast path, no metrics |
| Bond YTM | 5Y | ~50-70 μs | Iterative solver |
| Bond Duration | 10Y | ~40-50 μs | Numerical bumping |
| Bond DV01 | 10Y | ~45-60 μs | Bump + reprice |
| Bond Cashflow Gen | 30Y | ~15-20 μs | 60 semi-annual flows |
| **Swaps** | | | |
| Swap PV | 5Y | ~15-25 μs | Fixed + float legs |
| Swap DV01 | 5Y | ~35-50 μs | Bump + reprice |
| Swap Par Rate | 5Y | ~25-40 μs | Annuity + rate |
| Swap Cashflow Gen | 30Y | ~20-30 μs | 120 quarterly flows |
| **Options** | | | |
| Option PV | 6M | ~2-5 μs | Black-Scholes analytical |
| Option Greeks | 6M | ~5-10 μs | All 5 Greeks analytical |
| **Credit** | | | |
| CDS PV | 5Y | ~50-100 μs | ISDA integration |
| CDS CS01 | 5Y | ~100-150 μs | Credit bump + reprice |
| CDS Par Spread | 5Y | ~100-150 μs | Risky annuity calc |
| **CDS Options** | | | |
| CDS Option NPV | 6M | ~50-100 μs | Black76 on spreads |
| CDS Option Greeks | 6M | ~150-250 μs | All Greeks sequential |
| CDS Option Implied Vol | 6M | ~500-800 μs | Iterative solver |
| **CDS Tranches** | | | |
| Tranche NPV (Homog.) | 5Y | ~300-500 μs | Gauss-Hermite integration |
| Tranche NPV (Hetero.) | 5Y 125 names | ~1-2 ms | SPA/convolution |
| Tranche CS01 | 5Y | ~600-1000 μs | Bump + reprice |
| Tranche Corr Delta | 5Y | ~600-1000 μs | Correlation bump |
| Tranche Par Spread | 5Y | ~600-1000 μs | Premium/protection balance |
| **Swaptions** | | | |
| Swaption PV (Black) | 6Mx5Y | ~20-40 μs | Forward rate + annuity + Black76 |
| Swaption PV (SABR) | 6Mx5Y | ~50-100 μs | SABR vol + Black76 |
| Swaption Greeks | 6Mx5Y | ~100-200 μs | Delta, gamma, vega, theta |
| Forward Swap Rate | 6Mx5Y | ~15-30 μs | Discount factors + annuity |
| Swap Annuity | 6Mx5Y | ~10-25 μs | Schedule + DFs |
| **CDS Indices** | | | |
| Index PV (Single) | 5Y | ~100-200 μs | Synthetic CDS |
| Index PV (10 names) | 5Y | ~1-2 ms | Constituents aggregation |
| Index PV (125 names) | 5Y | ~10-15 ms | Full index |
| Index Par Spread | 5Y | ~200-400 μs | Risky annuity |
| Index CS01 | 5Y | ~200-400 μs | Credit bump |
| Index Risky PV01 | 5Y | ~200-400 μs | Premium leg sensitivity |
| **Structured Credit** | | | |
| SC NPV (CLO) | 10 assets | ~500-800 μs | Waterfall + discounting |
| SC NPV (CLO) | 50 assets | ~2-3 ms | Larger pool |
| SC NPV (CLO) | 200 assets | ~8-12 ms | Full deal |
| SC Cashflows | 25 assets | ~400-700 μs | Schedule generation |
| SC WAL | CLO/RMBS | ~600-1000 μs | Weighted average life |
| SC Duration | 25 assets | ~1-2 ms | Modified + spread duration |
| SC CS01 | 25 assets | ~2-3 ms | Credit spread sensitivity |
| SC Pool Metrics | 50 assets | ~300-600 μs | WAC, WAS, WAM, diversity |
| SC WARF | 100 assets | ~200-400 μs | Rating factor calc |
| SC Prices | 20 assets | ~800-1200 μs | Dirty, clean, accrued |
| SC Full Metrics | 50 assets | ~5-8 ms | All metrics combined |
| **Convertible Bonds** | | | |
| Convertible PV (Binomial) | 50 steps | ~300-600 μs | Tree-based hybrid pricing |
| Convertible PV (Binomial) | 200 steps | ~1.5-2.5 ms | Higher accuracy |
| Convertible PV (Trinomial) | 50 steps | ~500-900 μs | Trinomial tree |
| Convertible Greeks | 50 steps | ~2-4 ms | All Greeks (bump-reprice) |
| Convertible Parity | - | ~5-10 μs | Equity conversion value |
| Convertible Convergence | 500 steps | ~8-12 ms | Maximum accuracy |
| **Calibration** | | | |
| Discount Curve | 8 instruments | ~135-140 μs | Bootstrap with solver |
| Discount Curve | 16 instruments | ~295-300 μs | Medium curve |
| Discount Curve | 22 instruments | ~730-770 μs | Large curve |
| Forward Curve | 4 FRAs | ~220-225 μs | Short-end calibration |
| Forward Curve | 16 FRAs | ~910-925 μs | Full curve |
| Hazard Curve | 3 tenors | ~840-850 μs | Credit spread bootstrap |
| Hazard Curve | 6 tenors | ~2.5-2.6 ms | Full credit curve |
| Simple Calibration | Minimal | ~200-205 μs | Deposits + swaps only |
| Simple Calibration | Medium | ~4.7-4.8 ms | Rates + credit |
| Simple Calibration | Full | ~5.2-5.3 ms | Complete market with vol |
| Base Correlation | 3 tranches | ~1.0-1.1 s | Sequential bootstrap |
| Base Correlation | 5 tranches | ~1.6-1.7 s | Full tranche curve |
| Inflation Curve | 3 tenors | ~150-200 μs | CPI curve bootstrap |
| Inflation Curve | 10 tenors | ~500-600 μs | Full CPI curve |
| SABR Surface | 2exp×5strikes | ~80-100 μs | Small vol surface |
| SABR Surface | 4exp×7strikes | ~200-250 μs | Medium vol surface |
| Swaption Vol | 2exp×2ten | ~150-200 μs | Small swaption surface |
| Swaption Vol | 3exp×3ten | ~350-400 μs | Medium swaption surface |
| **Cashflows** | | | |
| Schedule Builder | 30Y | ~11 μs | Fixed semi-annual |
| Kahan Sum | 20 flows | ~60 ns | Fast path |
| Kahan Sum | 100 flows | ~500 ns | Precision path |
| Kahan Sum | 500 flows | ~2.5 μs | Long leg |

## Viewing Results

After running benchmarks, results are available in:
- **Terminal:** Summary statistics
- **HTML Report:** `target/criterion/*/report/index.html`
- **CSV Data:** `target/criterion/*/base/raw.csv`

Open HTML report:
```bash
open target/criterion/bond_pv/report/index.html
```

## Regression Tracking

To track performance over time:

1. **Establish baseline:**
   ```bash
   cargo bench --package finstack-valuations -- --save-baseline initial
   ```

2. **Compare after changes:**
   ```bash
   cargo bench --package finstack-valuations -- --baseline initial
   ```

3. **Results show:**
   - Performance changes (faster/slower)
   - Statistical significance
   - Confidence intervals

## Optimization Targets

Based on Market Standards Review recommendations:

- **Bond YTM:** Target < 100μs p99 (currently ~70μs p50) ✅
- **Swap PV:** Target < 50μs p99 (currently ~25μs p50) ✅
- **Option Greeks:** Target < 20μs p99 (currently ~10μs p50) ✅
- **CDS Par Spread:** Target < 200μs p99 (currently ~150μs p50) ✅

All targets met in initial benchmarking.

## Portfolio-Level Benchmarks

Portfolio-level benchmarks are located in the `finstack-portfolio` package:

```bash
cargo bench --package finstack-portfolio
```

See `finstack/portfolio/benches/README.md` for portfolio benchmark documentation.

## Notes

- Benchmarks use **release build** (optimized)
- Results may vary by hardware
- Criterion automatically determines sample size for statistical significance
- Use `--quick` for faster iteration during development

