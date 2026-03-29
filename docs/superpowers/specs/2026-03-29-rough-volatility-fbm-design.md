# Rough Volatility & Fractional Brownian Motion Design Spec

**Date**: 2026-03-29
**Status**: Draft
**Scope**: fBM simulation, rBergomi, rough Heston (MC + Fourier), hybrid Cheyette + rough vol

## Overview

Add rough volatility models to the finstack quantitative library. Rough vol (H ~ 0.07-0.12) has become the dominant paradigm in academic volatility modeling, capturing the power-law behavior of realized volatility at short time scales that classical models miss.

This spec covers:
- Fractional Brownian motion simulation infrastructure (Cholesky exact + hybrid scheme)
- rBergomi model (Bayer, Friz, Gatheral 2016) for equity/FX exotic pricing
- Rough Heston model (Gatheral, Jaisson, Rosenbaum 2018) with both MC simulation and Fourier pricing
- Hybrid Cheyette + rough vol for rates derivatives

All components extend the existing `finstack/core` and `finstack/monte_carlo` crates, following established patterns.

## Architecture: Bottom-Up Layered

```text
Layer 5: Hybrid Cheyette + Rough Vol     (monte_carlo/process/)
Layer 4: Rough Heston Fourier Pricing    (core/math/volatility/)
Layer 3: Rough Processes (rBergomi, rHeston MC)  (monte_carlo/process/)
Layer 2: fBM Generators (Cholesky, Hybrid)       (monte_carlo/rng/)
Layer 1: fBM Primitives & Kernel Functions        (core/math/)
```

Each layer depends only on the layers below it. Layers 3-5 all depend on Layer 2 for fBM generation. Layer 4 (Fourier) is independent of Layer 2-3 (MC) — it solves the fractional Riccati ODE directly.

---

## Layer 1: fBM Primitives & Kernel Functions

**Location**: `finstack/core/src/math/fractional.rs`

Pure mathematical building blocks — no simulation, no randomness.

### Hurst Exponent

```rust
/// Validated Hurst exponent H in (0, 1).
/// H < 0.5: rough (anti-persistent), H = 0.5: standard BM, H > 0.5: smooth (persistent).
/// Rough vol models use H in (0, 0.5), typically 0.07-0.12.
#[derive(Debug, Clone, Copy)]
pub struct HurstExponent(f64);

impl HurstExponent {
    pub fn new(h: f64) -> Result<Self>;  // validates (0, 1)
    pub fn value(&self) -> f64;
    pub fn alpha(&self) -> f64;          // H + 0.5, the fractional index
    pub fn is_rough(&self) -> bool;      // H < 0.5
}
```

### Fractional Kernels

```rust
/// Kernel K(t, s) for the Volterra representation of fBM:
/// B_H(t) = integral_0^t K(t, s) dW(s)
pub trait FractionalKernel: Send + Sync {
    fn evaluate(&self, t: f64, s: f64) -> f64;
    fn hurst(&self) -> HurstExponent;
}
```

Concrete implementations:

- **`MolchanGolosovKernel`**: `K(t,s) = c_H * (t-s)^{H-1/2}` — the power-law kernel used in rBergomi. Simple, singular at t=s for H < 0.5. The constant `c_H = sqrt(2H)` normalizes variance.

- **`RiemannLiouvilleKernel`**: The classical fBM kernel with the additional integral term for stationarity of increments. More complex, used when exact fBM distributional properties are needed.

### Covariance Utilities

```rust
/// fBM covariance: Cov(B_H(t), B_H(s)) = 0.5 * (|t|^{2H} + |s|^{2H} - |t-s|^{2H})
pub fn fbm_covariance(t: f64, s: f64, h: f64) -> f64;

/// Build the full n x n covariance matrix for times t_1, ..., t_n
/// Used by the Cholesky fBM generator
pub fn fbm_covariance_matrix(times: &[f64], h: f64) -> DMatrix<f64>;

/// Variance of B_H(t): t^{2H}
pub fn fbm_variance(t: f64, h: f64) -> f64;

/// Covariance of fBM increments: Cov(B_H(t_{i+1})-B_H(t_i), B_H(t_{j+1})-B_H(t_j))
pub fn fbm_increment_covariance(ti: f64, ti1: f64, tj: f64, tj1: f64, h: f64) -> f64;
```

### Mittag-Leffler Function

Required for the rough Heston characteristic function (fractional Riccati ODE).

```rust
/// Generalized Mittag-Leffler function: E_{alpha,beta}(z) = sum_{k=0}^inf z^k / Gamma(alpha*k + beta)
/// Uses series expansion with Horner-like evaluation for |z| < R,
/// and asymptotic expansion for large |z|.
/// Convergence accelerated via Euler summation for alternating series.
pub fn mittag_leffler(z: Complex64, alpha: f64, beta: f64) -> Complex64;

/// Real-valued Mittag-Leffler for negative real arguments (common in rough Heston).
/// Uses the specialized algorithm from Gorenflo, Loutchko, Luchko (2002).
pub fn mittag_leffler_real(x: f64, alpha: f64, beta: f64) -> f64;
```

---

## Layer 2: fBM Generators

**Location**: `finstack/monte_carlo/src/rng/fbm.rs`

### Generator Trait

```rust
/// Generates fractional Brownian motion increments over a time grid.
/// Unlike RandomStream (which produces i.i.d. normals), fBM samples are
/// correlated across time steps -- the generator must see the full grid upfront.
pub trait FractionalNoiseGenerator: Send + Sync {
    /// Generate one full path of fBM increments: dB_H[i] = B_H(t_{i+1}) - B_H(t_i)
    fn generate<R: RandomStream>(&self, rng: &mut R, out: &mut [f64]);

    /// Number of time steps (length of output)
    fn num_steps(&self) -> usize;

    /// Hurst exponent
    fn hurst(&self) -> f64;
}
```

### Cholesky Generator

```rust
/// Exact fBM generation via Cholesky decomposition of the increment covariance matrix.
/// O(n^3) setup (one-time), O(n^2) per path (matrix-vector multiply).
/// Best for validation and short paths (n < ~500).
pub struct CholeskyFbm {
    hurst: f64,
    /// Lower-triangular Cholesky factor of the increment covariance matrix.
    /// Computed once at construction, reused for all paths.
    cholesky_factor: DMatrix<f64>,
    num_steps: usize,
}

impl CholeskyFbm {
    /// Build from a time grid and Hurst exponent.
    /// Computes the n x n increment covariance matrix and its Cholesky factor.
    pub fn new(times: &[f64], hurst: f64) -> Result<Self>;
}
```

Algorithm per path:
1. Draw `n` i.i.d. standard normals Z
2. Compute `L * Z` where L is the cached Cholesky factor
3. Result is a vector of correlated fBM increments with exact distributional properties

### Hybrid Scheme Generator

```rust
/// Hybrid fBM generation (Bennedsen, Lunde, Pakkanen 2017).
/// Splits the Volterra kernel into near-field (exact) and far-field (approximate).
/// O(n * b) per path where b is the near-field window size.
/// Production-grade for long paths.
pub struct HybridFbm {
    hurst: f64,
    num_steps: usize,
    near_field_size: usize,          // b: number of exact recent steps (10-30 typical)
    near_field_cholesky: DMatrix<f64>, // b x b Cholesky factor for near-field
    far_field_weights: Vec<Vec<f64>>, // power-law weights for compressed history
    time_grid: Vec<f64>,
}

impl HybridFbm {
    pub fn new(times: &[f64], hurst: f64, config: HybridFbmConfig) -> Result<Self>;
}

pub struct HybridFbmConfig {
    /// Near-field window size. Default: auto-selected based on H and n.
    /// Larger b = more accurate but slower. Typical range: 10-30.
    pub near_field_size: Option<usize>,
}
```

Algorithm per path:
1. For steps 0..b: use Cholesky on the small b x b block (exact)
2. For steps b..n: near-field contribution from Cholesky of last b steps + far-field contribution as weighted sum of earlier increments grouped into geometric bins
3. The far-field weights approximate the Volterra kernel integral via Riemann sums with power-law decay

### Default Selection

`FbmGeneratorType::Auto` selects Cholesky for n < 200, Hybrid otherwise. This balances the O(n^3) Cholesky setup cost against the approximation error of the hybrid scheme.

---

## Layer 3: Rough Volatility Processes (MC)

### Forward Variance Curve

**Location**: `finstack/core/src/market_data/term_structures/forward_variance.rs`

```rust
/// Forward variance curve xi_0(t) for rough vol models.
/// Represents the market-implied forward variance strip, typically extracted
/// from the vol surface via: xi_0(t) = d/dt [sigma_imp^2(t) * t]
pub struct ForwardVarianceCurve {
    // Uses existing interpolation infrastructure (linear on variance * time)
    interp: PiecewiseLinear,
}

impl ForwardVarianceCurve {
    /// Flat forward variance (constant v0)
    pub fn flat(v0: f64) -> Self;

    /// From a set of (time, forward_variance) pairs
    pub fn from_points(points: &[(f64, f64)]) -> Result<Self>;

    /// Extract from an implied vol surface: xi_0(t) = d/dt [sigma^2(t) * t]
    pub fn from_vol_surface(surface: &VolSurface, atm_strikes: &[f64]) -> Result<Self>;

    /// Evaluate at time t
    pub fn value(&self, t: f64) -> f64;
}
```

### rBergomi Process

**Location**: `finstack/monte_carlo/src/process/rough_bergomi.rs`

Model dynamics:

```text
dS_t = sqrt(V_t) * S_t * dW_t
V_t  = xi_0(t) * exp(eta * W_tilde_H(t) - 0.5 * eta^2 * t^{2H})

W_tilde_H(t) = integral_0^t (t-s)^{H-0.5} dW_tilde(s)   (Volterra fBM)
dW * dW_tilde = rho * dt
```

```rust
pub struct RoughBergomiParams {
    pub r: f64,                        // risk-free rate
    pub q: f64,                        // dividend yield
    pub hurst: HurstExponent,          // H in (0, 0.5), typically 0.07-0.12
    pub eta: f64,                      // vol-of-vol scaling (> 0)
    pub rho: f64,                      // spot-vol correlation in [-1, 1]
    pub xi: ForwardVarianceCurve,      // initial forward variance curve
}

pub struct RoughBergomiProcess {
    params: RoughBergomiParams,
}
```

`StochasticProcess` implementation:
- `dim() = 1` (spot only; variance is functional of fBM, not a diffusive state)
- `num_factors() = 2` (W for spot, W_tilde for Volterra process)
- `populate_path_state`: maps spot to `state_keys::SPOT`, reconstructed V_t to `state_keys::VARIANCE`

```rust
/// Euler discretization for rBergomi.
/// Operates in log-spot for numerical stability.
pub struct RoughBergomiEuler {
    params: RoughBergomiParams,
}
```

Discretization step (receives z = [Z_spot_independent, fbm_increment_i]):
1. Accumulate Volterra integral: `W_tilde_H(t) += fbm_increment_i`
2. Reconstruct variance: `V_t = xi_0(t) * exp(eta * W_tilde_H(t) - 0.5 * eta^2 * t^{2H})`
3. Correlate spot noise: `Z_spot = rho * Z_fbm_underlying + sqrt(1 - rho^2) * Z_spot_independent`
   where `Z_fbm_underlying` is the standard normal that generated `fbm_increment_i` (passed through or reconstructed)
4. Log-spot step: `ln_S += (r - q - V_t/2) * dt + sqrt(V_t * dt) * Z_spot`

Note: The correlation structure requires care. The fBM increment `dB_H` is generated from a linear combination of standard normals (via Cholesky or hybrid). The spot BM must be correlated with the *underlying* standard BM driving the Volterra process, not with the fBM increment directly. The discretization tracks the underlying normal for correlation purposes.

### Rough Heston Process

**Location**: `finstack/monte_carlo/src/process/rough_heston.rs`

Model dynamics:

```text
dS_t = sqrt(V_t) * S_t * dW_t
V_t  = V_0 + (1/Gamma(alpha)) * integral_0^t (t-s)^{alpha-1} * [kappa*(theta - V_s) ds + sigma_v * sqrt(V_s) dW_tilde_s]

alpha = H + 0.5,  H in (0, 0.5)
dW * dW_tilde = rho * dt
```

```rust
pub struct RoughHestonParams {
    pub r: f64,
    pub q: f64,
    pub hurst: HurstExponent,     // H in (0, 0.5)
    pub kappa: f64,               // mean reversion (> 0)
    pub theta: f64,               // long-run variance (> 0)
    pub sigma_v: f64,             // vol of vol (> 0)
    pub rho: f64,                 // spot-vol correlation in [-1, 1]
    pub v0: f64,                  // initial variance (> 0)
}

pub struct RoughHestonProcess {
    params: RoughHestonParams,
}
```

`StochasticProcess` implementation:
- `dim() = 2` (spot + variance)
- `num_factors() = 2`
- `is_diagonal() = false` (correlated BMs)
- `populate_path_state`: `x[0]` -> `SPOT`, `x[1]` -> `VARIANCE`

```rust
/// Hybrid Euler-Maruyama for rough Heston (El Euch & Rosenbaum 2019).
/// The Volterra integral is split into near-field (exact quadrature) and
/// far-field (geometric binning of history).
pub struct RoughHestonHybrid {
    params: RoughHestonParams,
    /// Number of recent steps for exact quadrature
    near_field_steps: usize,
    /// Geometric bin boundaries for compressed far-field history
    bin_boundaries: Vec<usize>,
}
```

Discretization step:
1. Compute Volterra integral contributions:
   - Near-field: exact quadrature `sum_{j=recent} (t_i - t_j)^{alpha-1} * f(V_j) * dt_j` over last `near_field_steps` time points
   - Far-field: weighted sum over geometric bins of older history
2. Update variance: `V_{i+1} = V_0 + (1/Gamma(alpha)) * total_volterra_integral`
3. Truncate variance at zero: `V_{i+1} = max(V_{i+1}, 0)`
4. Correlate BMs: `W_tilde = rho * W + sqrt(1 - rho^2) * W_perp`
5. Log-spot step: `ln_S += (r - q - V_i/2) * dt + sqrt(V_i * dt) * Z_spot`

The discretization stores the full variance history for the Volterra integral. Memory: O(n) per path where n is the number of time steps.

---

## Layer 4: Rough Heston Fourier Pricing

**Location**: `finstack/core/src/math/volatility/rough_heston.rs`

Semi-analytical pricing for European options via the characteristic function. Used for calibration (fast) and as a benchmark for MC.

### Fractional Riccati ODE

The rough Heston characteristic function:

```text
phi(u, t) = exp(C(u,t) + D(u,t) * v0)
```

where D(u, t) solves the fractional Riccati equation:

```text
D^alpha_t D(t) = F(D(t))
F(x) = 0.5*(u^2 - iu) + (iu*rho*sigma_v - kappa)*x + 0.5*sigma_v^2 * x^2

D^alpha is the Caputo fractional derivative of order alpha = H + 0.5
```

and C(u, t) is obtained by integrating D:

```text
C(u, t) = kappa * theta * integral_0^t D(u, s) ds
```

When H = 0.5 (alpha = 1), this reduces to the standard Heston Riccati ODE.

### Adams Scheme Solver

```rust
/// Solves the fractional Riccati ODE via the Adams predictor-corrector method
/// (Diethelm, Ford, Freed 2004) adapted for the rough Heston setting.
pub struct FractionalRiccatiSolver {
    alpha: f64,                    // H + 0.5
    num_steps: usize,              // time discretization points
    time_grid: Vec<f64>,           // non-uniform, refined near t=0
    /// Product integration weights for the fractional integral.
    /// Depend on alpha and the grid only, not on u -- cached and reused.
    a_weights: Vec<Vec<f64>>,      // predictor weights
    b_weights: Vec<Vec<f64>>,      // corrector weights
}

impl FractionalRiccatiSolver {
    /// Build solver for a given Hurst exponent and maturity.
    /// num_steps ~ 200 is sufficient for H ~ 0.1.
    pub fn new(hurst: f64, maturity: f64, num_steps: usize) -> Self;

    /// Solve D(u, t_j) for j = 0..num_steps for a given Fourier variable u.
    /// Returns the full trajectory (needed to compute C via quadrature).
    pub fn solve_d(&self, u: Complex64, params: &RoughHestonFourierParams) -> Vec<Complex64>;

    /// Compute C(u, T) = kappa * theta * integral_0^T D(u, s) ds via trapezoidal rule.
    pub fn solve_c(&self, d_trajectory: &[Complex64], params: &RoughHestonFourierParams) -> Complex64;
}
```

The Adams scheme:
1. **Predictor** (explicit): `D^P_{n+1} = sum_{j=0}^{n} a_{n+1,j} * F(D_j) * h^alpha / Gamma(alpha+1) + starting_terms`
2. **Corrector** (implicit, one step): `D_{n+1} = sum_{j=0}^{n} b_{n+1,j} * F(D_j) * h^alpha / Gamma(alpha+2) + b_{n+1,n+1} * F(D^P_{n+1}) * h^alpha / Gamma(alpha+2) + starting_terms`

The weights a_{n,j} and b_{n,j} are product integration weights that depend on alpha and the grid spacing. They are O(n^2) to store but computed once.

Non-uniform time grid: refined near t=0 where the kernel (t)^{alpha-1} is singular. Uses geometric spacing for the first ~20% of steps, then uniform.

### Pricing Interface

```rust
pub struct RoughHestonFourierParams {
    pub v0: f64,
    pub kappa: f64,
    pub theta: f64,
    pub sigma_v: f64,
    pub rho: f64,
    pub hurst: f64,
}

impl RoughHestonFourierParams {
    /// Validate parameters (same constraints as HestonParams + H in (0, 0.5)).
    pub fn new(v0: f64, kappa: f64, theta: f64, sigma_v: f64, rho: f64, hurst: f64) -> Result<Self>;

    /// Characteristic function E[exp(iu * ln(S_T/S_0))] under risk-neutral measure.
    /// Internally creates a FractionalRiccatiSolver, solves D and C, returns exp(C + D*v0).
    pub fn char_func(&self, u: Complex64, r: f64, q: f64, t: f64) -> Complex64;

    /// European option price via Gil-Pelaez inversion.
    /// Uses the same composite Gauss-Legendre quadrature as standard Heston.
    pub fn price_european(&self, spot: f64, strike: f64, r: f64, q: f64, t: f64, is_call: bool) -> f64;

    /// Implied vol extraction for calibration workflows.
    pub fn implied_vol(&self, spot: f64, strike: f64, r: f64, q: f64, t: f64) -> Option<f64>;
}
```

**Performance note**: The fractional Riccati solve is ~50-100x slower per characteristic function evaluation than standard Heston (O(n^2) Adams vs O(1) closed-form). For calibration over a strike grid, the solver is constructed once per maturity, and D(u, t) evaluations are parallelized across strikes via Rayon. With n=200 steps, one char func evaluation takes ~0.1ms, so a 50-point Gauss-Legendre integration takes ~5ms per strike, and a full surface calibration (10 maturities x 10 strikes) takes ~500ms — acceptable for end-of-day calibration.

---

## Layer 5: Hybrid Cheyette + Rough Vol

**Location**: `finstack/monte_carlo/src/process/cheyette_rough.rs`

Couples a Cheyette short-rate model with a rough volatility driver for rates derivatives (swaptions, caps/floors, Bermudans under stochastic vol).

### Cheyette Model

The Cheyette framework (1-factor Markovian HJM):

```text
r(t) = x(t) + phi(t)
dx(t) = [y(t) - kappa * x(t)] dt + sigma(t) dW(t)
dy(t) = [sigma(t)^2 - 2 * kappa * y(t)] dt
```

where x is the rate state, y is the accumulated variance state, and phi(t) is determined by the initial forward curve to ensure arbitrage-free pricing.

### Rough Vol Extension

Replace deterministic sigma(t) with a rough stochastic process:

```text
sigma(t) = sigma_0(t) * exp(eta * W_tilde_H(t) - 0.5 * eta^2 * t^{2H})
```

This is the rBergomi-style lognormal vol specification applied to the rates diffusion coefficient.

```rust
pub struct CheyetteRoughVolParams {
    pub kappa: f64,                        // mean reversion of short rate
    pub sigma_base: ForwardVarianceCurve,  // base vol term structure sigma_0(t)
    pub hurst: HurstExponent,              // H in (0, 0.5)
    pub eta: f64,                          // vol-of-vol for rough driver
    pub rho: f64,                          // correlation between rate and vol innovations
    pub forward_curve: ForwardCurve,       // initial OIS forward curve for phi(t)
}

pub struct CheyetteRoughVolProcess {
    params: CheyetteRoughVolParams,
    /// Spline for phi(t), precomputed from forward_curve
    phi_spline: CubicSpline,
}
```

`StochasticProcess` implementation:
- `dim() = 2` (x, y)
- `num_factors() = 2` (W for rate, W_tilde for rough vol)
- `populate_path_state`: `r(t) = x + phi(t)` -> `SHORT_RATE`

```rust
pub struct CheyetteRoughEuler {
    params: CheyetteRoughVolParams,
}
```

Discretization step (receives z = [Z_rate_independent, fbm_increment_i]):
1. Reconstruct sigma(t) from accumulated fBM path (same pattern as rBergomi)
2. Euler step x: `x += [y - kappa * x] * dt + sigma(t) * sqrt(dt) * Z_rate`
3. Deterministic step y: `y += [sigma(t)^2 - 2 * kappa * y] * dt`
4. Correlate rate noise with fBM driver via rho

The y equation is deterministic conditional on sigma(t), requiring no additional random input.

### Not In Scope

- Full HJM-type rough volatility (infinite-dimensional). This Markovian Cheyette approximation captures rough vol dynamics while remaining simulable.
- Swaption surface calibration. This spec provides the simulation engine; calibration would adapt the Fourier pricing from Layer 4 to rates.
- Multi-factor Cheyette. Single-factor with rough vol is the target.

---

## Engine Integration

### RequiresFractionalNoise Trait

**Location**: `finstack/monte_carlo/src/traits.rs`

```rust
/// Marker trait for discretizations that need pre-generated fBM increments.
/// The engine detects this at setup time (not per-step).
pub trait RequiresFractionalNoise {
    fn fractional_noise_config(&self) -> FractionalNoiseConfig;
}

pub struct FractionalNoiseConfig {
    pub hurst: f64,
    pub generator_type: FbmGeneratorType,
}

pub enum FbmGeneratorType {
    Cholesky,
    Hybrid { near_field_size: usize },
    /// Cholesky for n < 200, Hybrid otherwise
    Auto,
}
```

### Engine Modification

**Location**: `finstack/monte_carlo/src/engine.rs`

The `McEngine` path generation loop changes from:

```text
for each step i:
    rng.fill_std_normals(z)
    discretization.step(process, t, dt, x, z, work)
```

to (when `RequiresFractionalNoise` is detected):

```text
// Once per path:
fbm_generator.generate(rng, fbm_increments)

for each step i:
    rng.fill_std_normals(z_independent)
    z_combined = [z_independent[0..num_factors-1], fbm_increments[i]]
    discretization.step(process, t, dt, x, z_combined, work)
```

Detection is static: the engine checks at construction whether the discretization implements `RequiresFractionalNoise` (via `Any` downcast or a method on `Discretization`). A boolean flag selects the code path — no trait object dispatch per step.

The fBM increment buffer is allocated once per engine and reused across paths.

### Variance Reduction Compatibility

- **Antithetic**: Negate both i.i.d. normals AND fBM increments. Since fBM is Gaussian, -B_H is also an fBM with the same distribution. Works unchanged.
- **Control variate**: Works unchanged (the control variate estimator only needs E[Y] for the control).
- **Importance sampling**: Works unchanged (measure change applies to the underlying standard normals before they enter the fBM generator).

### Pricer Registry

**Location**: `finstack/valuations/src/pricer/keys.rs`

```rust
ModelKey::RoughBergomi       // = 50
ModelKey::RoughHeston        // = 51
ModelKey::RoughHestonFourier // = 52
ModelKey::CheyetteRoughVol   // = 53
```

Pricer mappings in `registry.rs`:
- `(EquityOption, RoughBergomi)` -> MC pricer with rBergomi process
- `(EquityOption, RoughHeston)` -> MC pricer with rough Heston process
- `(EquityOption, RoughHestonFourier)` -> Fourier pricer (vanillas only)
- `(Swaption, CheyetteRoughVol)` -> MC pricer with Cheyette rough vol
- `(CapFloor, CheyetteRoughVol)` -> MC pricer with Cheyette rough vol

---

## Testing Strategy

### Unit Tests

**`fractional.rs`**:
- fBM covariance function: verify symmetry, B_H(t) variance = t^{2H}, H=0.5 gives min(s,t)
- Mittag-Leffler: E_{1,1}(z) = exp(z), E_{2,1}(-z^2) = cos(z), known tabulated values
- Kernel evaluation: boundary cases (t=s returns 0 or limit), H->0.5 convergence

**`CholeskyFbm`**:
- Generated increments have correct covariance structure (sample covariance over 10k+ paths matches theoretical within statistical tolerance)
- H=0.5 recovers standard BM increments (independent, variance = dt)
- Variance scaling: Var(B_H(t)) ~ t^{2H} verified empirically

**`HybridFbm`**:
- Agreement with Cholesky for short paths (n=100): max difference in sample statistics < tolerance
- Correct variance scaling t^{2H}
- Near-field size sensitivity: larger b reduces approximation error

**`FractionalRiccatiSolver`**:
- H=0.5 recovers standard Heston Riccati: compare D(u, T) against closed-form
- Convergence order: halve step size, verify error shrinks by O(h^{1+alpha})
- Known values from El Euch & Rosenbaum (2019) numerical examples

**`RoughHestonFourierParams`**:
- price_european: put-call parity holds to machine precision
- H=0.5 matches standard Heston prices (within Riccati solver tolerance)
- Prices monotonic in eta (vol-of-vol), in |rho| (for skew), in T (time value)
- Non-negative prices, bounded by intrinsic value below and spot above (calls)

### Integration Tests

**rBergomi MC benchmark**:
- Reproduce Bayer, Friz, Gatheral (2016) Table 1: ATM implied vols for H=0.07, eta=1.9, rho=-0.9, T=1.0
- Match within MC standard error (run sufficient paths: 100k-500k)

**Rough Heston MC vs Fourier**:
- Price European calls across a strike range (80%-120% moneyness) with both methods
- Agreement within 2-3 MC standard errors
- Test at multiple maturities (0.25, 0.5, 1.0, 2.0)

**Rough Heston Fourier vs standard Heston**:
- H=0.499: prices should closely match standard Heston (within solver tolerance)
- Verify smooth convergence as H -> 0.5

**Cheyette rough vol**:
- Zero-coupon bond repricing: P(0,T) from simulation matches initial discount curve (arbitrage-free check)
- Cap prices: reasonable vs normal vol quotes (order-of-magnitude sanity)
- H=0.5 limit: should behave like deterministic-vol Cheyette

**Antithetic variance reduction**:
- Verify ~2x variance reduction ratio for rough processes (same benefit as standard processes)

### Convergence Tests

- **Cholesky fBM**: exact to machine epsilon (covariance matrix match)
- **Hybrid fBM**: error decreases as near_field_size increases (quantified)
- **Fractional Riccati Adams**: O(h^{min(2, 1+alpha)}) verified by Richardson extrapolation
- **MC pricing**: standard 1/sqrt(n) convergence of standard error

---

## Phasing

### Phase 1: Foundation + rBergomi

Core deliverable. All subsequent phases depend on this.

1. `core/math/fractional.rs` — HurstExponent, kernels, covariance utilities, Mittag-Leffler
2. `monte_carlo/rng/fbm.rs` — FractionalNoiseGenerator trait, CholeskyFbm, HybridFbm
3. `core/market_data/term_structures/forward_variance.rs` — ForwardVarianceCurve
4. `monte_carlo/process/rough_bergomi.rs` — RoughBergomiProcess + RoughBergomiEuler
5. `RequiresFractionalNoise` trait + engine integration in engine.rs
6. Unit + integration tests
7. rBergomi benchmark reproduction

### Phase 2: Rough Heston (MC + Fourier)

Depends on Phase 1 (fBM infrastructure). MC and Fourier are independent of each other.

1. `core/math/volatility/rough_heston.rs` — FractionalRiccatiSolver, char func, pricing
2. `monte_carlo/process/rough_heston.rs` — RoughHestonProcess + RoughHestonHybrid
3. MC vs Fourier cross-validation
4. H=0.5 regression tests against standard Heston
5. Pricer registry entries: RoughBergomi, RoughHeston, RoughHestonFourier

### Phase 3: Hybrid Cheyette + Rough Vol

Depends on Phase 1 (fBM infrastructure). Independent of Phase 2.

1. `monte_carlo/process/cheyette_rough.rs` — CheyetteRoughVolProcess + CheyetteRoughEuler
2. Forward curve integration for phi(t) extraction
3. Arbitrage-free validation (ZCB repricing)
4. Pricer registry entry: CheyetteRoughVol

Phases 2 and 3 can proceed in parallel after Phase 1 is complete.

---

## References

- Bayer, C., Friz, P., Gatheral, J. (2016). "Pricing under rough volatility." *Quantitative Finance*, 16(6), 887-904.
- Gatheral, J., Jaisson, T., Rosenbaum, M. (2018). "Volatility is rough." *Quantitative Finance*, 18(6), 933-949.
- El Euch, O., Rosenbaum, M. (2019). "The characteristic function of rough Heston models." *Mathematical Finance*, 29(1), 3-38.
- Bennedsen, M., Lunde, A., Pakkanen, M. S. (2017). "Hybrid scheme for Brownian semistationary processes." *Finance and Stochastics*, 21(4), 931-965.
- Diethelm, K., Ford, N. J., Freed, A. D. (2004). "Detailed error analysis for a fractional Adams method." *Numerical Algorithms*, 36(1), 31-52.
- Andersen, L. (2008). "Simple and efficient simulation of the Heston stochastic volatility model." *Journal of Computational Finance*, 11(3), 1-42.
- Gorenflo, R., Loutchko, J., Luchko, Y. (2002). "Computation of the Mittag-Leffler function and its derivatives." *Fractional Calculus and Applied Analysis*, 5(4), 491-518.
