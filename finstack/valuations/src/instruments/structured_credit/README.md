# Structured Credit

## Features
- Unified instrument for ABS/RMBS/CMBS/CLO deals built from components/config templates and a shared waterfall engine.
- Supports tranche definitions, collateral pools, triggers, fees, and schedule generation via `StructuredCredit` types and templates.
- Discounting pricer reuses generated cashflows; metrics leverage cached flows for duration/spread analytics.

## Methodology & References
- Cashflows produced by the waterfall engine implementing senior/sub waterfall logic, triggers, and coverage tests defined in `WaterfallSpec`.
- PV calculated by discounting projected deal cashflows on the selected discount curve; deterministic behavioral assumptions from inputs.
- Aligns with common structured-finance modeling practice; no embedded Monte Carlo unless provided by collateral/prepayment components.

## Usage Example
```rust
use finstack_valuations::instruments::structured_credit::StructuredCredit;

let deal = StructuredCredit::example();
let pv = deal.value(&market_context, as_of_date)?;
```

## Limitations / Known Issues
- Relies on provided collateral/default/prepayment assumptions; no endogenous calibration.
- Does not model secondary-market features such as step-up coupons or callable tranches beyond configured waterfall rules.
- Performance sensitive to waterfall specification completeness; missing triggers/defaults must be provided explicitly.

## Pricing Methodology
- Generates collateral cashflows per collateral models/assumptions, then applies deal waterfall (fees, triggers, OC/IC tests) to allocate to tranches.
- Discounts tranche cashflows on selected curve; deterministic assumptions unless collateral model includes stochastic elements.
- Supports schedule generation and trigger evaluation via shared waterfall engine for ABS/RMBS/CMBS/CLO templates.

## Metrics
- Tranche PVs, WAL, DM/OAS where configured, DV01/CS01 via cashflow-based bumping.
- Coverage test metrics (OC/IC ratios), expected loss/default speeds (if collateral model provides), and collateral/tranche cashflow breakdowns.
- Carry/roll and principal/interest split per period.

## Future Enhancements
- Add stochastic prepay/default models with scenario trees and correlation.
- Support base/curvature OAS grids and callable step-up tranches.
- Provide calibration helpers to market ABX/iTraxx/CMBX tranches and price-yield surfaces.
