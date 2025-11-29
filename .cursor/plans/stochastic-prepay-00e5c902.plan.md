<!-- 00e5c902-865f-49d6-9b39-5bb4aa05943f dd736f37-ba0b-478e-8b03-22920551cfa3 -->
# Stochastic Prepay/Default Models for Structured Credit

## Overview

Add industry-standard stochastic prepayment and default models to the `structured_credit` module with:

- Factor-driven CPR/CDR models with correlation
- Scenario tree infrastructure (non-recombining for accuracy)
- Shared correlation/copula module (reused with CDS tranche)
- Dual-mode scenarios integration (deterministic shocks + full stochastic)

## Key Files to Modify/Create

**New modules:**

- `common/models/correlation/` - Shared correlation infrastructure
- `structured_credit/components/stochastic/` - Stochastic prepay/default models
- `structured_credit/pricer/stochastic/` - Stochastic pricing engine
- `structured_credit/metrics/stochastic/` - Distribution-based metrics

**Files to modify:**

- [cds_tranche/copula/mod.rs](finstack/valuations/src/instruments/cds_tranche/copula/mod.rs) - Re-export from shared
- [cds_tranche/recovery/mod.rs](finstack/valuations/src/instruments/cds_tranche/recovery/mod.rs) - Re-export from shared
- [structured_credit/types.rs](finstack/valuations/src/instruments/structured_credit/types.rs) - Add stochastic specs
- [scenarios/src/spec.rs](finstack/scenarios/src/spec.rs) - Add correlation shock operations

---

## Phase 1: Shared Correlation Module

Move copula and recovery models to a shared location for reuse.

### 1.1 Create correlation module structure

Create `common/models/correlation/mod.rs`:

```rust
//! Shared correlation infrastructure for credit modeling.
pub mod copula;
pub mod recovery;
pub mod factor_model;
pub mod joint_probability;
```

### 1.2 Move copula implementations

Move from [cds_tranche/copula/](finstack/valuations/src/instruments/cds_tranche/copula/) to `common/models/correlation/copula/`:

- `traits.rs` - `Copula` trait and `CopulaSpec` enum
- `gaussian.rs` - Gaussian copula
- `student_t.rs` - Student-t copula
- `random_factor_loading.rs` - RFL copula
- `multi_factor.rs` - Multi-factor copula

### 1.3 Move recovery models

Move from [cds_tranche/recovery/](finstack/valuations/src/instruments/cds_tranche/recovery/) to `common/models/correlation/recovery/`:

- `mod.rs` - `RecoveryModel` trait and `RecoverySpec` enum
- `constant.rs` - Constant recovery
- `correlated.rs` - Market-correlated recovery

### 1.4 Add factor model trait

Create `common/models/correlation/factor_model.rs`:

```rust
pub trait FactorModel: Send + Sync {
    fn num_factors(&self) -> usize;
    fn realize_factors(&self, t: f64, rng: &mut dyn RandomStream) -> Vec<f64>;
    fn correlation_matrix(&self) -> &[f64];
}

pub enum FactorSpec {
    SingleFactor { volatility: f64, mean_reversion: f64 },
    TwoFactor { prepay_vol: f64, credit_vol: f64, correlation: f64 },
}
```

### 1.5 Add joint probability utilities

Create `common/models/correlation/joint_probability.rs` - extract the correlated Bernoulli logic from [two_factor_rates_credit.rs](finstack/valuations/src/instruments/common/models/trees/two_factor_rates_credit.rs):

```rust
pub fn joint_probabilities(p1: f64, p2: f64, correlation: f64) -> (f64, f64, f64, f64)
```

### 1.6 Update CDS tranche to re-export

Modify [cds_tranche/copula/mod.rs](finstack/valuations/src/instruments/cds_tranche/copula/mod.rs):

```rust
pub use crate::instruments::common::models::correlation::copula::*;
```

---

## Phase 2: Stochastic Prepayment Models

### 2.1 Create stochastic module structure

Create `structured_credit/components/stochastic/mod.rs`:

```rust
pub mod prepayment;
pub mod default;
pub mod correlation;
pub mod tree;
```

### 2.2 Stochastic prepayment trait

Create `structured_credit/components/stochastic/prepayment/traits.rs`:

```rust
pub trait StochasticPrepayment: Send + Sync {
    fn conditional_smm(&self, seasoning: u32, factors: &[f64], market_rate: f64, burnout: f64) -> f64;
    fn expected_smm(&self, seasoning: u32) -> f64;
    fn factor_loading(&self) -> f64;
}
```

### 2.3 Factor-correlated prepayment

Create `structured_credit/components/stochastic/prepayment/factor_correlated.rs`:

```rust
pub struct FactorCorrelatedPrepay {
    base_spec: PrepaymentModelSpec,
    factor_loading: f64,
    cpr_volatility: f64,
}
// CPR(Z) = base_cpr × exp(β × Z × σ)
```

### 2.4 Richard-Roll model (RMBS)

Create `structured_credit/components/stochastic/prepayment/richard_roll.rs`:

```rust
pub struct RichardRollPrepay {
    base_cpr: f64,
    refi_sensitivity: f64,
    burnout_rate: f64,
    seasonality_amplitude: f64,
}
// CPR = f(refinancing_incentive, seasoning, burnout, seasonality)
```

### 2.5 Stochastic prepayment spec

Create `structured_credit/components/stochastic/prepayment/spec.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "model")]
pub enum StochasticPrepaySpec {
    Deterministic(PrepaymentModelSpec),
    FactorCorrelated { base_spec: PrepaymentModelSpec, factor_loading: f64, cpr_volatility: f64 },
    RichardRoll { base_cpr: f64, refi_sensitivity: f64, burnout_rate: f64 },
    RegimeSwitching { low_cpr: f64, high_cpr: f64, transition_up: f64, transition_down: f64 },
}
```

---

## Phase 3: Stochastic Default Models

### 3.1 Stochastic default trait

Create `structured_credit/components/stochastic/default/traits.rs`:

```rust
pub trait StochasticDefault: Send + Sync {
    fn conditional_mdr(&self, seasoning: u32, factors: &[f64], credit_factors: &CreditFactors) -> f64;
    fn default_distribution(&self, n: usize, pds: &[f64], factors: &[f64], corr: f64) -> Vec<f64>;
    fn correlation(&self) -> f64;
}
```

### 3.2 Copula-based default

Create `structured_credit/components/stochastic/default/copula_based.rs`:

```rust
pub struct CopulaBasedDefault {
    base_cdr: f64,
    copula: Box<dyn Copula>,  // From shared correlation module
    correlation: f64,
}
```

### 3.3 Intensity process (Cox model)

Create `structured_credit/components/stochastic/default/intensity_process.rs`:

```rust
pub struct IntensityProcessDefault {
    base_hazard: f64,
    factor_sensitivity: f64,
    mean_reversion: f64,
    volatility: f64,
}
// λ(t) = λ₀ × exp(β × X(t))
```

### 3.4 Stochastic default spec

Create `structured_credit/components/stochastic/default/spec.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "model")]
pub enum StochasticDefaultSpec {
    Deterministic(DefaultModelSpec),
    Copula { base_cdr: f64, copula_spec: CopulaSpec, correlation: f64 },
    IntensityProcess { base_hazard: f64, factor_sensitivity: f64, mean_reversion: f64, volatility: f64 },
    FactorCorrelated { base_spec: DefaultModelSpec, factor_loading: f64, cdr_volatility: f64 },
}
```

---

## Phase 4: Correlation Structure

### 4.1 Correlation structure spec

Create `structured_credit/components/stochastic/correlation/structure.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "structure")]
pub enum CorrelationStructure {
    Flat { asset_correlation: f64, prepay_default_correlation: f64 },
    Sectored { intra_sector: f64, inter_sector: f64, prepay_default: f64 },
    Matrix { correlations: Vec<f64>, labels: Vec<String> },
}

impl CorrelationStructure {
    pub fn rmbs_standard() -> Self { Flat { asset_correlation: 0.05, prepay_default_correlation: -0.30 } }
    pub fn clo_standard() -> Self { Sectored { intra_sector: 0.30, inter_sector: 0.10, prepay_default: -0.20 } }
}
```

---

## Phase 5: Scenario Tree Infrastructure

### 5.1 Scenario node

Create `structured_credit/components/stochastic/tree/scenario_node.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioNodeId(pub usize);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioNode {
    pub id: ScenarioNodeId,
    pub period: usize,
    pub time: f64,
    pub parent: Option<ScenarioNodeId>,
    pub children: Vec<ScenarioNodeId>,
    pub transition_probability: f64,
    pub cumulative_probability: f64,
    pub factor_realizations: Vec<f64>,
    pub smm: f64,
    pub mdr: f64,
    pub recovery_rate: f64,
    pub pool_balance: f64,
    pub burnout_factor: f64,
    pub seasoning: u32,
}
```

### 5.2 Scenario tree

Create `structured_credit/components/stochastic/tree/scenario_tree.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioTree {
    nodes: Vec<ScenarioNode>,
    pub num_periods: usize,
    pub branching_factor: usize,
    pub num_paths: usize,
}

impl ScenarioTree {
    pub fn build(config: &ScenarioTreeConfig, ...) -> Result<Self>;
    pub fn terminal_paths(&self) -> Vec<ScenarioPath>;
    pub fn expected_value<F>(&self, f: F) -> f64 where F: Fn(&ScenarioNode) -> f64;
    pub fn iter_paths(&self) -> impl Iterator<Item = ScenarioPath>;
}
```

### 5.3 Non-recombining tree builder

Create `structured_credit/components/stochastic/tree/non_recombining.rs`:

```rust
pub struct NonRecombiningTreeBuilder {
    config: ScenarioTreeConfig,
    factor_model: Box<dyn FactorModel>,
    prepay_model: Box<dyn StochasticPrepayment>,
    default_model: Box<dyn StochasticDefault>,
    recovery_model: Box<dyn RecoveryModel>,
    correlation: CorrelationStructure,
}

impl NonRecombiningTreeBuilder {
    pub fn build(&self, rng_seed: u64) -> Result<ScenarioTree>;
}
```

### 5.4 Tree configuration

Create `structured_credit/components/stochastic/tree/config.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioTreeConfig {
    pub num_periods: usize,
    pub horizon_years: f64,
    pub branching: BranchingSpec,
    pub factor_spec: FactorSpec,
    pub prepay_spec: StochasticPrepaySpec,
    pub default_spec: StochasticDefaultSpec,
    pub recovery_spec: RecoverySpec,
    pub correlation: CorrelationStructure,
    pub seed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BranchingSpec {
    Fixed { branches: usize },
    Adaptive { min: usize, max: usize, variance_threshold: f64 },
}
```

---

## Phase 6: Stochastic Pricer

### 6.1 Stochastic pricing result

Create `structured_credit/pricer/stochastic/result.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StochasticValuationResult {
    pub expected_value: Money,
    pub std_deviation: f64,
    pub percentiles: BTreeMap<u8, Money>,  // 1, 5, 25, 50, 75, 95, 99
    pub num_scenarios: usize,
    pub method: StochasticMethod,
    pub base_result: ValuationResult,
}

pub enum StochasticMethod {
    ScenarioTree { branching: usize, periods: usize },
    MonteCarlo { num_paths: usize, seed: u64 },
}
```

### 6.2 Stochastic pricing engine

Create `structured_credit/pricer/stochastic/engine.rs`:

```rust
pub struct StochasticStructuredCreditPricer {
    tree_config: Option<ScenarioTreeConfig>,
    mc_config: Option<MonteCarloConfig>,
}

impl StochasticStructuredCreditPricer {
    pub fn price_with_tree(&self, instrument: &StructuredCredit, ctx: &MarketContext, as_of: Date) -> Result<StochasticValuationResult>;
    pub fn price_monte_carlo(&self, instrument: &StructuredCredit, ctx: &MarketContext, as_of: Date) -> Result<StochasticValuationResult>;
    pub fn generate_scenario_tree(&self, instrument: &StructuredCredit, ctx: &MarketContext, as_of: Date) -> Result<ScenarioTree>;
}
```

### 6.3 Tree-based tranche valuation

Create `structured_credit/pricer/stochastic/tree_valuator.rs` - implements `TreeValuator` from [tree_framework.rs](finstack/valuations/src/instruments/common/models/trees/tree_framework.rs):

```rust
pub struct StructuredCreditTreeValuator { ... }

impl TreeValuator for StructuredCreditTreeValuator {
    fn value_at_maturity(&self, state: &NodeState) -> Result<f64>;
    fn value_at_node(&self, state: &NodeState, continuation: f64, dt: f64) -> Result<f64>;
}
```

---

## Phase 7: Update StructuredCredit Type

### 7.1 Add stochastic specs to StructuredCredit

Modify [structured_credit/types.rs](finstack/valuations/src/instruments/structured_credit/types.rs):

```rust
pub struct StructuredCredit {
    // ... existing fields ...
    
    #[cfg_attr(feature = "serde", serde(default))]
    pub stochastic_prepay_spec: Option<StochasticPrepaySpec>,
    
    #[cfg_attr(feature = "serde", serde(default))]
    pub stochastic_default_spec: Option<StochasticDefaultSpec>,
    
    #[cfg_attr(feature = "serde", serde(default))]
    pub correlation_structure: Option<CorrelationStructure>,
}
```

### 7.2 Add stochastic pricing method

Add to `impl StructuredCredit`:

```rust
pub fn value_stochastic(
    &self,
    context: &MarketContext,
    as_of: Date,
    config: &StochasticConfig,
) -> Result<StochasticValuationResult>;
```

---

## Phase 8: Stochastic Metrics

### 8.1 Expected Loss calculator

Create `structured_credit/metrics/stochastic/expected_loss.rs`:

```rust
pub struct ExpectedLossCalculator;
// EL = Σ P(path) × Loss(path)
```

### 8.2 Unexpected Loss / Credit VaR

Create `structured_credit/metrics/stochastic/unexpected_loss.rs`:

```rust
pub struct UnexpectedLossCalculator { confidence_level: f64 }
// UL = VaR(α) - EL
```

### 8.3 Expected Shortfall

Create `structured_credit/metrics/stochastic/expected_shortfall.rs`:

```rust
pub struct ExpectedShortfallCalculator { confidence_level: f64 }
// ES = E[Loss | Loss > VaR(α)]
```

### 8.4 Correlation sensitivities

Create `structured_credit/metrics/stochastic/correlation_sensitivities.rs`:

```rust
pub struct Correlation01Calculator;  // dV/dρ
pub struct PrepayDefaultCorrelation01Calculator;  // dV/dρ_prepay_default
```

---

## Phase 9: Scenarios Integration

### 9.1 Add scenario shock operations

Modify [scenarios/src/spec.rs](finstack/scenarios/src/spec.rs):

```rust
pub enum OperationSpec {
    // ... existing ...
    
    /// Shock prepayment factor loading
    PrepayFactorLoadingBp { delta_bp: f64 },
    
    /// Shock default correlation
    DefaultCorrelationPts { delta_pts: f64 },
    
    /// Shock prepay-default correlation
    PrepayDefaultCorrelationPts { delta_pts: f64 },
    
    /// Shock recovery correlation with factor
    RecoveryCorrelationPts { delta_pts: f64 },
}
```

### 9.2 Add scenario adapter

Create `scenarios/src/adapters/structured_credit_stochastic.rs`:

```rust
pub fn apply_stochastic_shock(op: &OperationSpec, instrument: &mut StructuredCredit) -> Result<()>;
```

---

## Phase 10: Testing

### 10.1 Unit tests

- Copula conditional probabilities match Li (2000)
- Joint probability conservation (sums to 1)
- Richard-Roll prepayment matches published curves
- Tree probability paths sum to 1
- Backward induction correctness

### 10.2 Golden tests

- 100% PSA deterministic matches current implementation
- Gaussian copula matches CDS tranche implementation
- Known CLO calibration scenarios

### 10.3 Property tests

- Correlation matrix positive semi-definite
- Higher correlation → higher loss volatility
- Expected value bounded by [0, notional]

### To-dos

- [ ] Create shared correlation module and move copula/recovery from cds_tranche
- [ ] Implement stochastic prepayment models (factor-correlated, Richard-Roll)
- [ ] Implement stochastic default models (copula-based, intensity process)
- [ ] Create CorrelationStructure spec with industry presets
- [ ] Build scenario tree infrastructure (nodes, tree, non-recombining builder)
- [ ] Create stochastic pricing engine with tree and MC modes
- [ ] Add stochastic specs to StructuredCredit type
- [ ] Implement stochastic metrics (EL, UL, ES, correlation sensitivities)
- [ ] Add correlation shock operations to scenarios crate
- [ ] Unit tests, golden tests, and property tests