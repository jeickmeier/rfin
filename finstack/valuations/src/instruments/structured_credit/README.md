# Structured Credit Module

Unified implementation for ABS, RMBS, CMBS, and CLO instruments with waterfall modeling, behavioral assumptions, and comprehensive risk metrics.

## Quick Reference

```rust
use finstack_valuations::instruments::structured_credit::prelude::*;

// Create a CLO
let clo = StructuredCredit::new_clo(
    "MY_CLO", pool, tranches, waterfall,
    closing_date, legal_maturity, "USD-OIS",
);

// Value the deal
let pv = clo.value(&market_context, as_of)?;

// Get metrics
let result = clo.price_with_metrics(&context, as_of, &[
    MetricId::WAL, MetricId::DurationMod, MetricId::Cs01,
])?;
```

## Module Structure

```
structured_credit/
├── mod.rs              # Main module with comprehensive documentation
├── README.md           # This file
│
├── types/              # Main instrument types
│   ├── mod.rs          # StructuredCredit struct, trait implementations
│   ├── constructors.rs # new_abs(), new_clo(), new_cmbs(), new_rmbs()
│   ├── reinvestment.rs # ReinvestmentManager for CLO reinvestment periods
│   └── stochastic.rs   # Stochastic configuration helpers
│
├── config/             # Configuration and constants
│   ├── mod.rs          # Re-exports
│   ├── constants.rs    # Industry constants (PSA, SDA, fees, etc.)
│   └── structures.rs   # DealConfig, DealDates, DealFees, CoverageTestConfig
│
├── components/         # Building blocks (organized by pricing mode)
│   ├── mod.rs          # Re-exports with clear common/deterministic/stochastic grouping
│   │
│   │   # COMMON (both deterministic & stochastic)
│   ├── enums.rs        # DealType, AssetType, TrancheSeniority, PaymentMode
│   ├── pool.rs         # AssetPool, PoolAsset, PoolStats
│   ├── tranches.rs     # Tranche, TrancheStructure, TrancheCoupon
│   ├── waterfall.rs    # WaterfallEngine, WaterfallTier, Recipient
│   ├── coverage_tests.rs # OC/IC test calculations
│   ├── diversion.rs    # Diversion rules with cycle detection
│   ├── validation.rs   # WaterfallValidator, ValidationError
│   ├── rates.rs        # CPR/SMM, CDR/MDR, PSA conversions
│   ├── rate_helpers.rs # Floating rate projection helpers
│   ├── tranche_valuation.rs # Per-tranche WAL, duration, Z-spread, CS01
│   │
│   │   # DETERMINISTIC (single-path behavioral models)
│   ├── specs.rs        # PSA, SDA, constant CPR/CDR curves
│   ├── market_context.rs # MarketConditions, CreditFactors for behavioral models
│   │
│   │   # STOCHASTIC (multi-path simulation)
│   └── stochastic/     # Copula, intensity, factor models (see stochastic/README.md)
│
├── metrics/            # Risk metrics by category
│   ├── mod.rs          # Registration function, re-exports
│   ├── pricing/        # WAL, accrued, clean/dirty prices
│   ├── risk/           # Duration, spreads, DV01, CS01, sensitivities
│   ├── pool/           # WARF, WAS, CPR, CDR pool characteristics
│   └── deal_specific/  # ABS, CLO, CMBS, RMBS-specific metrics
│
├── templates/          # Reusable waterfall templates
│   ├── mod.rs          # Template registry
│   ├── clo.rs          # CLO 2.0 standard waterfall
│   ├── cmbs.rs         # CMBS standard waterfall
│   └── cre.rs          # CRE operating company waterfall
│
├── pricer.rs           # StructuredCreditDiscountingPricer
├── instrument_trait.rs # StructuredCreditInstrument trait + simulation
└── simulation_helpers.rs # Internal simulation helpers
```

## Deal Types

| Type | Collateral | Prepayment Model | Key Metrics |
|------|------------|------------------|-------------|
| **ABS** | Auto loans, credit cards | CPR/ABS speed | Charge-off, delinquency |
| **CLO** | Leveraged loans | 15% CPR default | WARF, WAS, covenant |
| **CMBS** | Commercial mortgages | 10% CPR (lockout) | DSCR, LTV |
| **RMBS** | Residential mortgages | PSA/SDA curves | FICO, LTV |

## Behavioral Models

### Deterministic Models (`components/specs.rs`)

Single-path models for standard valuation:

| Model | Type | Use Case |
|-------|------|----------|
| **PSA** | Prepayment | RMBS standard ramp curve |
| **Constant CPR** | Prepayment | Flat annual rate |
| **SDA** | Default | RMBS standard ramp curve |
| **Constant CDR** | Default | Flat annual rate |
| **Constant Recovery** | Recovery | Fixed rate with lag |

### Stochastic Models (`components/stochastic/`)

Multi-path simulation for advanced analytics:

| Model | Type | Use Case |
|-------|------|----------|
| **Copula-based** | Default | Correlated defaults, tail risk |
| **Intensity process** | Default | Time-varying hazard rates |
| **Factor-correlated** | Prepayment | Rate-sensitive prepayment |
| **Richard-Roll** | Prepayment | Mortgage refinancing model |

### When to Use Each

| Scenario | Use |
|----------|-----|
| Day-to-day valuation | Deterministic |
| Regulatory reporting | Deterministic |
| VaR / Expected Shortfall | Stochastic |
| Correlation risk analysis | Stochastic |
| Stress testing | Either (depends on complexity) |

## Waterfall Mechanics

```
Pool Cashflows → Fees → Senior Interest → Subordinate Interest → Principal → Equity
                        ↓ (if OC/IC fail)
                        Turbo to Senior
```

## Key APIs

### Creating Instruments
```rust
// With defaults
let clo = StructuredCredit::new_clo(id, pool, tranches, waterfall, close, maturity, curve);
let abs = StructuredCredit::new_abs(...);
let cmbs = StructuredCredit::new_cmbs(...);
let rmbs = StructuredCredit::new_rmbs(...);

// Using builder
let deal = StructuredCredit::builder()
    .id("DEAL_ID")
    .deal_type(DealType::CLO)
    .pool(pool)
    .tranches(tranches)
    // ...
    .build()?;
```

### Valuation
```rust
// Simple NPV
let pv = deal.value(&context, as_of)?;

// With metrics
let result = deal.price_with_metrics(&context, as_of, &[MetricId::WAL])?;

// Per-tranche
let tranche_val = deal.value_tranche_with_metrics("CLASS_A", &context, as_of, &metrics)?;
```

### Stochastic Pricing
```rust
// Enable stochastic models
deal.enable_stochastic_defaults();

// Or configure manually
deal.with_stochastic_prepay(StochasticPrepaySpec::rmbs_agency(0.045))
    .with_stochastic_default(StochasticDefaultSpec::copula(0.02, 0.25))
    .with_correlation(CorrelationStructure::rmbs_standard());
```

## Configuration

### Constants (`config/constants.rs`)
- Fee defaults (CLO_SENIOR_MGMT_FEE_BPS, etc.)
- PSA/SDA model parameters
- Concentration limits

### Structures (`config/structures.rs`)
- `DealConfig` - Complete deal configuration
- `DealDates` - Key dates
- `DealFees` - Fee structure by deal type
- `CoverageTestConfig` - OC/IC triggers
- `DefaultAssumptions` - Behavioral assumptions

## Primary Documentation

The most comprehensive documentation is in `mod.rs`, which includes:
- Detailed waterfall mechanics
- Pricing methodology
- Behavioral model explanations
- Academic/industry references
- Usage examples

See also `components/stochastic/README.md` for stochastic modeling.

## Related Crates

- `finstack_core::dates` - Schedule generation, day counts
- `finstack_core::money` - Currency-safe arithmetic
- `finstack_valuations::metrics` - Metric registry and calculators
