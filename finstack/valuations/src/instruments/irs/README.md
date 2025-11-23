# Interest Rate Swap (IRS) Module

A production-grade implementation of interest rate swaps following ISDA 2006/2021 market conventions with support for both term-rate (LIBOR-style) and overnight-indexed (RFR-style) swaps.

---

## Table of Contents

- [Overview](#overview)
- [Module Structure](#module-structure)
- [Core Features](#core-features)
- [Usage Examples](#usage-examples)
- [Available Metrics](#available-metrics)
- [Market Standards](#market-standards)
- [Adding New Features](#adding-new-features)
- [Testing](#testing)
- [References](#references)

---

## Overview

Interest rate swaps (IRS) are OTC derivatives where two parties exchange fixed and floating interest rate cashflows on a notional amount. This module provides:

- **Plain vanilla swaps**: Fixed vs. floating rate exchanges
- **OIS swaps**: Overnight-indexed swaps with compounded-in-arrears rates
- **Par rate calculations**: Computing the fair fixed rate for zero initial value
- **Risk metrics**: DV01, bucketed DV01, theta, annuity

Basis swaps have their own instrument type -- TODO: Should we consolidate?

### Key Characteristics

- **Deterministic**: Decimal-based arithmetic for accounting-grade accuracy
- **Currency-safe**: No implicit cross-currency operations
- **Market-standard**: ISDA 2006/2021 conventions with citations
- **Production-ready**: Comprehensive validation, error handling, and edge case coverage

---

## Module Structure

```
irs/
├── mod.rs              # Public API and module documentation
├── types.rs            # InterestRateSwap struct and trait implementations
├── pricer.rs           # NPV calculation and leg valuation helpers
├── cashflow.rs         # Cashflow schedule generation
├── compounding.rs      # Floating leg compounding conventions
├── metrics/            # Swap-specific analytics
│   ├── mod.rs          # Metric registration
│   ├── annuity.rs      # Fixed-leg annuity calculator
│   ├── par_rate.rs     # Par swap rate calculator
│   ├── pv_fixed.rs     # Fixed leg PV calculator
│   └── pv_float.rs     # Float leg PV calculator
└── README.md           # This file
```

### Design Philosophy

- **Separation of concerns**: Types in `types.rs`, pricing in `pricer.rs`, cashflows in `cashflow.rs`
- **Trait-based**: Implements `Instrument`, `CashflowProvider`, `HasDiscountCurve`, `CurveDependencies`
- **Generic metrics**: DV01 and bucketed DV01 use generic implementations from `metrics/`
- **Focused files**: Each file has a single, well-defined responsibility

---

## Core Features

### 1. **Swap Construction**

Multiple construction methods:

- **Builder pattern**: Fine-grained control over all parameters
- **Convenience constructors**: `create_usd_swap()`, `example()` for common cases -- TODO: Add more convenience constructors
- **Market standard configs**: ISDA-compliant defaults for major currencies

### 2. **Pricing**

Accurate NPV calculation under the risk-neutral measure:

```text
PV_swap = PV_fixed_leg - PV_float_leg (for PayFixed)
PV_swap = PV_float_leg - PV_fixed_leg (for ReceiveFixed)
```

**Fixed Leg:**
```text
PV_fixed = N × K × Σ τᵢ × DF(Tᵢ)
```

**Floating Leg (Term Rate):**
```text
PV_float = N × Σ τᵢ × [Fwd(t_i) + Spread] × DF(Tᵢ)
```

**Floating Leg (OIS):**
```text
PV_float = N × [DF(T_start) - DF(T_end)] + spread_annuity
```

### 3. **Compounding Conventions**

- **Simple (LIBOR-style)**: `FloatingLegCompounding::Simple`
- **Compounded in Arrears (RFR-style)**: `FloatingLegCompounding::CompoundedInArrears`
- **Market presets**: `sofr()`, `sonia()`, `estr()`, `tona()`

### 4. **Par Rate Calculation**

Two methods available:

- **ForwardBased** (default): `Par = Float_PV / (Notional × Annuity)`
  - Works for seasoned and unseasoned swaps
  - Requires forward curve
  
- **DiscountRatio**: `Par = [DF(start) - DF(end)] / Annuity`
  - Closed-form solution
  - Only valid for unseasoned swaps (as_of ≤ start_date)

### 5. **Risk Metrics**

- **DV01**: Dollar value of 1bp parallel curve shift
- **Bucketed DV01**: Key-rate sensitivities
- **Theta**: Time decay (P&L from rolling forward 1 day)
- **Annuity**: Present value of $1 paid each period

### 6. **Cashflow Generation**

Multiple formats:

- **Signed dated flows**: Simple `(Date, Money)` pairs
- **Full schedules**: Complete `CashFlowSchedule` with CFKind metadata
- **Leg-specific schedules**: Separate fixed and floating leg schedules

---

## Usage Examples

### Example 1: Create a Simple USD Swap

```rust
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use time::macros::date;

let swap = InterestRateSwap::create_usd_swap(
    InstrumentId::new("IRS-5Y-USD"),
    Money::new(10_000_000.0, Currency::USD),
    0.03,  // 3% fixed rate
    date!(2024-01-01),
    date!(2029-01-01),
    PayReceive::PayFixed,
)?;
```

### Example 2: Price a Swap

```rust
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_valuations::instruments::common::traits::Instrument;

// Build market curves
let disc_curve = DiscountCurve::builder("USD-OIS")
    .base_date(date!(2024-01-01))
    .day_count(DayCount::Act360)
    .knots([
        (0.0, 1.0),
        (1.0, 0.95),
        (5.0, 0.78),
        (10.0, 0.61),
    ])
    .build()?;

let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
    .base_date(date!(2024-01-01))
    .day_count(DayCount::Act360)
    .knots([
        (0.0, 0.05),
        (10.0, 0.05),
    ])
    .build()?;

let market = MarketContext::new()
    .insert_discount(disc_curve)
    .insert_forward(fwd_curve);

// Price the swap
let npv = swap.value(&market, date!(2024-01-01))?;
println!("Swap NPV: {}", npv);
```

### Example 3: Create an OIS Swap

```rust
use finstack_valuations::instruments::irs::FloatingLegCompounding;

let mut swap = InterestRateSwap::create_usd_swap(
    InstrumentId::new("OIS-5Y-USD"),
    Money::new(10_000_000.0, Currency::USD),
    0.025,
    date!(2024-01-01),
    date!(2029-01-01),
    PayReceive::PayFixed,
)?;

// Use overnight compounding and align float index with discount curve
swap.float.compounding = FloatingLegCompounding::sofr();
swap.float.forward_curve_id = swap.fixed.discount_curve_id.clone();

// Now pricing will use OIS-specific logic
let npv = swap.value(&market, date!(2024-01-01))?;
```

### Example 4: Calculate Par Rate

```rust
use finstack_valuations::metrics::MetricId;

let result = swap.price_with_metrics(
    &market,
    date!(2024-01-01),
    &[MetricId::ParRate],
)?;

let par_rate = result.get_metric(&MetricId::ParRate)?;
println!("Par swap rate: {:.4}%", par_rate * 100.0);
```

### Example 5: Compute Risk Metrics

```rust
let result = swap.price_with_metrics(
    &market,
    date!(2024-01-01),
    &[
        MetricId::Dv01,
        MetricId::BucketedDv01,
        MetricId::Theta,
        MetricId::Annuity,
    ],
)?;

println!("DV01: ${:.2}", result.get_metric(&MetricId::Dv01)?);
println!("Theta: ${:.2}", result.get_metric(&MetricId::Theta)?);
println!("Annuity: {:.6}", result.get_metric(&MetricId::Annuity)?);
```

### Example 6: Generate Cashflows

```rust
use finstack_valuations::cashflow::traits::CashflowProvider;

let flows = swap.build_schedule(&market, date!(2024-01-01))?;
for (date, amount) in flows {
    println!("{}: {}", date, amount);
}

// Or get the full schedule with metadata
let full_schedule = swap.build_full_schedule(&market, date!(2024-01-01))?;
for cf in full_schedule.flows {
    println!("{}: {} ({:?})", cf.date, cf.amount, cf.kind);
}
```

### Example 7: Using the Builder Pattern

```rust
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_valuations::instruments::common::parameters::legs::{FixedLegSpec, FloatLegSpec};

let swap = InterestRateSwap::builder()
    .id(InstrumentId::new("IRS-CUSTOM"))
    .notional(Money::new(5_000_000.0, Currency::EUR))
    .side(PayReceive::ReceiveFixed)
    .fixed(FixedLegSpec {
        discount_curve_id: CurveId::new("EUR-OIS"),
        rate: 0.02,
        freq: Frequency::annual(),
        dc: DayCount::Thirty360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: Some("EUR".to_string()),
        stub: StubKind::None,
        start: date!(2024-01-01),
        end: date!(2034-01-01),
        par_method: None,
        compounding_simple: true,
    })
    .float(FloatLegSpec {
        discount_curve_id: CurveId::new("EUR-OIS"),
        forward_curve_id: CurveId::new("EUR-EURIBOR-6M"),
        spread_bp: 25.0,  // 25bp spread
        freq: Frequency::semi_annual(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: Some("EUR".to_string()),
        stub: StubKind::None,
        reset_lag_days: 2,
        compounding: FloatingLegCompounding::Simple,
        start: date!(2024-01-01),
        end: date!(2034-01-01),
    })
    .build()?;
```

---

## Available Metrics

All metrics are registered in `metrics/mod.rs` and can be computed via `price_with_metrics()`.

| Metric ID | Description | Dependencies | Implementation |
|-----------|-------------|--------------|----------------|
| `Annuity` | Sum of discounted accrual factors on fixed leg | None | `annuity.rs` |
| `ParRate` | Fixed rate for zero NPV | `Annuity` | `par_rate.rs` |
| `PvFixed` | Present value of fixed leg only | None | `pv_fixed.rs` |
| `PvFloat` | Present value of float leg only | None | `pv_float.rs` |
| `Dv01` | Parallel curve shift sensitivity | None | Generic (unified) |
| `BucketedDv01` | Key-rate sensitivities | None | Generic (unified) |
| `Theta` | Time decay (1-day P&L) | None | Generic (standard) |

### Metric Registration

Metrics are registered in `metrics/mod.rs::register_irs_metrics()`:

```rust
pub fn register_irs_metrics(registry: &mut MetricRegistry) {
    registry.register(
        InstrumentType::IRS,
        MetricId::Annuity,
        Box::new(AnnuityCalculator),
    );
    // ... additional metrics
}
```

---

## Market Standards

### ISDA Conventions

This implementation follows **ISDA 2006 Definitions** (with 2008 OIS supplement) and **ISDA 2021 Definitions** for RFR swaps:

- **Section 4.1**: Fixed Rate Payer calculation conventions
- **Section 4.2**: Floating Rate Option conventions
- **Section 4.5**: Compounding methods
- **Section 4.16**: Business Day Conventions

### USD Market Standard

Per ISDA and US market practice:

- **Fixed Leg**: Semi-annual, 30/360, Modified Following
- **Floating Leg**: Quarterly, ACT/360, Modified Following
- **Reset Lag**: T-2 (2 business days before period start)
- **Discounting**: OIS curve (post-2008 multi-curve framework)

### Other Major Currencies

| Currency | Fixed Leg | Float Leg | Index |
|----------|-----------|-----------|-------|
| USD | Semi, 30/360 | Quarterly, ACT/360 | SOFR |
| EUR | Annual, 30/360 | Semi, ACT/360 | EURIBOR/€STR |
| GBP | Semi, ACT/365 | Semi, ACT/365 | SONIA |
| JPY | Semi, ACT/365 | Semi, ACT/365 | TONA |

### RFR Conventions (ISDA 2021)

- **SOFR (USD)**: 2-day lookback (ARRC)
- **SONIA (GBP)**: 5-day lookback (BoE)
- **€STR (EUR)**: 2-day shift (ECB)
- **TONA (JPY)**: 2-day lag (JSCC)

---

## Adding New Features

### 1. Adding a New Metric

Create a new file in `metrics/` and implement the `MetricCalculator` trait:

```rust
// metrics/my_metric.rs
use crate::instruments::InterestRateSwap;
use crate::metrics::{MetricCalculator, MetricContext};

pub struct MyMetricCalculator;

impl MetricCalculator for MyMetricCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let irs: &InterestRateSwap = context.instrument_as()?;
        let as_of = context.as_of;
        
        // Your calculation logic here
        Ok(0.0)
    }
    
    // Optional: declare dependencies on other metrics
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Annuity]
    }
}
```

Register the metric in `metrics/mod.rs`:

```rust
pub fn register_irs_metrics(registry: &mut MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "InterestRateSwap",
        metrics: [
            // ... existing metrics
            (MyMetric, my_metric::MyMetricCalculator),
        ]
    }
}
```

Add the metric ID to `metrics/metric_id.rs` (in the parent valuations module):

```rust
pub enum MetricId {
    // ... existing variants
    MyMetric,
}
```

### 2. Adding a New Compounding Method

Extend the `FloatingLegCompounding` enum in `compounding.rs`:

```rust
#[non_exhaustive]
pub enum FloatingLegCompounding {
    Simple,
    CompoundedInArrears { lookback_days: i32, observation_shift: Option<i32> },
    // Add your new variant:
    MyNewMethod { /* parameters */ },
}
```

Update the pricing logic in `pricer.rs` to handle the new method:

```rust
pub(crate) fn pv_float_leg(
    &self,
    disc: &DiscountCurve,
    fwd: &dyn Forward,
    as_of: Date,
) -> Result<Money> {
    match self.float.compounding {
        FloatingLegCompounding::Simple => { /* existing logic */ }
        FloatingLegCompounding::CompoundedInArrears { .. } => { /* existing logic */ }
        FloatingLegCompounding::MyNewMethod { .. } => {
            // Your implementation here
        }
    }
}
```

### 3. Adding a New Leg Type

If adding a new leg specification (e.g., amortizing notional):

1. **Extend `FixedLegSpec` or `FloatLegSpec`** in `common/parameters/legs.rs`
2. **Update cashflow builders** in `cashflow.rs` to handle new parameters
3. **Update pricer logic** in `pricer.rs` if pricing changes
4. **Add tests** in `tests/instruments/irs/`

### 4. Adding Support for a New Currency

Use the builder pattern with currency-specific conventions:

```rust
impl InterestRateSwap {
    pub fn create_eur_swap(
        id: InstrumentId,
        notional: Money,
        fixed_rate: f64,
        start: Date,
        end: Date,
        side: PayReceive,
    ) -> Result<Self> {
        let config = SwapConfig {
            disc_curve: "EUR-OIS",
            fwd_curve: "EUR-EURIBOR-6M",
            reset_lag_days: 2,
            sched: IRSScheduleConfig::eur_isda_standard(),
        };
        
        Self::create_swap_with_config(id, notional, fixed_rate, start, end, side, config)
    }
}
```

Add the standard schedule configuration:

```rust
impl IRSScheduleConfig {
    fn eur_isda_standard() -> Self {
        Self {
            fixed_freq: Frequency::annual(),
            fixed_dc: DayCount::Thirty360,
            float_freq: Frequency::semi_annual(),
            float_dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: Some("EUR".to_string()),
            stub: StubKind::None,
        }
    }
}
```

---

## Testing

The IRS module has comprehensive test coverage across multiple dimensions:

### Test Structure

```
tests/instruments/irs/
├── mod.rs                      # Test module organization
├── construction.rs             # Builder and constructor tests
├── cashflows.rs                # Cashflow generation tests
├── pricing.rs                  # NPV and leg valuation tests
├── proptests.rs                # Property-based tests
├── metrics/                    # Metric-specific tests
│   ├── annuity.rs
│   ├── par_rate.rs
│   ├── dv01.rs
│   ├── bucketed_dv01.rs
│   ├── theta.rs
│   ├── pv_fixed.rs
│   └── pv_float.rs
├── integration/                # Integration and parity tests
│   ├── complex_scenarios.rs
│   └── quantlib_parity.rs
└── validation/                 # Market standards validation
    └── market_standards.rs
```

### Running Tests

```bash
# Run all IRS tests
cargo test --package finstack-valuations irs

# Run specific test file
cargo test --package finstack-valuations --test pricing

# Run with output
cargo test --package finstack-valuations irs -- --nocapture

# Run property tests (longer running)
cargo test --package finstack-valuations proptests -- --ignored
```

### Test Categories

1. **Unit Tests**: Individual function validation
2. **Integration Tests**: End-to-end pricing and metrics
3. **Property Tests**: Invariants across random inputs
4. **Parity Tests**: Cross-validation with QuantLib/Bloomberg
5. **Market Standards Tests**: ISDA convention compliance

### Adding Tests

When adding new features, add corresponding tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_new_feature() {
        // Arrange
        let swap = InterestRateSwap::example().unwrap();
        
        // Act
        let result = my_new_function(&swap);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

---

## References

### ISDA Documentation

- **ISDA 2006 Definitions**: Standard definitions for interest rate derivatives
- **ISDA 2021 Definitions**: RFR conventions for overnight-indexed swaps
- **ISDA 2008 OIS Supplement**: Multi-curve framework post-financial crisis

### Books and Papers

- **"Interest Rate Swaps and Their Derivatives"** by Amir Sadr
- **"Interest Rate Risk Modeling"** by Sanjay Sharma
- **"The Eurodollar Futures and Options Handbook"** by Galen Burghardt

### Industry Standards

- **ARRC (Alternative Reference Rates Committee)**: SOFR conventions
- **BoE (Bank of England)**: SONIA conventions
- **ECB (European Central Bank)**: €STR conventions
- **JSCC (Japan Securities Clearing Corporation)**: TONA conventions

### Bloomberg Documentation

- **SWPM**: Swap Manager function
- **FWCV**: Forward Curve Analysis
- **YAS**: Yield and Spread Analysis

### Internal Documentation

- **Core Date Standards**: `/core/dates` for day count and calendars
- **Cashflow Primitives**: `/cashflow` for schedule generation
- **Metric Framework**: `/metrics` for analytics infrastructure
- **Error Handling**: Root-level error handling standards

---

## Glossary

| Term | Definition |
|------|------------|
| **Annuity** | Present value of $1 paid each period on the fixed leg |
| **DV01** | Dollar value of a 1 basis point parallel curve shift |
| **OIS** | Overnight Index Swap (compounded RFR rates) |
| **Par Rate** | Fixed rate that gives zero initial NPV |
| **RFR** | Risk-Free Rate (e.g., SOFR, SONIA, €STR) |
| **Theta** | Time decay; P&L from rolling forward 1 day |
| **Reset Lag** | Business days between rate fixing and period start |
| **Lookback** | Days to shift observation end date before period end |

---

## Version History

- **v1.0** (Phase 1 Complete): Core pricing, metrics, OIS support, market standards
- **Future**: Amortizing swaps, inflation-linked swaps, exotic compounding

---

## Contributing

When contributing to the IRS module:

1. Follow the coding standards in `.cursor/rules/rust/code-standards.mdc`
2. Add comprehensive tests for new features
3. Update this README with usage examples
4. Cite market standards and ISDA conventions where applicable
5. Run `make lint` and `make test-rust` before submitting changes

---

## License

Part of the Finstack library. See root LICENSE file for details.

