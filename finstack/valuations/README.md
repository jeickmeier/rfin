# Finstack Valuations

Production-ready financial instrument pricing, risk analytics, and cashflow generation with accounting-grade determinism and currency safety.

## Overview

The `finstack-valuations` crate provides a comprehensive valuation engine for fixed income, derivatives, credit, and structured products. Built on Decimal-based numerics and explicit currency handling, it ensures reproducible, audit-quality pricing and risk metrics suitable for production risk systems, regulatory reporting, and portfolio analytics.

**Core Capabilities:**

- **40+ Instrument Types**: Fixed income, rates/credit/equity/FX derivatives, exotic options, structured credit
- **Analytical & Numerical Pricing**: Black-Scholes, SABR, tree methods, Monte Carlo (with LSM for early exercise)
- **Comprehensive Risk Metrics**: Greeks, DV01/CS01, yields, spreads, convexity, theta, bucketed sensitivities
- **Curve Calibration**: Bootstrap and optimization for discount, forward, hazard, and volatility surfaces
- **Cashflow Generation**: Schedule building with amortization, floating rates, caps/floors, prepayment/default models
- **P&L Attribution**: Three methodologies (parallel, waterfall, metrics-based) with detailed factor decomposition
- **Covenant Management**: Financial and non-financial covenants with cure periods, consequences, and forward projection
- **Deterministic by Default**: Decimal numerics, stable ordering, and parallel ≡ serial results

---

## ⚠️ Version 0.8.0 Migration Notice

**Breaking changes in this release**. If upgrading from 0.7.x, please review:

- 📖 **Migration Guide**: See `MIGRATION_GUIDE.md` in repository root for comprehensive upgrade instructions
- 📋 **Changelog**: See `CHANGELOG.md` below for detailed changes
- ⏱️ **Estimated Migration Time**: 2-4 hours for most applications

**Key Changes**:

- 🔴 **Metrics now use strict mode by default** - Errors instead of `0.0` for unknown/failed metrics
- 🔴 **FX settlement dates corrected** - Now uses joint business day logic (ISDA-compliant)
- 🟠 **Calendar errors no longer silent** - Unknown calendar IDs return errors
- 🟡 **Panicking constructors removed** - Use `try_new()` instead of `new()` methods

**Quick Start Migration**:

1. Add error handling for `compute()` calls and prefer `Instrument::price_with_metrics()` for strict pricing + metrics
2. Update FX-related tests if using multi-currency instruments
3. Replace removed `new()` constructors with `try_new()` variants

See examples below for updated API usage patterns.

### Public API surface and deprecations

- **Canonical imports**: use the root modules for supported APIs:
  - `finstack_valuations::instruments::{Instrument, Attributes, PricingOptions, Bond, InterestRateSwap, ...}`
  - `finstack_valuations::pricer::{PricerRegistry, ModelKey, InstrumentType, create_standard_registry}`
  - `finstack_valuations::metrics::{MetricId, MetricRegistry, MetricContext, standard_registry}` (plus VaR via `metrics::risk`)
  - `finstack_valuations::covenants::{Covenant, CovenantType, CovenantEngine, GenericCovenantForecast, CovenantForecastConfig}`
  - `finstack_valuations::attribution::{AttributionMethod, AttributionEnvelope, attribute_pnl_parallel, attribute_pnl_waterfall, attribute_pnl_metrics_based, JsonEnvelope}`
  - `finstack_valuations::calibration::{api::*, SolverConfig, CalibrationConfig, ValidationConfig}` and bump helpers `calibration::bumps::{bump_discount_curve_synthetic, bump_hazard_spreads, bump_inflation_rates, BumpRequest}`
- **Deprecated module paths**: deep module imports (e.g., `instruments::common::models`, `calibration::bumps::rates`, `covenants::engine`, `attribution::types`) are no longer supported. Switch to the canonical imports above.

---

## Architecture

```
finstack-valuations/
│
├── instruments/           40+ instrument types (bonds, swaps, options, credit, etc.)
│   ├── common/           Shared traits, pricing models, MC engine, parameters
│   ├── bond/             Fixed/floating bonds, callable/putable, amortizing
│   ├── irs/              Interest rate swaps and basis swaps
│   ├── cds/              Credit default swaps and indices
│   ├── equity_option/    Vanilla and exotic equity options
│   ├── fx_option/        FX options (vanilla and barrier)
│   └── [30+ more...]     See instruments/README.md
│
├── pricer/               Registry-based pricing dispatch
│   ├── registry          Type-safe instrument → pricer routing
│   └── traits            Pricer<T> trait and model keys
│
├── metrics/              Risk metric calculators and registry
│   ├── core/             MetricId, registry, dependency resolution
│   └── sensitivities/    DV01, CS01, Greeks, Theta, Vega
│
├── cashflow/             Schedule generation and aggregation
│   ├── builder/          Composable cashflow schedule construction
│   ├── specs/            Coupon, fee, amortization, credit models
│   ├── emission/         Cashflow emission helpers
│   └── aggregation       Period rollup, PV calculation (currency-safe)
│
├── calibration/          Curve and surface calibration from market quotes
│   ├── bootstrap/        Linear and cubic bootstrapping
│   ├── optimization/     Levenberg-Marquardt curve fitting
│   ├── quotes/           Market quote types (deposit, swap, CDS, options)
│   └── workflows/        High-level calibration orchestration
│
├── attribution/          P&L attribution (carry, rates, credit, FX, vol, etc.)
│   ├── parallel          Independent factor isolation
│   ├── waterfall         Sequential factor application (guaranteed sum)
│   ├── metrics_based     Fast linear approximation via pre-computed metrics
│   └── spec              JSON specification framework
│
├── covenants/            Covenant evaluation, cure periods, consequences
│   ├── engine            Evaluation engine and consequence application
│   ├── forward           Covenant forecasting with headroom analytics
│   └── schedule          Threshold schedules for time-varying limits
│
├── results/              Valuation result envelopes and metadata
│   ├── valuation_result  ValuationResult (PV + metrics + meta)
│   └── dataframe         DataFrame export helpers
│
├── constants             Common numerical constants (basis points, etc.)
└── schema                JSON Schema generation for API contracts
```

---

## Key Features

### 1. Comprehensive Instrument Coverage

Over 40 instrument types with consistent pricing and risk interfaces:

**Fixed Income**

- Bonds (fixed, floating, callable, putable, amortizing, convertible)
- Inflation-linked bonds (TIPS)
- Term loans, revolving credit

**Interest Rate Derivatives**

- Interest rate swaps (vanilla, basis, inflation)
- Swaptions (European, Bermudan)
- Caps, floors, FRAs
- Interest rate futures

**Credit Derivatives**

- Credit default swaps (single-name, indices, tranches)
- CDS options
- Structured credit (ABS, RMBS, CMBS, CLO)

**Equity & FX Derivatives**

- Vanilla options (equity and FX)
- Exotic options (Asian, barrier, lookback, autocallable, quanto, cliquet)
- Total return swaps
- Variance swaps

**See [instruments/README.md](src/instruments/README.md) for complete catalog and usage.**

---

### 2. Pricing Models

**Analytical Models**

- Black-Scholes-Merton (equity/FX options)
- Black (1976) for caps, floors, swaptions
- Garman-Kohlhagen (FX options)
- SABR (stochastic volatility surfaces)
- Barrier formulas (Rubinstein-Reiner)
- Asian approximations (Turnbull-Wakeman, geometric averaging)

**Tree Methods**

- Binomial trees (Cox-Ross-Rubinstein, Jarrow-Rudd, Tian, LR)
- Trinomial trees (short rate models, convertibles)
- Hull-White interest rate trees

**Monte Carlo** (requires `mc` feature)

- Processes: GBM, Heston, CIR, Ornstein-Uhlenbeck, jump diffusion, Schwartz-Smith
- Discretization: Euler, Milstein, exact (GBM/HW1F), QE (Heston/CIR)
- Longstaff-Schwartz (LSM) for American/Bermudan exercise
- Variance reduction: antithetic variates, control variates, moment matching

---

### 3. Risk Metrics

**Fixed Income**

- YTM (yield to maturity)
- Duration (Macaulay, modified)
- Convexity
- DV01 (dollar value of a basis point)
- CS01 (credit spread sensitivity)
- Z-spread, I-spread, OAS (option-adjusted spread)

**Options**

- Delta, Gamma, Vega, Theta, Rho
- Volga, Vanna
- Charm, Vomma

**Swaps**

- Par rate
- DV01 (parallel and bucketed)
- Annuity

**See [metrics/README.md](src/metrics/README.md) for complete metric catalog and calculator framework.**

---

### 4. Cashflow Generation

Composable builder pattern for complex schedules:

- **Fixed and floating coupons** with amortization
- **Multiple amortization styles**: linear, step, percent-per-period
- **Tiered fee structures**: notional-based, balance-based
- **Credit events**: prepayment (CPR, PSA), default (CDR, SDA), recovery models
- **PIK capitalization** and notional draws/repayments
- **Period aggregation**: currency-safe rollup with PV calculation
- **Credit-adjusted PV**: integration with hazard curves

**See [cashflow/README.md](src/cashflow/README.md) for detailed builder API and examples.**

---

### 5. Curve Calibration

Bootstrap and optimize curves from market quotes:

**Quote Types**

- Deposit rates (money market)
- OIS, LIBOR, SOFR swaps
- CDS spreads (single-name and indices)
- Option implied volatilities (caps, swaptions, equity)

**Calibration Methods**

- Linear bootstrapping (exact fit to quotes)
- Cubic spline bootstrapping (smooth interpolation)
- Levenberg-Marquardt optimization (minimize squared errors)

**Curve Types**

- Discount curves (zero-coupon rates)
- Forward curves (floating rate projection)
- Hazard curves (credit default intensity)
- Volatility surfaces (strike/expiry grid)

**Outputs**

- Calibrated curves with JSON serialization
- Detailed calibration reports (quote errors, solver diagnostics)
- Jacobian matrices for sensitivity analysis

**See [calibration/README.md](src/calibration/README.md) for calibration workflows and market data integration.**

---

### 6. P&L Attribution

Decompose daily mark-to-market changes into constituent factors:

**Three Methodologies**

- **Parallel**: Independent factor isolation (5-15% residual for large moves)
- **Waterfall**: Sequential application (residual < 0.01%, guaranteed sum)
- **Metrics-Based**: Fast linear approximation via pre-computed metrics (2-10% residual with second-order terms)

**Attribution Factors**

1. Carry (time decay, theta)
2. Rates curves (discount and forward curve shifts)
3. Credit curves (hazard curve shifts)
4. Inflation curves
5. Correlations (base correlation for structured credit)
6. FX (currency translation and exposure)
7. Volatility (implied vol changes)
8. Model parameters (prepayment, default, recovery, conversion)
9. Market scalars (spot prices, dividends, indices)

**Detailed Breakdowns**

- Per-curve and per-tenor P&L
- Per-currency-pair FX attribution
- Per-surface volatility attribution
- Human-readable explanation trees

**See [attribution/README.md](src/attribution/README.md) for methodology details and usage examples.**

---

### 7. Covenant Management

Evaluate financial and non-financial covenants with cure periods and consequences:

**Covenant Types**

- Financial: leverage, coverage, liquidity, asset coverage
- Non-financial: affirmative and negative covenants
- Custom: user-defined metrics and evaluators

**Consequence Framework**

- Rate increases, cash sweeps, distribution blocks
- Collateral requirements, maturity acceleration
- Multi-level escalation support

**Forward Projection**

- Deterministic projection via time-series models
- Stochastic (MC) with breach probability estimation
- Headroom analytics (warning periods, minimum cushion)

**See [covenants/README.md](src/covenants/README.md) for evaluation engine and forecasting.**

---

### 8. Results Framework

Standardized result envelopes with metadata stamping:

**ValuationResult Structure**

- Present value (currency-safe `Money` type)
- Risk metrics (key-value pairs, extensible)
- Metadata (rounding policy, numeric mode, FX policy, timing)
- Optional covenants (compliance reports)
- Optional explanation (computation traces)

**DataFrame Exports**

- Flat row representation for analytics
- Polars/CSV/Parquet interoperability
- Promoted common metrics (DV01, convexity, duration, YTM)

**See [results/README.md](src/results/README.md) for result structure and export utilities.**

---

## Quick Start

### Basic Bond Pricing

```rust
use finstack_valuations::instruments::Bond;
use finstack_valuations::pricer::create_standard_registry;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::create_date;
use finstack_core::market_data::context::MarketContext;
use time::Month;

// Create pricing registry
let registry = create_standard_registry();

// Build a fixed-rate bond
let issue = create_date(2025, Month::January, 15)?;
let maturity = create_date(2030, Month::January, 15)?;
let bond = Bond::fixed(
    "US-BOND-001",
    Money::new(1_000_000.0, Currency::USD),
    0.05,           // 5% coupon
    issue,
    maturity,
    "USD-OIS"       // Discount curve ID
);

// Create market context with calibrated curves
let market = MarketContext::new()
    .with_discount_curve(usd_ois_curve);
let as_of = create_date(2025, Month::January, 1)?;

// Price the bond
let pv = bond.value(&market, as_of)?;
println!("Bond NPV: {}", pv);
```

### Computing Risk Metrics

```rust
use finstack_valuations::metrics::MetricId;

// Request specific metrics (strict mode - new in 0.8.0)
let metrics = vec![
    MetricId::Ytm,
    MetricId::DurationMod,  // Modified duration
    MetricId::Convexity,
    MetricId::Dv01,
];

// price_with_metrics now uses strict mode (errors for unknown/failed metrics)
let result = bond.price_with_metrics(&market, as_of, &metrics)?;

println!("NPV: {}", result.value);
println!("YTM: {:.2}%", result.metric("ytm").unwrap() * 100.0);
println!("Modified Duration: {:.2}", result.metric("duration_mod").unwrap());
println!("DV01: ${:.2}", result.metric("dv01").unwrap());

// Handle errors explicitly or use smaller metric sets if needed.
```

### Curve Calibration

```rust
use finstack_valuations::calibration::{
    SimpleCalibration, MarketQuote, RatesQuote, CalibrationConfig
};
use finstack_core::currency::Currency;
use finstack_core::dates::create_date;
use time::Month;

let base_date = create_date(2025, Month::January, 15)?;

// Create calibration builder
let mut calibration = SimpleCalibration::new(base_date, Currency::USD);

// Add market quotes
calibration.add_deposit_quote(/* ... */);
calibration.add_swap_quote(/* ... */);

// Calibrate discount curve
let (market_context, report) = calibration.calibrate()?;

println!("Calibration successful:");
println!("  Max quote error: {:.2} bps", report.max_error_bps);
println!("  Curves built: {:?}", report.curve_ids);
```

### Cashflow Schedule Building

```rust
use finstack_valuations::cashflow::builder::{
    CashFlowSchedule, FixedCouponSpec, CouponType
};
use finstack_core::dates::{Tenor, DayCount, BusinessDayConvention, StubKind};
use finstack_core::dates::DayCountCtx;

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .fixed_cf(FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05,
        freq: Tenor::semi_annual(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::Following,
        calendar_id: "weekends_only".to_string(),
        stub: StubKind::None,
        end_of_month: false,
        payment_lag_days: 0,
    })
    .build_with_curves(None)?;

// Compute periodized PV
let periods = vec![/* quarterly periods */];
let pv_map = schedule.pv_by_period_with_ctx(
    &periods,
    &discount_curve,
    as_of,
    DayCount::Act365F,
    DayCountCtx::default(),
)?;
```

### P&L Attribution

```rust
use finstack_valuations::attribution::attribute_pnl_parallel;
use finstack_core::config::FinstackConfig;

let attribution = attribute_pnl_parallel(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &FinstackConfig::default(),
)?;

println!("Total P&L: {}", attribution.total_pnl);
println!("  Carry: {}", attribution.carry);
println!("  Rates: {}", attribution.rates_curves_pnl);
println!("  Credit: {}", attribution.credit_curves_pnl);
println!("  FX: {}", attribution.fx_pnl);
println!("  Residual: {} ({:.2}%)", attribution.residual, attribution.meta.residual_pct);
```

---

## Determinism and Reproducibility

All pricing and risk calculations are deterministic by default:

1. **Decimal Arithmetic**: Via `rust_decimal`, ensures consistent results across platforms and runs
2. **Monte Carlo**: Seedable RNGs with stable algorithms; parallel ≡ serial
3. **Calibration**: Deterministic iteration orders and solver termination
4. **Currency Safety**: Explicit FX conversions with policy stamping
5. **Stable Ordering**: Cashflows, periods, and aggregation use deterministic sorting

**Metadata Stamping**: Every result includes:

- Numeric mode (Decimal vs f64)
- Rounding context and precision
- FX policy for cross-currency calculations
- Calculation timestamp and duration
- Parallel execution flag

---

## Performance

- **Vectorized Execution**: Polars-based expression engine for time-series operations
- **Caching**: Intermediate results (curves, cashflows) cached per valuation
- **Parallelism**: Optional Rayon parallelism without changing Decimal results
- **Lazy Evaluation**: Metrics computed only when requested
- **Fast Paths**: Optimized NPV-only calculations for portfolio aggregation

**Benchmarks** (M1 MacBook Pro, release build):

- Bond pricing (NPV only): ~10-50 μs
- Bond pricing + 5 metrics: ~100-200 μs
- Swap pricing (10Y): ~50-100 μs
- Equity option (Black-Scholes): ~20-40 μs
- MC Asian option (10k paths): ~5-10 ms
- Portfolio valuation (100 instruments): ~10-50 ms

---

## Error Handling

All public APIs return `Result<T, finstack_core::Error>` with structured error types:

- `CurveNotFound`: Missing discount, forward, or hazard curve
- `InvalidInstrument`: Inconsistent instrument parameters
- `CalibrationFailed`: Calibration did not converge
- `PricingError`: Pricing calculation failed
- `CurrencyMismatch`: Cross-currency arithmetic without explicit FX
- `DateError`: Invalid date or period parameters

---

## Feature Flags

```toml
[dependencies]
finstack-valuations = { version = "0.1", features = ["mc", "serde", "parallel"] }
```

**Available Features**:

- `mc`: Enable Monte Carlo pricing (~200KB binary increase)
- `serde`: Enable JSON serialization/deserialization
- `parallel`: Enable Rayon parallelism (deterministic results maintained)

---

## Language Bindings

### Python (finstack-py)

The valuations engine is available in Python via PyO3 bindings with Pydantic v2 models:

```python
from finstack import Bond, MarketContext, MetricId
from decimal import Decimal

# Create bond
bond = Bond.fixed(
    id="US-BOND-001",
    notional={"amount": Decimal("1000000"), "currency": "USD"},
    coupon_rate=0.05,
    issue_date="2025-01-15",
    maturity_date="2030-01-15",
    discount_curve_id="USD-OIS"
)

# Price with metrics
result = bond.price_with_metrics(
    market=market,
    as_of="2025-01-01",
    metrics=[MetricId.YTM, MetricId.DV01, MetricId.CONVEXITY]
)

print(f"NPV: {result.value}")
print(f"YTM: {result.measures['ytm']:.2%}")
print(f"DV01: ${result.measures['dv01']:.2f}")
```

**Features**:

- Wheels for Linux, macOS, Windows (x86_64, aarch64)
- Pydantic v2 models mirror Rust serde shapes
- Heavy compute releases the GIL for parallelism
- DataFrame-friendly outputs (Polars/pandas interop)

**Installation**:

```bash
pip install finstack
```

---

### WebAssembly (finstack-wasm)

Browser and Node.js support via wasm-bindgen:

```javascript
import init, { Bond, MarketContext } from 'finstack-wasm';

await init();

const bond = Bond.fixed({
  id: "US-BOND-001",
  notional: { amount: "1000000", currency: "USD" },
  couponRate: 0.05,
  issueDate: "2025-01-15",
  maturityDate: "2030-01-15",
  discountCurveId: "USD-OIS"
});

const result = await bond.priceWithMetrics(market, "2025-01-01", ["ytm", "dv01"]);
console.log(`NPV: ${result.value.amount} ${result.value.currency}`);
console.log(`YTM: ${(result.measures.ytm * 100).toFixed(2)}%`);
```

**Features**:

- JSON IO parity with Rust serde
- Feature flags for tree-shaking and small bundles
- Same determinism guarantees as native Rust
- SIMD support where available

**Installation**:

```bash
npm install finstack-wasm
```

---

## Module Reference

| Module | Purpose | Key Types | README |
|--------|---------|-----------|--------|
| `instruments` | 40+ instrument types | `Bond`, `InterestRateSwap`, `EquityOption`, `CreditDefaultSwap` | [instruments/README.md](src/instruments/README.md) |
| `pricer` | Pricing dispatch | `PricerRegistry`, `InstrumentType`, `ModelKey` | See lib.rs docs |
| `metrics` | Risk calculators | `MetricId`, `MetricCalculator`, `MetricRegistry` | [metrics/README.md](src/metrics/README.md) |
| `cashflow` | Schedule generation | `CashFlowSchedule`, `CashFlowBuilder`, `CFKind` | [cashflow/README.md](src/cashflow/README.md) |
| `calibration` | Curve fitting | `SimpleCalibration`, `MarketQuote`, `CalibrationReport` | [calibration/README.md](src/calibration/README.md) |
| `attribution` | P&L decomposition | `PnlAttribution`, `AttributionFactor`, `AttributionSpec` | [attribution/README.md](src/attribution/README.md) |
| `covenants` | Covenant evaluation | `CovenantEngine`, `Covenant`, `GenericCovenantForecast` | [covenants/README.md](src/covenants/README.md) |
| `results` | Result envelopes | `ValuationResult`, `ValuationRow`, `ResultsMeta` | [results/README.md](src/results/README.md) |
| `constants` | Numerical constants | `BP_TO_RATE`, `DAYS_PER_YEAR`, etc. | See source |
| `schema` | JSON schema generation | `schema_for`, `JsonSchema` | See source |

---

## Design Principles

1. **Correctness First**: Accounting-grade numerics, currency safety, no cross-currency arithmetic without explicit FX
2. **Performance Second**: Vectorization and parallelism without changing Decimal outputs
3. **Ergonomic APIs**: Builder patterns, fluent interfaces, sensible defaults
4. **Documentation**: Every public API documented with examples
5. **Testing**: Unit, property, golden, and parity tests across all modules
6. **Stability**: Strict serde names, schema versioning, backward-compatible APIs
7. **Extensibility**: Custom instruments, metrics, models, and consequences via traits

---

## Testing

Run tests for the valuations crate:

```bash
# All tests
cargo test -p finstack-valuations

# Specific module
cargo test -p finstack-valuations cashflow::

# Integration tests
cargo test -p finstack-valuations --test '*'

# With Monte Carlo features
cargo test -p finstack-valuations --features mc

# Ignored (slow) tests
cargo test -p finstack-valuations -- --ignored
```

---

## Development Workflow

```bash
# Lint and format
make lint

# Run all tests
make test-rust

# Build Python bindings (after Rust changes)
make python-dev

# Generate documentation
cargo doc -p finstack-valuations --open
```

---

## See Also

- **[finstack-core](../core/)**: Core primitives (Money, dates, curves, expressions)
- **[finstack-statements](../statements/)**: Financial statement modeling
- **[finstack-portfolio](../portfolio/)**: Multi-instrument portfolio aggregation
- **[finstack-scenarios](../scenarios/)**: Scenario analysis and stress testing
- **[finstack-io](../io/)**: Market data I/O (CSV/Parquet/Bloomberg/FRED)

---

## Contributing

When adding new features:

1. Follow established patterns (see sub-module READMEs)
2. Add comprehensive tests (unit + integration + property)
3. Document all public APIs with examples
4. Maintain currency-safety invariants
5. Preserve determinism (serial ≡ parallel)
6. Update relevant README.md files
7. Run `make lint` and `make test-rust` before committing

For questions or feature requests, please open an issue or contact the Finstack team.

---

## License

Copyright © 2025 Finstack. All rights reserved.
