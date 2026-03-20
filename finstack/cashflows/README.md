# Cashflow Module

Comprehensive cashflow schedule generation, aggregation, accrual calculations, and currency-safe operations for bonds, swaps, loans, and structured products.

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
- **Accrual Engine**: Generic schedule-driven interest accrual with Linear and Compounded methods

All cashflows flow through this module, ensuring consistent handling across bonds, derivatives, loans, and structured products.

---

## Architecture

```
cashflow/
├── mod.rs                  # Module entry point and documentation
├── traits.rs               # CashflowProvider trait and extensions
├── aggregation.rs          # Currency-preserving aggregation and PV functions
├── accrual.rs              # Generic schedule-driven interest accrual engine
├── builder/
│   ├── mod.rs             # Builder module exports
│   ├── builder.rs         # CashFlowBuilder implementation
│   ├── compiler.rs        # Internal compilation logic
│   ├── schedule.rs        # CashFlowSchedule type and methods
│   ├── date_generation.rs # Date rolling and schedule generation
│   ├── rate_helpers.rs    # Floating rate projection
│   ├── credit_rates.rs    # CDR/CPR conversions (cpr_to_smm, smm_to_cpr)
│   ├── dataframe.rs       # Polars DataFrame exports
│   ├── specs/             # Specification types
│   │   ├── mod.rs
│   │   ├── coupon.rs      # Fixed/floating coupon specs with caps/floors/gearing
│   │   ├── fees.rs        # Fee specifications with tiered utilization support
│   │   ├── schedule.rs    # Scheduling parameters
│   │   ├── amortization.rs # Amortization specs (linear, step, custom)
│   │   ├── prepayment.rs  # Prepayment models (CPR/PSA)
│   │   ├── default.rs     # Default models (CDR/SDA) and events
│   │   └── recovery.rs    # Recovery models with timing
│   └── emission/          # Cashflow emission helpers
│       ├── mod.rs
│       ├── coupons.rs     # Coupon emission
│       ├── fees.rs        # Fee emission (commitment, usage, facility)
│       ├── amortization.rs # Amortization emission
│       ├── credit.rs      # Credit event emission (default, prepay, recovery)
│       ├── helpers.rs     # Shared utilities
│       └── tests.rs       # Emission test suite
└── primitives (re-export from core)
    └── CashFlow, CFKind, etc.
```

### Module Responsibilities

- **`primitives`**: Re-exported from `finstack_core::cashflow::primitives` for fundamental types
- **`traits`**: `CashflowProvider` trait for instruments to expose schedules (with `notional()` method)
- **`aggregation`**: Currency-safe merging, period rollup, PV calculations with explicit `DayCountCtx` support
- **`accrual`**: Generic accrued interest engine supporting Linear and Compounded methods
- **`builder`**: Composable schedule construction with specs for coupons, fees, amortization, credit events

---

## Core Features

### 1. **Cashflow Classification**

Each cashflow is tagged with `CFKind`:

```rust
pub enum CFKind {
    // Coupon/Interest types
    Fixed,          // Fixed-rate coupon payment
    Floating,       // Floating-rate payment projected from index
    Stub,           // Stub period payment
    FloatReset,     // Floating rate reset event
    Interest,       // Generic interest/coupon payments
    PIK,            // Payment-in-kind (capitalized interest)

    // Principal types
    Principal,      // Principal repayments
    Amortization,   // Scheduled principal amortization
    Notional,       // Notional exchanges (draws/repays)
    PrePayment,     // Unscheduled prepayments (behavioral)

    // Fee types
    Fee,            // Generic management, servicing, structuring fees
    CommitmentFee,  // Fee on undrawn balance
    UsageFee,       // Fee on drawn balance
    FacilityFee,    // Fee on total commitment

    // Credit event types
    DefaultedNotional,  // Principal lost to default
    Recovery,           // Recovery payment after default
}
```

### 2. **Currency Safety**

- All cashflows use `Money` type with explicit currency tags
- Aggregation enforces currency matching—no implicit conversions
- FX conversion requires explicit policy and leaves audit trail

### 3. **Schedule Building**

Builder pattern supports:

- Fixed and floating coupons with caps, floors, and gearing
- Multiple amortization styles (none, linear, step, percent-per-period, custom)
- Tiered fee structures with utilization-based pricing
- Credit events (prepayment, default, recovery) with calendar-aware date adjustment
- PIK capitalization
- Notional draws/repayments

### 4. **Period Aggregation**

Efficient cashflow-to-period mapping:

- O(n + m) complexity for sorted flows
- Currency-preserving grouping
- Present value aggregation with discount curves and explicit `DayCountCtx`
- Credit-adjusted PV with hazard curves and recovery rate support

### 5. **Behavioral Models**

Serializable specifications for:

- **Prepayment**: CPR (Constant Prepayment Rate), PSA (Public Securities Association) with helper constructors
- **Default**: CDR (Constant Default Rate), SDA (Standard Default Assumption) with business-day-aware recovery dates
- **Recovery**: Constant recovery rates with timing conventions

### 6. **Accrued Interest Engine**

Generic schedule-driven accrual supporting:

- **Linear**: Simple interest interpolation (`Accrued = Coupon × elapsed / period`)
- **Compounded**: ICMA-style with Taylor expansion for small fractions
- **Ex-Coupon**: Calendar-aware ex-coupon date handling
- **PIK Support**: Optional inclusion of PIK interest in accrued amount

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
use finstack_core::dates::{Date, Tenor, DayCount, BusinessDayConvention, StubKind};
use time::Month;

let issue = Date::from_calendar_date(2025, Month::January, 15)?;
let maturity = Date::from_calendar_date(2030, Month::January, 15)?;

let fixed_spec = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.05,  // 5% annual
    freq: Tenor::semi_annual(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::Following,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
    end_of_month: false,
    payment_lag_days: 0,
};

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .fixed_cf(fixed_spec)
    .build_with_curves(None)?;

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
    .build_with_curves(None)?;

// Track outstanding balance
let outstanding = schedule.outstanding_by_date()?;
for (date, balance) in outstanding {
    println!("{}: Outstanding = {}", date, balance);
}
```

### Floating-Rate Note

```rust
use finstack_valuations::cashflow::builder::{FloatingCouponSpec, FloatingRateSpec};
use finstack_core::types::CurveId;

let float_spec = FloatingCouponSpec {
    coupon_type: CouponType::Cash,
    rate_spec: FloatingRateSpec {
        index_id: CurveId::new("USD-SOFR-3M"),
        spread_bp: 200.0,           // 200 bps spread
        gearing: 1.0,               // Rate multiplier (default: 1.0)
        gearing_includes_spread: true, // (index + spread) * gearing
        floor_bp: Some(0.0),        // 0% index floor
        all_in_floor_bp: None,      // No minimum coupon
        cap_bp: None,               // No all-in cap
        index_cap_bp: None,         // No index cap
        reset_freq: Tenor::quarterly(),
        reset_lag_days: 2,          // T-2 fixing convention
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: "weekends_only".to_string(),
        fixing_calendar_id: None,   // Uses calendar_id if None
        end_of_month: false,
        payment_lag_days: 0,
    },
    freq: Tenor::quarterly(),
    stub: StubKind::None,
};

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .floating_cf(float_spec)
    .build_with_curves(None)?;
```

### Accrued Interest Calculation

```rust
use finstack_valuations::cashflow::{
    AccrualMethod, AccrualConfig, ExCouponRule, accrued_interest_amount
};

// Build schedule first (any instrument that provides CashFlowSchedule)
let schedule = /* ... */;

// Configure accrual method
let config = AccrualConfig {
    method: AccrualMethod::Linear,  // or Compounded
    ex_coupon: Some(ExCouponRule {
        days_before_coupon: 7,
        calendar_id: Some("US".to_string()),  // Business days
    }),
    include_pik: true,  // Include PIK interest in accrued
};

// Calculate accrued interest as of a specific date
let accrued = accrued_interest_amount(&schedule, as_of, &config)?;
println!("Accrued interest: {:.2}", accrued);
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
let schedule: CashFlowSchedule = builder.build_with_curves(None)?;
```

#### Amortization Styles

```rust
// No amortization (bullet repayment)
AmortizationSpec::None

// Linear amortization to final notional
AmortizationSpec::LinearTo {
    final_notional: Money::new(0.0, Currency::USD),
}

// Custom step schedule (remaining principal after each date)
AmortizationSpec::StepRemaining {
    schedule: vec![
        (date1, Money::new(750_000.0, Currency::USD)),
        (date2, Money::new(500_000.0, Currency::USD)),
        (date3, Money::new(0.0, Currency::USD)),
    ],
}

// Fixed percentage of original notional per period (sinking-fund style)
AmortizationSpec::PercentOfOriginalPerPeriod {
    pct: 0.05,  // 5% of original notional per period
}

// Custom principal exchanges (absolute amounts)
AmortizationSpec::CustomPrincipal {
    items: vec![
        (date1, Money::new(100_000.0, Currency::USD)),
        (date2, Money::new(150_000.0, Currency::USD)),
    ],
}
```

#### Fee Specifications

```rust
use finstack_valuations::cashflow::builder::{FeeSpec, FeeBase, FeeTier, evaluate_fee_tiers};

// Fixed fee on specific date
let fixed_fee = FeeSpec::Fixed {
    date: fee_date,
    amount: Money::new(50_000.0, Currency::USD),
};

// Periodic fee on drawn balance
let drawn_fee = FeeSpec::PeriodicBps {
    base: FeeBase::Drawn,
    bps: 25.0,  // 25 bps annually
    freq: Tenor::quarterly(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
};

// Periodic fee on undrawn balance (commitment fee)
let undrawn_fee = FeeSpec::PeriodicBps {
    base: FeeBase::Undrawn {
        facility_limit: Money::new(10_000_000.0, Currency::USD),
    },
    bps: 50.0,
    freq: Tenor::quarterly(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
};

// Tiered fee evaluation based on utilization
let tiers = vec![
    FeeTier { threshold: 0.0, bps: 25.0 },   // 0-33%: 25 bps
    FeeTier { threshold: 0.33, bps: 37.5 },  // 33-66%: 37.5 bps
    FeeTier { threshold: 0.66, bps: 50.0 },  // 66-100%: 50 bps
];
let utilization = 0.5;  // 50% drawn
let fee_bps = evaluate_fee_tiers(&tiers, utilization);  // Returns 37.5
```

### 2. Using CashFlowSchedule

```rust
let schedule: CashFlowSchedule = /* ... */;

// Access all flows
let flows: &[CashFlow] = &schedule.flows;

// Get dates only
let dates: Vec<Date> = schedule.dates();

// Filter by kind
let coupons = schedule.flows.iter()
    .filter(|cf| cf.kind == CFKind::Fixed);
let amorts = schedule.flows.iter()
    .filter(|cf| cf.kind == CFKind::Amortization);
let redemptions = schedule.flows.iter()
    .filter(|cf| cf.kind == CFKind::Notional && cf.amount.amount() > 0.0);

// Convenience iterators
let all_coupons = schedule.coupons();  // Fixed + Stub kinds

// Outstanding balance tracking
// outstanding_path_per_flow: One entry per flow (Amortization, PIK only)
let path: Vec<(Date, Money)> = schedule.outstanding_path_per_flow()?;

// outstanding_by_date: One entry per date (includes Notional draws/repays)
// This is the canonical method for instruments like revolving credit facilities
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
use finstack_valuations::cashflow::aggregation::pv_by_period_with_ctx;
use finstack_core::market_data::traits::Discounting;
use finstack_core::dates::DayCountCtx;

let disc: &dyn Discounting = /* discount curve */;
let base = Date::from_calendar_date(2025, Month::January, 1)?;

// Use explicit day-count context for conventions requiring frequency or calendar
let pv_map = pv_by_period_with_ctx(
    &flows,
    &periods,
    disc,
    base,
    DayCount::Act365F,
    DayCountCtx::default(),
)?;
// Returns: IndexMap<PeriodId, IndexMap<Currency, Money>>
```

#### Credit-Adjusted PV

```rust
use finstack_valuations::cashflow::builder::CashFlowSchedule;
use finstack_core::market_data::traits::{Discounting, Survival};
use finstack_core::dates::DayCountCtx;

let schedule: CashFlowSchedule = /* build schedule */;
let disc: &dyn Discounting = /* discount curve */;
let hazard: &dyn Survival = /* hazard curve */;

let pv_map = schedule.pv_by_period_with_survival_and_ctx(
    &periods,
    disc,
    Some(hazard),
    Some(0.40),
    base,
    DayCount::Act365F,
    DayCountCtx::default(),
)?;
```

#### Credit-Adjusted PV with Recovery (Detailed)

For full `CashFlow` objects with recovery rate support:

```rust
use finstack_valuations::cashflow::aggregation::{
    pv_by_period_credit_adjusted_detailed, DateContext
};
use finstack_core::cashflow::primitives::CashFlow;

let flows: &[CashFlow] = /* full cashflow objects with CFKind */;
let recovery_rate = Some(0.40);  // 40% recovery on principal

let date_ctx = DateContext::new(base, DayCount::Act365F, DayCountCtx::default());

let pv_map = pv_by_period_credit_adjusted_detailed(
    flows,
    &periods,
    disc,
    Some(hazard),
    recovery_rate,
    date_ctx,
)?;
// Principal flows: PV = Amount * DF * (SP + R * (1 - SP))
// Interest flows:  PV = Amount * DF * SP (zero recovery)
```

### 4. CashflowProvider Trait

Instruments implement this to expose schedules:

```rust
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::cashflow::DatedFlows;

impl CashflowProvider for MyInstrument {
    fn build_dated_flows(
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

    // Optional: Override to provide instrument's notional
    fn notional(&self) -> Option<Money> {
        Some(self.notional)  // Used by default build_full_schedule
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
            .build_with_curves(Some(curves))
    }

}
```

### 5. Period PV from Schedule

```rust
use finstack_core::dates::DayCountCtx;

// Direct from schedule (convenience method, explicit day-count context)
let pv_map = schedule.pv_by_period_with_ctx(
    &periods,
    disc,
    base,
    DayCount::Act365F,
    DayCountCtx::default(),
)?;

// With market context (supports credit adjustment, explicit day-count context)
let pv_map = schedule.pv_by_period_with_market_and_ctx(
    &periods,
    &market,
    &CurveId::new("USD-OIS"),
    Some(&CurveId::new("AAPL-HAZARD")),
    base,
    DayCount::Act365F,
    DayCountCtx::default(),
)?;
```

### 6. Credit Event Emission Functions

Public functions for emitting credit-related cashflows:

```rust
use finstack_valuations::cashflow::builder::{
    emit_default_on, emit_prepayment_on,
    emit_commitment_fee_on, emit_usage_fee_on, emit_facility_fee_on,
    DefaultEvent,
};

// Emit default and recovery cashflows
let default_event = DefaultEvent {
    default_date: d,
    defaulted_amount: 100_000.0,
    recovery_rate: 0.40,
    recovery_lag: 12,  // months
    recovery_bdc: Some(BusinessDayConvention::Following),
    recovery_calendar_id: Some("US".to_string()),
    accrued_on_default: None,
};
let mut outstanding = 1_000_000.0;
let flows = emit_default_on(d, &[default_event], &mut outstanding, Currency::USD)?;

// Emit prepayment cashflow
let prepay_flows = emit_prepayment_on(d, 50_000.0, &mut outstanding, Currency::USD);

// Emit facility fees
let commitment_flows = emit_commitment_fee_on(d, undrawn, 50.0, year_frac, Currency::USD);
let usage_flows = emit_usage_fee_on(d, drawn, 25.0, year_frac, Currency::USD);
let facility_flows = emit_facility_fee_on(d, commitment, 10.0, year_frac, Currency::USD);
```

### 7. Behavioral Model Helpers

```rust
use finstack_valuations::cashflow::builder::{
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
    cpr_to_smm, smm_to_cpr,
};

// Prepayment models
let cpr_model = PrepaymentModelSpec::constant_cpr(0.06);  // 6% CPR
let psa_100 = PrepaymentModelSpec::psa_100();            // 100% PSA
let psa_150 = PrepaymentModelSpec::psa(1.5);             // 150% PSA

// Calculate SMM (monthly) from CPR (annual)
let smm = cpr_model.smm(seasoning_months);

// Default models
let cdr_model = DefaultModelSpec::constant_cdr(0.02);    // 2% CDR
let sda_100 = DefaultModelSpec::sda(1.0);                // 100% SDA

// Calculate MDR (monthly) from CDR (annual)
let mdr = cdr_model.mdr(seasoning_months);

// Recovery models
let recovery = RecoveryModelSpec::with_lag(0.40, 0);     // 40% recovery
let recovery_lag = RecoveryModelSpec::with_lag(0.70, 6); // 70%, 6-month lag

// Rate conversions
let smm = cpr_to_smm(0.06).unwrap();  // 6% CPR → SMM
let cpr = smm_to_cpr(0.005).unwrap(); // SMM → CPR
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
    Tenor::annual(),
    issue,
    maturity,
);

let coupon = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.06,
    freq: Tenor::quarterly(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
    end_of_month: false,
    payment_lag_days: 0,
};

let schedule = CashFlowSchedule::builder()
    .principal(init, issue, maturity)
    .amortization(amort)
    .fee(fee)
    .fixed_cf(coupon)
    .build_with_curves(None)?;

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
    freq: Tenor::semi_annual(),
    dc: DayCount::Thirty360,
    bdc: BusinessDayConvention::Following,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
    end_of_month: false,
    payment_lag_days: 0,
};

// Years 3-5: Cash
let cash_spec = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: 0.08,
    freq: Tenor::semi_annual(),
    dc: DayCount::Thirty360,
    bdc: BusinessDayConvention::Following,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
    end_of_month: false,
    payment_lag_days: 0,
};

let pik_end = Date::from_calendar_date(2027, Month::January, 1)?;

let schedule = CashFlowSchedule::builder()
    .principal(init, issue, maturity)
    .fixed_cf_window(pik_spec, issue, pik_end)   // PIK period
    .fixed_cf_window(cash_spec, pik_end, maturity) // Cash period
    .build_with_curves(None)?;

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
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::dates::DayCountCtx;

// Build market context
let disc_curve = DiscountCurve::builder("USD-OIS")
    .base_date(base)
    .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)])
    .interp(InterpStyle::Linear)
    .build()?;

let hazard_curve = HazardCurve::builder("CORP-HAZARD")
    .knots([(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
    .interp(InterpStyle::Linear)
    .build()?;

let market = MarketContext::new()
    .insert(disc_curve)
    .insert(hazard_curve);

// Compute credit-adjusted period PVs
let pv_map = schedule.pv_by_period_with_market_and_ctx(
    &periods,
    &market,
    &CurveId::new("USD-OIS"),
    Some(&CurveId::new("CORP-HAZARD")),
    base,
    DayCount::Act365F,
    DayCountCtx::default(),
)?;

// Analyze results
for (period_id, ccy_map) in pv_map {
    for (ccy, pv) in ccy_map {
        println!("{:?} {}: PV = {}", period_id, ccy, pv);
    }
}
```

### Example 5: Accrued Interest with Ex-Coupon

```rust
use finstack_valuations::cashflow::{
    AccrualMethod, AccrualConfig, ExCouponRule, accrued_interest_amount
};
use time::Month;

let issue = Date::from_calendar_date(2025, Month::January, 15)?;
let maturity = Date::from_calendar_date(2030, Month::January, 15)?;

// Build a bond schedule
let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .fixed_cf(FixedCouponSpec {
        coupon_type: CouponType::Cash,
        rate: 0.05,
        freq: Tenor::semi_annual(),
        dc: DayCount::Thirty360,
        bdc: BusinessDayConvention::Following,
        calendar_id: "weekends_only".to_string(),
        stub: StubKind::None,
        end_of_month: false,
        payment_lag_days: 0,
    })
    .build_with_curves(None)?;

// Configure accrual with ex-coupon rule
let config = AccrualConfig {
    method: AccrualMethod::Compounded,  // ICMA-style
    ex_coupon: Some(ExCouponRule {
        days_before_coupon: 5,
        calendar_id: Some("US".to_string()),  // Business days
    }),
    include_pik: false,
};

// Calculate accrued interest
let as_of = Date::from_calendar_date(2025, Month::April, 1)?;
let accrued = accrued_interest_amount(&schedule, as_of, &config)?;
println!("Accrued interest: ${:.2}", accrued);

// If as_of falls in ex-coupon window, returns 0.0
let ex_coupon_date = Date::from_calendar_date(2025, Month::July, 10)?;
let accrued_ex = accrued_interest_amount(&schedule, ex_coupon_date, &config)?;
assert_eq!(accrued_ex, 0.0);  // In ex-coupon window
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
    builder: &mut CashFlowBuilder,
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
        .build_with_curves(None)
        .unwrap();

    let amorts: Vec<_> = schedule
        .flows
        .iter()
        .filter(|cf| cf.kind == CFKind::Amortization)
        .collect();

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
    builder: &CashFlowBuilder,
    bps: f64,
    dates: &[Date],
) -> finstack_core::Result<Vec<CashFlow>> {
    let mut fees = Vec::new();
    let schedule = builder.build_with_curves(None)?;
    let outstanding_path = schedule.outstanding_path_per_flow()?;

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
| Fees | `specs/fees.rs`, `emission/fees.rs` | Fee calculations, tiering, commitment/usage/facility |
| Coupons | `specs/coupon.rs`, `emission/coupons.rs` | Interest payment patterns with caps/floors/gearing |
| Prepayment | `specs/prepayment.rs`, `emission/credit.rs` | CPR/PSA behavioral models |
| Default | `specs/default.rs`, `emission/credit.rs` | CDR/SDA models, recovery with calendar support |
| Recovery | `specs/recovery.rs` | Recovery rate and timing specifications |
| Aggregation | `aggregation.rs` | Period rollup algorithms with DayCountCtx |
| Accrual | `accrual.rs` | Schedule-driven accrued interest engine |

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
let total = schedule.flows.iter()
    .filter(|cf| cf.kind == CFKind::Fixed)
    .map(|cf| cf.amount.amount())
    .sum::<f64>();

// Less efficient: Collect first
let coupons: Vec<_> = schedule
    .flows
    .iter()
    .filter(|cf| cf.kind == CFKind::Fixed)
    .collect();
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
use finstack_core::dates::DayCountCtx;

let instrument_pvs: Vec<Money> = instruments.par_iter()
    .map(|inst| {
        let schedule = inst.build_full_schedule(&market, as_of)?;
        let pv_map = schedule
            .pv_by_period_with_ctx(&periods, disc, base, dc, DayCountCtx::default())
            .expect("period PV calculation should succeed");
        pv_map
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
        .build_with_curves(None)
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
            .build_with_curves(None)
            .unwrap();

        let total_amort: f64 = schedule
            .flows
            .iter()
            .filter(|cf| cf.kind == CFKind::Amortization)
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
    use finstack_core::dates::DayCountCtx;

    // Periodized PV with explicit context
    let pv_map = schedule
        .pv_by_period_with_ctx(&periods, &disc, base, DayCount::Act365F, DayCountCtx::default())
        .unwrap();
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

#### Accrual Test: Linear vs Compounded

```rust
#[test]
fn test_accrual_methods() {
    use finstack_valuations::cashflow::{accrued_interest_amount, AccrualConfig, AccrualMethod};

    let schedule = /* build schedule with known coupon */;
    let mid_period = /* date halfway through coupon period */;

    let linear_cfg = AccrualConfig {
        method: AccrualMethod::Linear,
        ex_coupon: None,
        include_pik: true,
    };
    let compounded_cfg = AccrualConfig {
        method: AccrualMethod::Compounded,
        ex_coupon: None,
        include_pik: true,
    };

    let linear_accrued = accrued_interest_amount(&schedule, mid_period, &linear_cfg).unwrap();
    let compounded_accrued = accrued_interest_amount(&schedule, mid_period, &compounded_cfg).unwrap();

    // Compounded accrual is slightly less than linear at mid-period
    assert!(compounded_accrued <= linear_accrued);
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
- **[`finstack_core::dates`]**: Date generation, day counts, calendars, `DayCountCtx`
- **[Valuations README](../README.md)**: Instrument pricing and risk
- **[Portfolio README](../../../portfolio/README.md)**: Position aggregation

---

## Summary

The cashflow module is the foundation for all instrument valuation in Finstack. Its currency-safe, deterministic design ensures:

- **Correctness**: No cross-currency arithmetic errors
- **Transparency**: Rich classification and metadata with expanded `CFKind` taxonomy
- **Flexibility**: Composable builder for complex schedules with caps, floors, gearing
- **Performance**: Efficient period aggregation and PV calculations with O(n+m) complexity
- **Extensibility**: Clear patterns for adding new features
- **Accrual Support**: Generic schedule-driven accrued interest with Linear, Compounded, and ex-coupon support
- **Credit Events**: Full default, prepayment, and recovery modeling with calendar-aware date adjustment

### Key API Changes (Recent)

| Feature | Description |
|---------|-------------|
| `accrual` module | Generic accrued interest engine (Linear/Compounded) with ex-coupon support |
| `FloatingRateSpec` | Enhanced with caps, floors, gearing, fixing calendars |
| `AmortizationSpec::CustomPrincipal` | Explicit principal exchanges on specific dates |
| `FeeBase::Drawn/Undrawn` | Fee base specification with facility limit support |
| `FeeTier` + `evaluate_fee_tiers` | Utilization-based tiered fee evaluation |
| `DefaultEvent` | Added `recovery_bdc` and `recovery_calendar_id` for date adjustment |
| `emit_*_on` functions | Public credit event emission (default, prepay, commitment/usage/facility fees) |
| `pv_by_period_*_with_ctx` | Aggregation functions now accept explicit `DayCountCtx` |
| `pv_by_period_credit_adjusted_detailed` | Recovery-aware PV with `CFKind` preservation |
| `CashflowProvider::notional()` | Optional method for instruments to expose notional |

For questions or contributions, see the main Finstack documentation or raise an issue on GitHub.
