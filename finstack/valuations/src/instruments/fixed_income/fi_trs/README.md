# Fixed Income Index Total Return Swap (TRS)

## Features

- Synthetic fixed income index exposure via total return swap
- Supports receive/pay total return via `TrsSide`
- Carry/yield model for pricing (`e^{y × dt} - 1` per period)
- ETF replication convenience constructor

## Methodology & References

- PV = PV(total-return leg) − PV(financing leg)
- Carry model: total return per period = `e^{y × dt} - 1` where `y` is the continuous index yield
- Rate sensitivity comes from discounting; yield sensitivity captured by `DurationDv01`
- Par spread ≈ yield − financing rate (for intuition)
- Deterministic curves and index yields; no stochastic credit modeling

## Usage Example

```rust
use finstack_valuations::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;

let trs = FIIndexTotalReturnSwap::example();
let pv = trs.value(&market_context, as_of_date)?;
```

## Complete Construction Example

```rust
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, BusinessDayConvention, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use rust_decimal::Decimal;
use finstack_valuations::cashflow::builder::ScheduleParams;
use finstack_valuations::instruments::FinancingLegSpec;
use finstack_valuations::instruments::IndexUnderlyingParams;
use finstack_valuations::instruments::fixed_income::fi_trs::{
    FIIndexTotalReturnSwap, TrsScheduleSpec, TrsSide,
};

// 1. Define the financing leg specification
let financing_spec = FinancingLegSpec {
    discount_curve_id: CurveId::new("USD-OIS"),      // OIS curve for discounting
    forward_curve_id: CurveId::new("USD-SOFR-3M"),   // SOFR 3M for floating rate
    spread_bp: Decimal::from(35),                     // 35bp spread over SOFR
    day_count: DayCount::Act360,
};

// 2. Define the schedule parameters
let schedule_params = ScheduleParams {
    freq: Tenor::quarterly(),                        // Quarterly resets
    dc: DayCount::Act360,                            // Day count for accrual
    bdc: BusinessDayConvention::ModifiedFollowing,   // Business day adjustment
    calendar_id: "NYC".to_string(),                  // New York calendar
    stub: StubKind::ShortFront,                      // Short stub at front
    end_of_month: false,
    payment_lag_days: 0,
};

// 3. Create the TRS schedule specification
let start_date = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
let end_date = Date::from_calendar_date(2025, time::Month::January, 15).unwrap();
let schedule_spec = TrsScheduleSpec::from_params(start_date, end_date, schedule_params);

// 4. Define the underlying index parameters (e.g., Bloomberg US Corporate Bond Index)
let underlying = IndexUnderlyingParams::new("LUACTRUU", Currency::USD)
    .with_yield("LUACTRUU-YIELD")         // Optional yield scalar ID
    .with_duration("LUACTRUU-DURATION");   // Optional duration scalar ID

// 5. Build the fixed income index TRS
let trs = FIIndexTotalReturnSwap::builder()
    .id("TRS-LUACTRUU-1Y".into())
    .notional(Money::new(25_000_000.0, Currency::USD))
    .underlying(underlying)
    .financing(financing_spec)
    .schedule(schedule_spec)
    .side(TrsSide::ReceiveTotalReturn)  // Long bond index exposure
    .build()
    .unwrap();

// 6. Price the instrument
let npv = trs.value(&market_context, as_of_date)?;
let financing_pv = trs.pv_financing_leg(&market_context, as_of_date)?;
let total_return_pv = trs.pv_total_return_leg(&market_context, as_of_date)?;
```

## ETF Replication (Shorthand)

```rust
use finstack_valuations::instruments::fixed_income::fi_trs::FIIndexTotalReturnSwap;

// Using the same financing_spec and schedule_spec from above
// Popular bond ETFs: LQD (IG Corp), HYG (HY Corp), AGG (Agg), TLT (Long Treasury)
let lqd_trs = FIIndexTotalReturnSwap::replicate_etf(
    "LQD",                                          // ETF ticker
    Money::new(10_000_000.0, Currency::USD),        // Notional
    financing_spec,                                 // Financing leg
    schedule_spec,                                  // Payment schedule
    Some("LQD-YIELD"),                              // Optional yield scalar ID
    Some("LQD-DURATION"),                           // Optional duration scalar ID
)?;
```

## Margining

Fixed income index TRS implement full margin support following **ISDA CSA** standards with duration-based SIMM IR bucket classification.

| SIMM Risk Class | Sensitivity Type |
|-----------------|------------------|
| Interest Rate | IR delta (based on index duration) |

## Metrics

- **DurationDv01**: Duration-based yield sensitivity (`Notional × Duration × 1bp`)
- **DV01**: Sensitivity to financing rate
- **BucketedDV01**: Key-rate DV01 on financing leg
- **ParSpread**: Spread that makes NPV = 0 (from receiver's perspective)
- **FinancingAnnuity**: PV01 of financing leg

## Limitations / Known Issues

- Total-return path is deterministic from supplied yields
- `CashflowProvider` returns placeholder zero-amount flows (actual TRS amounts depend on realized returns)
- Does not model early termination or bespoke fee structures

## TODO: Basket/Constituent Support

Currently, the underlying is modeled as a single index (e.g., "LUACTRUU" or "LQD"). In reality, fixed income indices represent baskets of constituent bonds:

- **LQD**: ~2,500 investment-grade corporate bonds
- **HYG**: ~1,200 high-yield corporate bonds
- **AGG**: ~12,000+ bonds (Treasuries, MBS, corporates, agencies)
- **TLT**: ~40 long-dated Treasury bonds

### Planned Enhancements

1. **Basket Definition**
   - Add `BondBasket` struct to define constituents with market values/weights
   - Handle index rebalancing (monthly reconstitution)

2. **Constituent-Level Metrics**
   - **DV01 by bond**: Breakdown of index DV01 to each underlying bond
   - **CS01 by issuer**: Aggregate rate risk by corporate issuer
   - **CS01 by rating bucket**: Credit spread sensitivity by rating (AAA, AA, A, BBB, etc.)
   - **Sector DV01**: Aggregate by attribute (i.e. industry sector (financials, utilities, etc.)
   - **Maturity bucket DV01**: Key-rate duration by tenor bucket

3. **Aggregate vs. Decomposed Pricing**
   - Index-level pricing (current): Uses index yield/duration estimates
   - Constituent-level pricing (future): Sum of individual bond valuations
   - Tracking difference: Index NAV vs. synthetic basket value

4. **Credit Risk Decomposition**

   ```rust
   // Future API sketch
   let basket = BondBasket::load_from_csv("lqd_constituents.csv")?;
   let trs = FIIndexTotalReturnSwap::builder()
       .underlying_basket(basket)
       // ...
       .build()?;

   // Constituent-level metrics
   let dv01_by_bond = trs.dv01_by_constituent(&ctx)?;
   let dv01_by_rating = trs.dv01_by_rating_bucket(&ctx)?;
   let cs01_by_issuer = trs.cs01_by_issuer(&ctx)?;
   let sector_exposure = trs.exposure_by_sector(&ctx)?;
   ```

5. **Index Characteristics Passthrough**
   - OAS (option-adjusted spread)
   - Effective duration / convexity
   - Yield-to-worst, yield-to-maturity
   - Average credit rating
   - Spread duration by rating tier
