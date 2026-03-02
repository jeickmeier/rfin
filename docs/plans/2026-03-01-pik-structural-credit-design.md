# PIK Structural Credit Pricing Design

**Date:** 2026-03-01
**Status:** Approved
**Scope:** Merton/KMV structural model, endogenous hazard rates, dynamic recovery, Monte Carlo toggle exercise

## Problem Statement

Price the fair value spread difference between three PIK coupon structures for a high-yield credit deal:

1. All fixed coupons paying in cash
2. PIK for the life of the transaction
3. Borrower option to pay cash or PIK (toggle)

A standard reduced-form model with a fixed hazard curve produces only ~5bps differential because it treats default probability as exogenous to the debt structure. Real-world differentials of 100-250bps arise from endogenous credit quality, recovery dilution, funding costs, and adverse selection in toggle exercise.

## Architecture: Compositional Model Layers

Four new composable model modules, one new pricing engine, and Python/WASM bindings.

```
finstack/valuations/src/instruments/common/models/
├── credit/                          ← NEW MODULE
│   ├── mod.rs
│   ├── merton.rs                    ← Merton/KMV structural model
│   ├── endogenous_hazard.rs         ← λ(t) = f(leverage, DD)
│   ├── dynamic_recovery.rs          ← R(t) = g(leverage)
│   └── toggle_exercise.rs           ← PIK toggle decision models

finstack/valuations/src/instruments/fixed_income/bond/pricing/
├── merton_mc_engine.rs              ← NEW: MC orchestrator engine

finstack-py/src/valuations/instruments/credit/
├── mod.rs                           ← NEW: Python bindings
├── merton.rs
├── endogenous_hazard.rs
├── dynamic_recovery.rs
└── toggle_exercise.rs

finstack-py/finstack/valuations/instruments/credit/
├── __init__.py                      ← NEW: Python package
├── merton.pyi                       ← Type stubs
├── endogenous_hazard.pyi
├── dynamic_recovery.pyi
└── toggle_exercise.pyi
```

### Reuse Map

| New Component | Reuses From |
|---------------|-------------|
| Merton calibration | `finstack_core::math::solver::NewtonSolver`, `BrentSolver` |
| Merton → HazardCurve | `HazardCurve::builder()`, existing `HazardBondEngine` |
| Asset path simulation | `finstack_core::math::random::Pcg64Rng`, `RandomNumberGenerator` trait |
| Normal distribution | `finstack_core::math::norm_cdf`, `standard_normal_inv_cdf` |
| Tabular hazard map | `finstack_core::math::interp::InterpStyle` |
| Cashflow generation | `CashFlowSchedule`, `CouponType::Cash/PIK/Split` |
| Risk-free discounting | `DiscountCurve.df()` within MC paths |
| Bond integration | `Bond::from_cashflows()`, existing builder pattern |
| Python bindings | PyO3 wrapper pattern from `stochastic.rs` |
| Metric bumping | `BumpSpec`, `Bumpable` trait for MC sensitivities |

---

## Component 1: Merton/KMV Structural Model

### Core Types

```rust
/// Asset value dynamics specification.
pub enum AssetDynamics {
    /// Standard GBM: dV = (r-q)V dt + σV dW
    GeometricBrownian,

    /// Merton (1976) jump-diffusion: dV/V = (r-q-λκ)dt + σdW + JdN
    /// where N ~ Poisson(λ_J), ln(1+J) ~ N(μ_J, σ_J²)
    JumpDiffusion {
        jump_intensity: f64,  // λ_J (Poisson arrival rate, e.g., 0.1 = ~1 jump/10yr)
        jump_mean: f64,       // μ_J (mean log-jump size, typically negative)
        jump_vol: f64,        // σ_J (jump size volatility)
    },

    /// CreditGrades (JP Morgan 2002): stochastic barrier with closed-form survival.
    /// Default barrier: B(t) = D·exp(λ·√T) where λ captures barrier uncertainty.
    CreditGrades {
        barrier_uncertainty: f64,  // λ (log-normal barrier dispersion, ~0.3)
        mean_recovery: f64,        // Expected recovery rate
    },
}

/// Default barrier specification.
pub enum BarrierType {
    /// Classic Merton: default only at maturity if V(T) < B
    Terminal,

    /// Black-Cox first-passage: default when V(t) first hits B(t) at any t.
    FirstPassage {
        /// Barrier growth rate (B(t) = B₀·exp(g·t), typically g = r)
        barrier_growth_rate: f64,
    },
}

/// Merton structural credit model.
///
/// Models firm value as geometric Brownian motion (with optional jumps):
///   dV = (r - q) V dt + σ_V V dW  (+ J dN for jump-diffusion)
///
/// Default occurs when V crosses the debt barrier B.
pub struct MertonModel {
    asset_value: f64,          // V₀
    asset_vol: f64,            // σ_V (diffusive vol, excluding jumps)
    debt_barrier: f64,         // B (short-term + 0.5 × long-term debt, KMV convention)
    risk_free_rate: f64,       // r
    payout_rate: f64,          // q (dividend yield / asset payout)
    barrier_type: BarrierType,
    dynamics: AssetDynamics,
}
```

### Calibration Constructors

```rust
impl MertonModel {
    /// Direct construction with known asset parameters.
    pub fn new(asset_value: f64, asset_vol: f64, debt_barrier: f64,
               risk_free_rate: f64) -> Result<Self>;

    /// KMV calibration from equity observables.
    /// Solves the simultaneous system using Newton iteration:
    ///   E = V·N(d₁) - B·e^(-rT)·N(d₂)      [equity = call on assets]
    ///   σ_E·E = N(d₁)·σ_V·V                  [equity vol identity]
    /// Reuses: NewtonSolver from finstack_core::math::solver
    pub fn from_equity(equity_value: f64, equity_vol: f64,
                       total_debt: f64, risk_free_rate: f64,
                       maturity: f64) -> Result<Self>;

    /// Calibrate asset vol to match observed CDS spread.
    /// find σ_V such that model_spread(σ_V) = target_spread.
    /// Reuses: BrentSolver from finstack_core::math::solver
    pub fn from_cds_spread(cds_spread_bp: f64, recovery: f64,
                           total_debt: f64, risk_free_rate: f64,
                           maturity: f64) -> Result<Self>;

    /// CreditGrades construction from equity observables.
    /// JP Morgan (2002) model with stochastic barrier uncertainty.
    pub fn credit_grades(equity_value: f64, equity_vol: f64,
                         total_debt: f64, risk_free_rate: f64,
                         barrier_uncertainty: f64,
                         mean_recovery: f64) -> Result<Self>;
```

### Key Methods

```rust
impl MertonModel {
    /// Distance to default: DD = (ln(V/B) + (r-q-σ²/2)T) / (σ√T)
    pub fn distance_to_default(&self, horizon: f64) -> f64;

    /// Default probability at horizon: PD(T) = N(-DD) for terminal barrier.
    /// For first-passage, uses closed-form Black-Cox formula.
    pub fn default_probability(&self, horizon: f64) -> f64;

    /// Credit spread implied by the model: s = -ln(1-PD·(1-R))/T
    pub fn implied_spread(&self, horizon: f64, recovery: f64) -> f64;

    /// MODE A: Generate a HazardCurve compatible with existing engines.
    /// Computes forward default probabilities at tenor grid → piecewise-constant λ(t).
    /// Reuses: HazardCurve::builder()
    pub fn to_hazard_curve(&self, id: &str, base_date: Date,
                           tenors: &[f64], recovery: f64) -> Result<HazardCurve>;

    /// MODE B: Simulate N asset value paths for Monte Carlo pricing.
    /// Reuses: Pcg64Rng, RandomNumberGenerator trait
    pub fn simulate_paths(&self, num_paths: usize, num_steps: usize,
                          horizon: f64, rng: &mut dyn RandomNumberGenerator,
                          antithetic: bool) -> SimulatedPaths;
}
```

### Path Simulation Output

```rust
pub struct SimulatedPaths {
    pub times: Vec<f64>,               // time grid [0, dt, 2dt, ..., T]
    pub asset_values: Vec<Vec<f64>>,   // paths[path_idx][time_idx]
    pub num_paths: usize,
    pub num_steps: usize,
}
```

---

## Component 2: Endogenous Hazard Rate

Maps leverage or credit state to a time-varying hazard rate. This creates the feedback loop: PIK accrual increases leverage, which increases the hazard rate, which increases expected loss.

```rust
/// Specification for endogenous (leverage-dependent) hazard rate.
pub struct EndogenousHazardSpec {
    base_hazard_rate: f64,     // λ₀ at initial leverage
    base_leverage: f64,        // L₀ = debt / assets at issuance
    leverage_hazard_map: LeverageHazardMap,
}

pub enum LeverageHazardMap {
    /// λ(t) = λ₀ × (L(t)/L₀)^β
    /// Empirically calibrated, β ≈ 2-4 for HY credits.
    PowerLaw { exponent: f64 },

    /// λ(t) mapped through Merton DD at current leverage.
    /// λ(t) = -ln(1 - N(-DD(L(t)))) / dt
    MertonImplied { merton: MertonModel },

    /// λ(t) = λ₀ × exp(β × (L(t) - L₀))
    Exponential { sensitivity: f64 },

    /// Empirical calibration from rating transition data.
    /// Reuses: InterpStyle from finstack_core::math::interp
    Tabular { leverage_points: Vec<f64>, hazard_points: Vec<f64> },
}

impl EndogenousHazardSpec {
    /// Compute hazard rate at a given leverage level.
    pub fn hazard_at_leverage(&self, leverage: f64) -> f64;

    /// Convenience: compute hazard after PIK accrual changes the notional.
    pub fn hazard_after_pik_accrual(&self, original_notional: f64,
                                     accreted_notional: f64,
                                     asset_value: f64) -> f64;
}
```

### Integration Point

During MC simulation, at each time step after PIK accrual:

```
leverage(t) = N(t) / V(t)    // accreted notional / asset value
λ(t) = endogenous_hazard.hazard_at_leverage(leverage(t))
survival_step = exp(-λ(t) × dt)
```

---

## Component 3: Dynamic Recovery

Recovery rate as a function of leverage. Captures the empirical fact that recovery declines when total debt is higher relative to the asset base.

```rust
pub struct DynamicRecoverySpec {
    base_recovery: f64,    // R₀ (typically 0.40 for senior unsecured)
    base_notional: f64,    // N₀ (original par at issuance)
    model: RecoveryModel,
}

pub enum RecoveryModel {
    /// Constant recovery (existing behavior, backward compatible).
    Constant,

    /// R(t) = R₀ × (N₀ / N(t))
    /// Direct proportional dilution.
    InverseLinear,

    /// R(t) = R₀ × (N₀ / N(t))^α, α ∈ (0, 1]
    /// α < 1 softens decline (some asset appreciation from retained earnings).
    InversePower { exponent: f64 },

    /// R(t) = max(floor, R₀ × (N₀ / N(t)))
    /// Prevents unrealistically low recovery.
    FlooredInverse { floor: f64 },

    /// R(t) = clamp(R₀ × (1 - β × (N(t)/N₀ - 1)), floor, R₀)
    /// Linear decline as leverage increases above initial.
    LinearDecline { sensitivity: f64, floor: f64 },
}

impl DynamicRecoverySpec {
    /// Compute recovery rate given current accreted notional.
    pub fn recovery_at_notional(&self, current_notional: f64) -> f64;
}
```

---

## Component 4: Toggle Exercise Models

Three models for the borrower's PIK/cash decision at each coupon date.

### Credit State

```rust
/// Observable credit state at a point in time.
pub struct CreditState {
    pub hazard_rate: f64,
    pub distance_to_default: Option<f64>,
    pub leverage: f64,
    pub accreted_notional: f64,
    pub asset_value: Option<f64>,
}

pub enum CreditStateVariable {
    HazardRate,
    DistanceToDefault,
    Leverage,
    Custom { id: String },
}
```

### Toggle Models

```rust
pub enum ToggleExerciseModel {
    /// Hard threshold: PIK when credit metric crosses boundary. DEFAULT.
    Threshold(ThresholdToggle),
    /// Optimal exercise: borrower maximizes equity value.
    OptimalExercise(OptimalToggle),
    /// Stochastic: PIK probability is smooth function of credit state.
    Stochastic(StochasticToggle),
}

pub struct ThresholdToggle {
    pub state_variable: CreditStateVariable,
    pub threshold: f64,
    pub direction: ThresholdDirection,  // Above or Below
}

pub struct StochasticToggle {
    pub state_variable: CreditStateVariable,
    /// P(PIK) = sigmoid(intercept + sensitivity × state)
    pub intercept: f64,
    pub sensitivity: f64,
}

pub struct OptimalToggle {
    /// Nested sub-simulations per decision point. Trade-off: accuracy vs speed.
    pub nested_paths: usize,  // typical: 100-500
    pub equity_discount_rate: f64,
}

impl ToggleExerciseModel {
    /// Returns true if borrower elects to PIK at this coupon date.
    pub fn should_pik(&self, state: &CreditState,
                       rng: &mut dyn RandomNumberGenerator) -> bool;

    /// Returns PIK fraction [0, 1] for partial toggle structures.
    pub fn pik_fraction(&self, state: &CreditState,
                         rng: &mut dyn RandomNumberGenerator) -> f64;
}
```

---

## Component 5: Monte Carlo PIK Pricing Engine

Orchestrates the four components into a single pricing pass.

### Configuration

```rust
pub struct MertonMcConfig {
    pub merton: MertonModel,
    pub endogenous_hazard: Option<EndogenousHazardSpec>,
    pub dynamic_recovery: Option<DynamicRecoverySpec>,
    pub toggle_model: Option<ToggleExerciseModel>,
    pub num_paths: usize,           // default: 10_000
    pub seed: u64,                  // default: 42
    pub antithetic: bool,           // default: true
    pub time_steps_per_year: usize, // default: 12 (monthly)
}
```

### Result

```rust
pub struct MertonMcResult {
    pub npv: Money,
    pub clean_price_pct: f64,
    pub expected_loss: f64,
    pub unexpected_loss: f64,
    pub expected_shortfall_95: f64,
    pub average_pik_fraction: f64,
    pub effective_spread_bp: f64,
    pub path_statistics: PathStatistics,
}

pub struct PathStatistics {
    pub default_rate: f64,
    pub avg_default_time: f64,
    pub avg_terminal_notional: f64,
    pub avg_recovery_pct: f64,
    pub pik_exercise_rate: f64,
}
```

### Simulation Algorithm

```
for each path (parallelizable via stream_id):
    V(0) = merton.asset_value
    N(0) = bond.notional
    defaulted = false
    path_cashflows = []

    for each time step t = dt, 2dt, ..., T:
        # 1. Evolve asset value
        Z = rng.normal(0, 1)
        V(t) = V(t-dt) × exp((r-q-σ²/2)dt + σ√dt·Z)
        if JumpDiffusion:
            n_jumps = rng.poisson(λ_J × dt)
            for j in 0..n_jumps:
                J = exp(μ_J + σ_J × rng.normal(0,1)) - 1
                V(t) *= (1 + J)

        # 2. Check default (first-passage)
        if V(t) < B(t):
            R = dynamic_recovery.recovery_at_notional(N(t))
            path_cashflows.push((t, R × N(t)))
            defaulted = true
            break

        # 3. At coupon dates
        if t is coupon_date:
            coupon_amount = N(t) × coupon_rate × accrual_factor

            # Determine cash vs PIK
            if toggle_model is Some:
                state = CreditState {
                    hazard_rate: endo_hazard.hazard_at_leverage(N(t)/V(t)),
                    distance_to_default: merton.dd_at(V(t), N(t)),
                    leverage: N(t) / V(t),
                    accreted_notional: N(t),
                    asset_value: V(t),
                }
                if toggle_model.should_pik(state, rng):
                    N(t) += coupon_amount  # PIK: accrete
                else:
                    path_cashflows.push((t, coupon_amount))  # Cash
            else:
                # Use bond's CouponType directly
                match bond.coupon_type:
                    Cash => path_cashflows.push((t, coupon_amount))
                    PIK  => N(t) += coupon_amount
                    Split(c, p) => {
                        path_cashflows.push((t, coupon_amount × c))
                        N(t) += coupon_amount × p
                    }

    # 4. Terminal payment (if survived)
    if not defaulted:
        path_cashflows.push((T, N(T)))

    # 5. Discount path cashflows
    path_pv = Σ cashflow × disc.df(t)

# Aggregate across paths
NPV = mean(path_pvs)
EL = mean(losses)
UL = std(losses)
ES = mean(losses | loss > VaR_95)
```

### Bond Integration

```rust
impl Bond {
    /// Price using Merton Monte Carlo engine.
    /// Works with any CouponType (Cash, PIK, Split).
    pub fn price_merton_mc(&self, config: &MertonMcConfig,
                            market: &MarketContext,
                            as_of: Date) -> Result<MertonMcResult>;
}
```

---

## Component 6: Python/WASM Bindings

### Module Structure

```
finstack-py/src/valuations/instruments/credit/
├── mod.rs              ← register() function, module setup
├── merton.rs           ← PyMertonModel, PyAssetDynamics, PyBarrierType
├── endogenous_hazard.rs ← PyEndogenousHazardSpec, PyLeverageHazardMap
├── dynamic_recovery.rs  ← PyDynamicRecoverySpec, PyRecoveryModel
└── toggle_exercise.rs   ← PyToggleExerciseModel, PyCreditState
```

### Binding Pattern (follows existing PyO3 conventions)

```rust
#[pyclass(module = "finstack.valuations.instruments.credit", name = "MertonModel", frozen)]
pub struct PyMertonModel {
    pub(crate) inner: RustMertonModel,
}

#[pymethods]
impl PyMertonModel {
    #[new]
    fn new(asset_value: f64, asset_vol: f64, debt_barrier: f64,
           risk_free_rate: f64, payout_rate: Option<f64>,
           barrier_type: Option<&PyBarrierType>,
           dynamics: Option<&PyAssetDynamics>) -> PyResult<Self>;

    #[classmethod]
    fn from_equity(_cls: &Bound<'_, PyType>, equity_value: f64,
                   equity_vol: f64, total_debt: f64,
                   risk_free_rate: f64, maturity: Option<f64>) -> PyResult<Self>;

    #[classmethod]
    fn from_cds_spread(_cls: &Bound<'_, PyType>, cds_spread_bp: f64,
                       recovery: f64, total_debt: f64,
                       risk_free_rate: f64, maturity: Option<f64>) -> PyResult<Self>;

    fn distance_to_default(&self, horizon: Option<f64>) -> f64;
    fn default_probability(&self, horizon: Option<f64>) -> f64;
    fn to_hazard_curve(&self, curve_id: &str, base_date: NaiveDate,
                       tenors: Option<Vec<f64>>,
                       recovery: Option<f64>) -> PyResult<PyHazardCurve>;
}
```

### Type Stubs (.pyi)

Full type stubs for IDE autocompletion, following the pattern in existing `.pyi` files.

### End-to-End Python Example

```python
from finstack import Bond, CouponType, MertonModel, MertonMcConfig
from finstack import EndogenousHazardSpec, DynamicRecoverySpec, ToggleExerciseModel

# Build Merton model from equity observables
merton = MertonModel.from_equity(
    equity_value=500e6, equity_vol=0.40,
    total_debt=800e6, risk_free_rate=0.04
)

# Mode A: Generate hazard curve for existing engines
hazard = merton.to_hazard_curve("ISSUER-CREDIT", date(2026, 3, 1), recovery=0.40)

# Mode B: Full MC with endogenous effects
endo = EndogenousHazardSpec.power_law(base_hazard=0.10, base_leverage=1.6, exponent=2.5)
dyn_rec = DynamicRecoverySpec.floored_inverse(base_recovery=0.40, base_notional=100e6, floor=0.15)
toggle = ToggleExerciseModel.threshold(variable="hazard_rate", threshold=0.15)

config = MertonMcConfig(merton=merton, endogenous_hazard=endo,
                         dynamic_recovery=dyn_rec, toggle_model=toggle,
                         num_paths=50_000, antithetic=True)

cash_result = cash_bond.price_merton_mc(config, market, as_of)
pik_result = pik_bond.price_merton_mc(config, market, as_of)
toggle_result = toggle_bond.price_merton_mc(config, market, as_of)
```

---

## Testing Strategy

1. **Unit tests per component**: Merton DD/PD against known values, recovery functions, toggle decisions
2. **Integration tests**: Full MC pricing, convergence as num_paths → ∞
3. **Regression tests**: Seeded determinism (same seed = same result)
4. **Boundary tests**: Zero vol → deterministic, zero hazard → risk-free, constant recovery → existing engine
5. **Python parity tests**: Rust MC result matches Python wrapper result
6. **Benchmark tests**: Performance with 10K, 50K, 100K paths

## Implementation Order

1. Merton model (core types + calibration + DD/PD + to_hazard_curve)
2. Endogenous hazard spec
3. Dynamic recovery spec
4. Toggle exercise models (threshold first, then stochastic, then optimal)
5. MC pricing engine (compose all four)
6. Bond integration (price_merton_mc method)
7. Python bindings + type stubs
8. Tests at each layer
