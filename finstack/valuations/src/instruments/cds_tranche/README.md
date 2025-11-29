# CDS Tranche

## Features
- Synthetic CDO tranche instrument with attachment/detachment points, IMM scheduling, and buy/sell protection support.
- Computes PV, par spread, upfront, spread DV01, expected loss, and jump-to-default via the tranche pricer.
- Supports accumulated loss input for seasoned tranches and optional standard IMM scheduling helpers.

## Methodology & References
- One-factor Gaussian copula base-correlation engine with Gauss–Hermite integration for tranche expected loss.
- Premium leg handles accrual-on-default and mid-period loss timing consistent with ISDA/CDX conventions.
- Correlation and hazard inputs sourced from `CreditIndexData` in `MarketContext`; deterministic recovery.

## Usage Example
```rust
use finstack_valuations::instruments::cds_tranche::CdsTranche;
use finstack_core::dates::Date;
use time::Month;

let as_of = Date::from_calendar_date(2024, Month::January, 5)?;
let tranche = CdsTranche::example();
let pv = tranche.value(&market_context, as_of)?;
let par = tranche.par_spread(&market_context, as_of)?;
```

## Limitations / Known Issues
- Assumes homogeneous portfolio (single hazard curve and recovery); no name-level correlation skews.
- Base-correlation surface must be supplied externally; no calibration in-module.
- No bespoke portfolio modeling (re-securitizations, stochastic recovery, or contagion effects).

## Pricing Methodology
- Base-correlation Gaussian copula: computes equity tranche EL curve, then derives [A,D] tranche EL via detachment/attachment differences.
- Protection/premium legs discounted on quote curve; accrual-on-default handled mid-period with Gauss–Hermite integration for accuracy near extreme correlations.
- Par spread solved via Brent using tranche RPV01; supports accumulated loss input and IMM scheduling.

## Metrics
- PV (buyer/seller), par spread, upfront, spread DV01, expected loss, jump-to-default, and correlation delta via finite differences.
- Premium vs protection leg PV breakdown; tranche notional outstanding profiles.
- Correlation sanity checks (clamping, monotonicity) exposed via diagnostics.

## Future Enhancements
- Add multi-factor copula and stochastic recovery options; base-correlation arbitrage checks and smoothing.
- Support bespoke portfolios and name-level heterogeneity (hazard/recovery per name).
- Provide tranche option (STO/CDO2) hooks and dynamic spread modeling for risk scenarios.
