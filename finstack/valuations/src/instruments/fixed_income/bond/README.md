# Bond Module

Comprehensive bond instrument implementation supporting fixed-rate, floating-rate, callable/putable, and amortizing bonds with advanced pricing and risk metrics.

## Overview

The bond module provides a complete implementation of bond instruments with:

- **Multiple bond types**: Fixed-rate, floating-rate (FRNs), zero-coupon, amortizing, callable/putable
- **Multiple pricing engines**: Discount curve, hazard-rate (credit), and tree-based (OAS) pricing
- **Comprehensive metrics**: Price, yield, duration, convexity, spreads, and risk measures
- **Market conventions**: Support for US Treasury, UK Gilt, Eurozone, and Japanese conventions
- **Holder-view cashflows**: Consistent positive cashflow convention for long positions

## Module Structure

```
bond/
├── mod.rs                 # Main module entry point and re-exports
├── types.rs               # Bond struct, CallPut, CallPutSchedule
├── cashflow_spec.rs       # CashflowSpec enum (Fixed/Floating/Amortizing)
├── cashflows.rs           # Cashflow generation utilities
├── pricing/               # Pricing engines
│   ├── mod.rs
│   ├── discount_engine.rs # Standard discount curve pricing
│   ├── hazard_engine.rs   # Credit-adjusted pricing (FRP)
│   ├── tree_engine.rs     # Tree-based OAS pricing for options
│   ├── quote_engine.rs    # Price/yield/spread conversions
│   ├── ytm_solver.rs      # Yield-to-maturity solver
│   └── pricer.rs          # Pricer registry implementations
└── metrics/               # Bond-specific metrics
    ├── mod.rs
    ├── accrued.rs         # Accrued interest calculator
    ├── duration_macaulay.rs
    ├── duration_modified.rs
    ├── convexity.rs
    └── price_yield_spread/  # Price, yield, and spread metrics
        ├── mod.rs
        ├── prices.rs      # Clean/dirty price
        ├── ytm.rs         # Yield to maturity
        ├── ytw.rs         # Yield to worst
        ├── z_spread.rs    # Zero-volatility spread
        ├── oas.rs         # Option-adjusted spread
        ├── i_spread.rs    # Interpolated spread
        ├── dm.rs          # Discount margin (FRNs)
        └── asw.rs         # Asset swap spreads
```

## Feature Set

### Bond Types

#### Fixed-Rate Bonds

Standard bonds with fixed coupon payments at regular intervals.

```rust
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use time::macros::date;

// Simple fixed-rate bond
let bond = Bond::fixed(
    "BOND-001",
    Money::new(1_000_000.0, Currency::USD),
    0.05,  // 5% coupon
    date!(2025 - 01 - 01),
    date!(2030 - 01 - 01),
    "USD-OIS",
);
```

#### Floating-Rate Notes (FRNs)

Bonds with floating coupon rates tied to an index (e.g., SOFR, LIBOR).

```rust
let frn = Bond::floating(
    "FRN-001",
    Money::new(1_000_000.0, Currency::USD),
    "USD-SOFR-3M".into(),  // Index curve
    200.0,  // Margin in basis points
    date!(2025 - 01 - 01),
    date!(2030 - 01 - 01),
    "USD-OIS",
);
```

#### Zero-Coupon Bonds

Bonds that pay no coupons, only principal at maturity.

```rust
let zero = Bond::builder()
    .id("ZERO-001".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::fixed(0.0, Tenor::annual(), DayCount::Act365F))
    .discount_curve_id("USD-OIS".into())
    .build()?;
```

#### Amortizing Bonds

Bonds with principal repayment schedules.

```rust
use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
use finstack_core::cashflow::builder::AmortizationSpec;

let amort_spec = AmortizationSpec::linear(
    date!(2025 - 01 - 01),
    date!(2030 - 01 - 01),
    Tenor::semi_annual(),
);

let amortizing = Bond::builder()
    .id("AMORT-001".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::amortizing(
        CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Thirty360),
        amort_spec,
    ))
    .discount_curve_id("USD-OIS".into())
    .build()?;
```

#### Callable/Putable Bonds

Bonds with embedded options allowing early redemption.

```rust
use finstack_valuations::instruments::fixed_income::bond::{Bond, CallPutSchedule, CallPut};

let call_schedule = CallPutSchedule {
    calls: vec![
        CallPut { date: date!(2027 - 01 - 01), price_pct_of_par: 102.0 },
        CallPut { date: date!(2028 - 01 - 01), price_pct_of_par: 101.0 },
    ],
    puts: vec![],
};

let callable = Bond::builder()
    .id("CALLABLE-001".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::fixed(0.06, Tenor::semi_annual(), DayCount::Thirty360))
    .discount_curve_id("USD-OIS".into())
    .call_put(Some(call_schedule))
    .build()?;
```

### Market Conventions

Pre-configured regional market conventions:

```rust
use finstack_valuations::instruments::BondConvention;

// US Treasury convention (30/360, semi-annual)
let ust = Bond::with_convention(
    "UST-10Y",
    Money::new(1_000_000.0, Currency::USD),
    0.0375,
    date!(2025 - 01 - 01),
    date!(2035 - 01 - 01),
    BondConvention::USTreasury,
    "USD-TREASURY",
);

// UK Gilt convention (ACT/ACT, semi-annual)
let gilt = Bond::with_convention(
    "GILT-10Y",
    Money::new(1_000_000.0, Currency::GBP),
    0.025,
    date!(2025 - 01 - 01),
    date!(2035 - 01 - 01),
    BondConvention::UKGilt,
    "GBP-GILTS",
);
```

### Pricing Engines

#### Discount Engine (Standard Pricing)

Standard present value calculation using discount curves.

```rust
use finstack_valuations::instruments::fixed_income::bond::pricing::discount_engine::BondEngine;
use finstack_core::market_data::context::MarketContext;

let market = MarketContext::new()
    .insert_discount(discount_curve);

let pv = BondEngine::price(&bond, &market, as_of)?;
```

#### Hazard Engine (Credit-Adjusted Pricing)

Credit-adjusted pricing using hazard curves with fractional recovery of par.

```rust
use finstack_valuations::instruments::fixed_income::bond::pricing::hazard_engine::HazardBondEngine;

let market = MarketContext::new()
    .insert_discount(discount_curve)
    .insert_hazard(hazard_curve);

let pv = HazardBondEngine::price(&bond, &market, as_of)?;
```

#### Tree Engine (OAS Pricing)

Tree-based pricing for callable/putable bonds and option-adjusted spread calculation.

```rust
use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;

let tree_pricer = TreePricer::new();
let oas = tree_pricer.calculate_oas(&callable_bond, &market, as_of, quoted_price)?;
```

### Metrics

The bond module provides comprehensive risk and valuation metrics:

#### Price Metrics

- **Clean Price**: Quoted price excluding accrued interest
- **Dirty Price**: Clean price plus accrued interest
- **Accrued Interest**: Interest accrued since last coupon

#### Yield Metrics

- **Yield to Maturity (YTM)**: Internal rate of return
- **Yield to Worst (YTW)**: Minimum yield across call/put/maturity paths

#### Risk Metrics

- **Macaulay Duration**: Weighted average time to cashflows
- **Modified Duration**: Interest rate sensitivity measure
- **Convexity**: Curvature of price/yield relationship
- **DV01**: Dollar value of 1bp rate change
- **CS01**: Credit spread sensitivity

#### Spread Metrics

- **Z-Spread**: Zero-volatility spread over discount curve
- **OAS**: Option-adjusted spread (for callable/putable bonds)
- **I-Spread**: Interpolated spread (YTM - par swap rate)
- **Discount Margin**: Spread measure for floating-rate notes
- **Asset Swap Spreads**: Par and market asset swap spreads

#### Using Metrics

```rust
use finstack_valuations::metrics::{MetricRegistry, MetricId};
use finstack_valuations::instruments::fixed_income::bond::metrics::register_bond_metrics;

// Register bond metrics
let mut registry = MetricRegistry::new();
register_bond_metrics(&mut registry);

// Price with metrics
let result = registry.price_with_metrics(
    &bond,
    "discounting",
    &market,
    as_of,
    &[MetricId::Ytm, MetricId::DurationMod, MetricId::Dv01],
)?;

let ytm = result.metric(MetricId::Ytm);
let duration = result.metric(MetricId::DurationMod);
let dv01 = result.metric(MetricId::Dv01);
```

## Usage Examples

### Basic Bond Pricing

```rust
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::market_data::{MarketContext, DiscountCurve};
use finstack_core::dates::{Date, DayCount};
use time::macros::date;

// Create bond
let bond = Bond::fixed(
    "BOND-001",
    Money::new(1_000_000.0, Currency::USD),
    0.05,
    date!(2025 - 01 - 01),
    date!(2030 - 01 - 01),
    "USD-OIS",
);

// Create discount curve
let curve = DiscountCurve::builder("USD-OIS")
    .base_date(date!(2025 - 01 - 01))
    .day_count(DayCount::Act365F)
    .knots([(0.0, 1.0), (5.0, 0.78)])
    .build()?;

// Build market
let market = MarketContext::new().insert_discount(curve);

// Price bond
let as_of = date!(2025 - 06 - 01);
let pv = bond.value(&market, as_of)?;
println!("Present Value: {}", pv);
```

### Bond with Quoted Price

```rust
use finstack_valuations::instruments::PricingOverrides;

let bond = Bond::builder()
    .id("BOND-QUOTED".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::fixed(0.05, Tenor::semi_annual(), DayCount::Thirty360))
    .discount_curve_id("USD-OIS".into())
    .pricing_overrides(PricingOverrides::default().with_clean_price(99.5))
    .build()?;
```

### Computing Multiple Metrics

```rust
use finstack_valuations::pricer::PricerRegistry;
use finstack_valuations::metrics::MetricId;

let registry = PricerRegistry::standard();
let metrics = vec![
    MetricId::CleanPrice,
    MetricId::Accrued,
    MetricId::Ytm,
    MetricId::DurationMod,
    MetricId::Convexity,
    MetricId::Dv01,
    MetricId::ZSpread,
];

let result = registry.price_with_metrics(
    &bond,
    "discounting",
    &market,
    as_of,
    &metrics,
)?;

println!("Clean Price: {}", result.metric(MetricId::CleanPrice));
println!("YTM: {}%", result.metric(MetricId::Ytm) * 100.0);
println!("Modified Duration: {}", result.metric(MetricId::DurationMod));
```

### Callable Bond OAS Calculation

```rust
use finstack_valuations::instruments::fixed_income::bond::pricing::tree_engine::TreePricer;

let callable_bond = Bond::builder()
    .id("CALLABLE".into())
    .notional(Money::new(1_000_000.0, Currency::USD))
    .issue(date!(2025 - 01 - 01))
    .maturity(date!(2030 - 01 - 01))
    .cashflow_spec(CashflowSpec::fixed(0.06, Tenor::semi_annual(), DayCount::Thirty360))
    .discount_curve_id("USD-OIS".into())
    .call_put(Some(call_schedule))
    .build()?;

let tree_pricer = TreePricer::new();
let quoted_price = 102.5;  // % of par
let oas = tree_pricer.calculate_oas(&callable_bond, &market, as_of, quoted_price)?;
println!("OAS: {} bps", oas * 10000.0);
```

## How to Add New Features

### Adding a New Metric

1. **Create the metric calculator** in `metrics/`:

```rust
// metrics/my_metric.rs
use crate::metrics::{MetricCalculator, MetricContext};
use crate::instruments::Bond;

pub struct MyMetricCalculator;

impl MetricCalculator for MyMetricCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // Access cached data from context
        let cashflows = context.cashflows.as_ref()
            .ok_or_else(|| finstack_core::Error::from("Cashflows not available"))?;

        // Compute metric
        let metric_value = /* your calculation */;

        Ok(metric_value)
    }
}
```

1. **Export the calculator** in `metrics/mod.rs`:

```rust
pub mod my_metric;
pub use my_metric::MyMetricCalculator;
```

1. **Register the metric** in `register_bond_metrics()`:

```rust
pub fn register_bond_metrics(registry: &mut crate::metrics::MetricRegistry) {
    crate::register_metrics! {
        registry: registry,
        instrument: "Bond",
        metrics: [
            // ... existing metrics ...
            (MyMetric, MyMetricCalculator),
        ]
    };
}
```

1. **Add MetricId** in the main metrics module (if needed):

```rust
// In finstack/valuations/src/metrics/mod.rs
pub enum MetricId {
    // ... existing variants ...
    MyMetric,
}
```

### Adding a New Pricing Engine

1. **Create the engine** in `pricing/`:

```rust
// pricing/my_engine.rs
use crate::instruments::bond::Bond;
use finstack_core::market_data::context::MarketContext;
use finstack_core::dates::Date;
use finstack_core::money::Money;

pub struct MyPricingEngine;

impl MyPricingEngine {
    pub fn price(
        bond: &Bond,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Your pricing logic
        let pv = /* calculate present value */;
        Ok(pv)
    }
}
```

1. **Export the engine** in `pricing/mod.rs`:

```rust
pub mod my_engine;
pub use my_engine::MyPricingEngine;
```

1. **Create a pricer** in `pricing/pricer.rs`:

```rust
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;

pub struct MyBondPricer;

impl Pricer for MyBondPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::Bond, ModelKey::Custom("my_model".into()))
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<ValuationResult, PricingError> {
        let bond = instrument
            .as_any()
            .downcast_ref::<Bond>()
            .ok_or_else(|| PricingError::type_mismatch(InstrumentType::Bond, instrument.key()))?;

        let pv = MyPricingEngine::price(bond, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        Ok(ValuationResult::stamped(bond.id(), as_of, pv))
    }
}
```

1. **Register the pricer** in the pricer registry (typically in `finstack/valuations/src/pricer/mod.rs`).

### Adding a New Bond Type

1. **Extend CashflowSpec** if needed (in `cashflow_spec.rs`):

```rust
#[derive(Clone, Debug)]
pub enum CashflowSpec {
    Fixed(FixedCouponSpec),
    Floating(FloatingCouponSpec),
    Amortizing { base: Box<CashflowSpec>, schedule: AmortizationSpec },
    // Add your new variant
    MyNewType(MyNewSpec),
}
```

1. **Add factory method** to `Bond` (in `types.rs`):

```rust
impl Bond {
    pub fn my_new_type(
        id: impl Into<InstrumentId>,
        notional: Money,
        // ... parameters ...
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self::builder()
            .id(id.into())
            .notional(notional)
            // ... set fields ...
            .cashflow_spec(CashflowSpec::MyNewType(MyNewSpec { /* ... */ }))
            .discount_curve_id(discount_curve_id.into())
            .build()
            .expect("Valid bond")
    }
}
```

1. **Update cashflow generation** in `cashflows.rs` to handle the new type.

### Best Practices

1. **Follow the holder-view convention**: All cashflows should be positive for a long holder
2. **Cache intermediate results**: Use `MetricContext` to cache cashflows and other expensive computations
3. **Use Decimal arithmetic**: Ensure all calculations use `Decimal` for accounting-grade precision
4. **Handle errors gracefully**: Return `Result` types and provide meaningful error messages
5. **Add tests**: Include unit tests for new features in `tests/instruments/bond/`
6. **Document public APIs**: Add doc comments with examples for all public functions
7. **Maintain parity**: Ensure Python and WASM bindings are updated if adding new public APIs

## Cashflow Convention

All bond cashflows follow a **holder-view** convention:

- **Positive amounts** represent contractual inflows to a long holder (coupons, amortization, redemption)
- **Initial draw / funding legs** are handled outside the schedule (e.g., via trade price) and are **not** included in the projected cashflow schedule

This convention is enforced by the `CashflowProvider::build_dated_flows` implementation for `Bond`, which turns the internal cashflow schedule into a simplified `(Date, Money)` stream used by pricing and risk engines.

## Accrual and Ex-Coupon Conventions

Accrued interest is driven directly off the true coupon schedule and outstanding notional (for amortizing structures), with explicit support for:

- **Linear vs. compounded accrual** (`AccrualMethod`)
- **Ex-coupon windows** where accrual drops to zero
- **Custom-cashflow bonds** that provide their own schedule and day-count

## Regional Market Conventions

Different bond markets follow distinct conventions:

- **US Treasuries**: 30/360, Semi-annual, T+1 settlement
- **UK Gilts**: ACT/ACT, Semi-annual, T+1 settlement
- **Eurozone**: 30E/360 or ACT/ACT, Annual, T+2 settlement
- **Japan**: ACT/365F, Semi-annual, T+3 settlement

Use `Bond::with_convention()` for standard regional conventions.

## See Also

- [`Bond`] for the main bond struct and factory methods
- [`CallPutSchedule`] for embedded option schedules
- [`CashflowSpec`] for fixed/floating/amortizing specifications
- [`AmortizationSpec`] for amortizing bonds
- [`metrics`] for bond-specific risk metrics
- [`pricing`] for bond pricing engines

## Limitations / Known Issues

- Deterministic curve inputs only; no stochastic rate/credit paths or optionality beyond the implemented call/put/OAS engines.
- Does not model tax/withholding, accrued settlement pricing, or fail penalties—these must be handled upstream.
- Inflation linkage and convertibility live in dedicated modules; keep parity if combining features across modules.

## Pricing Methodology

- Generates holder-view cashflows per `CashflowSpec` (fixed, float, amortizing, callable/putable) using schedule builders and day-count rules.
- Discounting/pricing via dedicated engines: discount curve PV, hazard-adjusted FRP, and tree-based OAS for embedded options.
- Quote conversions (price/yield/spread) solved with ytm/ycs solvers; accrual and ex-coupon handled explicitly.

## Metrics

- Price/yield/spread ladder (clean/dirty price, YTM/YTW), duration (Macaulay/Modified), convexity.
- Spread metrics (Z, OAS, I-spread), DV01/CS01 (parallel and bucketed), accrued interest, carry/roll.
- Option-adjusted measures when tree/OAS engine is used; cashflow PV breakdown by leg.

## Future Enhancements

- Add full callable/putable amortizing parity with more tree/PDE models and stochastic rates.
- Expand risk to include curve-shift scenarios (non-parallel) and callable bond Greeks.
