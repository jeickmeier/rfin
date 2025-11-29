# Cashflow Module

Comprehensive cashflow schedule generation, aggregation, and currency-safe operations for bonds, swaps, loans, and structured products.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Core Features](#core-features)
- [Key Concepts](#key-concepts)
- [Quick Start](#quick-start)
- [API Guide](#api-guide)
- [Examples](#examples)
- [Adding New Features](#adding-new-features)
- [Performance Considerations](#performance-considerations)
- [Testing](#testing)

---

## Overview

The cashflow module provides the foundation for modeling and analyzing cashflows in the Finstack valuations framework. It emphasizes:

- **Currency Safety**: All operations use currency-tagged `Money` types to prevent accidental cross-currency arithmetic
- **Determinism**: Decimal-based numerics and stable ordering ensure reproducible results
- **Composability**: Builder pattern for flexible schedule construction
- **Classification**: Rich `CFKind` taxonomy for cashflow categorization
- **Period Aggregation**: Efficient grouping and present value calculations by reporting period

All cashflows flow through this module, ensuring consistent handling across bonds, derivatives, loans, and structured products.

---

## Architecture

```
cashflow/
├── mod.rs                  # Module entry point and documentation
├── traits.rs               # CashflowProvider trait and extensions
├── aggregation.rs          # Currency-preserving aggregation and PV functions
├── builder/
│   ├── mod.rs             # Builder module exports
│   ├── builder.rs         # CashflowBuilder implementation
│   ├── compiler.rs        # Internal compilation logic
│   ├── schedule.rs        # CashFlowSchedule type and methods
│   ├── date_generation.rs # Date rolling and schedule generation
│   ├── rate_helpers.rs    # Floating rate projection
│   ├── credit_rates.rs    # CDR/CPR conversions
│   ├── dataframe.rs       # Polars DataFrame exports
│   ├── specs/             # Specification types
│   │   ├── mod.rs
│   │   ├── coupon.rs      # Fixed/floating coupon specs
│   │   ├── fees.rs        # Fee specifications
│   │   ├── schedule.rs    # Scheduling parameters
│   │   ├── amortization.rs # Amortization specs
│   │   ├── prepayment.rs  # Prepayment models (CPR/PSA)
│   │   ├── default.rs     # Default models (CDR/SDA)
│   │   └── recovery.rs    # Recovery models
│   └── emission/          # Cashflow emission helpers
│       ├── mod.rs
│       ├── coupons.rs     # Coupon emission
│       ├── fees.rs        # Fee emission
│       ├── amortization.rs # Amortization emission
│       ├── credit.rs      # Credit event emission
│       └── helpers.rs     # Shared utilities
└── primitives (re-export from core)
    └── CashFlow, CFKind, etc.
```

### Module Responsibilities

- **`primitives`**: Re-exported from `finstack_core::cashflow::primitives` for fundamental types
- **`traits`**: `CashflowProvider` trait for instruments to expose schedules
- **`aggregation`**: Currency-safe merging, period rollup, and present value calculations
- **`builder`**: Composable schedule construction with specs for coupons, fees, amortization, credit events

---

## Core Features

### 1. **Cashflow Classification**

Each cashflow is tagged with `CFKind`:

```rust
pub enum CFKind {
    Principal,      // Principal repayments
    Interest,       // Interest/coupon payments
    Fee,            // Management, servicing, structuring fees
    Fixed,          // Generic fixed payment
    Floating,       // Floating-rate payment projected from index
    Stub,           // Stub period payment
    FloatReset,     // Floating rate reset event
    Amortization,   // Principal amortization
    PIK,            // Payment-in-kind (capitalized interest)
    Notional,       // Notional exchanges (draws/repays)
}
```

### 2. **Currency Safety**

- All cashflows use `Money` type with explicit currency tags
- Aggregation enforces currency matching—no implicit conversions
- FX conversion requires explicit policy and leaves audit trail

### 3. **Schedule Building**

Builder pattern supports:
- Fixed and floating coupons
- Multiple amortization styles (linear, step, percent-per-period)
- Tiered fee structures
- Credit events (prepayment, default, recovery)
- PIK capitalization
- Notional draws/repayments

### 4. **Period Aggregation**

Efficient cashflow-to-period mapping:
- O(n + m) complexity for sorted flows
- Currency-preserving grouping
- Present value aggregation with discount curves
- Credit-adjusted PV with hazard curves

### 5. **Behavioral Models**

Serializable specifications for:
- **Prepayment**: CPR (Constant Prepayment Rate), PSA (Public Securities Association)
- **Default**: CDR (Constant Default Rate), SDA (Standard Default Assumption)
- **Recovery**: Constant recovery rates with timing conventions

---

## Key Concepts

### Currency Safety

```rust
// ✅ CORRECT: Same currency
let usd1 = Money::new(100.0, Currency::USD);
let usd2 = Money::new(50.0, Currency::USD);
let total = usd1.checked_add(usd2)?; // OK

// ❌ ERROR: Mismatched currencies
let eur = Money::new(50.0, Currency::EUR);
let bad = usd1.checked_add(eur)?; // CurrencyMismatch error
```

### CFKind Ordering

Cashflows are sorted by date, then by `CFKind` rank:
1. Fixed / Floating / Stub / FloatReset
2. Fee
3. Amortization
4. PIK
5. Notional
6. Others

This ensures deterministic ordering when multiple flows occur on the same date.

### Precision and Rounding

- All internal calculations use `Money` (Decimal-backed)
- Per-flow rounding at `Money::new` ingestion (ISO-4217 minor units, bankers rounding)
- Sums use exact currency-safe arithmetic
- Determinism: serial ≡ parallel execution

---

## Quick Start

### Basic Fixed-Rate Bond Schedule

```rust
use finstack_valuations::cashflow::builder::{CashFlowSchedule, FixedCouponSpec, CouponType};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::dates::{Date, Frequency, DayCount, BusinessDayConvention, StubKind};
use time::Month;

let issue = Date::from_calendar_date(2025, Month::January, 15)?;
let maturity = Date::from_calendar_date(2030, Month::January, 15)?;

let fixed_spec = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.05,  // 5% annual
    freq: Frequency::semi_annual(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::Following,
    calendar_id: None,
    stub: StubKind::None,
};

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .fixed_cf(fixed_spec)
    .build()?;

// Access flows
for cf in &schedule.flows {
    println!("{}: {} {:?}", cf.date, cf.amount, cf.kind);
}
```

### Amortizing Loan

```rust
use finstack_valuations::cashflow::builder::AmortizationSpec;

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(500_000.0, Currency::USD), issue, maturity)
    .amortization(AmortizationSpec::LinearTo {
        final_notional: Money::new(0.0, Currency::USD),
    })
    .fixed_cf(fixed_spec)
    .build()?;

// Track outstanding balance
let outstanding = schedule.outstanding_by_date()?;
for (date, balance) in outstanding {
    println!("{}: Outstanding = {}", date, balance);
}
```

### Floating-Rate Note

```rust
use finstack_valuations::cashflow::builder::{FloatingCouponSpec, FloatingRateSpec};
use finstack_core::types::IndexId;

let float_spec = FloatingCouponSpec {
    coupon_type: CouponType::Cash,
    rate_spec: FloatingRateSpec {
        index_id: IndexId::new("USD-SOFR"),
        spread_bps: 50,  // 50 bps spread
        freq: Frequency::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: None,
        stub: StubKind::None,
    },
};

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .floating_cf(float_spec)
    .build()?;
```

---

## API Guide

### 1. Building Schedules

#### Entry Point

```rust
let builder = CashFlowSchedule::builder();
```

#### Builder Methods

```rust
// Set principal
builder.principal(notional: Money, issue: Date, maturity: Date)

// Add fixed coupons
builder.fixed_cf(spec: FixedCouponSpec)

// Add floating coupons
builder.floating_cf(spec: FloatingCouponSpec)

// Add amortization
builder.amortization(spec: AmortizationSpec)

// Add fees
builder.fee(spec: FeeSpec)

// Build final schedule
let schedule: CashFlowSchedule = builder.build()?;
```

#### Amortization Styles

```rust
// Linear amortization to final notional
AmortizationSpec::LinearTo {
    final_notional: Money::new(0.0, Currency::USD),
}

// Custom step schedule
AmortizationSpec::StepRemaining {
    schedule: vec![
        (date1, Money::new(750_000.0, Currency::USD)),
        (date2, Money::new(500_000.0, Currency::USD)),
        (date3, Money::new(0.0, Currency::USD)),
    ],
}

// Percentage per period
AmortizationSpec::PercentPerPeriod {
    pct: 0.05,  // 5% per period
}
```

### 2. Using CashFlowSchedule

```rust
let schedule: CashFlowSchedule = /* ... */;

// Access all flows
let flows: &[CashFlow] = &schedule.flows;

// Get dates only
let dates: Vec<Date> = schedule.dates();

// Filter by kind
let coupons = schedule.flows_of_kind(CFKind::Fixed);
let amorts = schedule.amortizations();
let redemptions = schedule.redemptions();

// Outstanding balance tracking
let path: Vec<(Date, Money)> = schedule.outstanding_path()?;
let by_date: Vec<(Date, Money)> = schedule.outstanding_by_date()?;
```

### 3. Aggregation

#### By Period (Nominal)

```rust
use finstack_valuations::cashflow::aggregation::aggregate_by_period;
use finstack_core::dates::{Period, PeriodId};

let flows: Vec<(Date, Money)> = /* ... */;
let periods: Vec<Period> = /* ... */;

let aggregated = aggregate_by_period(&flows, &periods);
// Returns: IndexMap<PeriodId, IndexMap<Currency, Money>>

// Access Q1 2025 USD total
if let Some(q1_map) = aggregated.get(&PeriodId::quarter(2025, 1)) {
    if let Some(usd_total) = q1_map.get(&Currency::USD) {
        println!("Q1 USD: {}", usd_total);
    }
}
```

#### By Period (Present Value)

```rust
use finstack_valuations::cashflow::aggregation::pv_by_period;
use finstack_core::market_data::traits::Discounting;

let disc: &dyn Discounting = /* discount curve */;
let base = Date::from_calendar_date(2025, Month::January, 1)?;

let pv_map = pv_by_period(&flows, &periods, disc, base, DayCount::Act365F);
// Returns: IndexMap<PeriodId, IndexMap<Currency, Money>>
```

#### Credit-Adjusted PV

```rust
use finstack_valuations::cashflow::aggregation::pv_by_period_credit_adjusted;
use finstack_core::market_data::traits::Survival;

let hazard: Option<&dyn Survival> = /* hazard curve */;

let pv_map = pv_by_period_credit_adjusted(
    &flows,
    &periods,
    disc,
    hazard,
    base,
    DayCount::Act365F,
);
```

### 4. CashflowProvider Trait

Instruments implement this to expose schedules:

```rust
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::cashflow::DatedFlows;

impl CashflowProvider for MyInstrument {
    fn build_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Build and return (Date, Money) pairs
        Ok(vec![
            (date1, Money::new(100.0, Currency::USD)),
            (date2, Money::new(100.0, Currency::USD)),
        ])
    }

    // Optional: Override for precise CFKind classification
    fn build_full_schedule(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<CashFlowSchedule> {
        // Build schedule with full metadata
        CashFlowSchedule::builder()
            .principal(/* ... */)
            .fixed_cf(/* ... */)
            .build()
    }
}
```

### 5. Period PV from Schedule

```rust
// Direct from schedule (convenience method)
let pv_map = schedule.pre_period_pv(&periods, disc, base, DayCount::Act365F);

// With market context (supports credit adjustment)
let pv_map = schedule.pre_period_pv_with_market(
    &periods,
    &market,
    &CurveId::new("USD-OIS"),
    Some(&CurveId::new("AAPL-HAZARD")),
    base,
    DayCount::Act365F,
)?;
```

---

## Examples

### Example 1: Step Amortization with Fees

```rust
use finstack_valuations::cashflow::builder::{
    CashFlowSchedule, AmortizationSpec, FeeSpec, FeeBase, FixedCouponSpec, CouponType
};

let init = Money::new(1_000_000.0, Currency::USD);
let issue = Date::from_calendar_date(2025, Month::January, 1)?;
let maturity = Date::from_calendar_date(2028, Month::January, 1)?;

let amort = AmortizationSpec::StepRemaining {
    schedule: vec![
        (Date::from_calendar_date(2026, Month::January, 1)?, Money::new(750_000.0, Currency::USD)),
        (Date::from_calendar_date(2027, Month::January, 1)?, Money::new(500_000.0, Currency::USD)),
        (maturity, Money::new(0.0, Currency::USD)),
    ],
};

let fee = FeeSpec::periodic(
    FeeBase::NotionalBps,
    25.0,  // 25 bps per year
    Frequency::annual(),
    issue,
    maturity,
);

let coupon = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.06,
    freq: Frequency::quarterly(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: None,
    stub: StubKind::None,
};

let schedule = CashFlowSchedule::builder()
    .principal(init, issue, maturity)
    .amortization(amort)
    .fee(fee)
    .fixed_cf(coupon)
    .build()?;

// Print all flows grouped by date
for (date, balance) in schedule.outstanding_by_date()? {
    let day_flows: Vec<_> = schedule.flows.iter()
        .filter(|cf| cf.date == date)
        .collect();
    
    println!("\n{}", date);
    for cf in day_flows {
        println!("  {:?}: {}", cf.kind, cf.amount);
    }
    println!("  Outstanding: {}", balance);
}
```

### Example 2: PIK Toggle Bond

```rust
use finstack_valuations::cashflow::builder::CouponType;

// Years 1-2: PIK (capitalize interest)
let pik_spec = FixedCouponSpec {
    coupon_type: CouponType::PIK,
    rate: 0.08,
    freq: Frequency::semi_annual(),
    dc: DayCount::Thirty360,
    bdc: BusinessDayConvention::Following,
    calendar_id: None,
    stub: StubKind::None,
};

// Years 3-5: Cash
let cash_spec = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.08,
    freq: Frequency::semi_annual(),
    dc: DayCount::Thirty360,
    bdc: BusinessDayConvention::Following,
    calendar_id: None,
    stub: StubKind::None,
};

let pik_end = Date::from_calendar_date(2027, Month::January, 1)?;

let schedule = CashFlowSchedule::builder()
    .principal(init, issue, maturity)
    .fixed_cf_window(pik_spec, issue, pik_end)   // PIK period
    .fixed_cf_window(cash_spec, pik_end, maturity) // Cash period
    .build()?;

// Outstanding grows during PIK period
let outstanding = schedule.outstanding_by_date()?;
for (date, balance) in outstanding {
    println!("{}: {}", date, balance);
}
```

### Example 3: Multi-Currency Aggregation

```rust
use finstack_valuations::cashflow::aggregation::aggregate_by_period;

let flows = vec![
    (Date::from_calendar_date(2025, Month::March, 15)?, Money::new(100_000.0, Currency::USD)),
    (Date::from_calendar_date(2025, Month::March, 20)?, Money::new(50_000.0, Currency::EUR)),
    (Date::from_calendar_date(2025, Month::June, 15)?, Money::new(100_000.0, Currency::USD)),
];

let periods = vec![
    Period {
        id: PeriodId::quarter(2025, 1),
        start: Date::from_calendar_date(2025, Month::January, 1)?,
        end: Date::from_calendar_date(2025, Month::April, 1)?,
        is_actual: true,
    },
    Period {
        id: PeriodId::quarter(2025, 2),
        start: Date::from_calendar_date(2025, Month::April, 1)?,
        end: Date::from_calendar_date(2025, Month::July, 1)?,
        is_actual: false,
    },
];

let agg = aggregate_by_period(&flows, &periods);

// Q1 has both USD and EUR
let q1 = agg.get(&PeriodId::quarter(2025, 1)).unwrap();
println!("Q1 USD: {}", q1.get(&Currency::USD).unwrap());
println!("Q1 EUR: {}", q1.get(&Currency::EUR).unwrap());

// Q2 has only USD
let q2 = agg.get(&PeriodId::quarter(2025, 2)).unwrap();
println!("Q2 USD: {}", q2.get(&Currency::USD).unwrap());
```

### Example 4: Credit-Adjusted Periodized PV

```rust
use finstack_core::market_data::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};

// Build market context
let disc_curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)])
    .set_interp(InterpStyle::Linear)
    .build()?;

let hazard_curve = HazardCurve::builder("CORP-HAZARD")
    .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
    .set_interp(InterpStyle::Linear)
    .build()?;

let market = MarketContext::new()
    .insert_discount(disc_curve)
    .insert_hazard(hazard_curve);

// Compute credit-adjusted period PVs
let pv_map = schedule.pre_period_pv_with_market(
    &periods,
    &market,
    &CurveId::new("USD-OIS"),
    Some(&CurveId::new("CORP-HAZARD")),
    base,
    DayCount::Act365F,
)?;

// Analyze results
for (period_id, ccy_map) in pv_map {
    for (ccy, pv) in ccy_map {
        println!("{:?} {}: PV = {}", period_id, ccy, pv);
    }
}
```

---

## Adding New Features

### Adding a New Amortization Style

**Step 1**: Define the spec variant in `builder/specs/amortization.rs`

```rust
pub enum AmortizationSpec {
    // ... existing variants ...
    
    /// Custom: Accelerating amortization schedule
    Accelerating {
        acceleration_rate: f64,
    },
}
```

**Step 2**: Implement emission in `builder/emission/amortization.rs`

```rust
pub(crate) fn emit_accelerating_amort(
    builder: &mut CashflowBuilder,
    acceleration_rate: f64,
) -> finstack_core::Result<()> {
    let dates = &builder.schedule_params.payment_dates;
    let mut outstanding = builder.notional.initial.amount();
    
    for (i, &date) in dates.iter().enumerate() {
        let factor = 1.0 + (i as f64 * acceleration_rate);
        let payment = (outstanding * factor) / (dates.len() - i) as f64;
        
        builder.flows.push(CashFlow {
            date,
            reset_date: None,
            amount: Money::new(payment, builder.notional.initial.currency()),
            kind: CFKind::Amortization,
            accrual_factor: 0.0,
            rate: None,
        });
        
        outstanding -= payment;
    }
    
    Ok(())
}
```

**Step 3**: Wire into compiler in `builder/compiler.rs`

```rust
fn compile_amortization(&mut self) -> finstack_core::Result<()> {
    if let Some(spec) = &self.amortization {
        match spec {
            // ... existing cases ...
            
            AmortizationSpec::Accelerating { acceleration_rate } => {
                emission::emit_accelerating_amort(self, *acceleration_rate)?;
            }
        }
    }
    Ok(())
}
```

**Step 4**: Add tests in `tests/cashflow/amortization_spec.rs`

```rust
#[test]
fn test_accelerating_amortization() {
    let init = Money::new(1_000_000.0, Currency::USD);
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    
    let schedule = CashFlowSchedule::builder()
        .principal(init, issue, maturity)
        .amortization(AmortizationSpec::Accelerating {
            acceleration_rate: 0.05,
        })
        .build()
        .unwrap();
    
    let amorts: Vec<_> = schedule.amortizations().collect();
    
    // Verify increasing payments
    for i in 1..amorts.len() {
        assert!(amorts[i].amount.amount() > amorts[i-1].amount.amount());
    }
    
    // Verify full amortization
    let total: f64 = amorts.iter().map(|cf| cf.amount.amount()).sum();
    assert!((total - 1_000_000.0).abs() < 1.0);
}
```

### Adding a New Fee Type

**Step 1**: Extend `FeeBase` enum in `builder/specs/fees.rs`

```rust
pub enum FeeBase {
    // ... existing variants ...
    
    /// Fee based on outstanding principal
    OutstandingBps,
}
```

**Step 2**: Implement calculation in `builder/emission/fees.rs`

```rust
pub(crate) fn emit_outstanding_fee(
    builder: &CashflowBuilder,
    bps: f64,
    dates: &[Date],
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut fees = Vec::new();
    let outstanding_path = builder.outstanding_path()?;
    
    for &date in dates {
        if let Some(&(_, outstanding)) = outstanding_path.iter()
            .find(|(d, _)| *d == date) 
        {
            let fee_amount = outstanding.amount() * bps / 10_000.0;
            fees.push(CashFlow {
                date,
                reset_date: None,
                amount: Money::new(fee_amount, outstanding.currency()),
                kind: CFKind::Fee,
                accrual_factor: 0.0,
                rate: None,
            });
        }
    }
    
    fees
}
```

**Step 3**: Wire into fee compilation and add tests

### Adding a New Coupon Type

Follow similar pattern:
1. Define spec type in `builder/specs/coupon.rs`
2. Implement emission in `builder/emission/coupons.rs`
3. Wire into `builder/compiler.rs`
4. Add comprehensive tests

### Extension Points Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| Amortization | `specs/amortization.rs`, `emission/amortization.rs` | Principal repayment schedules |
| Fees | `specs/fees.rs`, `emission/fees.rs` | Fee calculations and tiering |
| Coupons | `specs/coupon.rs`, `emission/coupons.rs` | Interest payment patterns |
| Credit Events | `specs/prepayment.rs`, `specs/default.rs`, `emission/credit.rs` | Behavioral models |
| Aggregation | `aggregation.rs` | Period rollup algorithms |

---

## Performance Considerations

### 1. **Sorting Optimization**

Aggregation uses `sort_unstable_by_key` for ~5-10% faster sorting:

```rust
// Preferred for large schedules
sorted.sort_unstable_by_key(|(d, _)| *d);

// Stable sort only when relative order matters
sorted.sort_by_key(|(d, _)| *d);
```

### 2. **Aggregation Complexity**

Period aggregation is O(n + m) for n cashflows and m periods using a moving index:

```rust
// Efficient: Single pass through sorted flows
let agg = aggregate_by_period(&flows, &periods);

// Inefficient: Don't filter flows per-period
for p in periods {
    let period_flows: Vec<_> = flows.iter()
        .filter(|(d, _)| *d >= p.start && *d < p.end)
        .collect();  // O(n * m) - avoid this!
}
```

### 3. **Memory Efficiency**

Use iterators to avoid unnecessary allocations:

```rust
// Good: Iterator (lazy evaluation)
let total = schedule.flows_of_kind(CFKind::Fixed)
    .map(|cf| cf.amount.amount())
    .sum::<f64>();

// Less efficient: Collect first
let coupons: Vec<_> = schedule.flows_of_kind(CFKind::Fixed).collect();
let total: f64 = coupons.iter().map(|cf| cf.amount.amount()).sum();
```

### 4. **Precision vs Performance**

- Decimal arithmetic (via `Money`) is ~10-100x slower than `f64`
- Use `Money` at boundaries (construction, aggregation results)
- Intermediate calculations can use `f64` if precision loss acceptable
- Always round back to `Money` for final results

### 5. **Parallel PV Calculation**

For large portfolios, parallelize over instruments, not cashflows:

```rust
use rayon::prelude::*;

let instrument_pvs: Vec<Money> = instruments.par_iter()
    .map(|inst| {
        let schedule = inst.build_full_schedule(&market, as_of)?;
        schedule.pre_period_pv(&periods, disc, base, dc)
            .values()
            .flat_map(|m| m.values())
            .fold(Money::new(0.0, Currency::USD), |acc, pv| acc.checked_add(*pv).unwrap())
    })
    .collect();
```

---

## Testing

### Test Organization

```
tests/cashflow/
├── tests.rs                    # Integration tests
├── amortization_spec.rs        # Amortization unit tests
└── json_examples/              # Serialization golden tests
    ├── *.example.json
```

### Running Tests

```bash
# All cashflow tests
cargo test -p finstack-valuations cashflow

# Specific module
cargo test -p finstack-valuations cashflow::aggregation

# Single test
cargo test -p finstack-valuations test_linear_amortization
```

### Test Patterns

#### Unit Test: Schedule Construction

```rust
#[test]
fn test_fixed_coupon_schedule() {
    let init = Money::new(100_000.0, Currency::USD);
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    
    let spec = FixedCouponSpec { /* ... */ };
    
    let schedule = CashFlowSchedule::builder()
        .principal(init, issue, maturity)
        .fixed_cf(spec)
        .build()
        .unwrap();
    
    // Assertions
    assert_eq!(schedule.flows.len(), 3); // 2 coupons + 1 redemption
    assert_eq!(schedule.notional.initial, init);
}
```

#### Property Test: Amortization Invariants

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn amortization_sums_to_notional(
        notional in 1_000_000.0..10_000_000.0,
    ) {
        let init = Money::new(notional, Currency::USD);
        let schedule = CashFlowSchedule::builder()
            .principal(init, issue, maturity)
            .amortization(AmortizationSpec::LinearTo {
                final_notional: Money::new(0.0, Currency::USD),
            })
            .build()
            .unwrap();
        
        let total_amort: f64 = schedule.amortizations()
            .map(|cf| cf.amount.amount())
            .sum();
        
        prop_assert!((total_amort - notional).abs() < 1.0);
    }
}
```

#### Integration Test: End-to-End PV

```rust
#[test]
fn test_periodized_pv_matches_npv() {
    let schedule = /* build schedule */;
    let disc = /* build discount curve */;
    let periods = /* define periods */;
    
    // Periodized PV
    let pv_map = schedule.pre_period_pv(&periods, &disc, base, DayCount::Act365F);
    let period_sum: f64 = pv_map.values()
        .flat_map(|m| m.values())
        .map(|pv| pv.amount())
        .sum();
    
    // Direct NPV
    let flows: Vec<(Date, Money)> = schedule.flows.iter()
        .map(|cf| (cf.date, cf.amount))
        .collect();
    let npv = finstack_core::cashflow::discounting::npv_static(
        &disc, base, DayCount::Act365F, &flows
    ).unwrap();
    
    // Should match within tolerance
    assert!((period_sum - npv.amount()).abs() < 1e-6);
}
```

### Golden Tests

Serialization roundtrips in `json_examples/`:

```bash
# Generate example
cargo test -p finstack-valuations generate_amortization_example -- --ignored

# Verify deserialization
cargo test -p finstack-valuations roundtrip_amortization_spec
```

---

## See Also

- **[`finstack_core::cashflow`]**: Primitive types (`CashFlow`, `CFKind`)
- **[`finstack_core::money`]**: Currency-safe `Money` type
- **[`finstack_core::dates`]**: Date generation, day counts, calendars
- **[Valuations README](../README.md)**: Instrument pricing and risk
- **[Portfolio README](../../../portfolio/README.md)**: Position aggregation

---

## Summary

The cashflow module is the foundation for all instrument valuation in Finstack. Its currency-safe, deterministic design ensures:

- **Correctness**: No cross-currency arithmetic errors
- **Transparency**: Rich classification and metadata
- **Flexibility**: Composable builder for complex schedules
- **Performance**: Efficient period aggregation and PV calculations
- **Extensibility**: Clear patterns for adding new features

For questions or contributions, see the main Finstack documentation or raise an issue on GitHub.

