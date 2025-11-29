<!-- c9f58abc-f4a5-42b9-ba00-3af9b903285e ff52521b-291d-4e39-8cc3-6ba39ebb7377 -->
# Bermudan Swaption Implementation Plan

## Summary

Add comprehensive Bermudan swaption support with:

- Hull-White trinomial tree pricing (production standard)
- Enhanced LSMC pricing (leveraging existing infrastructure)
- Model calibration to swaption volatility surface
- Full risk metrics suite

## Phase 1: Type System Extensions

### Step 1.1: Add Bermudan Schedule Types

**File**: [finstack/valuations/src/instruments/swaption/types.rs](finstack/valuations/src/instruments/swaption/types.rs)

Add after existing `SwaptionExercise` enum:

```rust
/// Bermudan exercise schedule specification
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BermudanSchedule {
    /// Exercise dates (sorted, typically on swap coupon dates)
    pub exercise_dates: Vec<Date>,
    /// Lockout period end (no exercise before this date)
    pub lockout_end: Option<Date>,
    /// Notice period in business days
    pub notice_days: u32,
}

/// Co-terminal vs non-co-terminal exercise
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BermudanType {
    /// All exercise dates lead to same swap end date
    #[default]
    CoTerminal,
    /// Exercise dates may have different swap end dates
    NonCoTerminal,
}
```

### Step 1.2: Create BermudanSwaption Struct

**File**: [finstack/valuations/src/instruments/swaption/types.rs](finstack/valuations/src/instruments/swaption/types.rs)

Add new struct that wraps the existing `Swaption`:

```rust
/// Bermudan swaption with multiple exercise dates
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BermudanSwaption {
    /// Unique identifier
    pub id: InstrumentId,
    /// Base swaption parameters (strike, notional, settlement, etc.)
    pub option_type: OptionType,
    pub notional: Money,
    pub strike_rate: f64,
    pub swap_start: Date,
    pub swap_end: Date,
    pub fixed_freq: Frequency,
    pub float_freq: Frequency,
    pub day_count: DayCount,
    pub settlement: SwaptionSettlement,
    pub discount_curve_id: CurveId,
    pub forward_id: CurveId,
    pub vol_surface_id: CurveId,
    /// Bermudan-specific fields
    pub bermudan_schedule: BermudanSchedule,
    pub bermudan_type: BermudanType,
    pub attributes: Attributes,
}
```

Add helper methods for schedule generation and conversion from European swaption.

---

## Phase 2: Hull-White Tree Infrastructure

### Step 2.1: Create Hull-White Tree Module

**New File**: `finstack/valuations/src/instruments/common/models/trees/hull_white_tree.rs`

Core tree structure with calibration to yield curve:

```rust
//! Hull-White trinomial tree for Bermudan swaption pricing.
//!
//! Reference: Hull & White (1994) "Numerical Procedures for Implementing
//! Term Structure Models I: Single-Factor Models"

/// Hull-White 1F tree configuration
#[derive(Clone, Debug)]
pub struct HullWhiteTreeConfig {
    pub kappa: f64,           // Mean reversion (0.01-0.10 typical)
    pub sigma: f64,           // Short rate vol (0.005-0.015 typical)
    pub steps: usize,         // Tree steps (100-200 for production)
    pub adaptive_grid: bool,  // Concentrate steps at exercise dates
}

/// Calibrated Hull-White trinomial tree
pub struct HullWhiteTree {
    config: HullWhiteTreeConfig,
    time_grid: Vec<f64>,
    // x-space nodes: x(t) = r(t) - α(t)
    x_nodes: Vec<Vec<f64>>,
    // α(t) calibrated to match discount curve
    alpha: Vec<f64>,
    // Transition probabilities (pu, pm, pd) per (step, node)
    probs: Vec<Vec<(f64, f64, f64)>>,
    // Arrow-Debreu state prices for verification
    state_prices: Vec<Vec<f64>>,
}
```

Key methods to implement:

- `calibrate()` - Forward induction to fit α(t) to discount curve
- `bond_price(step, node, maturity)` - HW analytical P(t,T)
- `forward_swap_rate(step, node, schedule)` - S(t) from bond prices
- `annuity(step, node, schedule)` - A(t) = Σ τᵢ P(t,Tᵢ)

### Step 2.2: Implement Tree Calibration

In `hull_white_tree.rs`, implement the calibration algorithm:

```rust
impl HullWhiteTree {
    /// Calibrate tree to discount curve using forward induction
    pub fn calibrate(
        config: HullWhiteTreeConfig,
        discount_curve: &dyn Discounting,
        time_to_maturity: f64,
        exercise_dates: Option<&[f64]>,
    ) -> Result<Self> {
        // 1. Build time grid (adaptive if exercise dates provided)
        // 2. Build x-space trinomial lattice with branching:
        //    - Standard: pu=pd=1/6, pm=2/3 (Hull-White default)
        //    - Boundary nodes use adjusted probabilities
        // 3. Forward induction: solve for α(t) at each step
        //    such that Σ Q(t,j) * exp(-r(t,j)*dt) = P(0,t)
        // 4. Store state prices for later use
    }
}
```

### Step 2.3: Register in Trees Module

**File**: [finstack/valuations/src/instruments/common/models/trees/mod.rs](finstack/valuations/src/instruments/common/models/trees/mod.rs)

Add module export and re-exports for Hull-White tree.

---

## Phase 3: Bermudan Tree Valuator

### Step 3.1: Create Swaption Tree Valuator

**New File**: `finstack/valuations/src/instruments/swaption/pricing/tree_valuator.rs`

```rust
//! TreeValuator implementation for Bermudan swaptions

use crate::instruments::common::models::trees::{NodeState, TreeValuator};
use std::collections::HashSet;
use std::sync::Arc;

pub struct BermudanSwaptionTreeValuator {
    swaption: BermudanSwaption,
    hw_tree: Arc<HullWhiteTree>,
    exercise_steps: HashSet<usize>,
    // Remaining swap payment times from each exercise date
    remaining_payments: Vec<Vec<f64>>,
    remaining_accruals: Vec<Vec<f64>>,
}

impl TreeValuator for BermudanSwaptionTreeValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64> {
        // Last exercise opportunity
        if self.exercise_steps.contains(&state.step) {
            self.exercise_value(state)
        } else {
            Ok(0.0)
        }
    }

    fn value_at_node(&self, state: &NodeState, continuation: f64, _dt: f64) -> Result<f64> {
        if self.exercise_steps.contains(&state.step) {
            let exercise = self.exercise_value(state)?;
            Ok(continuation.max(exercise))  // Optimal exercise decision
        } else {
            Ok(continuation)
        }
    }
}
```

### Step 3.2: Create Pricing Module Structure

**New File**: `finstack/valuations/src/instruments/swaption/pricing/mod.rs`

```rust
//! Bermudan swaption pricing engines

pub mod tree_valuator;

pub use tree_valuator::BermudanSwaptionTreeValuator;
```

Update [finstack/valuations/src/instruments/swaption/mod.rs](finstack/valuations/src/instruments/swaption/mod.rs) to include pricing module.

---

## Phase 4: Enhanced LSMC Implementation

### Step 4.1: Enhance Existing Swaption LSMC

**File**: [finstack/valuations/src/instruments/common/models/monte_carlo/pricer/swaption_lsmc.rs](finstack/valuations/src/instruments/common/models/monte_carlo/pricer/swaption_lsmc.rs)

The existing stub has the structure. Enhance with:

1. **Better basis functions** - Add swap rate powers + annuity
2. **Antithetic variates** - Generate (Z, -Z) path pairs
3. **Control variate** - Use European swaption as control
```rust
/// Enhanced swaption LSMC configuration
#[derive(Clone, Debug)]
pub struct SwaptionLsmcConfig {
    pub num_paths: usize,      // 50,000-100,000 typical
    pub seed: u64,
    pub basis_degree: usize,   // Polynomial degree (2-4)
    pub antithetic: bool,      // Variance reduction
    pub control_variate: bool, // Use European as control
}

impl Default for SwaptionLsmcConfig {
    fn default() -> Self {
        Self {
            num_paths: 50_000,
            seed: 42,
            basis_degree: 3,
            antithetic: true,
            control_variate: true,
        }
    }
}
```


### Step 4.2: Add Variance Reduction

Implement control variate method in `swaption_lsmc.rs`:

```rust
/// Price with control variate using European swaption
fn price_with_control_variate(&self, ...) -> Result<MoneyEstimate> {
    // V_cv = V_mc + β(V_analytical - V_mc_euro)
    // where β is regression coefficient
}
```

---

## Phase 5: Model Calibration

### Step 5.1: Create Calibration Module

**New File**: `finstack/valuations/src/calibration/hull_white.rs`

```rust
//! Hull-White model calibration to swaption volatility surface

/// Calibration target specification
#[derive(Clone, Debug)]
pub enum CalibrationTargets {
    /// Co-terminal swaptions (standard for Bermudan pricing)
    CoTerminal { swap_end: Date, expiries: Vec<Date> },
    /// Diagonal swaptions (constant tenor)
    Diagonal { tenor_years: f64, expiries: Vec<f64> },
}

/// Calibration result
#[derive(Clone, Debug)]
pub struct HullWhiteCalibrationResult {
    pub kappa: f64,
    pub sigma: f64,  // Or Vec<(f64, f64)> for time-dependent
    pub rmse_bp: f64,
    pub individual_errors: Vec<(String, f64)>,
}

/// Calibrate Hull-White parameters to swaption market
pub fn calibrate_hull_white_to_swaptions(
    market: &MarketContext,
    targets: CalibrationTargets,
    fix_kappa: Option<f64>,
) -> Result<HullWhiteCalibrationResult> {
    // 1. Build target swaption grid from vol surface
    // 2. Define objective: Σ (model_vol - market_vol)²
    // 3. Optimize using Levenberg-Marquardt (reuse existing solver)
    // 4. Return calibrated parameters + diagnostics
}
```

### Step 5.2: Register Calibration Module

**File**: `finstack/valuations/src/calibration/mod.rs`

Create module if it doesn't exist, add hull_white submodule.

---

## Phase 6: Pricer Integration

### Step 6.1: Add Bermudan Pricer to Swaption Module

**File**: [finstack/valuations/src/instruments/swaption/pricer.rs](finstack/valuations/src/instruments/swaption/pricer.rs)

Add after existing `SimpleSwaptionBlackPricer`:

```rust
/// Bermudan swaption pricer with multiple method support
pub struct BermudanSwaptionPricer {
    method: BermudanPricingMethod,
    hw_params: HullWhite1FParams,
    tree_config: Option<HullWhiteTreeConfig>,
    lsmc_config: Option<SwaptionLsmcConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BermudanPricingMethod {
    #[default]
    HullWhiteTree,
    LSMC,
}

impl Pricer for BermudanSwaptionPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::BermudanSwaption, ModelKey::HullWhite)
    }

    fn price_dyn(&self, instrument: &dyn Instrument, market: &MarketContext, as_of: Date)
        -> Result<ValuationResult, PricingError>
    {
        let swaption = instrument.as_any().downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| PricingError::type_mismatch(...))?;
        
        match self.method {
            BermudanPricingMethod::HullWhiteTree => self.price_tree(swaption, market, as_of),
            BermudanPricingMethod::LSMC => self.price_lsmc(swaption, market, as_of),
        }
    }
}
```

### Step 6.2: Register Instrument Type

**File**: [finstack/valuations/src/pricer.rs](finstack/valuations/src/pricer.rs)

Add `BermudanSwaption` to `InstrumentType` enum.

---

## Phase 7: Risk Metrics

### Step 7.1: Bermudan Greeks

**New File**: `finstack/valuations/src/instruments/swaption/metrics/bermudan_greeks.rs`

Implement bump-and-revalue Greeks:

```rust
/// Delta - parallel rate sensitivity
pub struct BermudanDeltaCalculator;

/// Vega - swaption vol sensitivity  
pub struct BermudanVegaCalculator;

/// Gamma - second-order rate sensitivity
pub struct BermudanGammaCalculator;

/// Exercise probability profile from tree
pub struct ExerciseProbabilityProfile {
    pub exercise_dates: Vec<Date>,
    pub probabilities: Vec<f64>,  // P(exercise at date i | not exercised before)
}
```

### Step 7.2: Register Metrics

**File**: [finstack/valuations/src/instruments/swaption/metrics/mod.rs](finstack/valuations/src/instruments/swaption/metrics/mod.rs)

Add exports for Bermudan-specific metrics.

---

## Phase 8: Testing and Validation

### Step 8.1: Unit Tests

**New File**: `finstack/valuations/tests/swaption/bermudan_tree_test.rs`

- Tree calibration to discount curve
- Bond price accuracy vs analytical HW formula
- Forward swap rate calculation

### Step 8.2: Integration Tests

**New File**: `finstack/valuations/tests/swaption/bermudan_integration_test.rs`

- Tree vs LSMC convergence (same parameters should give similar prices)
- Bermudan → European limit (single exercise date = European)
- Exercise boundary extraction

### Step 8.3: Golden Tests

**New File**: `finstack/valuations/tests/swaption/bermudan_golden_test.rs`

- Compare against known values from QuantLib or Bloomberg
- Regression tests for calibration

---

## File Structure Summary

```
finstack/valuations/src/
├── instruments/swaption/
│   ├── mod.rs                    # Add pricing module
│   ├── types.rs                  # Add BermudanSwaption, BermudanSchedule
│   ├── pricer.rs                 # Add BermudanSwaptionPricer
│   ├── pricing/
│   │   ├── mod.rs                # NEW
│   │   └── tree_valuator.rs      # NEW
│   └── metrics/
│       ├── mod.rs                # Add bermudan exports
│       └── bermudan_greeks.rs    # NEW
├── instruments/common/models/trees/
│   ├── mod.rs                    # Add hull_white_tree export
│   └── hull_white_tree.rs        # NEW
├── calibration/
│   ├── mod.rs                    # NEW (if doesn't exist)
│   └── hull_white.rs             # NEW
└── pricer.rs                     # Add BermudanSwaption instrument type
```

---

## Implementation Dependencies

```
Phase 1 (Types) ──────────────────────────┐
                                          │
Phase 2 (HW Tree) ────────────────────────┼──► Phase 6 (Pricer)
                                          │
Phase 3 (Tree Valuator) ──────────────────┤
                                          │
Phase 4 (LSMC) ───────────────────────────┤
                                          │
Phase 5 (Calibration) ────────────────────┘
                                          
Phase 6 (Pricer) ──► Phase 7 (Metrics) ──► Phase 8 (Testing)
```

Phases 2-5 can be parallelized after Phase 1 is complete.

### To-dos

- [ ] Add BermudanSchedule, BermudanType, and BermudanSwaption to types.rs
- [ ] Implement HullWhiteTree with trinomial lattice and yield curve calibration
- [ ] Create BermudanSwaptionTreeValuator implementing TreeValuator trait
- [ ] Enhance SwaptionLsmcPricer with variance reduction techniques
- [ ] Implement HW calibration to swaption volatility surface
- [ ] Create BermudanSwaptionPricer with tree/LSMC dispatch
- [ ] Add Bermudan-specific Greeks and exercise probability metrics
- [ ] Unit tests, integration tests, golden tests for validation