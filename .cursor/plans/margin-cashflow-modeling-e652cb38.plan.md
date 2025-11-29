<!-- e652cb38-7bec-4eb9-b99c-cd783ac05059 7f4d5545-b072-4ff5-9132-9ac10da5cc70 -->
# Margin & Collateral Cashflow Modeling Implementation

## Overview

Add comprehensive margin modeling following ISDA, BCBS-IOSCO, and GMRA standards. Primary target is Repo instruments with TRS as secondary. All margin logic resides in `valuations` except CFKind enum variants in `core`.

---

## Phase 1: Core CFKind Extensions

**File:** [finstack/core/src/cashflow/primitives.rs](finstack/core/src/cashflow/primitives.rs)

Add margin-specific cashflow classification variants to the existing `CFKind` enum:

```rust
pub enum CFKind {
    // ... existing variants (Fixed, FloatReset, Fee, etc.) ...
    
    /// Initial margin posting (collateral transfer out)
    InitialMarginPost,
    /// Initial margin return (collateral returned)
    InitialMarginReturn,
    /// Variation margin received
    VariationMarginReceive,
    /// Variation margin paid
    VariationMarginPay,
    /// Interest accrued on posted margin collateral
    MarginInterest,
    /// Collateral substitution inflow
    CollateralSubstitutionIn,
    /// Collateral substitution outflow
    CollateralSubstitutionOut,
}
```

---

## Phase 2: Margin Types Module

**Location:** `finstack/valuations/src/margin/`

### 2.1 Module Structure

Create new directory `margin/` with:

- `mod.rs` - Module root with re-exports
- `types/mod.rs` - Type definitions
- `types/csa.rs` - CSA specification
- `types/collateral.rs` - Eligible collateral and haircuts
- `types/thresholds.rs` - Threshold/MTA/IA parameters
- `types/call.rs` - Margin call event types
- `types/enums.rs` - Shared enums (frequency, methodology)

### 2.2 CSA Specification (`types/csa.rs`)

```rust
/// Credit Support Annex specification (ISDA standard).
pub struct CsaSpec {
    pub id: String,
    pub base_currency: Currency,
    pub vm_params: VmParameters,
    pub im_params: Option<ImParameters>,
    pub eligible_collateral: EligibleCollateralSchedule,
    pub call_timing: MarginCallTiming,
    pub collateral_curve_id: CurveId,
}

pub struct VmParameters {
    pub threshold: Money,
    pub mta: Money,
    pub rounding: Money,
    pub independent_amount: Money,
    pub frequency: MarginFrequency,
    pub settlement_lag: u32,
}

pub struct ImParameters {
    pub methodology: ImMethodology,
    pub mpor_days: u32,
    pub threshold: Money,
    pub mta: Money,
    pub segregated: bool,
}
```

### 2.3 Eligible Collateral (`types/collateral.rs`)

```rust
pub struct EligibleCollateralSchedule {
    pub eligible: Vec<CollateralEligibility>,
    pub default_haircut: Option<f64>,
    pub rehypothecation_allowed: bool,
}

pub struct CollateralEligibility {
    pub asset_class: CollateralAssetClass,
    pub min_rating: Option<String>,
    pub maturity_constraints: Option<MaturityConstraints>,
    pub haircut: f64,
    pub fx_haircut_addon: f64,
    pub concentration_limit: Option<f64>,
}

pub enum CollateralAssetClass {
    Cash, GovernmentBonds, AgencyBonds, CoveredBonds,
    CorporateBonds, Equity, Gold, MutualFunds,
}
```

### 2.4 Margin Call Types (`types/call.rs`)

```rust
pub struct MarginCall {
    pub call_date: Date,
    pub settlement_date: Date,
    pub call_type: MarginCallType,
    pub amount: Money,
    pub collateral_type: Option<CollateralAssetClass>,
    pub mtm_trigger: Money,
    pub threshold: Money,
    pub mta_applied: Money,
}

pub enum MarginCallType {
    InitialMargin,
    VariationMarginDelivery,
    VariationMarginReturn,
    TopUp,
    Substitution,
}
```

### 2.5 Shared Enums (`types/enums.rs`)

```rust
pub enum MarginFrequency {
    Daily, Weekly, Monthly, OnDemand,
}

pub enum ImMethodology {
    Haircut, Simm, Schedule, InternalModel, ClearingHouse,
}
```

---

## Phase 3: Margin Calculators

**Location:** `finstack/valuations/src/margin/calculators/`

### 3.1 Calculator Trait (`calculators/traits.rs`)

```rust
pub trait ImCalculator: Send + Sync {
    fn calculate(
        &self,
        instrument: &dyn Instrument,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult>;
}
```

### 3.2 Variation Margin Calculator (`calculators/vm.rs`)

Implements ISDA CSA logic:

```rust
pub struct VmCalculator { csa: CsaSpec }

impl VmCalculator {
    /// Credit Support Amount = max(0, Exposure - Threshold + IA) - Collateral_Value
    /// Delivery Amount = max(0, CSA - MTA)
    /// Return Amount = max(0, -CSA - MTA)
    pub fn calculate(&self, exposure: Money, posted: Money, as_of: Date) -> Result<VmResult>;
    
    pub fn generate_cashflows(
        &self,
        exposures: &[(Date, Money)],
        context: &MarketContext,
    ) -> Result<Vec<MarginCashflow>>;
}

pub struct VmResult {
    pub date: Date,
    pub gross_exposure: Money,
    pub net_exposure: Money,
    pub delivery_amount: Money,
    pub return_amount: Money,
    pub settlement_date: Date,
}
```

### 3.3 Haircut IM Calculator (`calculators/im/haircut.rs`)

Standard for repos:

```rust
pub struct HaircutImCalculator {
    eligible_collateral: EligibleCollateralSchedule,
}

impl ImCalculator for HaircutImCalculator {
    // IM = Collateral_Value × Haircut
    // Applies asset class, rating, maturity, and FX haircuts
}
```

### 3.4 SIMM Calculator (`calculators/im/simm.rs`)

For OTC derivatives (stub implementation initially):

```rust
pub struct SimmCalculator {
    version: SimmVersion,
    risk_weights: SimmRiskWeights,
}
// Full SIMM: delta/vega/curvature sensitivities with correlation
```

### 3.5 Schedule IM Calculator (`calculators/im/schedule.rs`)

Regulatory fallback:

```rust
pub struct ScheduleImCalculator {
    schedule: RegulatorySchedule,
}
// Notional × Grid Rate based on asset class/maturity
```

---

## Phase 4: Repo Margin Integration

**Location:** [finstack/valuations/src/instruments/repo/](finstack/valuations/src/instruments/repo/)

### 4.1 Add Margin Types (`repo/margin/mod.rs`)

Create `margin/` subdirectory under repo with:

- `mod.rs` - Module exports
- `spec.rs` - RepoMarginSpec type
- `cashflows.rs` - Margin cashflow generation
- `metrics.rs` - Margin-specific metrics

### 4.2 RepoMarginSpec (`repo/margin/spec.rs`)

```rust
/// GMRA 2011 compliant margin specification.
pub struct RepoMarginSpec {
    pub margin_type: RepoMarginType,
    pub margin_ratio: f64,           // e.g., 1.02 for 2% over-collat
    pub margin_call_threshold: f64,  // % deviation triggering call
    pub call_frequency: MarginFrequency,
    pub settlement_lag: u32,
    pub pays_margin_interest: bool,
    pub margin_interest_rate: Option<f64>,
    pub substitution_allowed: bool,
    pub eligible_substitutes: Option<EligibleCollateralSchedule>,
}

pub enum RepoMarginType {
    None,         // Fixed haircut only
    MarkToMarket, // Daily MTM with calls
    NetExposure,  // Netting set margining
    Triparty,     // Triparty agent manages
}
```

### 4.3 Update Repo Struct (`repo/types.rs`)

Add margin spec field:

```rust
pub struct Repo {
    // ... existing fields ...
    pub margin_spec: Option<RepoMarginSpec>,
}
```

Update builder to support margin_spec.

### 4.4 Margin Cashflow Generation (`repo/margin/cashflows.rs`)

Extend `CashflowProvider` impl:

```rust
impl Repo {
    fn build_margin_schedule(
        &self,
        context: &MarketContext,
        as_of: Date,
        spec: &RepoMarginSpec,
    ) -> Result<DatedFlows> {
        // Generate margin call dates based on frequency
        // For each date: check collateral value vs required
        // Generate VM delivery/return cashflows
        // Add margin interest if applicable
    }
}
```

### 4.5 Repo Margin Metrics (`repo/margin/metrics.rs`)

- `MarginUtilization` - Posted / Required ratio
- `ExcessCollateral` - Collateral Value - Required Value
- `Haircut01` - PV sensitivity to 1bp haircut
- `MarginFundingCost` - Margin × (Funding Rate - Collateral Return)

Register metrics in `repo/metrics/mod.rs`.

---

## Phase 5: TRS Margin Integration

**Location:** [finstack/valuations/src/instruments/trs/](finstack/valuations/src/instruments/trs/)

### 5.1 TRS Margin Spec

Similar structure to repo:

- `trs/margin/mod.rs`
- `trs/margin/spec.rs` - TrsMarginSpec
- `trs/margin/cashflows.rs`

### 5.2 TrsMarginSpec

```rust
pub struct TrsMarginSpec {
    pub csa: CsaSpec,               // Full CSA for bilateral TRS
    pub im_methodology: ImMethodology,
    pub vm_frequency: MarginFrequency,
    pub settlement_lag: u32,
}
```

### 5.3 Update TRS Types

Add `margin_spec: Option<TrsMarginSpec>` to both:

- `EquityTotalReturnSwap`
- `FIIndexTotalReturnSwap`

---

## Phase 6: Testing

### 6.1 Unit Tests

**Location:** `finstack/valuations/src/margin/` (inline tests)

- VM calculation respects threshold and MTA
- Haircut IM uses correct asset class haircuts
- Margin call generation produces correct dates
- Collateral eligibility filtering works

### 6.2 Integration Tests

**Location:** `finstack/valuations/tests/`

Create `test_margin.rs`:

- Margined repo full lifecycle
- TRS margin cashflow generation
- Margin metrics accuracy

### 6.3 Golden Tests

**Location:** `finstack/valuations/tests/golden/`

- ISDA CSA example calculations
- GMRA margin maintenance scenarios
- BCBS-IOSCO haircut schedule validation

---

## Phase 7: Documentation & Examples

### 7.1 Update READMEs

- [finstack/valuations/src/instruments/repo/README.md](finstack/valuations/src/instruments/repo/README.md) - Add margin section
- [finstack/valuations/src/instruments/trs/README.md](finstack/valuations/src/instruments/trs/README.md) - Add margin section

### 7.2 Module Documentation

Add comprehensive rustdoc to `margin/mod.rs` covering:

- Industry standards (ISDA, BCBS-IOSCO, GMRA)
- CSA parameters explanation
- Usage examples

### 7.3 Example Scripts

**Location:** `finstack/examples/valuations/`

Create `margined_repo.rs`:

```rust
// Demonstrates creating and valuing a margined repo
// Shows margin cashflow schedule and metrics
```

### 7.4 Update future_enhancements.md

Mark repo/TRS margin items as complete.

---

## File Summary

| New Files | Purpose |

|-----------|---------|

| `valuations/src/margin/mod.rs` | Module root |

| `valuations/src/margin/types/mod.rs` | Type re-exports |

| `valuations/src/margin/types/csa.rs` | CSA specification |

| `valuations/src/margin/types/collateral.rs` | Eligible collateral |

| `valuations/src/margin/types/thresholds.rs` | VM/IM parameters |

| `valuations/src/margin/types/call.rs` | Margin call types |

| `valuations/src/margin/types/enums.rs` | Shared enums |

| `valuations/src/margin/calculators/mod.rs` | Calculator exports |

| `valuations/src/margin/calculators/traits.rs` | Calculator trait |

| `valuations/src/margin/calculators/vm.rs` | VM calculator |

| `valuations/src/margin/calculators/im/mod.rs` | IM calculator exports |

| `valuations/src/margin/calculators/im/haircut.rs` | Haircut IM |

| `valuations/src/margin/calculators/im/simm.rs` | SIMM (stub) |

| `valuations/src/margin/calculators/im/schedule.rs` | Schedule IM |

| `valuations/src/margin/metrics/mod.rs` | Metrics exports |

| `valuations/src/margin/metrics/utilization.rs` | Utilization metric |

| `valuations/src/margin/metrics/excess.rs` | Excess collateral |

| `valuations/src/margin/metrics/funding_cost.rs` | Funding cost |

| `valuations/src/margin/metrics/haircut01.rs` | Haircut sensitivity |

| `valuations/src/instruments/repo/margin/mod.rs` | Repo margin module |

| `valuations/src/instruments/repo/margin/spec.rs` | RepoMarginSpec |

| `valuations/src/instruments/repo/margin/cashflows.rs` | Margin cashflows |

| `valuations/src/instruments/repo/margin/metrics.rs` | Repo margin metrics |

| `valuations/src/instruments/trs/margin/mod.rs` | TRS margin module |

| `valuations/src/instruments/trs/margin/spec.rs` | TrsMarginSpec |

| `valuations/tests/test_margin.rs` | Integration tests |

| `finstack/examples/valuations/margined_repo.rs` | Example |

| Modified Files | Changes |

|----------------|---------|

| `core/src/cashflow/primitives.rs` | Add 7 CFKind variants |

| `valuations/src/lib.rs` | Add `pub mod margin;` |

| `valuations/src/instruments/repo/mod.rs` | Add `pub mod margin;` |

| `valuations/src/instruments/repo/types.rs` | Add `margin_spec` field |

| `valuations/src/instruments/trs/mod.rs` | Add `pub mod margin;` |

| `valuations/src/instruments/trs/equity/types.rs` | Add `margin_spec` field |

| `valuations/src/instruments/trs/fi_index/types.rs` | Add `margin_spec` field |

| `valuations/src/instruments/future_enhancements.md` | Mark complete |

### To-dos

- [ ] Add 7 margin CFKind variants to core/src/cashflow/primitives.rs
- [ ] Create margin/types/ module with CSA, collateral, threshold, call types
- [ ] Create shared margin enums (MarginFrequency, ImMethodology, etc.)
- [ ] Implement VmCalculator with ISDA threshold/MTA logic
- [ ] Implement HaircutImCalculator for repos
- [ ] Create SIMM calculator stub for future derivatives support
- [ ] Implement ScheduleImCalculator (regulatory fallback)
- [ ] Create RepoMarginSpec and add to Repo struct
- [ ] Implement margin cashflow generation for Repo
- [ ] Add margin metrics (utilization, excess, haircut01, funding cost)
- [ ] Add TrsMarginSpec and margin support to TRS instruments
- [ ] Write unit tests, integration tests, and golden tests
- [ ] Update READMEs, add rustdoc, create example script