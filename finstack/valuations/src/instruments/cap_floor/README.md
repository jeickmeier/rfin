# Interest Rate Cap/Floor

## Features
- Supports caps, floors, caplets, and floorlets via `RateOptionType` with configurable schedule (freq/day-count/BDC/stub).
- Uses explicit discount, forward, and volatility curve IDs for market data alignment; settlement and exercise style fields follow standard conventions.
- Helper constructors (`new_cap`, `new_floor`, and `InterestRateOptionParams`) simplify building standard structures.

## Methodology & References
- Black (1976) lognormal model for caplet/floorlet pricing and Greeks (`pricing/black.rs`).
- Deterministic projection of forward rates with discounting from the chosen curve; no stochastic rates beyond the supplied curves.
- Conventions follow ISDA interest-rate option market standards (Act/360, modified following, IMM-style stubs).

## Usage Example
```rust
use finstack_valuations::instruments::cap_floor::InterestRateOption;
use finstack_core::{currency::Currency, dates::*, money::Money, types::CurveId};
use time::Month;

let cap = InterestRateOption::new_cap(
    "CAP-1Y",
    Money::new(10_000_000.0, Currency::USD),
    0.035,
    Date::from_calendar_date(2024, Month::January, 3)?,
    Date::from_calendar_date(2025, Month::January, 3)?,
    Frequency::quarterly(),
    DayCount::Act360,
    CurveId::new("USD-OIS"),
    CurveId::new("USD-SOFR-3M"),
    CurveId::new("USD-CAP-VOL"),
);
let pv = cap.value(&market_context, Date::from_calendar_date(2024, Month::January, 3)?)?;
```

## Limitations / Known Issues
- Pricing assumes European exercise and Black lognormal dynamics; no Bachelier/normal or displaced-diffusion variants.
- Volatility smile handled only through the supplied surface; no stochastic volatility or SABR inside the pricer.
- Does not include convexity adjustments for futures-style margined underlyings.

## Pricing Methodology
- Projects forward rates on the specified forward curve and discounts on the chosen discount curve.
- Prices each caplet/floorlet via Black (1976) using surface or override vol; aggregates across schedule with day-count accruals.
- Handles stubs/BDC/holiday adjustments via schedule parameters; supports cash settlement.

## Metrics
- PV plus cap/floor par strike (implied volatility to match price), delta/vega/theta via bump-and-revalue.
- DV01 on discount curve and forward-curve sensitivities (parallel/key-rate) through generic calculators.
- Bucketed caplet contributions for attribution.

## Future Enhancements
- Add Bachelier/normal and displaced-diffusion pricing paths for low-rate regimes.
- Support SABR/shifted-lognormal smile integration for more accurate vol skews.
- Include gamma/volga analytics and callable-cap style optionality extensions.
