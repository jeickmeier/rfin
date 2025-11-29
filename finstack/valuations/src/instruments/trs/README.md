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

---

## Margining

Total return swaps implement full margin support following **ISDA CSA** standards. TRS instruments are categorized by their underlying asset type for SIMM risk classification.

### Risk Classification by TRS Type

| TRS Type | SIMM Risk Class | Sensitivity Type |
|----------|-----------------|------------------|
| **EquityTotalReturnSwap** | Equity | Equity delta (100% of notional) |
| **FIIndexTotalReturnSwap** | Interest Rate | IR delta (based on index duration) |

### Adding Margin Specification

```rust
use finstack_valuations::instruments::trs::equity::EquityTotalReturnSwap;
use finstack_valuations::instruments::trs::fixed_income_index::FIIndexTotalReturnSwap;
use finstack_valuations::margin::{
    OtcMarginSpec, CsaSpec, ClearingStatus, ImMethodology, MarginFrequency,
};

// Equity TRS with margin spec
let mut equity_trs = EquityTotalReturnSwap::example();
equity_trs.margin_spec = Some(OtcMarginSpec {
    csa: CsaSpec::usd_regulatory(),
    clearing_status: ClearingStatus::Bilateral,
    im_methodology: ImMethodology::Simm,
    vm_frequency: MarginFrequency::Daily,
    settlement_lag: 1,
});

// Fixed Income Index TRS with margin spec
let mut fi_trs = FIIndexTotalReturnSwap::example();
fi_trs.margin_spec = Some(OtcMarginSpec {
    csa: CsaSpec::usd_regulatory(),
    clearing_status: ClearingStatus::Bilateral,
    im_methodology: ImMethodology::Simm,
    vm_frequency: MarginFrequency::Daily,
    settlement_lag: 1,
});
```

### SIMM Sensitivities by TRS Type

#### Equity TRS

Equity TRS produce **Equity Delta** sensitivities based on the underlying ticker:

```rust
use finstack_valuations::margin::{Marginable, SimmSensitivities};

let trs = EquityTotalReturnSwap::example();
let sensitivities = trs.simm_sensitivities(&market, as_of)?;

// Equity delta = notional × direction
// ReceiveTotalReturn = long equity exposure
// PayTotalReturn = short equity exposure
for (underlier, delta) in &sensitivities.equity_delta {
    println!("{}: ${:.2}", underlier, delta);
}
```

#### Fixed Income Index TRS

FI Index TRS produce **Interest Rate Delta** sensitivities based on estimated index duration:

```rust
let fi_trs = FIIndexTotalReturnSwap::example();
let sensitivities = fi_trs.simm_sensitivities(&market, as_of)?;

// IR delta assigned to tenor bucket based on index duration
// Default duration estimate: ~6 years for broad bond indices
for ((currency, tenor), delta) in &sensitivities.ir_delta {
    println!("{} {}: ${:.2}", currency, tenor, delta);
}
```

### Calculating Margin Requirements

```rust
use finstack_valuations::margin::metrics::{
    InitialMarginMetric, VariationMarginMetric, TotalMarginMetric,
};

// Calculate margins for equity TRS
let trs = EquityTotalReturnSwap::example();
let metric = TotalMarginMetric::new();
let result = metric.calculate(&trs, &market, as_of)?;

println!("Initial Margin: {}", result.initial_margin);
println!("Variation Margin: {}", result.variation_margin);
println!("Total Margin: {}", result.total_margin);
println!("IM Methodology: {:?}", result.im_methodology);

// Check netting set for aggregation
if let Some(netting_set) = result.netting_set {
    println!("Netting Set: {}", netting_set);
}
```

### Using Marginable Trait

Both TRS types implement the `Marginable` trait:

```rust
use finstack_valuations::margin::Marginable;

// Works for both EquityTotalReturnSwap and FIIndexTotalReturnSwap
fn process_trs_margin<T: Marginable>(trs: &T, market: &MarketContext, as_of: Date) {
    if trs.has_margin() {
        let sensitivities = trs.simm_sensitivities(market, as_of).unwrap();
        let mtm = trs.mtm_for_vm(market, as_of).unwrap();
        
        println!("MTM: {}", mtm);
        println!("Total Equity Delta: ${:.2}", sensitivities.total_equity_delta());
        println!("Total IR Delta: ${:.2}", sensitivities.total_ir_delta());
    }
}
```

---

## Limitations / Known Issues
- Total-return path is deterministic from supplied prices/yields; no simulation of underlying index volatility.
- Does not model early termination, resettable notionals, or bespoke fee structures beyond leg specs.
- FI Index TRS duration is estimated; actual index duration from market data would improve SIMM accuracy.

## Pricing Methodology
- Builds total-return leg using underlying price/return plus income; financing leg uses floating index plus spread with schedule params.
- Discounts leg cashflows via discount curve; converts index returns to PV with appropriate accrual/day-count handling.
- Deterministic index paths; relies on market quotes/dividend yields for equity TRS and price indices for FI TRS.

## Metrics
- PV, financing vs total-return leg contribution, carry/roll, and funding spread sensitivity.
- DV01/CS01 on discount/financing curves via generic calculators; delta to underlying index via total-return leg exposure.
- Scenario metrics for funding spread and underlying price shocks through bump hooks.
- Initial margin (SIMM-based) and variation margin via `Marginable` trait.

## Future Enhancements
- Support resettable notionals and pathwise financing accrual.
- Support stochastic equity/credit processes for total-return legs and correlation to financing leg.
- Provide coupon reinvestment/fee modeling and early termination options.
- Integrate actual index duration data for improved FI TRS SIMM calculations.
