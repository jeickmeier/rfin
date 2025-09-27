# `/structured_credit` Crate — Technical Design (CLO/ABS)

**Status:** Draft (feature-gated, implementation-ready)
**Last updated:** 2025-01-25
**MSRV:** 1.75 (aligned with core)
**License:** Apache-2.0 (project standard)

---

## 1) Purpose & Scope

`/structured_credit` specializes in modeling, projecting, and valuing structured credit products such as CLOs and ABS. It is split out from `/valuations` to keep common instruments lean while providing a focused, extensible surface for specialized use cases.

- Products: CLO, ABS, and similar tranche-based securitizations
- Focus: pool modeling, tranche definitions, waterfalls, triggers, fees, coverage tests, and analytics
- Outputs: tranche cashflows, waterfall distributions, coverage ratios, tranche valuations and risk metrics

Out of scope here: generic instruments (bonds/swaps/options), portfolio aggregation, statements, or IO.

---

## 2) Position in Workspace & Features

```
finstack/
├─ core/
├─ valuations/
├─ structured_credit/    # THIS CRATE (feature-gated in meta as `structured_credit`)
├─ portfolio/
├─ scenarios/
└─ analysis/
```

Cargo (sketch):

```toml
[package]
name = "finstack-structured-credit"
version = "0.2.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
finstack-core = { path = "../core" }
finstack-valuations = { path = "../valuations" } # reuse Cashflow, traits, MarketData
thiserror = "1"
serde = { version = "1", features = ["derive"] }
indexmap = { version = "2", features = ["serde"] }
smallvec = "1"

[features]
default = []
# Follow core numeric flags when useful
fast_f64 = ["finstack-core/fast_f64"]
deterministic = ["finstack-core/deterministic"]
rayon = ["finstack-core/rayon"]
```

Meta-crate features and re-exports:
- Feature `structured_credit` adds dependency and re-exports `finstack_structured_credit` as `structured_credit`.

---

## 3) Dependencies & Reuse Policy

Reuse-first:
- Types, `Decimal`, `Currency`, `Amount`, time/day-count, calendars: from `core`
- Cashflow primitives, traits (`CashflowProvider`, `Priceable`, `RiskMeasurable`), `MarketData`: from `valuations`
- No additional time-series engine beyond what core re-exports (Polars optional via callers)

---

## 4) Domain Model

```rust
use finstack_core::prelude::*;
use finstack_valuations as valuations;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuredProduct {
    pub id: String,
    pub collateral_pool: CollateralPool,
    pub tranches: Vec<Tranche>,
    pub waterfall: Waterfall,
    pub triggers: Vec<Trigger>,
    pub reserve_accounts: Vec<ReserveAccount>,
    pub fees: StructuredProductFees,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollateralPool {
    pub assets: Vec<PooledAsset>,
    pub reinvestment_period: Option<(time::Date, time::Date)>,
    pub eligibility: EligibilityCriteria,
    pub concentration_limits: ConcentrationLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PooledAsset {
    pub asset_id: String,
    pub asset: Box<dyn CashflowProvider>,
    pub par_amount: Amount,
    pub purchase_price: Decimal,
    pub credit_quality: CreditRating,
    pub industry: String,
    pub obligor: String,
    pub status: DefaultStatus,
    pub recovery_assumption: Option<RecoveryAssumption>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditRating { AAA, AAPlus, AA, AAMinus, APlus, A, AMinus, BBBPlus, BBB, BBBMinus, BBPlus, BB, BBMinus, BPlus, B, BMinus, CCCPlus, CCC, CCCMinus, CC, C, D, NR }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DefaultStatus {
    Performing,
    Delinquent { days: i32 },
    Defaulted { default_date: time::Date, recovery_rate: Option<Decimal> },
    Recovered { recovery_date: time::Date, recovery_amount: Amount },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryAssumption { pub recovery_rate: Decimal, pub recovery_lag: i32, pub recovery_costs: Decimal }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tranche {
    pub id: String,
    pub class_: TrancheClass,
    pub seniority: i32,
    pub original_balance: Amount,
    pub current_balance: Amount,
    pub coupon: CouponSpec,
    pub credit_enhancement: CreditEnhancement,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TrancheClass { Senior, Mezzanine, Subordinated, Equity }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CouponSpec {
    Fixed(Rate),
    Floating { index: IndexId, spread: Bps, cap: Option<Rate>, floor: Option<Rate> },
    Deferrable { base: Box<CouponSpec>, deferral: DeferralConditions },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeferralConditions { pub trigger_on: Vec<DeferralTrigger>, pub accumulate: bool, pub capitalize: bool }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeferralTrigger { CoverageRatioBreach { ratio_type: String, threshold: Decimal }, CashShortfall, TriggerEvent(String) }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreditEnhancement { pub subordination: Decimal, pub overcollateralization: Option<Decimal>, pub reserve_fund: Option<Amount> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Waterfall { pub payment_dates: Vec<time::Date>, pub interest: Vec<WaterfallStep>, pub principal: Vec<WaterfallStep>, pub post_acceleration: Option<Vec<WaterfallStep>> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterfallStep { pub priority: i32, pub description: String, pub recipient: PaymentRecipient, pub amount: WaterfallAmountType, pub conditions: Vec<WaterfallCondition> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PaymentRecipient { Tranche(String), FeeRecipient(String), ReserveAccount(String), Residual }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WaterfallAmountType { CurrentInterest, DeferredInterest, Principal, FullBalance, FixedAmount(Amount), PercentageOfAvailable(Decimal), UpToTarget { target: Amount } }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WaterfallCondition { TriggerNotBreached(String), TriggerBreached(String), ReinvestmentActive, ReinvestmentEnded }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger { pub id: String, pub kind: TriggerType, pub threshold: Decimal, pub cure_period_days: Option<i32>, pub consequences: Vec<TriggerConsequence> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerType { OC { tranche: String }, IC { tranche: String }, DiversityScore, WAC, WAS, WAL, DefaultRate { months: i32 }, CumulativeDefaultRate }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerConsequence { DivertCashflow { to: PaymentRecipient }, Accelerate, StopReinvestment, TrapExcessSpread }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReserveAccount { pub id: String, pub target: ReserveTarget, pub current_balance: Amount, pub funding_priority: i32 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReserveTarget { Fixed(Amount), PercentageOfPool(Decimal), PercentageOfTranche { tranche: String, percentage: Decimal } }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuredProductFees { pub management_fee: FeeSpec, pub servicing_fee: FeeSpec, pub trustee_fee: FeeSpec, pub other_fees: Vec<FeeSpec> }
```

---

## 5) Algorithms & Functions

- Pool projections with defaults/prepayments and recoveries
- Waterfall execution honoring priority rules and trigger consequences
- Coverage ratio calculation (OC/IC, etc.)
- Tranche pricing from projected cashflows (uses `valuations::npv` et al.)

APIs (selected):

```rust
impl StructuredProduct {
    pub fn run_waterfall(&self, collections: Amount, as_of: time::Date) -> Result<WaterfallResults, StructuredError> { /* ... */ }
    pub fn calculate_coverage_ratios(&self, as_of: time::Date) -> Result<CoverageRatios, StructuredError> { /* ... */ }
    pub fn project_cashflows(&self, market: &MarketData, assumptions: &StructuredProductAssumptions) -> Result<ProjectedCashflows, StructuredError> { /* ... */ }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StructuredProductAssumptions {
    pub default_curve: DefaultCurve,
    pub prepayment_curve: PrepaymentCurve,
    pub recovery_rates: indexmap::IndexMap<CreditRating, Decimal>,
    pub recovery_lag: i32,
    pub reinvestment_spread: Option<Bps>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefaultCurve { pub curve_type: DefaultCurveType, pub parameters: Vec<Decimal> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DefaultCurveType { Constant, Vector(Vec<(i32, Decimal)>), Exponential { lambda: Decimal }, Logistic { midpoint: i32, steepness: Decimal } }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrepaymentCurve { pub curve_type: PrepaymentCurveType, pub parameters: Vec<Decimal> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrepaymentCurveType { Constant, PSA { multiplier: Decimal }, Vector(Vec<(i32, Decimal)>), Custom }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterfallResults { pub distributions: indexmap::IndexMap<PaymentRecipient, Amount>, pub triggers_breached: Vec<String>, pub remaining_balance: Amount }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageRatios { pub oc: indexmap::IndexMap<String, Decimal>, pub ic: indexmap::IndexMap<String, Decimal> }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectedCashflows { pub periods: Vec<Period>, pub tranche_cashflows: indexmap::IndexMap<String, Vec<Cashflow>>, pub coverage: Vec<CoverageRatios>, pub cumulative_defaults: Vec<Amount>, pub cumulative_prepayments: Vec<Amount> }
```

---

## 6) Traits & Integration

- Implements `CashflowProvider` for `StructuredProduct` to generate tranche-level flows
- Provides `Priceable` for `Tranche` to value from projected flows
- Risk extensions (e.g., DV01, CS01) may be added as feature-gated modules

---

## 7) Scenarios Integration

- Scenario paths for structured-product-specific knobs (e.g., coverage thresholds, fee changes) are registered via the `scenarios` crate adapters
- Cache invalidation aligns with Overall §6.3 phases

---

## 8) Testing Strategy

- Unit: waterfall steps, trigger consequences, deferrable coupon logic
- Property: conservation of cash in waterfall; OC/IC monotonicity constraints
- Parity: compare against reference models for standard structures
- Bench: waterfall execution and projection performance

---

## 9) Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum StructuredError {
    #[error(transparent)] Core(#[from] finstack_core::CoreError),
    #[error(transparent)] Valuations(#[from] finstack_valuations::ValuationError),
    #[error("Invalid waterfall specification: {0}")] InvalidWaterfall(String),
    #[error("Trigger configuration error: {0}")] TriggerConfig(String),
    #[error("Projection failed: {0}")] Projection(String),
}
```

---

## 10) Acceptance Criteria

- Feature-gated crate compiles and integrates cleanly when enabled
- Waterfall engine deterministic and currency-preserving
- Coverage tests computed correctly and used in triggers
- Tranche valuation matches reference within tolerance
- Serde-stable public types; schema documented

---

## 11) Migration Notes

- All structured credit content removed from `/valuations`; consumers enable `structured_credit` to access it
- Meta-crate feature `structured_credit` re-exports the crate under `finstack::structured_credit`

---

This crate isolates the complexity of structured products while preserving tight integration with the shared `core` and `valuations` layers.
