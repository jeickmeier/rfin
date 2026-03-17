# Correlation Models

Shared correlation infrastructure for credit portfolio modeling. Provides copula
models, factor models, stochastic recovery, and joint probability utilities used
by CDS tranche pricing, ABS/CLO/CMBS/RMBS engines, and portfolio credit risk
analytics.

## Module Structure

```
correlation/
├── mod.rs                           # Public re-exports
├── joint_probability.rs             # Correlated Bernoulli (re-exports from finstack_core)
├── factor_model.rs                  # Factor models, Cholesky, correlation matrix validation
├── copula/
│   ├── mod.rs                       # Copula trait, CopulaSpec, quadrature helpers
│   ├── gaussian.rs                  # One-factor Gaussian copula
│   ├── student_t.rs                 # Student-t copula (tail dependence)
│   ├── random_factor_loading.rs     # Random Factor Loading copula (stochastic correlation)
│   └── multi_factor.rs             # Multi-factor Gaussian copula (sector structure)
└── recovery/
    ├── mod.rs                       # RecoveryModel trait, RecoverySpec
    ├── constant.rs                  # Fixed recovery rate
    └── correlated.rs               # Market-correlated stochastic recovery (Andersen-Sidenius)
```

## Features

### Copula Models (`copula/`)

All copulas implement the `Copula` trait, providing a unified interface for
conditional default probability, factor-space integration, and tail dependence.

#### One-Factor Gaussian (`gaussian.rs`)

The industry-standard model for credit index tranche pricing. Assumes a single
systematic factor drives all defaults through a latent variable.

**Latent variable:**

```
Aᵢ = √ρ · Z + √(1-ρ) · εᵢ
```

**Conditional default probability:**

```
P(default | Z) = Φ((Φ⁻¹(PD) - √ρ · Z) / √(1-ρ))
```

| Property            | Value                                    |
|---------------------|------------------------------------------|
| Factors             | 1 (systematic Z)                         |
| Tail dependence     | λ_L = 0                                  |
| Integration         | Gauss-Hermite quadrature (default: 20pt) |
| Correlation range   | Clamped to [0.01, 0.99]                  |

**Limitations:** Zero tail dependence underestimates joint extreme events.
Requires base correlation framework to capture the correlation smile.

#### Student-t (`student_t.rs`)

Addresses the zero-tail-dependence limitation of Gaussian by modeling joint
defaults through a shared mixing variable W ~ Gamma(ν/2, ν/2).

**Latent variable:**

```
M  = Z_M / √W     (systematic factor, t(ν)-distributed)
εᵢ = Zᵢ  / √W     (idiosyncratic, same W)
Aᵢ = √ρ · M + √(1-ρ) · εᵢ
```

**Conditional default probability:**

```
P(default | M=m) = t_{ν+1}( (c - √ρ·m) / √(1-ρ) · √((ν+1)/(ν + m²)) )
```

**Tail dependence coefficient:**

```
λ_L = 2 · t_{ν+1}(-√((ν+1)(1-ρ)/(1+ρ)))
```

| Property            | Value                                                     |
|---------------------|-----------------------------------------------------------|
| Factors             | 1 (systematic M)                                          |
| Tail dependence     | λ_L > 0, increases with ρ and decreases with ν            |
| Integration         | Gauss-Laguerre (Gamma) × Gauss-Hermite (inner Gaussian)   |
| Degrees of freedom  | ν > 2 required; typical calibration: ν ∈ [4, 10] for CDX  |
| Convergence         | As ν → ∞, converges to Gaussian copula                    |

#### Random Factor Loading (`random_factor_loading.rs`)

Models correlation as stochastic rather than fixed, capturing the empirical
observation that correlation increases during market stress.

**Latent variable:**

```
β ~ N(β̄, σ²_β)           (random loading, β̄ = √ρ)
Aᵢ = β · Z + √(1-β²) · εᵢ
```

Effective correlation ρ_eff = β² is itself stochastic, producing higher realized
correlation in stress scenarios.

| Property            | Value                                          |
|---------------------|------------------------------------------------|
| Factors             | 2 (market Z, loading shock η)                  |
| Tail dependence     | Implicit positive (from stochastic correlation) |
| Integration         | Gauss-Hermite × Gauss-Hermite (η, Z)           |
| Loading vol range   | Clamped to [0, 0.5]; typical: 0.05–0.20        |
| Loading bounds      | β clamped to [0.01, 0.99]                      |

**Impact on tranches:** Senior tranches are most affected because correlation
uncertainty matters most at the portfolio tail.

#### Multi-Factor Gaussian (`multi_factor.rs`)

Extends the single-factor model with sector-specific factors to capture
intra-sector vs. inter-sector correlation differences.

**Latent variable:**

```
Aᵢ = β_G · Z_G + β_S(i) · Z_S(i) + γᵢ · εᵢ
```

**Correlation structure:**

```
ρᵢⱼ = β_G² + β_S²    (same sector)
ρᵢⱼ = β_G²            (different sectors)
```

| Property            | Value                                              |
|---------------------|----------------------------------------------------|
| Factors             | 1 or 2 (global + sector); capped at 2              |
| Tail dependence     | λ_L = 0 (sum of Gaussians is Gaussian)              |
| Integration         | Nested Gauss-Hermite quadrature (order 5 per dim)   |
| Default loadings    | β_G = 0.4, β_S = 0.3, sector_fraction = 0.4        |
| Variance constraint | β_G² + β_S² ≤ 1 (enforced via clamping)             |

**Use cases:** Bespoke CDOs with sector concentration, portfolios with industry
clustering, systematic vs. sector risk decomposition.

### Factor Models (`factor_model.rs`)

Factor models drive correlated behavior (prepayment + default) through common
systematic factors. All implementations satisfy the `FactorModel` trait.

| Model                 | Factors | Use Case                                   |
|-----------------------|---------|--------------------------------------------|
| `SingleFactorModel`   | 1       | Common market factor                       |
| `TwoFactorModel`      | 2       | Prepayment + credit (RMBS, CLO)            |
| `MultiFactorModel`    | N       | Custom correlation matrix with Cholesky    |

**Linear factor structure:**

```
Aᵢ = β₁·Z₁ + β₂·Z₂ + ... + γᵢ·εᵢ    where γᵢ = √(1 - Σβₖ²)
```

**Standard calibrations:**

| Preset               | Prepay Vol | Credit Vol | Correlation |
|-----------------------|------------|------------|-------------|
| `rmbs_standard()`    | 0.20       | 0.25       | -0.30       |
| `clo_standard()`     | 0.15       | 0.30       | -0.20       |

**Correlation matrix utilities:**

| Function                       | Description                                         |
|--------------------------------|-----------------------------------------------------|
| `validate_correlation_matrix`  | Checks size, unit diagonal, symmetry, bounds, PSD   |
| `cholesky_decompose`           | Returns lower triangular L where Σ = L·Lᵀ           |

The `CorrelationMatrixError` enum provides structured diagnostics: `InvalidSize`,
`DiagonalNotOne`, `NotSymmetric`, `NotPositiveSemiDefinite`, `OutOfBounds`.

### Recovery Models (`recovery/`)

Recovery rate models for credit portfolio pricing, implementing the
`RecoveryModel` trait.

#### Constant Recovery (`constant.rs`)

Fixed recovery rate regardless of market conditions. The baseline model
compatible with standard Gaussian copula pricing.

| Preset              | Rate |
|---------------------|------|
| `isda_standard()`   | 40%  |
| `senior_secured()`  | 55%  |
| `subordinated()`    | 25%  |

#### Market-Correlated Stochastic Recovery (`correlated.rs`)

Recovery inversely correlated with the systematic factor (Andersen-Sidenius),
capturing the "double hit" effect: defaults cluster AND recovery falls
simultaneously in stressed environments.

**Model:**

```
R(Z) = μ_R + ρ_R · σ_R · Z    clamped to [min, max]
```

| Property            | Typical Value                    |
|---------------------|----------------------------------|
| Mean recovery (μ_R) | 40%                              |
| Recovery vol (σ_R)  | 20–30%                           |
| Factor corr (ρ_R)   | -30% to -50% (negative)          |
| Bounds              | [0, 1] default; customizable     |

**Preset calibrations:**

| Preset            | Mean | Vol  | Correlation |
|-------------------|------|------|-------------|
| `market_standard` | 40%  | 25%  | -40%        |
| `conservative`    | 40%  | 30%  | -50%        |

#### RecoverySpec (configuration enum)

Allows recovery model selection via serialization without constructing
the full model:

| Variant           | Status     | Notes                                            |
|-------------------|------------|--------------------------------------------------|
| `Constant`        | Full       | Fixed rate                                       |
| `MarketCorrelated`| Full       | Andersen-Sidenius R(Z) = μ + ρ·σ·Z              |

### Joint Probability (`joint_probability.rs`)

Re-exports from `finstack_core::math::probability`:

| Function                | Description                                     |
|-------------------------|-------------------------------------------------|
| `joint_probabilities`   | Joint default/survival probabilities             |
| `correlation_bounds`    | Fréchet-Hoeffding bounds on joint probability    |
| `CorrelatedBernoulli`   | Correlated Bernoulli random variables            |

## Integration with Pricing Engines

The correlation models integrate with credit index tranche and CDO pricing:

```
Engine
├── copula: Box<dyn Copula>          ← default correlation model
├── recovery: Box<dyn RecoveryModel> ← recovery rate model
├── factor_model: Box<dyn FactorModel>  ← correlated factor sampling
└── ...

Pricing loop (per tranche, per integration point):
  1. Draw systematic factor(s) from copula quadrature
  2. Compute P(default | Z) for each name via copula.conditional_default_prob()
  3. Compute conditional recovery R(Z) if stochastic
  4. Compute conditional tranche loss E[L_tranche | Z]
  5. Integrate over factor space via copula.integrate_fn()
```

## Usage Examples

### Rust

```rust
use finstack_valuations::instruments::common::models::correlation::{
    // Copulas
    GaussianCopula, StudentTCopula, RandomFactorLoadingCopula, MultiFactorCopula,
    Copula, CopulaSpec,
    // Recovery
    ConstantRecovery, CorrelatedRecovery, RecoveryModel, RecoverySpec,
    // Factor models
    SingleFactorModel, TwoFactorModel, MultiFactorModel, FactorModel, FactorSpec,
    // Utilities
    validate_correlation_matrix, cholesky_decompose,
};
use finstack_core::math::standard_normal_inv_cdf;

// --- Gaussian copula (market standard) ---
let copula = GaussianCopula::new();
let pd = 0.05;
let threshold = standard_normal_inv_cdf(pd);
let correlation = 0.30;

// Conditional default probability given Z = -1.5 (stressed market)
let cond_pd = copula.conditional_default_prob(threshold, &[-1.5], correlation);

// Integrate to recover unconditional: E[P(default|Z)] = PD
let integrated_pd = copula.integrate_fn(
    &|z| copula.conditional_default_prob(threshold, z, correlation)
);

// --- Student-t copula (tail dependence) ---
let t_copula = StudentTCopula::new(5.0); // df=5
let tail_dep = t_copula.tail_dependence(0.5); // λ_L > 0

// --- Random Factor Loading (stochastic correlation) ---
let rfl = RandomFactorLoadingCopula::new(0.15); // loading_vol = 15%

// --- Multi-factor with sector structure ---
let mf = MultiFactorCopula::with_loadings(2, 0.4, 0.3);
let inter = mf.inter_sector_correlation(); // β_G² = 0.16
let intra = mf.intra_sector_correlation(); // β_G² + β_S² = 0.25

// --- Copula via spec (serializable configuration) ---
let spec = CopulaSpec::student_t(5.0);
let built = spec.build();

// --- Recovery models ---
let constant = ConstantRecovery::isda_standard(); // 40%
let stochastic = CorrelatedRecovery::market_standard(); // μ=40%, σ=25%, ρ=-40%
let recovery_stressed = stochastic.conditional_recovery(-2.0); // lower in stress
let lgd = stochastic.lgd(); // 1 - E[R] = 0.60

// --- Recovery via spec ---
let r_spec = RecoverySpec::market_standard_stochastic();
let r_model = r_spec.build();

// --- Factor models ---
let two_factor = TwoFactorModel::rmbs_standard(); // RMBS: prepay/credit correlated
let cholesky_l10 = two_factor.cholesky_l10();

// Correlated factor generation from independent normals
let corr_matrix = vec![1.0, 0.6, 0.6, 1.0];
let model = MultiFactorModel::validated(2, vec![0.2, 0.3], corr_matrix).unwrap();
let factors = model.generate_correlated_factors(&[1.0, 0.5]); // L · z scaled by vol

// --- Correlation matrix validation ---
let matrix = vec![1.0, 0.5, 0.3,  0.5, 1.0, 0.4,  0.3, 0.4, 1.0];
validate_correlation_matrix(&matrix, 3).expect("valid 3×3 correlation matrix");
let cholesky_l = cholesky_decompose(&matrix, 3).expect("Cholesky decomposition");
```

## Academic References

| Model / Concept                 | Reference |
|---------------------------------|-----------|
| Gaussian copula                 | Li, D. X. (2000). "On Default Correlation: A Copula Function Approach." *Journal of Fixed Income*, 9(4), 43-54. |
| Student-t copula                | Demarta, S., & McNeil, A. J. (2005). "The t Copula and Related Copulas." *International Statistical Review*, 73(1), 111-129. |
| Student-t valuation             | Hull, J., Predescu, M., & White, A. (2005). "The valuation of correlation-dependent credit derivatives using a structural model." |
| Random Factor Loading           | Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula: Random Recovery and Random Factor Loadings." *Journal of Credit Risk*. |
| Multi-factor CDO                | Andersen, L., Sidenius, J., & Basu, S. (2003). "All Your Hedges in One Basket." *Risk*, November 2003. |
| Multi-factor valuation          | Hull, J., & White, A. (2004). "Valuation of a CDO and an n-th to Default CDS Without Monte Carlo Simulation." |
| Stochastic recovery             | Andersen, L., & Sidenius, J. (2005). "Extensions to the Gaussian Copula." |
| LGD-default correlation         | Altman, E., et al. (2005). "The Link between Default and Recovery Rates." *Journal of Business*, 78(6). |
| Stochastic recovery calibration | Krekel, M., & Stumpp, P. (2006). "Pricing Correlation Products: CDOs." |

## Adding New Features

### Adding a new copula model

1. Create `copula/your_copula.rs` with the module-level doc comment explaining
   the mathematical model, conditional default formula, and integration approach.
2. Implement the `Copula` trait:
   - `conditional_default_prob()` — P(default | factor values, correlation)
   - `integrate_fn()` — quadrature over the factor distribution
   - `num_factors()` — number of systematic factors
   - `tail_dependence()` — lower tail dependence coefficient λ_L
3. Add `mod your_copula` and `pub use` in `copula/mod.rs`.
4. Add a variant to `CopulaSpec` with `build()` support.
5. Add unit tests verifying:
   - Integration recovers unconditional PD: E[P(default|Z)] ≈ PD (critical self-consistency check)
   - Conditional probability is monotone in the factor (negative Z → higher default prob)
   - Extreme correlation handling (near 0 and near 1)
   - Convergence to Gaussian for appropriate parameter limits
6. Re-export from `correlation/mod.rs`.

### Adding a new recovery model

1. Create `recovery/your_model.rs` implementing the `RecoveryModel` trait:
   - `expected_recovery()` — unconditional E[R]
   - `conditional_recovery(market_factor)` — R(Z)
   - `recovery_volatility()` — σ_R (0 for constant models)
2. Add `mod your_model` and `pub use` in `recovery/mod.rs`.
3. Add a variant to `RecoverySpec` with `build()` support.
4. Add unit tests verifying:
   - Mean recovery at Z=0 equals expected recovery
   - Recovery is bounded [0, 1] for all factor values
   - Stochastic models have positive `recovery_volatility()`
   - Zero-volatility or zero-correlation reduces to constant behavior
5. Re-export from `correlation/mod.rs`.

### Adding a new factor model

1. Implement the `FactorModel` trait in `factor_model.rs`:
   - `num_factors()`, `correlation_matrix()`, `volatilities()`, `factor_names()`
   - `diagonal_factor_contribution()` — diagonal-only factor contribution from one standard normal draw
2. Add a variant to `FactorSpec` with `build()` and `num_factors()` support.
3. For multi-factor models, provide `generate_correlated_factors()` using Cholesky.
4. Add unit tests verifying:
   - Correlation matrix reconstruction: L·Lᵀ = Σ
   - Factor generation produces correctly correlated outputs
   - Input validation (clamping, fallback to identity)

### General checklist

- All spec types must derive `Serialize, Deserialize` for configuration persistence.
- Use `#[serde(tag = "type", deny_unknown_fields)]` on spec enums.
- Mark spec enums `#[non_exhaustive]` to allow future variants.
- Clamp inputs to numerically safe ranges (avoid division by zero, NaN, overflow).
- Copula CDF arguments should be clipped to [-10, 10] to prevent overflow.
- Quadrature should be cached in structs for performance (avoid recomputation).
- Python bindings live under `finstack-py/src/valuations/common/`.
- Python stub files live under `finstack-py/finstack/valuations/common/`.

## Testing

Unit tests are co-located in each module. Run with:

```bash
cargo test -p finstack-valuations -- instruments::common::models::correlation
```

### Test coverage highlights

- **Gaussian copula**: integration recovers unconditional PD, conditional probability
  monotone in Z, extreme correlation handling, zero tail dependence.
- **Student-t copula**: tail dependence positive and increases with correlation,
  decreases with df, convergence to Gaussian for high df, integration recovers
  unconditional PD across multiple df and PD values, CDF/inverse CDF round-trip,
  gamma quadrature weight sum ≈ 1.
- **Random Factor Loading**: loading volatility clamping, effective loading bounds,
  integration recovers unconditional PD, zero-volatility reduces to Gaussian,
  tail dependence small but positive.
- **Multi-factor**: correlation decomposition round-trip, intra > inter sector
  correlation, single-factor convergence to Gaussian, loading variance constraint,
  integration recovers unconditional PD.
- **Factor models**: single/two-factor creation, standard calibrations (RMBS, CLO),
  correlation matrix validation (size, diagonal, symmetry, PSD, bounds), Cholesky
  decomposition and reconstruction, correlated factor generation.
- **Constant recovery**: clamping, conditional equals unconditional, LGD, standard
  presets (ISDA, senior secured, subordinated).
- **Correlated recovery**: stress/calm conditional behavior, mean at Z=0, bounding
  to [0, 1], stochastic flag, zero-vol and zero-corr reduce to constant, LGD.
- **RecoverySpec / CopulaSpec**: default values, builder round-trips, clamping,
  build and type checks.

## Numerical Considerations

- **Quadrature order**: Default 20-point Gauss-Hermite matches industry standard
  (QuantLib, Bloomberg use 20–50 points). Multi-factor uses 5 points per dimension
  to keep the tensor product manageable.
- **Correlation boundaries**: All copulas clamp correlation to [0.01, 0.99] to
  avoid division by zero in √(1-ρ) and numerical overflow.
- **CDF clipping**: Normal CDF arguments clipped to [-10, 10] to prevent
  floating-point overflow (norm_cdf(10) ≈ 1 - 7.6e-24).
- **Cholesky fallback**: `MultiFactorModel::new()` falls back to the identity
  matrix on invalid input, logging a warning via `tracing`.
- **Student-t mixing**: The variance-gamma representation uses Gauss-Laguerre
  quadrature for the Gamma(ν/2, ν/2) outer integral with an explicit
  u^{α-1}/Γ(α) weight correction.
