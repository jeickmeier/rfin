# Discounted Cash Flow (DCF)

## Features

- Corporate valuation instrument with explicit projected free-cash-flow schedule and terminal value specification (Gordon Growth or Exit Multiple).
- Net debt adjustment to derive equity value, using explicit valuation date and discount curve.
- Builder pattern via `DiscountedCashFlowBuilder` with serde-stable fields for storage/pipeline use.

## Methodology & References

- Standard DCF formula: present value of projected FCFs plus discounted terminal value minus net debt.
- Terminal value options implement Gordon Growth and Exit Multiple methods common in equity research/IB practice.
- Year fractions use an ACT/365.25 basis; discounting uses deterministic WACC or supplied curve.

## Usage Example

```rust
use finstack_valuations::instruments::equity::dcf_equity::{DiscountedCashFlow, TerminalValueSpec};
use finstack_core::{currency::Currency, dates::Date, money::Money, types::{CurveId, InstrumentId}};
use time::Month;

let flows = vec![
    (Date::from_calendar_date(2025, Month::December, 31)?, 12.0),
    (Date::from_calendar_date(2026, Month::December, 31)?, 14.0),
];

let dcf = DiscountedCashFlow::builder()
    .id(InstrumentId::new("DCF-ACME"))
    .currency(Currency::USD)
    .flows(flows)
    .wacc(0.09)
    .terminal_value(TerminalValueSpec::GordonGrowth { growth_rate: 0.02 })
    .net_debt(50.0)
    .valuation_date(Date::from_calendar_date(2024, Month::December, 31)?)
    .discount_curve_id(CurveId::new("USD-OIS"))
    .attributes(Default::default())
    .build()?;

let equity_value = dcf.value(&market_context, dcf.valuation_date)?;
```

## Limitations / Known Issues

- Simplified corporate-finance DCF; no scenario trees, tax shields, or capital structure optimization.
- Uses deterministic discounting (WACC/curve); no pathwise uncertainty or simulation.
- Terminal value guards only check WACC > growth; no explicit handling of negative cashflows beyond provided inputs.

## Pricing Methodology

- Discounts explicit free cashflows and terminal value using WACC/discount curve on ACT/365.25 basis.
- Terminal value via Gordon Growth `(FCF_T × (1+g)/(WACC-g))` or Exit Multiple (`metric × multiple`), discounted to valuation date.
- Equity value = Enterprise Value − Net Debt; deterministic single-scenario valuation.

## Metrics

- Enterprise value, equity value, implied perpetual growth rate (if solved), and sensitivity tables via simple bump scripts.
- Duration-like sensitivity to WACC and growth (finite-difference) for scenario analysis.
- Cashflow/terminal value contribution breakdown.

## Future Enhancements

- Add probabilistic/scenario-weighted DCF paths and tax/CapEx/depreciation schedules.
- Support multi-stage growth/discount curves and mid-year discounting options.
- Provide built-in sensitivity tables (WACC/g matrices) and Monte Carlo on key drivers.
