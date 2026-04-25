# finstack-cashflows

Cashflow schedule construction, accrual, and period aggregation for bonds,
loans, swaps, and structured products.

## Overview

`finstack-cashflows` is the cashflow-focused crate in the Finstack workspace. It
provides:

- schedule construction through `CashFlowSchedule::builder()`
- finance-facing specification types for coupons, fees, amortization, default,
  prepayment, and recovery
- schedule-driven accrued-interest calculations
- currency-preserving aggregation utilities
- schedule-level periodized PV helpers that accept explicit `DayCountContext`

The crate is designed around a few explicit conventions:

- amounts are represented with currency-tagged `Money`
- rates are usually decimals, while spreads and fee quotes are often basis
  points
- payment and reset lags are business-day based when a calendar is supplied
- day-count behavior is explicit and should not be inferred from examples alone

## Import Paths

Use `finstack_cashflows::...` when you depend on this crate directly.

```rust
use finstack_cashflows::builder::CashFlowSchedule;
```

The `finstack-valuations` crate also re-exports this crate as
`finstack_valuations::cashflow`, which is convenient inside valuations code but
is not the canonical path for this package README.

## Main Entry Points

- `finstack_cashflows::builder`
  Schedule construction, schedule specs, and `CashFlowSchedule`. Includes
  schedule-inspection methods (`weighted_average_life`, `coupons`,
  `outstanding_path_per_flow`, `outstanding_by_date`,
  `merge_cashflow_schedules`, `normalize_public`) and ten market-convention
  presets on `ScheduleParams` (USD/EUR/GBP/JPY swaps and bonds, including
  `jpy_tona_swap`).
- `finstack_cashflows::aggregation`
  Currency-preserving nominal aggregation helpers, plus the
  [`RecoveryTiming`] enum that controls how the recovery leg on
  surviving principal flows is placed in time (`AtPaymentDate` vs
  `AtDefaultIntegrated`).
- `finstack_cashflows::accrual`
  Schedule-driven accrued interest configuration and calculations.
- `finstack_cashflows::traits`
  `CashflowProvider`, `schedule_from_dated_flows`, and
  `schedule_from_classified_flows` (for callers that already carry
  pre-classified flows).
- `finstack_cashflows::primitives`
  Re-exports from `finstack_core::cashflow`, including `CashFlow` and `CFKind`.

[`RecoveryTiming`]: https://docs.rs/finstack-cashflows/latest/finstack_cashflows/aggregation/enum.RecoveryTiming.html

## Quick Start

### Build a Fixed-Rate Schedule

```rust
use finstack_cashflows::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use rust_decimal_macros::dec;
use time::Month;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let issue = Date::from_calendar_date(2025, Month::January, 15)?;
let maturity = Date::from_calendar_date(2030, Month::January, 15)?;

let fixed_spec = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: dec!(0.05), // 5% annual coupon rate
    freq: Tenor::semi_annual(),
    dc: DayCount::Thirty360,
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

assert!(!schedule.flows.is_empty());
# Ok(())
# }
```

### Add Amortization and Periodic Fees

```rust
use finstack_cashflows::builder::{
    AmortizationSpec, CashFlowSchedule, CouponType, FeeBase, FeeSpec, FixedCouponSpec,
};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use rust_decimal_macros::dec;
use time::Month;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let issue = Date::from_calendar_date(2025, Month::January, 1)?;
let maturity = Date::from_calendar_date(2028, Month::January, 1)?;

let fee = FeeSpec::PeriodicBps {
    base: FeeBase::Drawn,
    bps: dec!(25),
    freq: Tenor::quarterly(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
    accrual_basis: Default::default(),
};

let coupon = FixedCouponSpec {
    coupon_type: CouponType::Cash,
    rate: dec!(0.06),
    freq: Tenor::quarterly(),
    dc: DayCount::Act360,
    bdc: BusinessDayConvention::ModifiedFollowing,
    calendar_id: "weekends_only".to_string(),
    stub: StubKind::None,
    end_of_month: false,
    payment_lag_days: 0,
};

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .amortization(AmortizationSpec::LinearTo {
        final_notional: Money::new(0.0, Currency::USD),
    })
    .fee(fee)
    .fixed_cf(coupon)
    .build_with_curves(None)?;

let balances = schedule.outstanding_by_date()?;
assert!(!balances.is_empty());
# Ok(())
# }
```

### Build a Floating-Rate Schedule

```rust
use finstack_cashflows::builder::{CashFlowSchedule, CouponType, FloatingCouponSpec, FloatingRateSpec};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use rust_decimal_macros::dec;
use time::Month;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let issue = Date::from_calendar_date(2025, Month::January, 15)?;
let maturity = Date::from_calendar_date(2027, Month::January, 15)?;

let float_spec = FloatingCouponSpec {
    coupon_type: CouponType::Cash,
    rate_spec: FloatingRateSpec {
        index_id: CurveId::new("USD-SOFR-3M"),
        spread_bp: dec!(200),
        gearing: dec!(1),
        gearing_includes_spread: true,
        index_floor_bp: Some(dec!(0)),
        all_in_floor_bp: None,
        all_in_cap_bp: None,
        index_cap_bp: None,
        reset_freq: Tenor::quarterly(),
        reset_lag_days: 2,
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: "weekends_only".to_string(),
        fixing_calendar_id: None,
        end_of_month: false,
        payment_lag_days: 0,
        overnight_compounding: None,
        overnight_basis: None,
        fallback: Default::default(),
    },
    freq: Tenor::quarterly(),
    stub: StubKind::None,
};

let schedule = CashFlowSchedule::builder()
    .principal(Money::new(1_000_000.0, Currency::USD), issue, maturity)
    .floating_cf(float_spec)
    .build_with_curves(None)?;

assert!(!schedule.flows.is_empty());
# Ok(())
# }
```

### Compute Accrued Interest

```rust
use finstack_cashflows::{accrued_interest_amount, AccrualConfig, AccrualMethod, ExCouponRule};

# fn demo(schedule: &finstack_cashflows::builder::CashFlowSchedule, as_of: finstack_core::dates::Date) -> finstack_core::Result<f64> {
let config = AccrualConfig {
    method: AccrualMethod::Compounded,
    ex_coupon: Some(ExCouponRule {
        days_before_coupon: 5,
        calendar_id: Some("usny".to_string()),
    }),
    include_pik: false,
    frequency: None,
};

let accrued = accrued_interest_amount(schedule, as_of, &config)?;
# Ok(accrued)
# }
```

The return value is a scalar amount in the schedule's amount space. Use the
schedule notional or flow currency to interpret it.

## Common Workflows

### Aggregate Dated Flows by Reporting Period

```rust
use finstack_cashflows::aggregation::aggregate_by_period;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::money::Money;
use time::Month;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let flows = vec![
    (
        Date::from_calendar_date(2025, Month::March, 15)?,
        Money::new(100_000.0, Currency::USD),
    ),
    (
        Date::from_calendar_date(2025, Month::March, 20)?,
        Money::new(50_000.0, Currency::EUR),
    ),
];

let periods = vec![Period {
    id: PeriodId::quarter(2025, 1),
    start: Date::from_calendar_date(2025, Month::January, 1)?,
    end: Date::from_calendar_date(2025, Month::April, 1)?,
    is_actual: true,
}];

let aggregated = aggregate_by_period(&flows, &periods);
assert!(aggregated.contains_key(&PeriodId::quarter(2025, 1)));
# Ok(())
# }
```

### Compute Periodized PV from a Schedule

Schedule-level PV helpers are the stable public interface in this crate.

```rust,no_run
use finstack_cashflows::aggregation::DateContext;
use finstack_cashflows::builder::CashFlowSchedule;
use finstack_cashflows::builder::{PvCreditAdjustment, PvDiscountSource};
use finstack_core::dates::{Date, DayCount, DayCountContext, Period};
use finstack_core::market_data::traits::{Discounting, Survival};

fn periodized_pv(
    schedule: &CashFlowSchedule,
    periods: &[Period],
    disc: &dyn Discounting,
    base: Date,
) -> finstack_core::Result<()> {
    let pv_map = schedule.pv_by_period(
        periods,
        PvDiscountSource::Discount { disc, credit: None },
        DateContext::new(base, DayCount::Act365F, DayCountContext::default()),
    )?;

    let _ = pv_map;
    Ok(())
}

fn credit_adjusted_periodized_pv(
    schedule: &CashFlowSchedule,
    periods: &[Period],
    disc: &dyn Discounting,
    hazard: &dyn Survival,
    base: Date,
) -> finstack_core::Result<()> {
    let pv_map = schedule.pv_by_period(
        periods,
        PvDiscountSource::Discount {
            disc,
            credit: Some(PvCreditAdjustment {
                hazard: Some(hazard),
                recovery_rate: Some(0.40),
            }),
        },
        DateContext::new(base, DayCount::Act365F, DayCountContext::default()),
    )?;

    let _ = pv_map;
    Ok(())
}
```

### Implement `CashflowProvider`

`CashflowProvider` requires `cashflow_schedule`. The default
`dated_cashflows` implementation derives holder-view `(Date, Money)` pairs
from the returned `CashFlowSchedule`.

```rust,no_run
use finstack_cashflows::builder::{CashFlowSchedule, CouponType, FixedCouponSpec};
use finstack_cashflows::CashflowProvider;
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use rust_decimal_macros::dec;

struct FixedBondLike {
    notional: Money,
    issue: Date,
    maturity: Date,
}

impl CashflowProvider for FixedBondLike {
    fn notional(&self) -> Option<Money> {
        Some(self.notional)
    }

    fn cashflow_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<CashFlowSchedule> {
        CashFlowSchedule::builder()
            .principal(self.notional, self.issue, self.maturity)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: dec!(0.05),
                freq: Tenor::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::Following,
                calendar_id: "weekends_only".to_string(),
                stub: StubKind::None,
                end_of_month: false,
                payment_lag_days: 0,
            })
            .build_with_curves(None)
    }
}
```

### Inspect a Schedule

`CashFlowSchedule` exposes several inspection helpers that downstream desks
typically reach for after building a schedule. All accessors operate on the
canonical sorted flow list and respect the schedule's currency invariants.

```rust,no_run
use finstack_cashflows::builder::{CashFlowSchedule, merge_cashflow_schedules};
use finstack_core::dates::Date;

fn inspect(schedule: &CashFlowSchedule, as_of: Date) -> finstack_core::Result<()> {
    // Weighted Average Life (Act/365F regardless of schedule day count).
    let wal_years = schedule.weighted_average_life(as_of)?;

    // Iterate interest-like coupons only (excludes PIK, fees, principal).
    let coupon_count = schedule.coupons().count();

    // Two outstanding-balance views — pick by use case:
    // - per_flow: simple amortization view, ignores notional draws/repays.
    // - by_date:  full balance tracker including draws/repays (RCFs, etc.).
    let per_flow = schedule.outstanding_path_per_flow()?;
    let by_date  = schedule.outstanding_by_date()?;

    let _ = (wal_years, coupon_count, per_flow, by_date);
    Ok(())
}

// Compose multiple legs into a single deterministic composite schedule.
// Metadata fields (calendar IDs, issue date, facility limit, representation)
// are merged conservatively — see `merge_cashflow_schedules` rustdoc for the
// exact rules.
fn compose(legs: Vec<CashFlowSchedule>) -> CashFlowSchedule {
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_cashflows::builder::Notional;
    merge_cashflow_schedules(
        legs,
        Notional::par(0.0, Currency::USD),
        DayCount::Act365F,
    )
}
```

For the `CashflowProvider` boundary, callers commonly need to re-stamp a
schedule as future-only and `Projected` before handing it downstream.
[`CashFlowSchedule::normalize_public`](https://docs.rs/finstack-cashflows/latest/finstack_cashflows/builder/schedule/struct.CashFlowSchedule.html#method.normalize_public)
performs the canonical `filter_future + omit_pure_pik + re-sort + tag`
pipeline in a single call.

## Hidden Integration Helpers

The builder module re-exports a small set of `emit_*` helpers with
`#[doc(hidden)]`. They exist for internal interoperability and tests, but this
README does not treat them as the primary stable API surface. Prefer the
schedule builder and the public spec types unless you are intentionally working
close to the emission pipeline.

## `CFKind` Guidance

`CFKind` is defined in `finstack_core::cashflow` and is `#[non_exhaustive]`.
Prefer linking to or matching the authoritative type instead of copying the enum
into downstream docs. The schedule builder relies on `CFKind` for deterministic
ordering, accrual behavior, and credit-adjusted PV treatment.

## Testing

Useful crate-local commands:

```bash
# package tests
cargo test -p finstack-cashflows

# doc tests
cargo test -p finstack-cashflows --doc

# generate rustdoc and fail on warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-cashflows --no-deps --all-features
```

## References

- Day-count and business-day conventions:
  `docs/REFERENCES.md#isda-2006-definitions`
- Bond accrued-interest conventions:
  `docs/REFERENCES.md#icma-rule-book`
- Discounting and fixed-income intuition:
  `docs/REFERENCES.md#hull-options-futures`
- Multi-curve and rates conventions:
  `docs/REFERENCES.md#andersen-piterbarg-interest-rate-modeling`

## See Also

- `finstack_core::cashflow` for `CashFlow` and `CFKind`
- `finstack_core::money` for currency-safe `Money`
- `finstack_core::dates` for `DayCount`, `DayCountContext`, calendars, and schedule
  helpers
- `finstack/valuations/src/lib.rs` for the valuations-side `cashflow`
  re-export
