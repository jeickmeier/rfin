# Attribution Module

## Overview

The attribution module provides comprehensive **P&L attribution** capabilities for financial instruments, decomposing daily mark-to-market changes into constituent factors such as carry, curve shifts, credit spreads, FX, volatility, model parameters, and market scalars.

P&L attribution answers the critical question: *"Why did my position's value change from T₀ to T₁?"* by isolating the impact of each market factor and model parameter through systematic repricing.

### Key Features

- **Three Attribution Methodologies**: Parallel (independent isolation), Waterfall (sequential), and Metrics-Based (linear approximation)
- **Comprehensive Factor Decomposition**: Nine attribution factors covering all major risk dimensions
- **Deterministic & Currency-Safe**: Uses Decimal arithmetic and explicit FX conversion policies
- **Detailed Breakdowns**: Per-curve, per-tenor, and per-currency-pair detail where applicable
- **Stable JSON Schemas**: Schema-versioned serialization for long-term compatibility
- **DataFrame Exports**: CSV and JSON exports for analysis and reporting

### Attribution Factors

The module decomposes P&L into the following factors:

1. **Carry** - Time decay (theta) and accruals
2. **RatesCurves** - Discount and forward curve shifts (interest rate risk)
3. **CreditCurves** - Hazard curve shifts (credit spread risk)
4. **InflationCurves** - Inflation curve shifts
5. **Correlations** - Base correlation curve changes (structured credit)
6. **Fx** - FX rate changes (translation and exposure effects)
7. **Volatility** - Implied volatility changes (vega risk)
8. **ModelParameters** - Model-specific parameters (prepayment, default, recovery, conversion)
9. **MarketScalars** - Spot prices, dividends, equity/commodity prices, inflation indices

---

## Module Structure

```
attribution/
├── mod.rs               # Module exports and documentation
├── types.rs             # Core data structures (PnlAttribution, AttributionFactor, etc.)
├── spec.rs              # JSON specification framework (AttributionSpec, AttributionEnvelope)
├── parallel.rs          # Parallel attribution methodology
├── waterfall.rs         # Waterfall attribution methodology
├── metrics_based.rs     # Metrics-based attribution (linear approximation)
├── factors.rs           # Factor extraction/restoration logic
├── model_params.rs      # Model parameter snapshot and modification
├── helpers.rs           # Shared utilities (repricing, FX conversion, P&L computation)
└── dataframe.rs         # DataFrame export utilities (CSV, JSON)
```

### File Responsibilities

- **types.rs**: Defines `PnlAttribution`, `AttributionFactor`, `AttributionMethod`, and detailed attribution structs (`RatesCurvesAttribution`, `CreditCurvesAttribution`, etc.)
- **spec.rs**: Schema-versioned JSON specifications for attribution runs and results (`AttributionSpec`, `AttributionEnvelope`, `AttributionResult`)
- **parallel.rs**: Independent factor isolation methodology (may not sum due to cross-effects)
- **waterfall.rs**: Sequential factor application (guarantees sum = total P&L)
- **metrics_based.rs**: Fast linear approximation using pre-computed risk metrics (Theta, DV01, CS01, Vega, etc.)
- **factors.rs**: Market context manipulation (extract/restore rates, credit, FX, vol, scalars)
- **model_params.rs**: Instrument-specific parameter handling (prepayment, default, recovery, conversion)
- **helpers.rs**: Shared repricing and P&L computation utilities
- **dataframe.rs**: Export to CSV/JSON for downstream analysis

---

## Feature Set

### 1. Three Attribution Methodologies

#### Parallel Attribution (Default)

**Algorithm**: Each factor is isolated independently by restoring T₀ values for that factor while keeping all others at T₁. Cross-effects and non-linearities are captured in the residual.

**Advantages**:

- Isolates pure factor impacts
- Suitable for factor-level risk analysis
- Parallelizable (each factor independent)

**Disadvantages**:

- Residual can be non-trivial (5-15% for large moves)
- Factors may not sum to total P&L due to cross-effects

**When to Use**: Factor-level sensitivity analysis, understanding individual risk contributions.

---

#### Waterfall Attribution

**Algorithm**: Factors are applied sequentially in a specified order. Each factor's P&L is computed after applying all previous factors at T₁. Residual is minimal by construction.

**Advantages**:

- Guarantees sum of factors ≈ total P&L (residual < 0.01%)
- Suitable for risk reporting and attribution reconciliation
- Order-dependent (different orders yield different attributions)

**Disadvantages**:

- First factors in order absorb more P&L (order matters)
- Less suitable for factor isolation
- Not parallelizable (sequential by design)

**When to Use**: Risk reporting, P&L reconciliation, regulatory reporting where sum must equal total.

**Default Order**:

1. Carry
2. RatesCurves
3. CreditCurves
4. InflationCurves
5. Correlations
6. Fx
7. Volatility
8. ModelParameters
9. MarketScalars

---

#### Metrics-Based Attribution

**Algorithm**: Uses pre-computed risk metrics (Theta, DV01, CS01, Vega, etc.) to approximate P&L contributions via linear approximation. Supports second-order metrics (Convexity, Gamma, Volga) for improved accuracy.

**Formula (Enhanced)**:

- **Carry**: Theta × Δt
- **RatesCurves**: DV01 × Δr + ½ × Convexity × (Δr)²
- **CreditCurves**: CS01 × Δs + ½ × CS-Gamma × (Δs)²
- **Fx**: FX01 × Δfx
- **Volatility**: Vega × Δσ + ½ × Volga × (Δσ)² + Vanna × Δspot × Δσ
- **MarketScalars**: Delta × Δspot + ½ × Gamma × (Δspot)²

**Advantages**:

- Fast (no additional repricings required)
- Works with already-computed `ValuationResult` metrics
- Second-order metrics reduce residual from ~18% to <5%
- Graceful degradation (works with or without second-order metrics)

**Disadvantages**:

- Still approximate (third-order+ effects ignored)
- Less accurate than parallel/waterfall for extreme market moves
- Requires metrics to be pre-computed

**When to Use**: High-frequency attribution, screening, or when full repricing is too expensive.

---

### 2. Detailed Breakdowns

Attribution results can include detailed breakdowns:

- **RatesCurvesAttribution**: Per-curve and per-tenor P&L, discount vs. forward totals
- **CreditCurvesAttribution**: Per-curve and per-tenor credit spread P&L
- **InflationCurvesAttribution**: Per-curve inflation P&L (optional tenor detail)
- **CorrelationsAttribution**: Per-curve base correlation P&L
- **FxAttribution**: Per-currency-pair FX P&L
- **VolAttribution**: Per-surface volatility P&L
- **ModelParamsAttribution**: Prepayment, default, recovery, conversion parameter P&L
- **ScalarsAttribution**: Dividends, inflation indices, equity/commodity prices

---

### 3. JSON Specification Framework

The module provides schema-versioned JSON specifications for attribution runs:

```rust
pub struct AttributionEnvelope {
    pub schema: String,  // "finstack.attribution/1"
    pub attribution: AttributionSpec,
}

pub struct AttributionSpec {
    pub instrument: InstrumentJson,
    pub market_t0: MarketContextState,
    pub market_t1: MarketContextState,
    pub as_of_t0: Date,
    pub as_of_t1: Date,
    pub method: AttributionMethod,
    pub config: Option<AttributionConfig>,
}
```

**Benefits**:

- Stable wire formats for long-lived pipelines
- Schema versioning for backward compatibility
- Strict deserialization (deny unknown fields)
- Executable specifications (call `.execute()` on `AttributionSpec`)

---

### 4. Model Parameters Support

The module supports extraction and modification of instrument-specific model parameters:

- **StructuredCredit**: Prepayment, default, recovery models (PSA, SDA, CDR, constant)
- **ConvertibleBond**: Conversion ratio and policies

Parameter snapshots can be extracted, modified, and applied to instruments for attribution:

```rust
let params_t0 = extract_model_params(&instrument);
let params_t1 = extract_model_params(&instrument_at_t1);

let prepay_shift = measure_prepayment_shift(&params_t0, &params_t1);  // in bps
let default_shift = measure_default_shift(&params_t0, &params_t1);    // in bps
```

---

### 5. Currency Safety and FX Policies

Attribution respects Finstack's currency-safety principles:

- All factor P&Ls are in the same currency as `total_pnl`
- Currency validation via `validate_currencies()`
- FX attribution uses explicit conversion with policy stamping
- FX policy metadata recorded in `AttributionMeta::fx_policy`

---

### 6. Residual Validation

Attribution results include residual validation:

```rust
pub fn residual_within_tolerance(&self, pct_tolerance: f64, abs_tolerance: f64) -> bool;
pub fn residual_within_meta_tolerance(&self) -> bool;
```

**Tolerances**:

- `tolerance_pct`: Percentage tolerance (e.g., 0.1 for 0.1%)
- `tolerance_abs`: Absolute tolerance (e.g., 100.0 for $100)

**Typical Residuals** (market-standard targets):

- **Waterfall**: < 0.1% (minimal by construction)
- **Parallel (single factor)**: < 1%
- **Parallel (multiple factors)**: < 5% for normal moves, < 10% for large moves
- **Metrics-Based (with second-order)**: < 5%
- **Metrics-Based (first-order only)**: < 10%

---

## Usage Examples

### Example 1: Basic Parallel Attribution

```rust
use finstack_valuations::attribution::attribute_pnl_parallel;
use finstack_core::config::FinstackConfig;

let attribution = attribute_pnl_parallel(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &FinstackConfig::default(),
)?;

println!("Total P&L: {}", attribution.total_pnl);
println!("Carry: {} ({:.1}%)",
    attribution.carry,
    100.0 * attribution.carry.amount() / attribution.total_pnl.amount()
);
println!("Rates: {}", attribution.rates_curves_pnl);
println!("Credit: {}", attribution.credit_curves_pnl);
println!("FX: {}", attribution.fx_pnl);
println!("Residual: {} ({:.2}%)",
    attribution.residual,
    attribution.meta.residual_pct
);
```

**Output**:

```
Total P&L: 125430.00 USD
Carry: 45000.00 USD (35.8%)
Rates: 65000.00 USD (51.7%)
Credit: 5000.00 USD (4.0%)
FX: 12000.00 USD (9.5%)
Residual: -1570.00 USD (-1.2%)
```

---

### Example 2: Waterfall Attribution with Custom Order

```rust
use finstack_valuations::attribution::{
    attribute_pnl_waterfall, default_waterfall_order, AttributionFactor
};

// Custom order: prioritize credit and FX
let factor_order = vec![
    AttributionFactor::Carry,
    AttributionFactor::CreditCurves,
    AttributionFactor::Fx,
    AttributionFactor::RatesCurves,
    AttributionFactor::Volatility,
];

let attribution = attribute_pnl_waterfall(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &FinstackConfig::default(),
    factor_order,
)?;

// Residual should be minimal
assert!(attribution.residual_within_tolerance(0.01, 1.0));

// Export to CSV
let csv = attribution.to_csv();
std::fs::write("pnl_attribution.csv", csv)?;
```

---

### Example 3: Metrics-Based Attribution

```rust
use finstack_valuations::attribution::attribute_pnl_metrics_based;
use finstack_valuations::metrics::MetricId;

// Pre-compute valuations with metrics
let metrics = vec![
    MetricId::Theta,
    MetricId::Dv01,
    MetricId::Cs01,
    MetricId::Vega,
    MetricId::Convexity,    // Second-order
    MetricId::CsGamma,      // Second-order
    MetricId::Volga,        // Second-order
];

let val_t0 = instrument.price_with_metrics(&market_t0, as_of_t0, &metrics)?;
let val_t1 = instrument.price_with_metrics(&market_t1, as_of_t1, &metrics)?;

let attribution = attribute_pnl_metrics_based(
    &instrument,
    &market_t0,
    &market_t1,
    &val_t0,
    &val_t1,
    as_of_t0,
    as_of_t1,
)?;

// Metrics-based is fast but approximate
println!("Residual: {:.1}%", attribution.meta.residual_pct);
```

---

### Example 4: JSON Specification

```rust
use finstack_valuations::attribution::{AttributionEnvelope, AttributionSpec};
use finstack_valuations::instruments::json_loader::InstrumentJson;

let spec = AttributionSpec {
    instrument: InstrumentJson::Bond(bond),
    market_t0: market_t0.to_state(),
    market_t1: market_t1.to_state(),
    as_of_t0,
    as_of_t1,
    method: AttributionMethod::Parallel,
    config: None,
};

let envelope = AttributionEnvelope::new(spec);

// Serialize to JSON
let json = envelope.to_string()?;
std::fs::write("attribution_spec.json", json)?;

// Execute attribution from spec
let result = envelope.execute()?;
println!("Attribution completed: {} repricings",
    result.attribution.meta.num_repricings
);
```

---

### Example 5: Detailed Rates Breakdown

```rust
let attribution = attribute_pnl_parallel(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &FinstackConfig::default(),
)?;

// Access detailed rates attribution
if let Some(rates_detail) = &attribution.rates_detail {
    println!("Discount Total: {}", rates_detail.discount_total);
    println!("Forward Total: {}", rates_detail.forward_total);

    println!("\nPer-Curve Breakdown:");
    for (curve_id, pnl) in &rates_detail.by_curve {
        println!("  {}: {}", curve_id, pnl);
    }

    println!("\nPer-Tenor Breakdown:");
    for ((curve_id, tenor), pnl) in &rates_detail.by_tenor {
        println!("  {} {}: {}", curve_id, tenor, pnl);
    }
}
```

**Output**:

```
Discount Total: 50000.00 USD
Forward Total: 15000.00 USD

Per-Curve Breakdown:
  USD-OIS: 50000.00 USD
  EUR-OIS: 15000.00 USD

Per-Tenor Breakdown:
  USD-OIS 2Y: 10000.00 USD
  USD-OIS 5Y: 25000.00 USD
  USD-OIS 10Y: 15000.00 USD
  EUR-OIS 5Y: 15000.00 USD
```

---

### Example 6: Human-Readable Explanation Tree

```rust
let attribution = attribute_pnl_parallel(
    &instrument,
    &market_t0,
    &market_t1,
    as_of_t0,
    as_of_t1,
    &FinstackConfig::default(),
)?;

// Generate structured tree explanation
println!("{}", attribution.explain());
```

**Output**:

```
Total P&L: 125430.00 USD
  ├─ Carry: 45000.00 USD (35.8%)
  ├─ Rates Curves: 65000.00 USD (51.7%)
  │   ├─ USD-OIS: 50000.00 USD
  │   └─ EUR-OIS: 15000.00 USD
  ├─ Credit Curves: 5000.00 USD (4.0%)
  ├─ FX: 12000.00 USD (9.5%)
  ├─ Vol: 2000.00 USD (1.6%)
  └─ Residual: -1570.00 USD (-1.2%)
```

---

## How to Add New Features

### Adding a New Attribution Factor

**Example**: Adding a new "Correlation Skew" factor for structured credit.

#### Step 1: Add Factor to `AttributionFactor` Enum

Edit `types.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AttributionFactor {
    Carry,
    RatesCurves,
    CreditCurves,
    InflationCurves,
    Correlations,
    Fx,
    Volatility,
    ModelParameters,
    MarketScalars,
    CorrelationSkew,  // NEW
}
```

Add Display implementation:

```rust
impl std::fmt::Display for AttributionFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing arms ...
            AttributionFactor::CorrelationSkew => write!(f, "CorrelationSkew"),
        }
    }
}
```

#### Step 2: Add Field to `PnlAttribution`

Edit `types.rs`:

```rust
pub struct PnlAttribution {
    pub total_pnl: Money,
    pub carry: Money,
    // ... existing fields ...
    pub correlation_skew_pnl: Money,  // NEW

    // Optional detailed breakdown
    pub correlation_skew_detail: Option<CorrelationSkewAttribution>,  // NEW

    pub meta: AttributionMeta,
}
```

Define detailed attribution struct:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorrelationSkewAttribution {
    pub by_curve: IndexMap<CurveId, Money>,
    pub by_strike: IndexMap<(CurveId, f64), Money>,
}
```

Update `PnlAttribution::new()` to initialize the new field:

```rust
pub fn new(
    total_pnl: Money,
    instrument_id: impl Into<String>,
    t0: Date,
    t1: Date,
    method: AttributionMethod,
) -> Self {
    let zero = Money::new(0.0, total_pnl.currency());

    Self {
        total_pnl,
        carry: zero,
        // ... existing fields ...
        correlation_skew_pnl: zero,  // NEW
        correlation_skew_detail: None,  // NEW
        // ...
    }
}
```

Update `compute_residual()` to include new factor:

```rust
pub fn compute_residual(&mut self) -> Result<()> {
    // Validate currencies first
    self.validate_currencies()?;

    let mut attributed_sum = self.carry;
    attributed_sum = attributed_sum.checked_add(self.rates_curves_pnl)?;
    // ... existing adds ...
    attributed_sum = attributed_sum.checked_add(self.correlation_skew_pnl)?;  // NEW

    self.residual = self.total_pnl.checked_sub(attributed_sum)?;
    // ...
}
```

Update `validate_currencies()` to check new field:

```rust
pub fn validate_currencies(&self) -> Result<()> {
    let expected = self.total_pnl.currency();

    let factors = [
        ("carry", self.carry.currency()),
        // ... existing factors ...
        ("correlation_skew", self.correlation_skew_pnl.currency()),  // NEW
    ];

    for (name, ccy) in &factors {
        if *ccy != expected {
            return Err(Error::CurrencyMismatch { expected, actual: *ccy });
        }
    }

    Ok(())
}
```

#### Step 3: Add Snapshot/Restoration Logic

Edit `factors.rs`:

```rust
#[derive(Debug, Clone)]
pub struct CorrelationSkewSnapshot {
    pub skew_params: HashMap<CurveId, SkewParameters>,
}

pub fn extract_correlation_skew(market: &MarketContext) -> CorrelationSkewSnapshot {
    // Extract skew parameters from market context
    // (implementation depends on how skew is stored in MarketContext)
    CorrelationSkewSnapshot {
        skew_params: HashMap::new(),  // Populate from market
    }
}

pub fn restore_correlation_skew(
    market: &MarketContext,
    snapshot: &CorrelationSkewSnapshot,
) -> MarketContext {
    let mut new_market = market.clone();
    // Apply skew parameters from snapshot
    // (implementation depends on MarketContext API)
    new_market
}
```

#### Step 4: Integrate into Parallel Attribution

Edit `parallel.rs`:

```rust
// In attribute_pnl_parallel()

// Add correlation skew attribution
let correlation_skew_snapshot_t0 = extract_correlation_skew(&market_t0);
let market_skew = restore_correlation_skew(&market_t1, &correlation_skew_snapshot_t0);
let pv_skew = reprice_instrument(instrument, &market_skew, as_of_t1)?;
let correlation_skew_pnl = compute_pnl(pv_skew, val_t1, val_t1.currency(), &market_t1, as_of_t1)?;

attribution.correlation_skew_pnl = correlation_skew_pnl;
num_repricings += 1;

// Optionally populate detailed breakdown
if detailed {
    attribution.correlation_skew_detail = Some(compute_skew_detail(
        instrument, &market_t0, &market_t1, as_of_t1, &config
    )?);
}
```

#### Step 5: Integrate into Waterfall Attribution

Edit `waterfall.rs`:

Update `apply_factor_to_t1()`:

```rust
fn apply_factor_to_t1(
    // ... parameters ...
) -> Result<(MarketContext, Money)> {
    // ... existing code ...

    let new_market = match factor {
        // ... existing cases ...
        AttributionFactor::CorrelationSkew => {
            let skew_t1 = extract_correlation_skew(market_t1);
            restore_correlation_skew(current_market, &skew_t1)
        }
    };

    // ... rest of function ...
}
```

Update `attribute_pnl_waterfall()` to record factor P&L:

```rust
// In apply factor loop
match factor {
    // ... existing cases ...
    AttributionFactor::CorrelationSkew => {
        attribution.correlation_skew_pnl = factor_pnl
    }
}
```

#### Step 6: Update Default Waterfall Order

Edit `waterfall.rs`:

```rust
pub fn default_waterfall_order() -> Vec<AttributionFactor> {
    vec![
        AttributionFactor::Carry,
        AttributionFactor::RatesCurves,
        AttributionFactor::CreditCurves,
        AttributionFactor::InflationCurves,
        AttributionFactor::Correlations,
        AttributionFactor::CorrelationSkew,  // NEW (after Correlations)
        AttributionFactor::Fx,
        AttributionFactor::Volatility,
        AttributionFactor::ModelParameters,
        AttributionFactor::MarketScalars,
    ]
}
```

#### Step 7: Add Metrics-Based Approximation (Optional)

Edit `metrics_based.rs`:

```rust
// In attribute_pnl_metrics_based()

// 10. Correlation skew attribution
if let Some(skew_sensitivity) = val_t0.measures.get("skew_sensitivity") {
    if let Some(surface_id) = instrument.correlation_surface_id() {
        if let Ok(skew_shift) = measure_skew_shift(surface_id.as_str(), market_t0, market_t1) {
            let skew_amount = skew_sensitivity * skew_shift;
            attribution.correlation_skew_pnl = Money::new(skew_amount, val_t1.value.currency());
        }
    }
}
```

#### Step 8: Update DataFrame Exports

Edit `dataframe.rs`:

```rust
pub fn to_csv(&self) -> String {
    let mut lines = Vec::new();

    // Update header
    lines.push(
        "instrument_id,currency,total,carry,rates_curves,credit_curves,\
         inflation_curves,correlations,correlation_skew,fx,vol,model_params,\
         market_scalars,residual,residual_pct".to_string(),
    );

    // Update data row
    lines.push(format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        self.meta.instrument_id,
        self.total_pnl.currency(),
        self.total_pnl.amount(),
        self.carry.amount(),
        self.rates_curves_pnl.amount(),
        self.credit_curves_pnl.amount(),
        self.inflation_curves_pnl.amount(),
        self.correlations_pnl.amount(),
        self.correlation_skew_pnl.amount(),  // NEW
        self.fx_pnl.amount(),
        self.vol_pnl.amount(),
        self.model_params_pnl.amount(),
        self.market_scalars_pnl.amount(),
        self.residual.amount(),
        self.meta.residual_pct,
    ));

    lines.join("\n")
}
```

Add detailed export method:

```rust
pub fn correlation_skew_detail_to_csv(&self) -> Option<String> {
    self.correlation_skew_detail.as_ref().map(|detail| {
        let mut lines = Vec::new();
        lines.push("instrument_id,curve_id,strike,pnl,currency".to_string());

        for ((curve_id, strike), pnl) in &detail.by_strike {
            lines.push(format!(
                "{},{},{},{},{}",
                self.meta.instrument_id,
                curve_id.as_str(),
                strike,
                pnl.amount(),
                pnl.currency()
            ));
        }

        lines.join("\n")
    })
}
```

#### Step 9: Update `explain()` Method

Edit `types.rs`:

```rust
pub fn explain(&self) -> String {
    let mut lines = Vec::new();

    // ... existing code ...

    // Correlation skew
    if !rc.is_effectively_zero_money(
        self.correlation_skew_pnl.amount(),
        self.correlation_skew_pnl.currency(),
    ) {
        lines.push(format!(
            "  ├─ Correlation Skew: {}",
            fmt(&self.correlation_skew_pnl, &self.total_pnl)
        ));
        if let Some(ref detail) = self.correlation_skew_detail {
            for (curve_id, pnl) in &detail.by_curve {
                lines.push(format!("  │   ├─ {}: {}", curve_id, pnl));
            }
        }
    }

    // ... rest of code ...
}
```

#### Step 10: Write Tests

Add test file `tests/test_correlation_skew_attribution.rs`:

```rust
#[test]
fn test_parallel_correlation_skew() {
    // Create instrument with correlation exposure
    // Create markets with different skew parameters
    // Run attribution
    // Verify correlation_skew_pnl is non-zero
}

#[test]
fn test_waterfall_includes_skew() {
    // Verify skew is in default order
    let order = default_waterfall_order();
    assert!(order.contains(&AttributionFactor::CorrelationSkew));
}

#[test]
fn test_skew_detail_export() {
    // Create attribution with skew detail
    // Export to CSV
    // Verify CSV contains skew breakdown
}
```

---

### Adding a New Instrument Type to Model Parameters

**Example**: Adding support for "Exotic Option" model parameters.

#### Step 1: Define Parameter Snapshot

Edit `model_params.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ModelParamsSnapshot {
    StructuredCredit { /* ... */ },
    Convertible { /* ... */ },
    ExoticOption {  // NEW
        barrier_type: BarrierType,
        barrier_level: f64,
        rebate: Option<f64>,
    },
    None,
}
```

#### Step 2: Add Extraction Logic

```rust
pub fn extract_model_params(instrument: &Arc<dyn Instrument>) -> ModelParamsSnapshot {
    // ... existing downcasts ...

    // Try downcasting to ExoticOption
    if let Some(exotic) = instrument.as_any().downcast_ref::<ExoticOption>() {
        return ModelParamsSnapshot::ExoticOption {
            barrier_type: exotic.barrier_type,
            barrier_level: exotic.barrier_level,
            rebate: exotic.rebate,
        };
    }

    // ... rest of code ...
}
```

#### Step 3: Add Modification Logic

```rust
pub fn with_model_params(
    instrument: &Arc<dyn Instrument>,
    params: &ModelParamsSnapshot,
) -> Result<Arc<dyn Instrument>> {
    match params {
        // ... existing cases ...

        ModelParamsSnapshot::ExoticOption {
            barrier_type,
            barrier_level,
            rebate,
        } => {
            if let Some(exotic) = instrument.as_any().downcast_ref::<ExoticOption>() {
                let mut modified = exotic.clone();
                modified.barrier_type = *barrier_type;
                modified.barrier_level = *barrier_level;
                modified.rebate = *rebate;
                Ok(Arc::new(modified) as Arc<dyn Instrument>)
            } else {
                Err(Error::Validation(
                    "Instrument type mismatch: expected ExoticOption".to_string(),
                ))
            }
        }

        // ... rest of code ...
    }
}
```

#### Step 4: Add Measurement Functions

```rust
pub fn measure_barrier_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::ExoticOption { barrier_level: b0, .. },
            ModelParamsSnapshot::ExoticOption { barrier_level: b1, .. },
        ) => {
            // Return shift in percentage points
            ((b1 - b0) / b0) * 100.0
        }
        _ => 0.0,
    }
}
```

#### Step 5: Update Tests

```rust
#[test]
fn test_extract_exotic_option_params() {
    let exotic = ExoticOption::new(
        "EXOTIC-001",
        BarrierType::UpAndOut,
        100.0,
        Some(5.0),
    );
    let instrument = Arc::new(exotic) as Arc<dyn Instrument>;

    let params = extract_model_params(&instrument);

    match params {
        ModelParamsSnapshot::ExoticOption { barrier_level, .. } => {
            assert_eq!(barrier_level, 100.0);
        }
        _ => panic!("Expected ExoticOption snapshot"),
    }
}
```

---

## Best Practices

### 1. Choose the Right Methodology

- **Parallel**: Factor-level analysis, understanding individual risk contributions
- **Waterfall**: Risk reporting, P&L reconciliation, regulatory compliance
- **Metrics-Based**: High-frequency attribution, screening, performance-critical applications

### 2. Validate Residuals

Always check residual tolerances after attribution:

```rust
if !attribution.residual_within_meta_tolerance() {
    eprintln!("Warning: Residual {} ({:.2}%) exceeds tolerance",
        attribution.residual,
        attribution.meta.residual_pct
    );
}
```

### 3. Use Detailed Breakdowns

For production risk systems, enable detailed breakdowns:

```rust
// Per-tenor rates breakdown helps identify key risk buckets
if let Some(rates_detail) = &attribution.rates_detail {
    for ((curve_id, tenor), pnl) in &rates_detail.by_tenor {
        risk_report.add_bucket(curve_id, tenor, pnl);
    }
}
```

### 4. Persist JSON Specifications

For audit trails and reproducibility:

```rust
// Save specification for later replay
let envelope = AttributionEnvelope::new(spec);
let json = envelope.to_string()?;
std::fs::write(format!("attribution_{}_{}.json", as_of_t0, as_of_t1), json)?;
```

### 5. Monitor Attribution Quality

Track residual statistics over time:

```rust
metrics_collector.record_gauge(
    "attribution.residual_pct",
    attribution.meta.residual_pct,
    &[("instrument", instrument.id()), ("method", &attribution.meta.method.to_string())]
);
```

---

## Performance Considerations

### Parallel Attribution

- **Repricings**: ~9-11 (one per factor + T₀/T₁)
- **Memory**: Moderate (creates ~9 market snapshots)
- **Time**: ~500ms for complex instruments with full market data

### Waterfall Attribution

- **Repricings**: ~9-11 (one per factor + T₀/T₁)
- **Memory**: Moderate (sequential, so only one market clone at a time)
- **Time**: ~500ms (similar to parallel, but not parallelizable)

### Metrics-Based Attribution

- **Repricings**: 0 (uses pre-computed metrics)
- **Memory**: Low (no market cloning)
- **Time**: ~5-10ms (fast)

**Optimization Tips**:

1. Use metrics-based for daily portfolio-level attribution
2. Use parallel/waterfall for deep-dives and month-end reporting
3. Pre-compute metrics at T₀ and T₁ during valuation runs
4. Consider caching market snapshots for repeated attribution runs

---

## Error Handling

Attribution can fail for several reasons:

```rust
match attribute_pnl_parallel(&instrument, &market_t0, &market_t1, as_of_t0, as_of_t1, &config) {
    Ok(attribution) => {
        println!("Attribution successful");
    }
    Err(Error::CurrencyMismatch { expected, actual }) => {
        eprintln!("Currency mismatch: expected {}, got {}", expected, actual);
    }
    Err(Error::MissingCurve { curve_id, .. }) => {
        eprintln!("Missing market data: {}", curve_id);
    }
    Err(e) => {
        eprintln!("Attribution failed: {}", e);
    }
}
```

**Common Error Cases**:

- Missing discount/forward curves at T₀ or T₁
- Missing FX rates for cross-currency instruments
- Currency mismatches in P&L computation
- Invalid instrument state (matured, settled)

---

## Future Enhancements

Planned improvements (tracked in TODOs):

1. **Polars DataFrame Integration**: Native DataFrame exports for time-series analysis
2. **Parallel Execution**: Parallelize factor repricings in parallel attribution
3. **Incremental Attribution**: Efficient attribution for small market moves
4. **Multi-Period Attribution**: Aggregate attribution over multiple periods
5. **Cross-Gamma Terms**: Higher-order cross-effects (e.g., rates-credit interaction)
6. **Portfolio-Level Attribution**: Aggregate attribution across positions
7. **Attribution Caching**: Cache factor snapshots for repeated runs

---

## References

- **Core Types**: `finstack/valuations/src/attribution/types.rs`
- **JSON Specs**: `finstack/valuations/src/attribution/spec.rs`
- **Parallel Implementation**: `finstack/valuations/src/attribution/parallel.rs`
- **Waterfall Implementation**: `finstack/valuations/src/attribution/waterfall.rs`
- **Metrics-Based Implementation**: `finstack/valuations/src/attribution/metrics_based.rs`
- **Factor Utilities**: `finstack/valuations/src/attribution/factors.rs`
- **Model Parameters**: `finstack/valuations/src/attribution/model_params.rs`

---

## Contributing

When adding new features:

1. Follow the pattern established for existing factors
2. Update all three methodologies (parallel, waterfall, metrics-based)
3. Add comprehensive tests (unit + integration)
4. Update DataFrame exports and `explain()` methods
5. Document new parameters in attribution metadata
6. Maintain currency-safety invariants
7. Preserve schema versioning for JSON specs

For questions or feature requests, please open an issue or contact the Finstack team.
