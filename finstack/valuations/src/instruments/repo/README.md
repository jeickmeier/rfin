# Repurchase Agreement (Repo)

## Features
- Term, open, or overnight repos with configurable repo rate, haircut, collateral spec (general vs special), and cashflow schedule.
- Supports explicit collateral notional, haircut application, and optional special-rate adjustments for specials.
- Uses generic discounting pricer for PV of borrow vs. repayment cashflows with curve-driven discounting.

## Methodology & References
- PV computed from deterministic initial cash outflow and discounted repurchase cash inflow; haircut adjusts collateral requirement.
- Collateral type flag allows special-rate adjustment; otherwise treats trade as general collateral.
- Aligns with standard money-market repo conventions; no tri-party or margining simulation.

## Usage Example
```rust
use finstack_valuations::instruments::repo::{CollateralSpec, CollateralType, Repo, RepoType};
use finstack_core::{currency::Currency, dates::Date, money::Money, types::CurveId};
use time::Month;

let repo = Repo::builder()
    .id("REPO-1")
    .repo_type(RepoType::Term)
    .collateral(CollateralSpec::general())
    .cash_amount(Money::new(10_000_000.0, Currency::USD))
    .start(Date::from_calendar_date(2024, Month::January, 3)?)
    .end(Date::from_calendar_date(2024, Month::February, 3)?)
    .rate(0.0525)
    .haircut(0.02)
    .discount_curve_id(CurveId::new("USD-OIS"))
    .attributes(Default::default())
    .build()?;

let pv = repo.value(&market_context, Date::from_calendar_date(2024, Month::January, 3)?)?;
```

## Limitations / Known Issues
- No margining, fails-to-deliver, or collateral substitution logic; deterministic single-period cashflows only.
- Haircut applied statically; rehypothecation and reinvestment income not modeled.
- Currency/funding bases handled only through supplied discount curve.

## Pricing Methodology
- Two-leg cashflow model: cash out at start, cash back at end with repo rate applied; haircut determines required collateral.
- Discounts leg cashflows using chosen discount curve; collateral type may adjust effective rate if special.
- Deterministic term/open/overnight handling based on start/end/notice settings.

## Metrics
- PV, implied repo rate (solve for rate matching price), and haircut-adjusted collateral requirement.
- DV01 on discount curve via generic bump calculators.
- Carry/roll to settlement via day-count accrual of repo interest.

## Future Enhancements
- Add margining and variation/initial margin cashflow modeling.
- Support triparty eligibility schedules and collateral substitution events.
- Include fail/recall penalties and optional early termination features.
