# Margin, Collateral, and XVA

`finstack-margin` is the standalone home for Finstack's margin, collateral, and
XVA primitives. It separates agreement terms, margin engines, collateral
eligibility, registry-backed defaults, and exposure-adjustment logic from
`finstack-valuations` so consumer crates can reuse the same domain model without
pulling in the full instrument stack.

The crate covers three related workflows:

- OTC and repo margin specifications such as CSA terms, VM parameters, IM
  methodology, eligible collateral schedules, and repo maintenance rules.
- Calculation engines for variation margin, haircut-based repo IM, SIMM,
  BCBS-IOSCO schedule IM, CCP proxy IM, and margin-oriented metrics.
- XVA exposure profiling and valuation adjustments (CVA, DVA, FVA, bilateral
  XVA), with optional stochastic exposure under the `mc` feature.

## What This Crate Owns

### Domain types

- `CsaSpec`, `VmParameters`, `ImParameters`, `MarginCallTiming`
- `OtcMarginSpec` for bilateral and cleared OTC derivatives
- `RepoMarginSpec` and `RepoMarginType` for repo-style margin maintenance
- `EligibleCollateralSchedule`, `CollateralEligibility`, `CollateralAssetClass`
- `SimmSensitivities`, `SimmRiskClass`, `SimmCreditSector`
- `NettingSetId` and `InstrumentMarginResult`

### Engines and metrics

- `VmCalculator` for CSA-style variation margin
- `SimmCalculator` for ISDA SIMM
- `ScheduleImCalculator` for BCBS-IOSCO schedule IM
- `HaircutImCalculator` for repo / securities-financing style IM
- `ClearingHouseImCalculator` for CCP-specific cleared IM proxies or externally
  supplied CCP numbers
- `InternalModelImCalculator` as the extension point for internal-model IM
- `InitialMarginMetric`, `VariationMarginMetric`, `TotalMarginMetric`
- `MarginUtilization`, `ExcessCollateral`, `MarginFundingCost`, `Haircut01`

### Configuration and data

- Embedded registry loading via `registry::embedded_registry()`
- Config-driven overlays via `registry::margin_registry_from_config()`
- JSON schema for external margin specs and overlays

### XVA surface

- `xva::exposure::compute_exposure_profile`
- `xva::cva::{compute_cva, compute_dva, compute_fva, compute_bilateral_xva}`
- `xva::types::{XvaConfig, FundingConfig, NettingSet, CsaTerms, XvaResult}`
- `xva::traits::Valuable` for instrument-level XVA integration

## Design Goals

- Keep the margin API independent from `finstack-valuations`.
- Use registry-backed defaults instead of hard-coded regulatory parameters in
  constructors.
- Make core agreement terms serializable and schema-friendly.
- Support both direct calculator use and trait-driven integration from consumer
  crates.
- Stay explicit about simplified paths and placeholders rather than hiding them.

## Start Here

The typical integration flow is:

1. Attach an `OtcMarginSpec` or `RepoMarginSpec` to your instrument model.
2. Implement `Marginable` in the consumer crate so the margin engine can obtain
   an id, netting-set id, SIMM sensitivities, and current MTM.
3. Use `InitialMarginMetric`, `VariationMarginMetric`, or the lower-level
   calculators directly.
4. If you need XVA, implement `xva::Valuable` and compute an exposure profile
   before calling the CVA/DVA/FVA functions.
5. Override embedded defaults only when you have house-specific schedules,
   CCP parameters, or SIMM settings.

## Feature Flags

| Flag | Purpose |
|------|---------|
| `mc` | Enables stochastic XVA exposure simulation via `finstack-monte-carlo` in addition to the always-available deterministic exposure engine. |

Without `mc`, the crate still supports:

- all margin and collateral types
- all IM and VM calculators
- deterministic exposure profiles
- CVA, DVA, FVA, and bilateral XVA on deterministic exposures

## Module Map

| Module | Purpose |
|--------|---------|
| `calculators` | VM and IM engines |
| `config` | Thin wrapper for config-driven registry resolution |
| `constants` | Shared heuristics and constants used by the crate |
| `metrics` | Margin analytics and instrument-level metric adapters |
| `registry` | Embedded data, overlay merging, and typed registry resolution |
| `traits` | `Marginable` trait used by consumer crates |
| `types` | CSA, collateral, repo, SIMM, and netting types |
| `xva` | Exposure engines and valuation adjustments |

## Quick Examples

### 1. Build a standard bilateral OTC margin spec

```rust,no_run
use finstack_margin::{CsaSpec, OtcMarginSpec};

# fn main() -> finstack_core::Result<()> {
let csa = CsaSpec::usd_regulatory()?;
let spec = OtcMarginSpec::bilateral_simm(csa);

assert!(spec.csa.requires_im());
assert_eq!(spec.vm_frequency.to_string(), "daily");
# Ok(())
# }
```

### 2. Calculate SIMM directly from sensitivities

```rust,no_run
use finstack_core::currency::Currency;
use finstack_margin::{SimmCalculator, SimmSensitivities, SimmVersion};

# fn main() -> finstack_core::Result<()> {
let calc = SimmCalculator::new(SimmVersion::V2_6)?;

let mut sensitivities = SimmSensitivities::new(Currency::USD);
sensitivities.add_ir_delta(Currency::USD, "5y", 50_000.0);
sensitivities.add_equity_delta("AAPL", 100_000.0);

let (total_im, breakdown) = calc.calculate_from_sensitivities(&sensitivities, Currency::USD);

println!("total_im={total_im:.2}");
println!("risk classes={}", breakdown.len());
# Ok(())
# }
```

### 3. Calculate variation margin from exposure and posted collateral

```rust,no_run
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_margin::{CsaSpec, VmCalculator};
use time::Date;
use time::Month;

# fn main() -> finstack_core::Result<()> {
let csa = CsaSpec::usd_regulatory()?;
let calc = VmCalculator::new(csa);

let exposure = Money::new(5_000_000.0, Currency::USD);
let posted = Money::new(3_000_000.0, Currency::USD);
let as_of = Date::from_calendar_date(2025, Month::January, 15)?;

let result = calc.calculate(exposure, posted, as_of)?;

println!("delivery={}", result.delivery_amount);
println!("return={}", result.return_amount);
println!("settlement={}", result.settlement_date);
# Ok(())
# }
```

### 4. Dispatch instrument-level IM and VM via `Marginable`

```rust,no_run
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_margin::{
    InitialMarginMetric, Marginable, OtcMarginSpec, SimmSensitivities, VariationMarginMetric,
};
use time::Month;

struct ExampleTrade {
    id: String,
    spec: OtcMarginSpec,
    mtm: Money,
    sensitivities: SimmSensitivities,
}

impl Marginable for ExampleTrade {
    fn id(&self) -> &str { &self.id }
    fn margin_spec(&self) -> Option<&OtcMarginSpec> { Some(&self.spec) }
    fn netting_set_id(&self) -> Option<finstack_margin::NettingSetId> { None }
    fn simm_sensitivities(
        &self,
        _market: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<SimmSensitivities> {
        Ok(self.sensitivities.clone())
    }
    fn mtm_for_vm(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        Ok(self.mtm)
    }
}

# fn main() -> finstack_core::Result<()> {
let spec = OtcMarginSpec::usd_bilateral()?;
let mut sensitivities = SimmSensitivities::new(Currency::USD);
sensitivities.add_ir_delta(Currency::USD, "5y", 25_000.0);

let trade = ExampleTrade {
    id: "SWAP-001".to_string(),
    spec,
    mtm: Money::new(1_000_000.0, Currency::USD),
    sensitivities,
};

let market = MarketContext::new();
let as_of = Date::from_calendar_date(2025, Month::January, 15)?;

let im = InitialMarginMetric::new().calculate(&trade, &market, as_of)?;
let vm = VariationMarginMetric::new().calculate(&trade, &market, as_of)?;

println!("IM={}", im.amount);
println!("VM delivery={}", vm.delivery_amount);
# Ok(())
# }
```

### 5. Run a deterministic XVA flow

```rust,no_run
use std::sync::Arc;

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_margin::xva::{
    cva::compute_bilateral_xva,
    exposure::compute_exposure_profile,
    traits::Valuable,
    types::{FundingConfig, NettingSet, XvaConfig},
};
use time::Month;

struct XvaTrade;

impl Valuable for XvaTrade {
    fn id(&self) -> &str { "XVA-TRADE-001" }
    fn value(&self, _market: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        todo!("bridge to a consumer crate's valuation engine")
    }
}

# fn main() -> finstack_core::Result<()> {
# let market = MarketContext::new();
# let cpty_hazard = todo!("provide counterparty hazard curve");
# let own_hazard = todo!("provide own hazard curve");
# let discount = todo!("provide discount curve");
let config = XvaConfig {
    funding: Some(FundingConfig {
        funding_spread_bps: 50.0,
        funding_benefit_bps: Some(25.0),
    }),
    ..XvaConfig::default()
};

let netting_set = NettingSet {
    id: "NS-001".into(),
    counterparty_id: "CP-001".into(),
    csa: None,
    reporting_currency: None,
};

let as_of = Date::from_calendar_date(2025, Month::January, 15)?;
let instruments: Vec<Arc<dyn Valuable>> = vec![Arc::new(XvaTrade)];
let exposure = compute_exposure_profile(&instruments, &market, as_of, &config, &netting_set)?;
let xva = compute_bilateral_xva(
    &exposure,
    &cpty_hazard,
    &own_hazard,
    &discount,
    config.recovery_rate,
    config.own_recovery_rate.unwrap_or(config.recovery_rate),
    config.funding.as_ref(),
)?;

println!("CVA={}", xva.cva);
println!("DVA={:?}", xva.dva);
println!("FVA={:?}", xva.fva);
# Ok(())
# }
```

## Margin Conventions

- Rates, spreads, and haircuts are stored as decimal fractions, not basis
  points, unless a field explicitly says otherwise.
- `VmParameters` and `ImParameters` store thresholds, MTAs, and independent
  amounts as `Money`.
- `VmParameters::calculate_margin_call` uses a symmetric threshold around signed
  exposure, then applies rounding and MTA to the transfer amount.
- `Marginable::simm_sensitivities` expects currency-valued risk measures such as
  DV01/CS01-style inputs, not raw quote moves.
- `NettingSetId` is the unit of aggregation for portfolio-level margining in
  consumer crates such as `finstack-portfolio`.

## Registry and Embedded Data

The crate ships with embedded JSON assets and exposes a typed, resolved registry
through `registry::embedded_registry()`.

| File | Purpose |
|------|---------|
| `data/margin/defaults.v1.json` | Default VM, IM, timing, and settlement terms |
| `data/margin/schedule_im.v1.json` | Schedule IM grids such as `bcbs_iosco` |
| `data/margin/collateral_schedules.v1.json` | Eligible collateral schedules and haircuts |
| `data/margin/ccp_methodologies.v1.json` | CCP proxy parameters such as conservative rates and MPOR |
| `data/margin/simm.v1.json` | Registry-backed SIMM weights, correlations, and concentration thresholds |
| `schemas/margin/1/margin.schema.json` | Schema for external margin specifications and related JSON payloads |

Use `registry::margin_registry_from_config()` when you need to apply a JSON
overlay from `FinstackConfig`. The extension key is
`valuations.margin_registry.v1`.

Illustrative overlay shape:

```json
{
  "extensions": {
    "valuations.margin_registry.v1": {
      "defaults": {
        "vm": {
          "threshold": 0.0,
          "mta": 250000.0
        }
      }
    }
  }
}
```

The full accepted surface is defined by `schemas/margin/1/margin.schema.json`
plus the typed parsing and validation in `registry`.

## Calculator Semantics

### Variation margin

- `VmCalculator` is the most literal implementation in the crate today.
- It validates currency consistency, applies CSA threshold / MTA / rounding
  rules, and computes settlement dates using a currency-appropriate holiday
  calendar where possible.

### SIMM

- `SimmCalculator` is the primary bilateral IM engine for OTC derivatives.
- Versioning is explicit through `SimmVersion`.
- Risk weights, tenor correlations, risk-class correlations, and concentration
  thresholds come from the registry rather than being duplicated in code.

### Schedule IM

- `ScheduleImCalculator::calculate_for_notional(...)` is the closest path to the
  regulatory schedule formula.
- The trait-driven `ImCalculator::calculate(...)` fallback uses
  `instrument.mtm_for_vm(...).abs()` as a proxy exposure base because
  `Marginable` does not currently expose regulatory notional.

### Cleared IM

- `ClearingHouseImCalculator` can consume external CCP margin values through
  `CcpMarginInputSource`.
- Without an external source, the built-in fallback is a conservative proxy:
  absolute current MTM times a registry-backed CCP rate.

### Repo IM

- `HaircutImCalculator` works with repo-style collateral schedules and repo
  margin specs.
- `RepoMarginSpec` covers no-margin, mark-to-market, net-exposure, and
  triparty-style workflows.

## XVA Model Scope

The always-available XVA engine is deterministic:

- markets are rolled forward on a constant-curves basis
- instruments are revalued on each future horizon
- close-out netting and CSA collateral reduction are applied
- EPE, ENE, effective EPE, and PFE-shaped outputs are produced

Under deterministic exposure:

- `PFE` equals `EPE`
- explicit wrong-way risk is not modeled
- margin period of risk is not modeled directly
- carry / theta / future market scenarios are not simulated

With the `mc` feature enabled, the crate also exposes stochastic exposure types
and Monte Carlo-based exposure simulation hooks via `finstack-monte-carlo`.

## Known Limits

- `ScheduleImCalculator` and `ClearingHouseImCalculator` use MTM-based proxy
  exposures when invoked through the generic `Marginable` interface.
- `InternalModelImCalculator` is an extension point, not a production internal
  VaR / ES implementation.
- `Marginable` and `xva::Valuable` are intentionally narrow traits; full
  instrument behavior remains in consumer crates.
- The deterministic XVA engine is useful for baseline analytics and integration,
  but it is not a replacement for a full stochastic exposure stack.

## Verification

```bash
cargo test -p finstack-margin
```

If you are working on XVA with Monte Carlo enabled:

```bash
cargo test -p finstack-margin --features mc
```

## References

- [ISDA SIMM](../../docs/REFERENCES.md#isda-simm)
- [ISDA 2016 VM CSA](../../docs/REFERENCES.md#isda-vm-csa-2016)
- [ISDA 2018 IM CSA](../../docs/REFERENCES.md#isda-im-csa-2018)
- [BCBS-IOSCO uncleared margin framework](../../docs/REFERENCES.md#bcbs-iosco-uncleared-margin)
- [Gregory XVA Challenge](../../docs/REFERENCES.md#gregory-xva-challenge)
- [Green XVA](../../docs/REFERENCES.md#green-xva)
- [BCBS 279 SA-CCR](../../docs/REFERENCES.md#bcbs-279-saccr)
