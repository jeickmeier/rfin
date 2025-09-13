# `/Valuations` Crate — Technical Design

**Status:** Draft (implementation-ready)
**Last updated:** 2025-01-25
**MSRV:** 1.75 (aligned with core)
**License:** Apache-2.0 (project standard)

## 1) Purpose & Scope

`/valuations` is the quantitative finance engine for finstack. It provides:

* **Cash flow generation**: Schedule building, accrual calculations, and cash flow projections
* **Pricing & valuation**: NPV, PV, clean/dirty prices, yields, spreads
* **Performance metrics**: XIRR calculations with robust solvers
* **Risk measures**: DV01, CS01, duration, convexity, option Greeks
* **Private credit instruments**: 
  - Loans with time-varying rates (fixed/floating schedules)
  - Cash/PIK/Toggle coupon structures
  - Complex amortization (fixed percentage, fixed amount, custom schedules)
  - Call schedules and prepayment options
  - Revolving credit facilities with commitment/utilization fees
  - Custom fee schedules (origination, amendment, exit fees)
* **Period aggregation**: Currency-preserving cash flow rollups to financial periods

**Depends on core for:**
* Strong types (`Amount`, `Currency`, `Rate`, `Id<T>`)
* Expression engine for valuation formulas
* FX infrastructure (`FxProvider`, `FxMatrix`)
* Math kernels (root finding, summation)
* Time utilities (day-count, calendars, schedules, periods)
* Validation framework
* Polars re-exports for time-series

**Out of scope:**
* Portfolio management (→ `portfolio` crate)
* Financial statements (→ `statements` crate)
* Scenario engines (→ `scenarios` crate)
* Risk aggregation across portfolios (→ `portfolio`/`risk`)

---

## 2) Architecture & Dependencies

```
finstack/
├─ core/               # Foundation layer
├─ valuations/         # THIS CRATE
│  ├─ src/
│  │  ├─ lib.rs
│  │  ├─ traits.rs           # Core traits (CashflowProvider, Priceable, RiskMeasurable)
│  │  ├─ cashflow/           # Cash flow types and generation
│  │  ├─ instruments/        # Instrument implementations
│  │  ├─ market/            # Market data structures and curves
│  │  ├─ pricing/           # Pricing engines and calculators
│  │  ├─ risk/              # Risk measure calculations
│  │  ├─ performance/       # XIRR implementations
│  │  ├─ aggregation/       # Period-based aggregation
│  │  ├─ covenants/         # Covenant types, evaluator, and consequence application
│  │  ├─ formulas/          # Valuation-specific formulas

│  │  └─ error.rs
│  ├─ benches/
│  └─ tests/
├─ portfolio/          # Depends on valuations
├─ scenarios/          # Depends on valuations
└─ ...
```

**Cargo.toml (sketch):**

```toml
[package]
name = "finstack-valuations"
version = "0.2.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
finstack-core = { path = "../core" }
thiserror = "1"
serde = { version = "1", features = ["derive"] }
indexmap = { version = "2", features = ["serde"] }
hashbrown = "0.14"
smallvec = "1"

# Use Polars via core re-exports
# No direct Polars dependency - use through finstack_core::prelude

# Pure function registry for deterministic custom logic
fnv = "1"

[features]
default = ["multi_curve", "quotes"]
# Enable OIS discounting + projection/discount curve split
multi_curve = []
# Enable inflation-linked instruments (TIPS/ILBs)
inflation = []
# Enable street quoting adapters (clean/dirty, settlement/ex-coupon)
quotes = []

[dev-dependencies]
proptest = "1"
criterion = "0.5"
approx = "0.6"
```

---

## 3) Core Traits & Contracts

### 3.1 CashflowProvider Trait

```rust
use finstack_core::prelude::*;

pub trait CashflowProvider: Send + Sync {
    /// Build complete cash flow schedule
    fn build_schedule(
        &self,
        market: &MarketData,
        as_of: time::Date,
    ) -> Result<CashflowSchedule, FinstackError>;
    
    /// Aggregate cash flows by period, preserving currency
    fn aggregate_period_cashflow(
        &self,
        periods: &[Period],
        tags: Option<&TagSet>,
    ) -> Result<indexmap::IndexMap<PeriodId, indexmap::IndexMap<Currency, Decimal>>, FinstackError>;
    
    /// Convert to model currency (requires explicit FX)
    fn aggregate_to_model_ccy(
        &self,
        periods: &[Period],
        tags: Option<&TagSet>,
        fx: &dyn FxProvider,
        model_ccy: Currency,
    ) -> Result<indexmap::IndexMap<PeriodId, Decimal>, FinstackError>;
}
```

**Invariants:**
* `aggregate_period_cashflow` MUST be currency-preserving
* FX conversion happens only in `aggregate_to_model_ccy` with explicit provider
* Tags allow filtering by cash flow type (interest, principal, fees, etc.)

### 3.2 Priceable Trait

```rust
pub trait Priceable: Send + Sync {
    fn price(
        &self,
        market: &MarketData,
        as_of: time::Date,
    ) -> Result<ValuationResult, FinstackError>;
}
```

### 3.3 RiskMeasurable Trait

```rust
pub trait RiskMeasurable: Priceable {
    fn risk_report(
        &self,
        market: &MarketData,
        as_of: time::Date,
        buckets: Option<&[Bucket]>,
    ) -> Result<RiskReport, FinstackError>;
}
pub enum Bucket { Tenor(String), Curve(CurveId), Issuer(IssuerId) }
```

### 3.4 Attributes & Metadata (NEW)

To enable selector-based scenarios (e.g., shock all CCC-rated instruments or a specific sector), instruments expose uniform attributes:

```rust
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attributes {
    /// Free-form tags for selection and grouping
    #[serde(default)]
    pub tags: TagSet,

    /// Arbitrary structured metadata; examples: {"rating":"CCC", "sector":"Technology"}
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub meta: indexmap::IndexMap<String, serde_json::Value>,
}

/// Instruments implement this to surface attributes for selection and scenarios
pub trait Attributable {
    fn attributes(&self) -> &Attributes;
    fn attributes_mut(&mut self) -> &mut Attributes;
}
```

Requirements:
- All concrete instrument structs include an `attrs: Attributes` field and implement `Attributable`.
- Bindings expose `tags` and `meta` as first-class, serde-stable fields.
- Scenario selectors (see Scenarios §2.6) can filter instruments by `attrs.tags` and `attrs.meta`.

---

## 4) Cash Flow Types

### 4.1 Core Cash Flow Structures

```rust
use finstack_core::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cashflow {
    pub date: time::Date,
    pub amount: Amount,        // from core, includes currency
    pub flow_type: CashflowType,
    pub tags: TagSet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CashflowType {
    Interest { rate: Rate, accrual_days: i32 },
    Principal,
    Fee { fee_type: String },
    Premium,
    Settlement,
    Dividend,
    Protection,  // CDS
    Custom(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TagSet(indexmap::IndexSet<String>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CashflowSchedule {
    pub flows: Vec<Cashflow>,
    pub instrument_id: String,
    pub currency: Currency,
    pub generated_at: time::Date,
}
```

### 4.2 Schedule Generation

```rust
pub struct ScheduleBuilder {
    start: time::Date,
    end: time::Date,
    frequency: Frequency,
    calendar: Box<dyn BusinessCalendar>,
    bdc: BusinessDayConvention,
    eom_rule: bool,
    stub_type: StubType,
}

#[derive(Clone, Copy, Debug)]
pub enum Frequency {
    Annual,
    SemiAnnual,
    Quarterly,
    Monthly,
    Biweekly,
    Weekly,
    Daily,
}

#[derive(Clone, Copy, Debug)]
pub enum StubType {
    ShortFirst,
    ShortLast,
    LongFirst,
    LongLast,
}

impl ScheduleBuilder {
    pub fn build(&self) -> Result<Vec<time::Date>, ValuationError> {
        // Uses core::time utilities for calendar and BDC
    }
}
```

---

## 5) Market Data Structures

### 5.1 Market Data Container

```rust
use finstack_core::prelude::*;

pub struct MarketData {
    pub as_of: time::Date,
    pub discount: std::collections::HashMap<CurveId, DiscountCurve>,
    pub indices: std::collections::HashMap<IndexId, RateIndex>,
    /// Projection curves by index (multi-curve)
    pub projection: std::collections::HashMap<IndexId, ForwardCurve>,
    pub credit: std::collections::HashMap<IssuerId, CreditCurve>,
    pub fx: FxMatrix,  // from core::money
    pub vol: std::collections::HashMap<SurfaceId, VolSurface>,
    pub dividends: std::collections::HashMap<Ticker, DividendSchedule>,
    pub prices: std::collections::HashMap<InstrumentId, Decimal>,
    /// Inflation index series (e.g., CPI, RPI) for ILBs
    pub inflation_indices: std::collections::HashMap<IndexId, finstack_core::time::IndexSeries>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CurveId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct IndexId(pub String);  // e.g., "USD-SOFR-3M"

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct IssuerId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SurfaceId(pub String);
```

### 5.2 Curves

```rust
pub struct DiscountCurve {
    pub id: CurveId,
    pub currency: Currency,
    pub pillars: Vec<time::Date>,
    pub rates: Vec<Decimal>,
    pub interpolation: InterpolationMethod,
    pub day_count: DayCount,  // from core::time
    pub compounding: Compounding,
    /// Provenance of arithmetic applied to this curve (for auditability)
    #[serde(default)]
    pub provenance: Vec<CurveOp>,
}

#[derive(Clone, Copy, Debug)]
pub enum InterpolationMethod {
    Linear,
    LogLinear,
    CubicSpline,
    MonotoneConvex,
}

#[derive(Clone, Copy, Debug)]
pub enum Compounding {
    Simple,
    Continuous,
    Annual,
    SemiAnnual,
    Quarterly,
    Monthly,
}

impl DiscountCurve {
    pub fn discount_factor(&self, date: time::Date) -> Result<Decimal, ValuationError> {
        // Cached interpolation using core::time::year_fraction
    }
    
    pub fn forward_rate(
        &self,
        start: time::Date,
        end: time::Date,
    ) -> Result<Rate, ValuationError> {
        // Calculate forward rate between dates
    }

    /// Parallel shift by X basis points. Records provenance.
    pub fn shift_parallel(&mut self, bp: Decimal) -> Result<(), ValuationError> { /* impl */ }

    /// Twist around a tenor pivot: short/long end shifts (in bp). Records provenance.
    pub fn twist(&mut self, pivot: String, short_bp: Decimal, long_bp: Decimal) -> Result<(), ValuationError> { /* impl */ }

    /// Bucketed shifts by named buckets with per-bucket bp amounts. Records provenance.
    pub fn bucket_shift(&mut self, buckets: indexmap::IndexMap<String, Decimal>) -> Result<(), ValuationError> { /* impl */ }
}

/// Curve arithmetic provenance for scenarios/analysis
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CurveOp {
    ParallelShift { bp: Decimal, applied_by: String },
    Twist { pivot: String, short_bp: Decimal, long_bp: Decimal, applied_by: String },
    BucketShift { buckets: indexmap::IndexMap<String, Decimal>, applied_by: String },
}
```

### 5.4 Multi-Curve (OIS Discounting)

```rust
/// Forward (projection) curve used to project floating leg rates
#[derive(Clone, Debug)]
pub struct ForwardCurve {
    pub id: IndexId,                      // e.g., "USD-SOFR-3M"
    pub pillars: Vec<time::Date>,
    pub forwards: Vec<Decimal>,           // forward rates on pillars
    pub interpolation: InterpolationMethod,
    pub day_count: DayCount,
    pub compounding: Compounding,
}
```

Requirements:
- Discounting uses OIS discount curves per currency; projection uses index-specific `ForwardCurve`.
- IBOR→RFR fallbacks are explicit and recorded in `CurveOp` provenance.
- APIs accept `(discount: &DiscountCurve, projection: Option<&ForwardCurve>)` where applicable; single-curve remains supported.

### 5.3 Credit Curves

```rust
pub struct CreditCurve {
    pub issuer: IssuerId,
    pub seniority: Seniority,
    pub recovery_rate: Decimal,
    pub pillars: Vec<time::Date>,
    pub spreads: Vec<Bps>,  // from core::types
}

#[derive(Clone, Copy, Debug)]
pub enum Seniority {
    Senior,
    Subordinated,
    Junior,
}
```

---

## 6) Instrument Library

### 6.0 Spot Instruments (Equity, FX Spot)

```rust
// Type aliases used across instruments
pub type InstrumentId = Id<Instrument>;  // from core::types
pub type EntityId = Id<Entity>;          // from core::types
pub type Ticker = String;

/// Simple equity (spot) instrument
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Equity {
    pub id: InstrumentId,
    pub ticker: Ticker,
    pub currency: Currency,
    /// Optional reference to an entity's statements for derived analytics
    /// (e.g., use node values for ratios; pricing still uses MarketData.prices)
    #[serde(default)]
    pub reference: Option<EquityReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EquityReference {
    /// Link by entity and optional node id in statements for analytics
    Entity { entity_id: EntityId, node_id: Option<String> },
}

impl Priceable for Equity {
    fn price(&self, market: &MarketData, as_of: time::Date)
        -> Result<ValuationResult, ValuationError>
    {
        let px = market
            .prices
            .get(&self.id)
            .or_else(|| market.prices.get(&self.ticker))
            .cloned()
            .ok_or_else(|| ValuationError::MarketDataNotFound(format!("price: {}", self.ticker)))?;

        Ok(ValuationResult {
            instrument_id: self.id.to_string(),
            as_of,
            value: Amount { value: px, ccy: self.currency },
            measures: indexmap::IndexMap::new(),
            cash_flows: None,
            meta: ResultsMeta { /* filled by caller */ numeric_mode: NumericMode::Decimal, parallel: false, seed: 0, model_currency: None, rounding: finstack_core::config::RoundingContext { mode: finstack_core::config::RoundingMode::Bankers, ingest_scale_by_ccy: indexmap::IndexMap::new(), output_scale_by_ccy: indexmap::IndexMap::new(), version: 1 } },
        })
    }
}

/// FX Spot instrument (1 unit of `base` priced in `quote`)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxSpot {
    pub id: InstrumentId,
    pub base: Currency,
    pub quote: Currency,
    /// Optional settlement date (informational; pricing uses spot as_of by default)
    pub settlement: Option<time::Date>,
}

impl Priceable for FxSpot {
    fn price(&self, market: &MarketData, as_of: time::Date)
        -> Result<ValuationResult, ValuationError>
    {
        let rate = market
            .fx
            .rate(self.base, self.quote, as_of)
            .map_err(|_| ValuationError::MarketDataNotFound(format!("fx {}/{:?}", self.base, self.quote)))?;

        Ok(ValuationResult {
            instrument_id: self.id.to_string(),
            as_of,
            // Price of one unit of BASE in QUOTE currency
            value: Amount { value: rate, ccy: self.quote },
            measures: indexmap::IndexMap::new(),
            cash_flows: None,
            meta: ResultsMeta { /* filled by caller */ numeric_mode: NumericMode::Decimal, parallel: false, seed: 0, model_currency: None, rounding: finstack_core::config::RoundingContext { mode: finstack_core::config::RoundingMode::Bankers, ingest_scale_by_ccy: indexmap::IndexMap::new(), output_scale_by_ccy: indexmap::IndexMap::new(), version: 1 } },
        })
    }
}
```

Note (NEW): All concrete instrument structs MUST include an `attrs: Attributes` field and implement `Attributable` so scenarios can select by attributes (e.g., `rating="CCC"`, `sector="Energy"`, `seniority="Senior Secured"`). Bindings expose these fields with stable serde names.

### 6.1 Fixed Income

```rust
pub struct Bond {
    pub id: InstrumentId,
    pub issuer: IssuerId,
    pub maturity: time::Date,
    pub coupon_rate: Rate,
    pub frequency: Frequency,
    pub day_count: DayCount,
    pub notional: Amount,
    pub schedule: Option<Vec<time::Date>>,  // pre-built schedule
}

impl CashflowProvider for Bond {
    fn build_schedule(&self, market: &MarketData, as_of: time::Date) 
        -> Result<CashflowSchedule, ValuationError> {
        // Generate coupon and principal cash flows
    }
}

impl Priceable for Bond {
    fn price(&self, market: &MarketData, as_of: time::Date) 
        -> Result<ValuationResult, ValuationError> {
        // Discount cash flows using appropriate curve
    }
}

impl RiskMeasurable for Bond {
    fn risk_report(&self, market: &MarketData, as_of: time::Date, buckets: Option<&[RiskBucket]>) 
        -> Result<RiskReport, ValuationError> {
        // Calculate duration, convexity, DV01
    }
}
```

#### 6.1.1 Inflation-Linked Bonds (TIPS/ILBs)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InflationIndexationSpec {
    pub index_id: IndexId,                      // e.g., US CPI, UK RPI
    pub base_reference_date: time::Date,        // dated date or accrual base
    pub interpolation: finstack_core::time::IndexInterpolation,
    pub lag: finstack_core::time::IndexLag,     // e.g., 3 months
    pub seasonality: Option<[Decimal; 12]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InflationLinkedBond {
    pub id: InstrumentId,
    pub issuer: IssuerId,
    pub currency: Currency,
    pub maturity: time::Date,
    pub coupon_rate: Rate,                      // real coupon
    pub frequency: Frequency,
    pub day_count: DayCount,
    pub notional: Amount,                       // real notional
    pub indexation: InflationIndexationSpec,
    pub schedule: Option<Vec<time::Date>>,      // optional pre-built schedule
}

impl CashflowProvider for InflationLinkedBond {
    fn build_schedule(&self, market: &MarketData, as_of: time::Date)
        -> Result<CashflowSchedule, ValuationError>
    {
        // Use market.inflation_indices[index_id] to compute index ratio I(settle)/I(base)
        // Apply to coupons and principal per instrument spec (deflation floor policies documented)
    }
}

impl Priceable for InflationLinkedBond {
    fn price(&self, market: &MarketData, as_of: time::Date)
        -> Result<ValuationResult, ValuationError>
    {
        // Discount with OIS; project index ratios via IndexSeries
    }
}
```

### 6.2 Loans & Private Credit Facilities

```rust
// Type aliases and core types (see also §6.0)

pub struct Loan {
    pub id: InstrumentId,
    pub borrower: EntityId,
    pub original_amount: Amount,
    pub outstanding: Amount,
    pub maturity: time::Date,
    pub interest_type: InterestType,
    pub amortization: AmortizationType,
    pub prepayment: Option<PrepaymentSchedule>,
    pub fees: Vec<FeeSpec>,
    pub call_schedule: Option<CallSchedule>,
    pub covenants: Vec<Covenant>,
}

#[derive(Clone, Debug)]
pub struct PrepaymentSchedule {
    pub prepayment_type: PrepaymentType,
    pub lockout_period: Option<(time::Date, time::Date)>,
    pub penalties: Vec<PrepaymentPenalty>,
}

#[derive(Clone, Debug)]
pub enum PrepaymentType {
    Allowed,
    ProhibitedWithMakeWhole,
    SoftCall { premium: Decimal },
    HardCall,  // No prepayment allowed
}

#[derive(Clone, Debug)]
pub struct PrepaymentPenalty {
    pub start: time::Date,
    pub end: Option<time::Date>,
    pub penalty: PenaltyType,
}

#[derive(Clone, Debug)]
pub enum PenaltyType {
    Fixed(Amount),
    Percentage(Decimal),
    MakeWhole { benchmark: CurveId, spread: Bps },
    YieldMaintenance,
}

#[derive(Clone, Debug)]
pub struct Covenant {
    pub covenant_type: CovenantType,
    pub test_frequency: Frequency,
    pub cure_period: Option<i32>,  // Days
    pub consequences: Vec<CovenantConsequence>,
}

#[derive(Clone, Debug)]
pub enum CovenantType {
    FinancialRatio {
        ratio_type: FinancialRatioType,
        threshold: Decimal,
        direction: ThresholdDirection,
    },
    Negative { restriction: String },
    Affirmative { requirement: String },
}

#[derive(Clone, Debug)]
pub enum FinancialRatioType {
    DebtToEBITDA,
    InterestCoverage,
    FixedChargeCoverage,
    TotalLeverage,
    SeniorLeverage,
    AssetCoverage,
    Custom(String),
}

#[derive(Clone, Copy, Debug)]
pub enum ThresholdDirection {
    Maximum,  // Ratio must be <= threshold
    Minimum,  // Ratio must be >= threshold
}

#[derive(Clone, Debug)]
pub enum CovenantConsequence {
    Default,
    RateIncrease(Bps),
    CashSweep,
    BlockDistributions,
    RequireAdditionalCollateral,
}

#[derive(Clone, Debug)]
pub enum InterestType {
    Fixed(RateSchedule),
    Floating {
        index: IndexId,
        spread: SpreadSchedule,
        floor: Option<RateSchedule>,
        cap: Option<RateSchedule>,
    },
    PIK(RateSchedule),  // Payment-in-kind
    CashPlusPIK {
        cash_schedule: RateSchedule,
        pik_schedule: RateSchedule,
    },
    PIKToggle {
        cash_rate: RateSchedule,
        pik_rate: RateSchedule,
        toggle_conditions: ToggleConditions,
    },
}

#[derive(Clone, Debug)]
pub struct RateSchedule {
    pub periods: Vec<RatePeriod>,
}

#[derive(Clone, Debug)]
pub struct RatePeriod {
    pub start: time::Date,
    pub end: Option<time::Date>,
    pub rate: Rate,
}

#[derive(Clone, Debug)]
pub struct SpreadSchedule {
    pub periods: Vec<SpreadPeriod>,
}

#[derive(Clone, Debug)]
pub struct SpreadPeriod {
    pub start: time::Date,
    pub end: Option<time::Date>,
    pub spread: Bps,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToggleConditions {
    BorrowerElection,
    LeverageRatio { threshold: Decimal },
    InterestCoverage { threshold: Decimal },
    /// Named, pure predicate registered in the function registry
    Fn(ToggleFnSpec),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToggleFnSpec {
    /// Registry function name (stable identifier)
    pub name: String,
    /// Serde-serializable parameters for the function
    pub params: serde_json::Value,
}

#[derive(Clone, Debug)]
pub enum AmortizationType {
    Bullet,
    Linear { frequency: Frequency },
    FixedPercentage { 
        schedule: Vec<(time::Date, Decimal)>,  // (date, percentage of original)
    },
    FixedAmount {
        schedule: Vec<(time::Date, Amount)>,   // (date, amount)
    },
    Custom {
        schedule: Vec<AmortizationEvent>,
    },
}

#[derive(Clone, Debug)]
pub struct AmortizationEvent {
    pub date: time::Date,
    pub amount: Option<Amount>,
    pub percentage: Option<Decimal>,
    pub mandatory: bool,
}

#[derive(Clone, Debug)]
pub struct CallSchedule {
    pub periods: Vec<CallPeriod>,
}

#[derive(Clone, Debug)]
pub struct CallPeriod {
    pub start: time::Date,
    pub end: time::Date,
    pub call_price: Decimal,  // As percentage of par (e.g., 102.0 = 102%)
    pub notice_days: i32,
}

#[derive(Clone, Debug)]
pub struct FeeSpec {
    pub fee_type: FeeType,
    pub amount: FeeAmount,
    pub payment_date: FeePaymentDate,
    pub amortizable: bool,
}

#[derive(Clone, Debug)]
pub enum FeeType {
    Origination,
    Commitment,
    Utilization,
    Amendment,
    Exit,
    Prepayment,
    Custom(String),
}

#[derive(Clone, Debug)]
pub enum FeeAmount {
    Fixed(Amount),
    Percentage(Decimal),  // Of commitment or outstanding
    BasisPoints(Bps),
}

#[derive(Clone, Debug)]
pub enum FeePaymentDate {
    Upfront,
    AtMaturity,
    Periodic(Frequency),
    OnEvent(String),
}

// Revolving Credit Facility
pub struct RevolvingCreditFacility {
    pub id: InstrumentId,
    pub borrower: EntityId,
    pub commitment: Amount,
    pub drawn_amount: Amount,
    pub availability_period: (time::Date, time::Date),
    pub maturity: time::Date,
    pub interest_type: InterestType,
    pub commitment_fee: Rate,
    pub utilization_fee: Option<UtilizationFeeSchedule>,
    pub draw_schedule: Vec<DrawEvent>,
    pub repayment_schedule: Vec<RepaymentEvent>,
    pub covenants: Vec<Covenant>,
}

#[derive(Clone, Debug)]
pub struct UtilizationFeeSchedule {
    pub tiers: Vec<UtilizationTier>,
}

#[derive(Clone, Debug)]
pub struct UtilizationTier {
    pub min_utilization: Decimal,  // As percentage
    pub max_utilization: Decimal,
    pub fee_rate: Bps,
}

#[derive(Clone, Debug)]
pub struct DrawEvent {
    pub date: time::Date,
    pub amount: Amount,
    pub purpose: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RepaymentEvent {
    pub date: time::Date,
    pub amount: Amount,
    pub mandatory: bool,
}

impl CashflowProvider for RevolvingCreditFacility {
    fn build_schedule(&self, market: &MarketData, as_of: time::Date) 
        -> Result<CashflowSchedule, ValuationError> {
        // Generate interest on drawn amounts
        // Generate commitment fees on undrawn amounts
        // Handle utilization fees based on drawn percentage
    }
}
```

#### 6.2.1 Delayed‑Draw Term Loan (DDTL)

```rust
#[derive(Clone, Debug)]
pub struct DelayedDrawTermLoan {
    pub id: InstrumentId,
    pub borrower: EntityId,
    /// Aggregate committed amount available for delayed draws
    pub commitment: Amount,
    /// Expiry of drawing rights (after which undrawn commitment lapses)
    pub commitment_expiry: time::Date,
    /// Minimum/maximum draw size rules and notice requirements
    pub draw_rules: DrawRules,
    /// Scheduled/conditional draws (subject to conditions precedent)
    pub planned_draws: Vec<DrawEvent>,
    /// Interest/PIK/cash toggle and floors/caps as per loan terms
    pub interest_type: InterestType,
    /// Fees: commitment (undrawn), ticking, delayed‑draw fees
    pub dd_fees: Vec<FeeSpec>,
    /// Covenants referenced for draw conditions (e.g., leverage <= X)
    pub draw_conditions: Vec<Covenant>,
    /// Maturity and amortization after drawn
    pub maturity: time::Date,
    pub amortization: AmortizationType,
}

#[derive(Clone, Debug)]
pub struct DrawRules {
    pub min_draw: Amount,
    pub max_draw: Option<Amount>,
    pub notice_days: i32,
}

impl CashflowProvider for DelayedDrawTermLoan {
    fn build_schedule(&self, market: &MarketData, as_of: time::Date)
        -> Result<CashflowSchedule, ValuationError>
    {
        // Generate commitment fee cash flows until expiry, incorporate planned draws that
        // satisfy draw_conditions at their dates; undrawn amount accrues commitment/ticking fees.
        // Once drawn, interest and amortization follow loan terms.
    }
}
```

### 6.3 Derivatives - Interest Rate Swaps

```rust
pub struct InterestRateSwap {
    pub id: InstrumentId,
    pub fixed_leg: SwapLeg,
    pub float_leg: SwapLeg,
    pub start_date: time::Date,
    pub maturity: time::Date,
}

pub struct SwapLeg {
    pub pay_receive: PayReceive,
    pub notional: Amount,
    pub rate_spec: RateSpec,
    pub frequency: Frequency,
    pub day_count: DayCount,
    pub calendar: CalendarId,
    pub bdc: BusinessDayConvention,
}

#[derive(Clone, Copy, Debug)]
pub enum PayReceive {
    Pay,
    Receive,
}

#[derive(Clone, Debug)]
pub enum RateSpec {
    Fixed(Rate),
    Floating {
        index: IndexId,
        spread: Bps,
        observation_shift: i32,  // days
    },
}
```

### 6.4 Credit Derivatives

```rust
pub struct CreditDefaultSwap {
    pub id: InstrumentId,
    pub reference_entity: IssuerId,
    pub notional: Amount,
    pub spread: Bps,
    pub maturity: time::Date,
    pub frequency: Frequency,
    pub recovery_rate: Option<Decimal>,
    pub imm_dates: bool,
}

impl Priceable for CreditDefaultSwap {
    fn price(&self, market: &MarketData, as_of: time::Date) 
        -> Result<ValuationResult, ValuationError> {
        // Price using ISDA standard model
        // Calculate par spread, risky PV01
    }
}
```

### 6.5 Options

```rust
pub struct VanillaOption {
    pub id: InstrumentId,
    pub underlying: UnderlyingSpec,
    pub strike: Decimal,
    pub expiry: time::Date,
    pub option_type: OptionType,
    pub exercise: ExerciseType,
}

#[derive(Clone, Debug)]
pub enum UnderlyingSpec {
    Equity(Ticker),
    FX { base: Currency, quote: Currency },
    Rate(IndexId),
}

#[derive(Clone, Copy, Debug)]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Clone, Copy, Debug)]
pub enum ExerciseType {
    European,
    American,
    Bermudan(Vec<time::Date>),
}

impl Priceable for VanillaOption {
    fn price(&self, market: &MarketData, as_of: time::Date) 
        -> Result<ValuationResult, ValuationError> {
        // Black-Scholes for European equity
        // Garman-Kohlhagen for FX
        // Black model for rates
    }
}

impl RiskMeasurable for VanillaOption {
    fn risk_report(&self, market: &MarketData, as_of: time::Date, buckets: Option<&[RiskBucket]>) 
        -> Result<RiskReport, ValuationError> {
        // Calculate Greeks: delta, gamma, vega, theta, rho
    }
}
```

### 6.6 Structured Credit Products (CLO/ABS)

Moved to a separate, feature-gated crate: `finstack-structured-credit`.

- Motivation: highly specialized domain with significant complexity and maintenance surface.
- Dependency: `structured_credit` depends on `core` and `valuations` and is enabled via the meta-crate feature `structured_credit`.
- See: `docs/new/09_structured_credit.md` for the full design and APIs.

---




## 7) Pricing & Valuation

### 7.1 Valuation Results

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuationResult {
    pub instrument_id: String,
    pub as_of: time::Date,
    pub value: Amount,  // includes currency
    pub measures: indexmap::IndexMap<String, Decimal>,
    pub cash_flows: Option<CashflowSchedule>,
    pub meta: ResultsMeta,  // from core; includes rounding context
}

impl ValuationResult {
    pub fn with_measure(mut self, name: &str, value: Decimal) -> Self {
        self.measures.insert(name.to_string(), value);
        self
    }
}

// DataFrame flattening (required for collections)
pub struct ValuationRow {
    pub instrument_id: String,
    pub as_of: time::Date,
    pub ccy: Currency,
    pub measure: String,
    pub value: Decimal,
}

pub trait ValuationResultsDfExt {
    /// Long-format DataFrame from a slice of results
    fn to_polars_long(results: &[ValuationResult]) -> polars::prelude::DataFrame;
}

// Common measure names
pub mod measures {
    pub const NPV: &str = "npv";
    pub const PV: &str = "pv";
    pub const CLEAN_PRICE: &str = "clean_price";
    pub const DIRTY_PRICE: &str = "dirty_price";
    pub const ACCRUED_INTEREST: &str = "accrued_interest";
    pub const YTM: &str = "ytm";
    pub const SPREAD: &str = "spread";
    pub const G_SPREAD: &str = "g_spread";       // Govvie spread
    pub const I_SPREAD: &str = "i_spread";       // Interpolated swap spread
    pub const Z_SPREAD: &str = "z_spread";       // Zero-vol spread
    pub const OAS: &str = "oas";                 // Option-adjusted spread
    pub const DURATION: &str = "duration";
    pub const CONVEXITY: &str = "convexity";
    pub const DV01: &str = "dv01";
    pub const CS01: &str = "cs01";
    // Greeks
    pub const DELTA: &str = "delta";
    pub const GAMMA: &str = "gamma";
    pub const VEGA: &str = "vega";
    pub const THETA: &str = "theta";
    pub const RHO: &str = "rho";

}
```

### 7.2 NPV & Discounting

```rust
use finstack_core::prelude::*;

pub fn npv(
    cash_flows: &[Cashflow],
    curve: &DiscountCurve,
    as_of: time::Date,
) -> Result<Decimal, ValuationError> {
    let mut total = Decimal::ZERO;
    
    for cf in cash_flows {
        if cf.date > as_of {
            let df = curve.discount_factor(cf.date)?;
            total += cf.amount.value * df;
        }
    }
    
    Ok(total)
}

pub fn npv_with_credit(
    cash_flows: &[Cashflow],
    discount_curve: &DiscountCurve,
    credit_curve: &CreditCurve,
    as_of: time::Date,
) -> Result<Decimal, ValuationError> {
    // Risky discounting with survival probabilities
    let mut total = Decimal::ZERO;
    
    for cf in cash_flows {
        if cf.date > as_of {
            let df = discount_curve.discount_factor(cf.date)?;
            let sp = credit_curve.survival_probability(cf.date)?;
            total += cf.amount.value * df * sp;
        }
    }
    
    Ok(total)
}
```

### 7.3 Covenant Engine (Evaluation + Consequence Application)

Purpose: deterministically evaluate covenant tests per period using statement nodes and apply consequences to instrument terms and cashflows going forward.

```rust
use finstack_core::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantSpec {
    pub id: CovenantId,
    /// Metric to test, resolved from statements (e.g., "fin.leverage_total", "fin.icr")
    pub metric_node: String,
    /// Comparison operator and threshold, e.g., <= 3.0x or >= 1.5x
    pub test: CovenantTestSpec,
    /// Cure/grace windows
    pub windows: CovenantWindows,
    /// Consequences to apply if breach not cured by window end
    pub consequences: Vec<CovenantConsequence>,
    /// Enable/disable enforcement (scenario toggle)
    #[serde(default)]
    pub enforce: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CovenantTestSpec {
    Leq(Decimal), // metric <= threshold
    Geq(Decimal), // metric >= threshold
    Between { min: Decimal, max: Decimal, inclusive: bool },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CovenantWindows {
    /// Days after breach detection before considered a breach event (grace)
    pub grace_days: i32,
    /// Cure period days after grace to remediate before consequences apply
    pub cure_days: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CovenantConsequence {
    /// Rate step-up in basis points applied prospectively
    RateStepUpBp(Bps),
    /// Cash sweep percentage of excess cash to mandatory prepayment
    CashSweepPct(Decimal),
    /// Block distributions until covenant back in compliance
    DistributionBlock,
    /// Require additional collateral (metadata only; affects LTV/analysis)
    AdditionalCollateral { description: String },
    /// Default path
    Default,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantPeriodResult {
    pub period: PeriodId,
    pub tested_value: Decimal,
    pub passed: bool,
    pub breach_start: Option<time::Date>,
    pub cure_deadline: Option<time::Date>,
    pub effective_consequence: Option<CovenantConsequence>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CovenantReport {
    pub covenant_id: CovenantId,
    pub results: indexmap::IndexMap<PeriodId, CovenantPeriodResult>,
    /// Summary flags for fast checks
    pub ever_breached: bool,
    pub currently_in_breach: bool,
}

pub struct CovenantEngine;

impl CovenantEngine {
    /// Evaluate one covenant across periods using statements results
    pub fn evaluate(
        spec: &CovenantSpec,
        statement_results: &finstack_statements::Results,
    ) -> Result<CovenantReport, ValuationError> {
        // For each period, pull metric_node value, compare with threshold,
        // compute breach windows and effective consequences.
    }

    /// Apply consequences deterministically to an instrument's forward cashflows/terms
    pub fn apply_consequences<I: CashflowProvider + Priceable>(
        instrument: &mut I,
        report: &CovenantReport,
        periods: &[Period],
    ) -> Result<(), ValuationError> {
        // Mutate instrument parameters prospectively:
        // - RateStepUpBp: adjust spread from first uncured breach period end
        // - CashSweepPct: add mandatory principal prepay flows tagged `cash_sweep`
        // - DistributionBlock: suppress distribution/dividend flows
        // - AdditionalCollateral: metadata only
    }
}
```

### 7.9 Yields & Yield-to-Worst (YTW)

```rust
/// Enumerate call/put dates and compute the investor-worst yield.
pub fn yield_to_worst(
    schedule: &CashflowSchedule,
    settlement: time::Date,
    price_clean: Decimal,
    day_count: DayCount,
    ex_coupon_days: Option<i32>,
) -> Result<(Decimal /* ytw */, Option<time::Date> /* exercise */), ValuationError> {
    // Street tie-breakers: equal yields -> earliest exercise date wins.
}
```

Notes:
- Handles clean/dirty conversion and ex-coupon conventions.
- Callable/putable schedules sourced from instrument metadata.

### 7.10 Street Quote Adapters

Provide helpers to echo trade-desk numbers in `ValuationResult` using standard conventions.

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct QuoteConvention {
    pub settlement: time::Date,
    pub ex_coupon_days: i32,
    pub day_count: DayCount,
    pub price_format: PriceFormat,   // Clean | Dirty | Yield
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PriceFormat { Clean, Dirty, Yield }

pub struct QuoteAdapter;

impl QuoteAdapter {
    pub fn accrued_interest(bond: &Bond, settle: time::Date) -> Decimal { /* spec */ }
    pub fn clean_from_dirty(dirty: Decimal, ai: Decimal) -> Decimal { dirty - ai }
    pub fn dirty_from_clean(clean: Decimal, ai: Decimal) -> Decimal { clean + ai }
    pub fn yield_from_price(bond: &Bond, price_clean: Decimal, qc: QuoteConvention) -> Result<Decimal, ValuationError> { /* spec */ }
    pub fn price_from_yield(bond: &Bond, yld: Decimal, qc: QuoteConvention) -> Result<Decimal, ValuationError> { /* spec */ }
}

/// Optional echo into valuation results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuoteEcho {
    pub settlement: time::Date,
    pub ex_coupon_days: i32,
    pub accrued: Decimal,
    pub price_clean: Option<Decimal>,
    pub price_dirty: Option<Decimal>,
    pub yield_pct: Option<Decimal>,
}
```

Results surfacing:

- Valuation outputs include a `covenants` field (see §7.8) and stamp `meta` with enforcement policy notes.
- Portfolio analytics may aggregate covenant statuses across positions.

### 7.4 Pricing Grid Margins (Deterministic Registry)

Common private loan feature: spread depends on leverage or interest coverage buckets.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridMarginSpec {
    /// Metric source in statements (e.g., "fin.leverage_total", "fin.icr")
    pub metric_node: String,
    /// Ordered buckets: first matching wins
    pub buckets: Vec<GridBucket>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GridBucket {
    pub condition: CovenantTestSpec,   // reuse operators
    pub spread_bp: Bps,                // additional spread when in bucket
}

/// Registered deterministic function (FFI‑safe)
/// name: "grid_margin"
/// params schema: GridMarginSpec
/// returns: Bps for the period
```

Implementation:

- Extend floating `InterestType` to allow a `SpreadSchedule::Grid(GridMarginSpec)` variant, which at reset uses `FN_REGISTRY.get("grid_margin")` with the current period metric value resolved from statements to compute the applicable spread.
- Deterministic resolution order; serde params enable Python/WASM parity.

### 7.5 Index Fallback Policy (SOFR floors/fallback)

Add an index fallback for floating legs when the observed index is missing.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexFallbackSpec {
    /// "last_observed" | "policy_rate" | custom via registry name
    pub strategy: String,
    /// Parameters for custom strategies (e.g., policy rate series key)
    #[serde(default)]
    pub params: serde_json::Value,
}

// Registered functions:
// name: "index_fallback_last_observed" (no params)
// name: "index_fallback_policy_rate" (params: { series: String })
// Both return Decimal rate for the reset date deterministically.
```

Integration:

- `InterestType::Floating` uses `IndexFallbackSpec` when index fixings are unavailable; floors/caps apply after fallback resolution.
- Fallback decisions are stamped in `ValuationResult.meta` notes.

### 7.7 Scenario Hooks (Selectors, Threshold Shocks, Enforcement Toggles)

Expose knobs via the scenarios DSL using attribute selectors (see scenarios §2.6):

Examples:

```
# Toggle enforcement off for CCC‑rated
valuations.instruments?{rating:"CCC"}.covenants."Leverage".enforce:=false

# Shock leverage threshold by +0.5x
valuations.instruments?{sector:"Technology"}.covenants."Leverage".threshold:=+0.5

# Change grid margin buckets
valuations.instruments?{seniority:"Senior Secured"}.pricing.grid_margin:=
  { metric_node: "fin.leverage_total", buckets: [ { condition: { Leq: 3.0 }, spread_bp: 275 }, { condition: { Leq: 4.0 }, spread_bp: 325 } ] }
```

Planner behavior:

- Deterministic expansion via selectors; adapter updates instrument covenant specs and pricing grid params; precise cache invalidation: schedules and pricing recomputed for affected instruments.

### 7.8 ValuationResult Extension (Covenant Report)

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValuationResult {
    pub instrument_id: String,
    pub as_of: time::Date,
    pub value: Amount,
    pub measures: indexmap::IndexMap<String, Decimal>,
    pub cash_flows: Option<CashflowSchedule>,
    pub meta: ResultsMeta,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covenants: Option<indexmap::IndexMap<String, CovenantReport>>, // per covenant id
}
```

Stamping policy in `meta`:

- Continue to stamp `meta.fx_policies["valuations"]` as specified in core/overall docs.
- Covenant enforcement status are conveyed via `ValuationResult.covenants` content and instrument `Attributes`/tags; no change to `ResultsMeta` shape is required.

---

## 8) Performance Metrics

### 8.1 XIRR (Extended Internal Rate of Return)

```rust
use finstack_core::math::brent;

pub fn xirr(
    cash_flows: &[(time::Date, Decimal)],
    guess: Option<Decimal>,
) -> Result<Decimal, ValuationError> {
    if cash_flows.len() < 2 {
        return Err(ValuationError::InsufficientCashflows);
    }
    
    // Validate sign changes
    if !has_sign_change(cash_flows) {
        return Err(ValuationError::NoSignChange);
    }
    
    let first_date = cash_flows[0].0;
    
    // NPV function for root finding
    let npv_fn = |rate: f64| -> f64 {
        let mut sum = 0.0;
        for (date, amount) in cash_flows {
            let years = year_fraction(first_date, *date, DayCount::Act365F)
                .unwrap_or(0.0);
            let divisor = (1.0 + rate).powf(years);
            sum += amount.to_f64().unwrap_or(0.0) / divisor;
        }
        sum
    };
    
    // Use core's HybridSolver for Newton-Raphson with Brent fallback
    use finstack_core::math::solver::{HybridSolver, Solver};
    let solver = HybridSolver::new().with_tolerance(1e-6);
    let initial_guess = guess.map(|g| g.to_f64().unwrap()).unwrap_or(0.1);
    let result = solver.solve(npv_fn, initial_guess)?;
    
    Ok(Decimal::from_f64(result).ok_or(ValuationError::NumericError)?)
}
```

---

## 9) Period Aggregation

### 9.1 Currency-Preserving Aggregation

```rust
use finstack_core::prelude::*;

pub fn aggregate_cashflows_by_period(
    cash_flows: &[Cashflow],
    periods: &[Period],
    tags: Option<&TagSet>,
) -> indexmap::IndexMap<PeriodId, indexmap::IndexMap<Currency, Decimal>> {
    let mut result = indexmap::IndexMap::new();
    
    for period in periods {
        let mut period_totals: indexmap::IndexMap<Currency, Decimal> = indexmap::IndexMap::new();
        
        for cf in cash_flows {
            // Check if cash flow falls within period
            if cf.date >= period.start && cf.date <= period.end {
                // Apply tag filter if provided
                if let Some(tag_filter) = tags {
                    if !cf.tags.intersects(tag_filter) {
                        continue;
                    }
                }
                
                // Aggregate by currency
                *period_totals.entry(cf.amount.ccy).or_insert(Decimal::ZERO) += cf.amount.value;
            }
        }
        
        if !period_totals.is_empty() {
            result.insert(period.id.clone(), period_totals);
        }
    }
    
    result
}
```

### 9.2 FX Conversion Policy

Uses the shared `FxConversionPolicy` from `finstack_core::money`.

Defaults:
- Valuations default to `FxConversionPolicy::CashflowDate`.
- Results MUST stamp `ResultsMeta` with an entry like `fx_policies["valuations"] = FxPolicyMeta { strategy: CashflowDate, target_ccy: Some(base_ccy), notes: "" }` when conversion occurs.

```rust
pub fn aggregate_to_base_currency(
    cash_flows: &[Cashflow],
    periods: &[Period],
    fx: &dyn FxProvider,
    base_ccy: Currency,
    policy: finstack_core::money::FxConversionPolicy,
) -> Result<indexmap::IndexMap<PeriodId, Decimal>, ValuationError> {
    let mut result = indexmap::IndexMap::new();
    
    for period in periods {
        let mut period_total = Decimal::ZERO;
        
        for cf in cash_flows {
            if cf.date >= period.start && cf.date <= period.end {
                let fx_date = match policy {
                    finstack_core::money::FxConversionPolicy::CashflowDate => cf.date,
                    finstack_core::money::FxConversionPolicy::PeriodEnd => period.end,
                    finstack_core::money::FxConversionPolicy::PeriodAverage => {
                        // Implementation would calculate average
                        period.end  // Simplified
                    },
                    finstack_core::money::FxConversionPolicy::Custom => period.end,
                };
                
                let rate = fx.rate(cf.amount.ccy, base_ccy, fx_date)?;
                period_total += cf.amount.value * rate;
            }
        }
        
        if !period_total.is_zero() {
            result.insert(period.id.clone(), period_total);
        }
    }
    
    Ok(result)
}
```

---

## 10) Valuation Formulas

Using core's expression engine for complex valuation formulas:

```rust
use finstack_core::prelude::*;

pub struct ValuationContext {
    market: MarketData,
    instrument: Box<dyn Priceable>,
    overrides: indexmap::IndexMap<String, Decimal>,
}

impl ExpressionContext for ValuationContext {
    type Value = Decimal;
    
    fn resolve(&self, name: &str) -> Option<Self::Value> {
        // Check overrides first
        if let Some(value) = self.overrides.get(name) {
            return Some(*value);
        }
        
        // Resolve market data references
        match name {
            "SOFR" => self.market.indices.get(&IndexId("USD-SOFR".into()))
                .and_then(|idx| idx.current_rate()),
            "USD_CURVE" => self.market.discount.get(&CurveId("USD".into()))
                .and_then(|c| c.short_rate()),
            // Add more market data resolutions
            _ => None,
        }
    }
}

// Example formula usage
pub fn evaluate_custom_pricing(
    formula: &str,
    market: &MarketData,
    instrument: Box<dyn Priceable>,
) -> Result<Decimal, ValuationError> {
    let expr = Expr::parse(formula)?;
    let compiled = expr.compile()?;
    let context = ValuationContext {
        market: market.clone(),
        instrument,
        overrides: indexmap::IndexMap::new(),
    };
    
    compiled.eval(&context).map_err(|e| ValuationError::FormulaError(e))
}
```

---

## 10.5 Named Pure Function Registry (Deterministic Custom Logic)

To replace ad‑hoc closures, valuations provides a registry of named, pure functions with serde‑able parameters. This keeps bindings deterministic, snapshot‑testable, and serializable across language boundaries.

```rust
/// Registry of deterministic functions used by instruments and policies
pub struct FunctionRegistry {
    // fx policy: (cf, period, params) -> date used for conversion
    fx_policies: indexmap::IndexMap<String, Arc<dyn Fn(&Cashflow, &Period, &serde_json::Value) -> Result<time::Date, ValuationError> + Send + Sync>>,
    // toggle predicates: (market, as_of, params) -> bool
    toggles: indexmap::IndexMap<String, Arc<dyn Fn(&MarketData, time::Date, &serde_json::Value) -> Result<bool, ValuationError> + Send + Sync>>,
    // dscr sweep/policies: (market, as_of, params) -> Decimal sweep_pct
    policies: indexmap::IndexMap<String, Arc<dyn Fn(&MarketData, time::Date, &serde_json::Value) -> Result<Decimal, ValuationError> + Send + Sync>>,
}

impl FunctionRegistry {
    pub fn new() -> Self { /* default built‑ins */ }
    pub fn register_fx_policy<F>(&mut self, name:&str, f:F) where F: Fn(&Cashflow,&Period,&serde_json::Value)->Result<time::Date,ValuationError> + Send + Sync + 'static { /* ... */ }
    pub fn get_fx_policy(&self, name:&str) -> Option<&Arc<dyn Fn(&Cashflow,&Period,&serde_json::Value)->Result<time::Date,ValuationError> + Send + Sync>> { /* ... */ }

    pub fn register_toggle<F>(&mut self, name:&str, f:F) where F: Fn(&MarketData,time::Date,&serde_json::Value)->Result<bool,ValuationError> + Send + Sync + 'static { /* ... */ }
    pub fn get_toggle(&self, name:&str) -> Option<&Arc<dyn Fn(&MarketData,time::Date,&serde_json::Value)->Result<bool,ValuationError> + Send + Sync>> { /* ... */ }

    pub fn register_policy<F>(&mut self, name:&str, f:F) where F: Fn(&MarketData,time::Date,&serde_json::Value)->Result<Decimal,ValuationError> + Send + Sync + 'static { /* ... */ }
    pub fn get_policy(&self, name:&str) -> Option<&Arc<dyn Fn(&MarketData,time::Date,&serde_json::Value)->Result<Decimal,ValuationError> + Send + Sync>> { /* ... */ }
}

/// Global default registry (overrideable in hosts/tests)
pub static FN_REGISTRY: once_cell::sync::Lazy<FunctionRegistry> = once_cell::sync::Lazy::new(FunctionRegistry::new);
```

Adoptions:
- `InterestType::PIKToggle.toggle_conditions` uses `ToggleConditions::Fn(ToggleFnSpec)` to reference registry functions.
- `aggregate_to_base_currency` resolves `FxConversionPolicy::Custom { name, params }` via `FN_REGISTRY`.

---

### 10.1 Named Function Registry (Deterministic, FFI-safe)

To avoid non-serializable closures and ensure deterministic, cross-FFI behavior (Rust/Python/WASM), `/valuations` uses a registry of named, pure functions with serde parameters.

```rust
/// Function registry available at runtime (thread-safe)
pub struct FunctionRegistry;

impl FunctionRegistry {
    pub fn global() -> &'static Self { /* singleton */ }

    // Toggle predicates used in PIKToggle and similar features
    pub fn register_toggle(
        &self,
        name: &str,
        f: fn(&MarketData, time::Date, &serde_json::Value) -> Result<bool, ValuationError>,
    ) -> bool { /* returns false if name exists */ }

    pub fn get_toggle(
        &self,
        name: &str,
    ) -> Option<fn(&MarketData, time::Date, &serde_json::Value) -> Result<bool, ValuationError>> { /* ... */ }

    // FX policy hooks used by FxConversionPolicy::Custom
    pub fn register_fx_policy(
        &self,
        name: &str,
        f: fn(&Cashflow, &Period, &serde_json::Value) -> Result<time::Date, ValuationError>,
    ) -> bool { /* ... */ }

    pub fn get_fx_policy(
        &self,
        name: &str,
    ) -> Option<fn(&Cashflow, &Period, &serde_json::Value) -> Result<time::Date, ValuationError>> { /* ... */ }
}

/// Convenience alias
pub static FN_REGISTRY: &FunctionRegistry = FunctionRegistry::global();
```

Rules:
- All registered functions must be pure, deterministic, and side-effect free.
- Inputs/outputs are fully serde-serializable to guarantee round-trip across FFI.
- Registration is explicit; link-time registration is allowed via features, but runtime manual registration is always supported.

Serde Shapes:
- Call sites carry a stable name and a `serde_json::Value` blob of parameters (e.g., thresholds). Callers validate schema out of band or via helper validators.

Testing:
- Golden tests assert registry-based decisions are deterministic and portable (Python/WASM executions match Rust).

---

## 11) Public API

```rust
// Core re-exports
pub use finstack_core::prelude::*;

// Traits
pub use traits::{CashflowProvider, Priceable, RiskMeasurable};

// Cash flows
pub use cashflow::{Cashflow, CashflowType, CashflowSchedule, TagSet};
pub use cashflow::ScheduleBuilder;

// Market data
pub use market::{MarketData, DiscountCurve, CreditCurve, VolSurface};
pub use market::{CurveId, IndexId, IssuerId, SurfaceId};

// Instruments
pub use instruments::{
    Equity, FxSpot,
    Bond, Loan, InterestRateSwap, CreditDefaultSwap, VanillaOption,
    // Private Credit
    RevolvingCreditFacility, InterestType, AmortizationType, 
    RateSchedule, SpreadSchedule, PIKToggle, ToggleConditions,
    CallSchedule, FeeSpec, FeeType,
};



// Structured Credit instruments and utilities are provided by the separate
// `finstack-structured-credit` crate and re-exported by the meta-crate when
// the `structured_credit` feature is enabled.

// Pricing
pub use pricing::{ValuationResult, npv, npv_with_credit};

// Covenants & policies
pub use covenants::{CovenantSpec, CovenantTestSpec, CovenantWindows, CovenantConsequence, CovenantReport, CovenantEngine};
pub use policy::{GridMarginSpec, GridBucket, IndexFallbackSpec};

// Performance
pub use performance::{xirr};

// Aggregation
pub use aggregation::{aggregate_cashflows_by_period, aggregate_to_base_currency};

// Risk
pub use risk::{RiskReport, Sensitivities, calculate_dv01, calculate_cs01};

// Builder pattern for complex instruments
pub struct InstrumentBuilder<T> {
    instrument: T,
}

impl InstrumentBuilder<Bond> {
    pub fn new(id: &str) -> Self { /* ... */ }
    pub fn issuer(mut self, issuer: IssuerId) -> Self { /* ... */ }
    pub fn maturity(mut self, date: time::Date) -> Self { /* ... */ }
    pub fn coupon(mut self, rate: Rate) -> Self { /* ... */ }
    pub fn build(self) -> Result<Bond, ValuationError> { /* ... */ }
}
```

---

## 12) Testing Strategy

### 12.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    
    #[test]
    fn test_bond_pricing() {
        let bond = Bond {
            id: InstrumentId("TEST-BOND".into()),
            issuer: IssuerId("TEST-ISSUER".into()),
            maturity: date!(2025-12-31),
            coupon_rate: Rate(dec!(0.05)),
            frequency: Frequency::SemiAnnual,
            day_count: DayCount::Act365F,
            notional: Amount::new(dec!(1_000_000), Currency::USD),
            schedule: None,
        };
        
        let market = create_test_market();
        let result = bond.price_with_metrics(&market, date!(2025-01-01), &[MetricId::Ytm])?;
        
        assert_relative_eq!(
            result.value.value.to_f64().unwrap(),
            1_050_000.0,
            epsilon = 0.01
        );
    }


}
```

### 12.2 Property Tests

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_cashflow_aggregation_preserves_total(
            flows in prop::collection::vec(cashflow_strategy(), 1..100),
            periods in period_strategy(),
        ) {
            let aggregated = aggregate_cashflows_by_period(&flows, &periods, None);
            
            // Sum of aggregated should equal sum of original
            let original_sum = sum_cashflows_by_currency(&flows);
            let aggregated_sum = sum_aggregated(&aggregated);
            
            for (ccy, amount) in original_sum {
                assert_eq!(amount, aggregated_sum.get(&ccy).copied().unwrap_or_default());
            }
        }
    }
}
```

### 12.3 Parity Tests

```rust
#[test]
fn test_irs_npv_parity() {
    // Test against known good values from QuantLib or other sources
    let swap = create_test_swap();
    let market = create_market_from_fixture("test_data/market_20250101.json");
    
    let result = swap.price_with_metrics(&market, date!(2025-01-01), &[MetricId::ParRate])?;
    
    // Should match QuantLib within tolerance
    assert_relative_eq!(
        result.measures["npv"].to_f64().unwrap(),
        -12_345.67,  // Expected from QuantLib
        epsilon = 0.01
    );
}
```

### 12.4 Performance Benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_xirr(c: &mut Criterion) {
    let cash_flows = generate_cash_flows(100);
    
    c.bench_function("xirr_100_flows", |b| {
        b.iter(|| xirr(black_box(&cash_flows), None))
    });
}

fn benchmark_curve_interpolation(c: &mut Criterion) {
    let curve = create_large_curve(100);
    let dates = generate_random_dates(1000);
    
    c.bench_function("discount_factor_1000", |b| {
        b.iter(|| {
            for date in &dates {
                black_box(curve.discount_factor(*date));
            }
        })
    });
}

criterion_group!(benches, benchmark_xirr, benchmark_curve_interpolation);
criterion_main!(benches);
```

---

## 13) Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum ValuationError {
    #[error("Core error: {0}")]
    Core(#[from] finstack_core::CoreError),
    
    #[error("Insufficient cash flows for calculation")]
    InsufficientCashflows,
    
    #[error("No sign change in cash flows")]
    NoSignChange,
    
    #[error("Division by zero")]
    DivisionByZero,
    
    #[error("Numeric error")]
    NumericError,
    
    #[error("Market data not found: {0}")]
    MarketDataNotFound(String),
    
    #[error("Invalid instrument specification: {0}")]
    InvalidInstrument(String),
    
    #[error("Formula evaluation error: {0}")]
    FormulaError(String),
    
    #[error("Convergence failed after {0} iterations")]
    ConvergenceFailed(usize),
}
```

---

## 14) Performance Considerations

### 14.1 Caching Strategy

```rust
pub struct ValuationCache {
    discount_factors: lru::LruCache<(CurveId, time::Date), Decimal>,
    accrual_fractions: lru::LruCache<(time::Date, time::Date, DayCount), Decimal>,
    schedules: lru::LruCache<ScheduleKey, Vec<time::Date>>,
}

impl ValuationCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            discount_factors: lru::LruCache::new(capacity),
            accrual_fractions: lru::LruCache::new(capacity),
            schedules: lru::LruCache::new(capacity / 10),
        }
    }
}
```

### 14.2 Vectorization

* Use Polars DataFrames for bulk operations across many instruments
* Leverage SIMD where available through core's math kernels
* Batch curve interpolations for efficiency

### 14.3 Parallelization

```rust
pub fn price_portfolio_parallel<I>(
    instruments: &[I],
    market: &MarketData,
    as_of: time::Date,
) -> Vec<Result<ValuationResult, ValuationError>>
where
    I: Priceable + Sync,
{
    use rayon::prelude::*;
    
    instruments
        .par_iter()
        .map(|inst| inst.price_with_metrics(market, as_of, MetricId::ALL_STANDARD))
        .collect()
}
```

---

## 15) Integration Examples

### 15.1 Basic Usage

```rust
use finstack_valuations::prelude::*;

fn main() -> Result<(), ValuationError> {
    // Build market data
    let market = MarketDataBuilder::new(date!(2025-01-25))
        .add_curve("USD", create_usd_curve())
        .add_index("USD-SOFR", create_sofr_index())
        .fx_rate(Currency::EUR, Currency::USD, dec!(1.08))
        .build()?;
    
    // Create and price a bond
    let bond = InstrumentBuilder::<Bond>::new("AAPL-5Y")
        .issuer(IssuerId("AAPL".into()))
        .maturity(date!(2030-01-25))
        .coupon(Rate(dec!(0.04)))
        .build()?;
    
    let result = bond.price_with_metrics(&market, market.as_of, &[MetricId::Ytm])?;
    println!("NPV: {:?}", result.value);
    println!("YTM: {:?}", result.measures.get("ytm"));
    
    // Calculate XIRR for cash flows
    let flows = vec![
        (date!(2025-01-01), dec!(-100_000)),
        (date!(2025-07-01), dec!(5_000)),
        (date!(2026-01-01), dec!(5_000)),
        (date!(2026-07-01), dec!(105_000)),
    ];
    
    let irr = xirr(&flows, None)?;
    println!("XIRR: {:.2}%", irr * dec!(100));
    
    Ok(())
}
```

### 15.2 With Portfolio Integration

```rust
use finstack_valuations::prelude::*;
use finstack_portfolio::Portfolio;

fn value_portfolio(
    portfolio: &Portfolio,
    market: &MarketData,
) -> Result<indexmap::IndexMap<PositionId, ValuationResult>, ValuationError> {
    let mut results = indexmap::IndexMap::new();
    
    for (pos_id, position) in &portfolio.positions {
        let instrument = position.resolve_instrument()?;
        let result = instrument.price_with_metrics(market, market.as_of, MetricId::ALL_STANDARD)?;
        results.insert(pos_id.clone(), result);
    }
    
    Ok(results)
}
```

---

## 16) Future Enhancements

* **Stochastic volatility models** (Heston, SABR)
* **Monte Carlo pricing** for path-dependent instruments
* **American option pricing** using binomial trees or Longstaff-Schwartz
* **Exotic instruments** (barriers, lookbacks, Asians)
* **Real-time Greeks** with automatic differentiation
* **Structured Credit** with cashflow waterfalls, OC tests, pre-payments/defaults/recovery



---

## 17) Acceptance Criteria

- [ ] All core traits (`CashflowProvider`, `Priceable`, `RiskMeasurable`) implemented
- [ ] Currency-preserving aggregation with property tests
- [ ] XIRR calculations match Excel/QuantLib within 1bp
- [ ] Bond pricing matches Bloomberg/QuantLib within 0.01%
- [ ] IRS DV01 calculations match market standard
- [ ] Option Greeks match Black-Scholes analytical formulas
- [ ] CDS pricing follows ISDA standard model
- [ ] **Private Credit Features:**
  - [ ] Time-varying rate schedules correctly applied
  - [ ] PIK/Cash/Toggle interest calculations accurate
  - [ ] Amortization schedules (fixed %, fixed amount, custom) tested
  - [ ] Call option pricing integrated with loan valuation
  - [ ] Revolving facility commitment/utilization fees calculated correctly
  - [ ] Custom fee schedules properly accrued and paid
- [ ] All public types have serde serialization; no public closures in types
- [ ] Custom predicates/policies use named function registry with serde params (FFI-safe)
- [ ] Performance benchmarks meet targets (< 1ms per vanilla instrument)
- [ ] 90%+ test coverage on core pricing logic

 - [ ] Covenant engine evaluates leverage/ICR/FCCR/DSCR/asset coverage per period using statements
 - [ ] Cure/grace windows modeled; breach detection and windowing deterministic and snapshot-tested
 - [ ] Consequences applied prospectively to pricing/schedules: rate step-ups, cash sweeps, distribution blocks
 - [ ] ValuationResult exposes `covenants` per instrument; portfolio analytics aggregate statuses
 - [ ] Scenario hooks: selectors can toggle enforcement, shock thresholds, and edit grid margins deterministically
 - [ ] DDTL instrument: commitment/ticking fees, draw conditions, expiry, and post-draw accruals



---

**This document defines the complete technical specification for the finstack valuations crate, building on core's infrastructure while providing comprehensive quantitative finance capabilities.**
