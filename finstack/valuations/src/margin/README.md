# Margin & Collateral Management

A comprehensive margin and collateral management system for financial instruments, implementing industry-standard methodologies following ISDA, BCBS-IOSCO, GMRA, and clearing house standards.

## Overview

Margining is the process of exchanging collateral to mitigate counterparty credit risk in financial transactions. This module provides:

- **Variation Margin (VM)**: Daily mark-to-market payments to eliminate counterparty exposure
- **Initial Margin (IM)**: Collateral to cover potential future exposure during close-out
- **Collateral Management**: Eligible collateral schedules, haircuts, and substitution rules
- **Margin Metrics**: Utilization, excess collateral, funding cost, and sensitivity analysis

## Regulatory Framework

| Standard | Scope | Key Requirements |
|----------|-------|------------------|
| **BCBS-IOSCO** | Bilateral OTC derivatives | VM/IM requirements, eligible collateral, haircuts |
| **ISDA SIMM** | Initial margin calculation | Standardized sensitivities-based IM (v2.5/v2.6) |
| **GMRA 2011** | Repos | Margin maintenance, substitution, haircuts |
| **EMIR/Dodd-Frank** | Cleared & uncleared | Daily VM, IM for uncleared derivatives |

## Module Structure

```
margin/
├── mod.rs                    # Module root and re-exports
├── traits.rs                 # Marginable trait, SIMM sensitivities, netting sets
├── constants.rs              # Financial constants (day counts, basis points)
├── impls.rs                  # Marginable implementations for instruments
├── types/                    # Core type definitions
│   ├── csa.rs               # Credit Support Annex specification
│   ├── collateral.rs        # Collateral eligibility and haircuts
│   ├── thresholds.rs        # VM/IM parameters (threshold, MTA)
│   ├── enums.rs             # MarginFrequency, ImMethodology, ClearingStatus
│   ├── otc.rs               # OTC derivative margin specification
│   └── call.rs              # Margin call event types
├── calculators/              # Margin calculation engines
│   ├── vm.rs                # Variation margin calculator
│   ├── traits.rs            # ImCalculator trait and ImResult
│   └── im/                  # Initial margin calculators
│       ├── simm.rs          # ISDA SIMM implementation
│       ├── schedule.rs      # BCBS-IOSCO regulatory schedule
│       ├── haircut.rs       # Haircut-based IM for repos
│       └── clearing.rs      # CCP-specific methodologies
└── metrics/                  # Margin-specific metrics
    ├── mod.rs               # Utilization, excess, funding cost
    └── instrument.rs        # Instrument-level IM/VM metrics
```

## Configuration (registry-backed)

- Embedded JSON registries live under `finstack/valuations/data/margin/`:
  - `schedule_im.v1.json` — BCBS-IOSCO schedule IM rates
  - `collateral_schedules.v1.json` — asset-class defaults and named schedules
  - `defaults.v1.json` — VM/IM thresholds, timing, cleared settlement rounding
  - `ccp_methodologies.v1.json` — CCP MPOR and conservative rates
  - `simm.v1.json` — SIMM weights, correlations, and commodity buckets
- Runtime overrides: set `FinstackConfig.extensions["valuations.margin_registry.v1"]` to a JSON object (merged over embedded defaults). All sections are optional; provide only the keys you need (`schedule_im`, `collateral_schedules`, `defaults`, `ccp`, `simm`).

## Core Types

### Credit Support Annex (CsaSpec)

The CSA governs collateral exchange between counterparties under ISDA documentation:

```rust
use finstack_valuations::margin::{
    CsaSpec, VmParameters, ImParameters, EligibleCollateralSchedule,
    MarginCallTiming, ImMethodology, MarginFrequency,
};
use finstack_core::currency::Currency;
use finstack_core::money::Money;

// Standard regulatory CSA (post-2016)
let csa = CsaSpec::usd_regulatory();

// Custom CSA (e.g., bilateral thresholds)
let custom_csa = CsaSpec {
    id: "CUSTOM-CSA-2024".to_string(),
    base_currency: Currency::USD,
    vm_params: VmParameters {
        threshold: Money::new(1_000_000.0, Currency::USD),
        mta: Money::new(100_000.0, Currency::USD),
        rounding: Money::new(10_000.0, Currency::USD),
        independent_amount: Money::zero(Currency::USD),
        frequency: MarginFrequency::Daily,
        settlement_lag: 1,
    },
    im_params: Some(ImParameters::simm_standard(Currency::USD)),
    eligible_collateral: EligibleCollateralSchedule::bcbs_standard(),
    call_timing: MarginCallTiming::regulatory_standard(),
    collateral_curve_id: "USD-OIS".into(),
};
```

### OTC Margin Specification

Attach margin specifications to OTC derivatives:

```rust
use finstack_valuations::margin::{OtcMarginSpec, CsaSpec, ClearingStatus};

// Bilateral trade with SIMM
let bilateral = OtcMarginSpec::bilateral_simm(CsaSpec::usd_regulatory());

// Cleared through LCH
let cleared = OtcMarginSpec::lch_swapclear(Currency::USD);

// ICE Clear Credit for CDS
let ice_cleared = OtcMarginSpec::ice_clear_credit();
```

### Eligible Collateral Schedules

Define what collateral types are acceptable and associated haircuts:

```rust
use finstack_valuations::margin::{
    EligibleCollateralSchedule, CollateralEligibility, CollateralAssetClass,
    MaturityConstraints,
};

// BCBS-IOSCO compliant schedule
let schedule = EligibleCollateralSchedule::bcbs_standard();

// Cash only
let cash_only = EligibleCollateralSchedule::cash_only();

// US Treasuries (for repos)
let treasuries = EligibleCollateralSchedule::us_treasuries();

// Check eligibility and haircuts
let haircut = schedule.haircut_for(CollateralAssetClass::GovernmentBonds);
let is_eligible = schedule.is_eligible(CollateralAssetClass::Cash);
```

#### Collateral Asset Classes and Standard Haircuts

| Asset Class | Standard Haircut | FX Addon | Notes |
|-------------|-----------------|----------|-------|
| Cash | 0% | 8% | Zero haircut for same currency |
| Government Bonds (≤1yr) | 0.5% | 8% | G10 sovereigns |
| Government Bonds (1-5yr) | 2% | 8% | |
| Government Bonds (>5yr) | 4% | 8% | |
| Agency Bonds | 3% | 8% | Supranational included |
| Covered Bonds | 4% | 8% | Meeting eligibility criteria |
| Corporate Bonds (IG) | 2-8% | 8% | By maturity |
| Equity | 15% | 8% | Major indices only |
| Gold | 15% | 8% | Bullion |

## Calculators

### Variation Margin Calculator

Calculates VM based on mark-to-market exposure applying ISDA CSA rules:

```rust
use finstack_valuations::margin::{VmCalculator, CsaSpec, VmResult};
use finstack_core::money::Money;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;

let csa = CsaSpec::usd_regulatory();
let calc = VmCalculator::new(csa);

// Single calculation
let exposure = Money::new(5_000_000.0, Currency::USD);
let posted = Money::new(3_000_000.0, Currency::USD);
let as_of = Date::from_calendar_date(2025, time::Month::January, 15)?;

let result: VmResult = calc.calculate(exposure, posted, as_of)?;
println!("Delivery required: {}", result.delivery_amount);
println!("Return amount: {}", result.return_amount);

// Generate margin call series
let exposures = vec![
    (date1, Money::new(1_000_000.0, Currency::USD)),
    (date2, Money::new(2_000_000.0, Currency::USD)),
    (date3, Money::new(1_500_000.0, Currency::USD)),
];
let calls = calc.generate_margin_calls(&exposures, Money::zero(Currency::USD))?;
```

#### VM Calculation Formula

```
Credit Support Amount = max(0, Exposure - Threshold + IA) - Current_Collateral
Delivery Amount = max(0, CSA) if CSA ≥ MTA, else 0
Return Amount = max(0, -CSA) if |CSA| ≥ MTA, else 0
```

### Initial Margin Calculators

#### ISDA SIMM Calculator

Sensitivities-based IM calculation following ISDA SIMM methodology (v2.5/v2.6):

```rust
use finstack_valuations::margin::{SimmCalculator, SimmVersion, SimmSensitivities};
use std::collections::HashMap;

// Create calculator
let calc = SimmCalculator::new(SimmVersion::V2_6);

// Calculate from DV01 sensitivities
let dv01_by_tenor = HashMap::from([
    ("2Y".to_string(), 15_000.0),
    ("5Y".to_string(), 45_000.0),
    ("10Y".to_string(), 25_000.0),
]);
let ir_margin = calc.calculate_ir_delta(&dv01_by_tenor);

// Calculate from SIMM sensitivities struct
let mut sens = SimmSensitivities::new(Currency::USD);
sens.add_ir_delta(Currency::USD, "5Y", 45_000.0);
sens.add_credit_delta("CDX.NA.IG", true, "5Y", 50_000.0);

let (total_im, breakdown) = calc.calculate_from_sensitivities(&sens, Currency::USD);
```

> **Implementation note:** `calculate_from_sensitivities` applies the SIMM
> risk-class correlation matrix (delta-only). Bucket/tenor correlations,
> vega, and curvature aggregation are still simplified.

##### SIMM Risk Classes

| Risk Class | Components | Notes |
|------------|------------|-------|
| Interest Rate | Delta, Vega, Curvature | By currency and tenor |
| Credit Qualifying | Delta, Vega | Investment grade |
| Credit Non-Qualifying | Delta, Vega | High yield, EM |
| Equity | Delta, Vega, Curvature | By underlier |
| Commodity | Delta, Vega, Curvature | By bucket |
| FX | Delta, Vega | By currency pair |

##### SIMM v2.6 Risk Weights (Selected)

| Tenor | IR Weight | Credit IG Weight | Credit HY Weight | Equity Weight |
|-------|-----------|------------------|------------------|---------------|
| 2W | 109 | - | - | - |
| 1Y | 61 | - | - | - |
| 5Y | 51 | 73 (corporates) | 500 | 32 |
| 10Y | 51 | - | - | - |

#### BCBS-IOSCO Schedule Calculator

Grid-based IM calculation (simpler but more conservative):

```rust
use finstack_valuations::margin::{ScheduleImCalculator, ScheduleAssetClass};

let calc = ScheduleImCalculator::bcbs_standard()?
    .with_asset_class(ScheduleAssetClass::InterestRate)
    .with_maturity(5.0);

let notional = Money::new(100_000_000.0, Currency::USD);
let im = calc.calculate_for_notional(notional, ScheduleAssetClass::InterestRate, 5.0);
```

##### Schedule Rates by Asset Class

| Asset Class | <2 Years | 2-5 Years | >5 Years |
|-------------|----------|-----------|----------|
| Interest Rate | 1% | 2% | 4% |
| Credit | 2% | 5% | 10% |
| Equity | 15% | 15% | 15% |
| Commodity | 15% | 15% | 15% |
| FX | 6% | 6% | 6% |

#### Haircut-Based Calculator

For repos and securities financing:

```rust
use finstack_valuations::margin::{HaircutImCalculator, CollateralAssetClass};

let calc = HaircutImCalculator::us_treasuries()
    .with_fx_addon(true);

let collateral = Money::new(100_000_000.0, Currency::USD);
let im = calc.calculate_for_collateral(
    collateral,
    CollateralAssetClass::GovernmentBonds,
    false,  // same currency
)?;
```

#### Clearing House Calculator

CCP-specific methodologies:

```rust
use finstack_valuations::margin::{ClearingHouseImCalculator, CcpMethodology};

// Specific CCPs
let lch = ClearingHouseImCalculator::lch_swapclear();
let ice = ClearingHouseImCalculator::ice_clear_credit();
let cme = ClearingHouseImCalculator::cme();

// Generic VaR-based
let var_calc = ClearingHouseImCalculator::generic_var(0.99, 250);
```

Provide external VaR/SPAN outputs via a `CcpMarginInputSource`:

```rust
use finstack_valuations::margin::{CcpMarginInputSource, ClearingHouseImCalculator};
use finstack_core::money::Money;
use std::sync::Arc;

struct MyCcpInputs;

impl CcpMarginInputSource for MyCcpInputs {
    fn initial_margin(
        &self,
        _instrument: &dyn finstack_valuations::instruments::Instrument,
        _context: &finstack_core::market_data::context::MarketContext,
        _as_of: finstack_core::dates::Date,
        _methodology: &finstack_valuations::margin::CcpMethodology,
    ) -> Option<Money> {
        Some(Money::new(2_500_000.0, finstack_core::currency::Currency::USD))
    }
}

let calc = ClearingHouseImCalculator::lch_swapclear()
    .with_input_source(Arc::new(MyCcpInputs));
```

#### Internal Model Calculator

Internal model (VaR/ES) stub with conservative fallback:

```rust
use finstack_valuations::margin::InternalModelImCalculator;

let calc = InternalModelImCalculator::new().with_conservative_rate(0.05);
```

## Implementing Marginable for Instruments

Instruments that support margin calculations must implement the `Marginable` trait:

```rust
use finstack_valuations::margin::{Marginable, NettingSetId, SimmSensitivities, OtcMarginSpec};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::dates::Date;

impl Marginable for MyInstrument {
    fn margin_spec(&self) -> Option<&OtcMarginSpec> {
        self.margin_spec.as_ref()
    }

    fn netting_set_id(&self) -> Option<NettingSetId> {
        self.margin_spec.as_ref().map(|s| {
            match &s.clearing_status {
                ClearingStatus::Cleared { ccp } => NettingSetId::cleared(ccp),
                ClearingStatus::Bilateral =>
                    NettingSetId::bilateral(&s.csa.id, &s.csa.id),
            }
        })
    }

    fn simm_sensitivities(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SimmSensitivities> {
        let mut sens = SimmSensitivities::new(self.currency);

        // Calculate and add sensitivities
        let dv01 = self.calculate_dv01(market, as_of)?;
        sens.add_ir_delta(self.currency, "5Y", dv01);

        Ok(sens)
    }

    fn mtm_for_vm(&self, market: &MarketContext, as_of: Date) -> Result<Money> {
        self.npv(market, as_of)
    }
}
```

### Currently Supported Instruments

The following instruments have built-in `Marginable` implementations:

- `InterestRateSwap` - IR delta sensitivities
- `CreditDefaultSwap` - Credit delta sensitivities (CS01)
- `CDSIndex` - Credit index sensitivities
- `EquityTotalReturnSwap` - Equity delta sensitivities
- `FIIndexTotalReturnSwap` - IR delta (duration-based)
- `Repo` - Short-term IR sensitivity

## Margin Metrics

### Instrument-Level Metrics

```rust
use finstack_valuations::margin::metrics::{
    InitialMarginMetric, VariationMarginMetric, TotalMarginMetric,
    calculate_instrument_margins,
};

let market = MarketContext::new();
let as_of = Date::today();

// Individual metrics
let im_metric = InitialMarginMetric::new();
let im = im_metric.calculate(&swap, &market, as_of)?;

let vm_metric = VariationMarginMetric::new()
    .with_posted(Money::new(1_000_000.0, Currency::USD));
let vm = vm_metric.calculate(&swap, &market, as_of)?;

// Combined IM + VM
let total_metric = TotalMarginMetric::new();
let result = total_metric.calculate(&swap, &market, as_of)?;

// Batch calculation
let results = calculate_instrument_margins(
    instruments.iter(),
    &market,
    as_of,
);
```

### Margin Analysis Metrics

```rust
use finstack_valuations::margin::metrics::{
    MarginUtilization, ExcessCollateral, MarginFundingCost, Haircut01,
};

// Utilization ratio
let util = MarginUtilization::new(
    Money::new(12_000_000.0, Currency::USD),  // posted
    Money::new(10_000_000.0, Currency::USD),  // required
);
assert!(util.is_adequate());
println!("Utilization: {:.1}%", util.ratio * 100.0);

// Excess collateral
let excess = ExcessCollateral::new(
    Money::new(105_000_000.0, Currency::USD),
    Money::new(100_000_000.0, Currency::USD),
);
println!("Excess: {} ({:.1}%)", excess.excess, excess.excess_percentage() * 100.0);

// Funding cost
let cost = MarginFundingCost::calculate(
    Money::new(50_000_000.0, Currency::USD),
    0.055,  // funding rate
    0.053,  // collateral return rate
);
println!("Annual funding cost: {}", cost.annual_cost);

// Haircut sensitivity
let h01 = Haircut01::calculate(
    Money::new(100_000_000.0, Currency::USD),
    0.02,  // current haircut
);
println!("Haircut01: {}", h01.pv_change);
```

## Adding New Features

### Adding a New IM Calculator

1. Create a new file in `calculators/im/`:

```rust
// calculators/im/my_methodology.rs
use crate::margin::calculators::traits::{ImCalculator, ImResult};
use crate::margin::types::ImMethodology;

pub struct MyMethodologyCalculator {
    // Configuration fields
}

impl ImCalculator for MyMethodologyCalculator {
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Your implementation
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::InternalModel  // or add new variant
    }
}
```

1. Add to `calculators/im/mod.rs`:

```rust
mod my_methodology;
pub use my_methodology::MyMethodologyCalculator;
```

1. Update `calculators/mod.rs` to re-export.

### Adding a New Collateral Asset Class

1. Add variant to `CollateralAssetClass` in `types/collateral.rs`:

```rust
pub enum CollateralAssetClass {
    // ... existing variants
    CryptoCurrency,  // New
}
```

1. Implement `standard_haircut()` and `fx_addon()` for the new variant.

2. Update `FromStr` implementation for parsing.

### Adding a New CCP

1. Add variant to `CcpMethodology` in `calculators/im/clearing.rs`:

```rust
pub enum CcpMethodology {
    // ... existing variants
    NewCcp,
}
```

1. Implement `mpor_days()` and `conservative_rate()` for the new CCP.

2. Add convenience constructor:

```rust
impl ClearingHouseImCalculator {
    pub fn new_ccp() -> Self {
        Self::new(CcpMethodology::NewCcp)
    }
}
```

## Limitations & Future Enhancements

### Current Limitations

1. **SIMM Implementation**: Risk-class correlations implemented; bucket/tenor correlations, vega, and curvature remain simplified
2. **CCP Calculators**: Use conservative estimates unless external VaR/SPAN inputs are supplied
3. **Calendar Integration**: Settlement dates use simple day addition instead of business day calendars
4. **Wrong-Way Risk**: Not currently modeled in IM calculations
5. **Dynamic IM**: No real-time VaR-based IM calculation
6. **Portfolio Margining**: Limited netting set aggregation

### Planned Enhancements

1. **Full SIMM Bucket Correlations**: Complete ISDA SIMM implementation with bucket/tenor correlations, vega, and curvature
2. **CCP API Integration**: Interface with LCH SMART, CME CORE for actual margin calculations
3. **MVA (Margin Valuation Adjustment)**: Calculate funding cost of future margin requirements
4. **Dynamic IM**: Historical simulation and Monte Carlo for VaR/ES-based IM
5. **XVA Integration**: Link margin calculations with CVA/DVA/FVA framework
6. **Regulatory Reporting**: Generate EMIR/Dodd-Frank margin reports
7. **Collateral Optimization**: Optimal allocation of collateral across agreements
8. **Real-Time Monitoring**: Margin utilization alerts and breach detection

## References

### Regulatory Documents

- [BCBS-IOSCO Margin Requirements (2020)](https://www.bis.org/bcbs/publ/d499.pdf) - Margin requirements for non-centrally cleared derivatives
- [ISDA 2016 VM CSA](https://www.isda.org/) - Credit Support Annex for Variation Margin
- [ISDA 2018 IM CSA](https://www.isda.org/) - Credit Support Annex for Initial Margin
- [ISDA SIMM Methodology v2.6](https://www.isda.org/2023/12/04/isda-simm-v2-6/) - Standard Initial Margin Model

### Industry Documentation

- GMRA 2011 - Global Master Repurchase Agreement
- EMIR RTS - European Market Infrastructure Regulation technical standards
- Dodd-Frank Title VII - US derivatives regulation

## Testing

Run margin-specific tests:

```bash
# Unit tests
cargo test -p finstack-valuations margin::

# Integration tests
cargo test -p finstack-valuations --test margin_integration

# Specific calculator tests
cargo test -p finstack-valuations margin::calculators::simm
cargo test -p finstack-valuations margin::calculators::vm
```
