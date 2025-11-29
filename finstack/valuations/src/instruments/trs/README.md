# Total Return Swap (TRS)

## Features
- Equity and fixed-income index TRS variants with configurable total-return leg, financing leg, and schedules.
- Supports receive/pay total return via `TrsSide`, with financing leg specs (float index + spread) and total-return leg params (dividends/coupons, price sources).
- Example builders provided for `EquityTotalReturnSwap` and `FIIndexTotalReturnSwap`; shared scheduling utilities in `TrsScheduleSpec`.

## Methodology & References
- PV = PV(total-return leg) − PV(financing leg), using discount curves and projected returns from market data (spots, dividend yields, index prices).
- Financing leg uses standard floating-rate accrual via `FinancingLegSpec`; total-return leg computes price appreciation plus income over periods.
- Deterministic curves and index paths; no stochastic equity/credit modeling inside the pricer.

## Usage Example
```rust
use finstack_valuations::instruments::trs::equity::EquityTotalReturnSwap;

let trs = EquityTotalReturnSwap::example();
let pv = trs.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- No margining, funding spreads, or collateral modeling beyond chosen discount curves.
- Total-return path is deterministic from supplied prices/yields; no simulation of underlying index volatility.
- Does not model early termination, resettable notionals, or bespoke fee structures beyond leg specs.

## Pricing Methodology
- Builds total-return leg using underlying price/return plus income; financing leg uses floating index plus spread with schedule params.
- Discounts leg cashflows via discount curve; converts index returns to PV with appropriate accrual/day-count handling.
- Deterministic index paths; relies on market quotes/dividend yields for equity TRS and price indices for FI TRS.

## Metrics
- PV, financing vs total-return leg contribution, carry/roll, and funding spread sensitivity.
- DV01/CS01 on discount/financing curves via generic calculators; delta to underlying index via total-return leg exposure.
- Scenario metrics for funding spread and underlying price shocks through bump hooks.

## Future Enhancements
- Add margining and collateral modeling, plus resettable notionals and pathwise financing accrual.
- Support stochastic equity/credit processes for total-return legs and correlation to financing leg.
- Provide coupon reinvestment/fee modeling and early termination options.
