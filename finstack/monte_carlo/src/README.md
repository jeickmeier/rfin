# Monte Carlo Pricing Framework

A production-grade Monte Carlo simulation engine for derivative pricing. Covers the full stack from random number generation through stochastic processes, discretization schemes, payoff definitions, variance reduction, early exercise (LSMC), Greeks, and barrier corrections—with optional path capture for visualization and diagnostics.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Module Structure](#module-structure)
- [Core Traits](#core-traits)
- [Simulation Engine](#simulation-engine)
- [Random Number Generation](#random-number-generation)
- [Stochastic Processes](#stochastic-processes)
- [Discretization Schemes](#discretization-schemes)
- [Payoffs](#payoffs)
- [Pricers](#pricers)
- [Greeks](#greeks)
- [Variance Reduction](#variance-reduction)
- [Barrier Corrections](#barrier-corrections)
- [Path Capture](#path-capture)
- [Seed Management](#seed-management)
- [Feature Flags](#feature-flags)
- [Usage Examples](#usage-examples)
- [Adding New Components](#adding-new-components)
- [Performance Notes](#performance-notes)
- [Academic References](#academic-references)

---

## Architecture Overview

The framework is structured around five composable abstractions defined as traits:

```
RandomStream  →  StochasticProcess  →  Discretization  →  Payoff  →  McEngine
   (RNG)           (SDE spec)          (time-stepping)    (contract)   (orchestrator)
```

The `McEngine` orchestrates the simulation loop: for each path it splits the RNG into an independent stream, steps the process through the time grid, feeds each state to the payoff, and accumulates online statistics (Welford's algorithm) across all paths. Parallel execution uses Rayon with deterministic chunk-reduce; results are bit-identical regardless of thread count.

### Key design decisions

- **Trait-based composition**: Process, discretization, RNG, and payoff are all interchangeable via traits. Any `StochasticProcess` works with any compatible `Discretization`.
- **Currency safety**: All pricing results carry explicit `Currency` via the `Money` type. Payoffs return `Money`, not raw `f64`.
- **Deterministic reproducibility**: Counter-based RNG (Philox) with splittable streams ensures identical results across serial and parallel runs for the same seed.
- **Zero-copy state passing**: The engine reuses pre-allocated buffers for state vectors, normal shocks, and workspace arrays across paths. `PathState` uses `HashMap<&'static str, f64>` for named state variables.
- **Feature-gated complexity**: Basic GBM pricing works without any feature flags. Advanced processes (Heston, CIR, Bates), exotic payoffs, Greeks, and QMC require the `mc` feature.

---

## Module Structure

```
monte_carlo/
├── lib.rs                    # Crate root, prelude, re-exports
├── traits.rs                 # Core traits: RandomStream, StochasticProcess, Discretization, Payoff
├── engine.rs                 # McEngine, McEngineBuilder, McEngineConfig, path simulation loop
├── results.rs                # MoneyEstimate, MonteCarloResult, MonteCarloGreeks
├── paths.rs                  # PathPoint, SimulatedPath, PathDataset, ProcessParams, CashflowType
├── estimate.rs               # Estimate and compatibility diagnostics types
├── online_stats.rs           # OnlineStats, required_samples (re-export from core)
├── time_grid.rs              # TimeGrid (re-export from core)
├── seed.rs                   # Deterministic seed derivation for Greek scenarios
│
├── process/                  # Stochastic process definitions
│   ├── brownian.rs           # BrownianProcess, MultiBrownianProcess
│   ├── gbm.rs                # GbmProcess, MultiGbmProcess
│   ├── gbm_dividends.rs      # GbmWithDividends (discrete dividends)
│   ├── heston.rs             # HestonProcess (stochastic volatility)
│   ├── ou.rs                 # HullWhite1FProcess, VasicekProcess
│   ├── cir.rs                # CirProcess, CirPlusPlusProcess
│   ├── bates.rs              # BatesProcess (Heston + jumps)
│   ├── jump_diffusion.rs     # MertonJumpProcess
│   ├── schwartz_smith.rs     # SchwartzSmithProcess (commodity)
│   ├── multi_ou.rs           # MultiOuProcess (multi-factor OU)
│   ├── revolving_credit.rs   # RevolvingCreditProcess
│   ├── correlation.rs        # Cholesky decomposition, correlation matrix utilities
│   └── metadata.rs           # ProcessMetadata trait → ProcessParams
│
├── discretization/           # Time-stepping schemes
│   ├── exact.rs              # ExactGbm, ExactMultiGbm, ExactMultiGbmCorrelated
│   ├── exact_gbm_dividends.rs # ExactGbmWithDividends
│   ├── exact_hw1f.rs         # ExactHullWhite1F
│   ├── euler.rs              # EulerMaruyama, LogEuler
│   ├── milstein.rs           # Milstein, LogMilstein
│   ├── qe_heston.rs          # QeHeston (Andersen quadratic-exponential)
│   ├── qe_cir.rs             # QeCir
│   ├── jump_euler.rs         # JumpEuler (Merton jump-diffusion)
│   ├── schwartz_smith.rs     # ExactSchwartzSmith
│   └── revolving_credit.rs   # RevolvingCreditDiscretization
│
├── payoff/                   # Contract payoff definitions
│   ├── traits.rs             # TerminalPayoff, SimpleTerminalPayoff, compatibility helpers
│   ├── vanilla.rs            # EuropeanCall, EuropeanPut, Digital, Forward
│   ├── asian.rs              # AsianCall, AsianPut, AveragingMethod, geometric closed-form
│   ├── barrier.rs            # BarrierOptionPayoff, BarrierType (up/down, in/out)
│   ├── basket.rs             # BasketCall, BasketPut, ExchangeOption, Margrabe formula
│   ├── lookback.rs           # Lookback (fixed/floating strike)
│   ├── rates.rs              # RatesPayoff (caps, floors, swaps under Hull-White)
│   ├── swaption.rs           # BermudanSwaptionPayoff, SwapSchedule
│   ├── quanto.rs             # QuantoCallPayoff, QuantoPutPayoff
│   ├── autocallable.rs       # AutocallablePayoff, FinalPayoffType
│   ├── cms.rs                # CmsPayoff (constant maturity swap)
│   ├── cliquet.rs            # CliquetCallPayoff (periodic reset options)
│   ├── range_accrual.rs      # RangeAccrualPayoff
│   ├── fx_barrier.rs         # FxBarrierCall
│   └── default_calculator.rs # FirstPassageCalculator (credit default modeling)
│
├── pricer/                   # High-level pricing orchestrators
│   ├── european.rs           # EuropeanPricer, EuropeanPricerConfig
│   ├── path_dependent.rs     # PathDependentPricer (Sobol, Brownian bridge)
│   ├── lsmc.rs               # LsmcPricer (Longstaff-Schwartz for American/Bermudan)
│   ├── basis.rs              # BasisFunctions trait, PolynomialBasis, LaguerreBasis
│   ├── lsq.rs                # Least-squares regression utilities
│   ├── swaption_lsmc.rs      # Bermudan swaption LSMC pricer
│   └── swap_rate_utils.rs    # Swap rate computation helpers
│
├── greeks/                   # Sensitivity calculation
│   ├── pathwise.rs           # Pathwise delta (call/put), pathwise vega
│   ├── lrm.rs                # Likelihood ratio method: delta, vega
│   └── finite_diff.rs        # Finite-difference (CRN): delta, gamma, vega
│
├── variance_reduction/       # Variance reduction techniques
│   ├── antithetic.rs         # Antithetic variates (Z, -Z pairing)
│   ├── control_variate.rs    # Black-Scholes control variate
│   ├── moment_matching.rs    # Force N(0,1) moments on samples
│   └── importance_sampling.rs # Exponential tilting, ESS diagnostics
│
├── barriers/                 # Barrier monitoring corrections
│   ├── bridge.rs             # Brownian bridge hit probability
│   └── corrections.rs        # Gobet-Miri continuity correction
│
└── rng/                      # Random number generators
    ├── philox.rs             # Philox 4×32-10 counter-based PRNG
    ├── sobol.rs              # Sobol quasi-random sequences (Owen scrambling)
    ├── sobol_pca.rs          # PCA ordering for Sobol + Brownian bridge
    ├── brownian_bridge.rs    # Brownian bridge path construction
    └── poisson.rs            # Poisson variates (for jump processes)
```

---

## Core Traits

### `RandomStream`

Abstraction over random number generation with deterministic stream splitting for parallel execution.

```rust
pub trait RandomStream: Send + Sync {
    fn split(&self, stream_id: u64) -> Self where Self: Sized;
    fn fill_u01(&mut self, out: &mut [f64]);
    fn fill_std_normals(&mut self, out: &mut [f64]);
    fn next_u01(&mut self) -> f64;        // convenience
    fn next_std_normal(&mut self) -> f64;  // convenience
}
```

Each Monte Carlo path gets its own stream via `rng.split(path_id)`, ensuring reproducibility regardless of execution order.

### `StochasticProcess`

Defines an SDE system dX = μ(t,X)dt + Σ(t,X)dW:

```rust
pub trait StochasticProcess: Send + Sync {
    fn dim(&self) -> usize;
    fn num_factors(&self) -> usize;
    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]);
    fn diffusion(&self, t: f64, x: &[f64], out: &mut [f64]);
    fn is_diagonal(&self) -> bool;
    fn populate_path_state(&self, x: &[f64], state: &mut PathState);
}
```

`populate_path_state` maps the raw state vector to named keys (`SPOT`, `VARIANCE`, `SHORT_RATE`, etc.) so payoffs can access values by semantic name.

### `Discretization<P: StochasticProcess>`

Time-stepping scheme that advances state from t to t+dt:

```rust
pub trait Discretization<P: StochasticProcess + ?Sized>: Send + Sync {
    fn step(&self, process: &P, t: f64, dt: f64, x: &mut [f64], z: &[f64], work: &mut [f64]);
    fn work_size(&self, process: &P) -> usize;
}
```

### `Payoff`

Contract payoff with currency safety and event-driven accumulation:

```rust
pub trait Payoff: Send + Sync + Clone {
    fn on_event(&mut self, state: &mut PathState);
    fn value(&self, currency: Currency) -> Money;
    fn reset(&mut self);
    fn discount_factor(&self) -> f64;
    fn on_path_start<R: RandomStream>(&mut self, rng: &mut R);
}
```

Payoffs accumulate state through `on_event` calls at each time step and return a final `Money` value. The `on_path_start` hook enables per-path randomization (e.g., drawing a default threshold E ~ Exp(1) for credit instruments).

### State Variables

Named state keys provide semantic access to process state:

| Key | Description |
|-----|-------------|
| `SPOT` | Spot price (equity/FX) |
| `VARIANCE` | Stochastic variance (Heston) |
| `SHORT_RATE` | Short rate (Hull-White, CIR) |
| `TIME` | Time in years |
| `STEP` | Step index |
| `FX_RATE` | FX rate (quanto, multi-asset) |
| `EQUITY_SPOT` | Secondary equity spot |
| `NPV_CURRENT` | Current NPV of remaining cashflows |
| `NPV_PREVIOUS` | Previous step NPV |
| `MTM_PNL` | Mark-to-market P&L |

---

## Simulation Engine

`McEngine` is the central orchestrator. It is configured via a builder pattern:

```rust
let engine = McEngine::builder()
    .num_paths(100_000)
    .seed(42)
    .uniform_grid(1.0, 252)  // 1 year, daily steps
    .parallel(true)
    .antithetic(true)
    .target_ci(0.01)         // auto-stop when CI half-width < 0.01
    .build()?;
```

### Configuration options

| Option | Default | Description |
|--------|---------|-------------|
| `num_paths` | 100,000 | Number of Monte Carlo paths |
| `seed` | 42 | RNG seed for reproducibility |
| `time_grid` | required | Time grid (uniform or custom) |
| `use_parallel` | auto | Rayon-based parallel execution |
| `chunk_size` | 1,000 | Paths per parallel chunk (adaptive by default) |
| `antithetic` | false | Antithetic variate pairing (Z, -Z) |
| `target_ci_half_width` | None | Auto-stop threshold for 95% CI |
| `path_capture` | disabled | Path capture for visualization |

### Pricing methods

- **`price()`** — Returns `MoneyEstimate` (mean, stderr, 95% CI, num_paths)
- **`price_with_capture()`** — Returns `MonteCarloResult` with optional `PathDataset` containing per-step state vectors, cashflows, payoff values, and IRR

### Execution model

- **Serial**: Single-threaded loop with buffer reuse
- **Parallel**: Rayon `par_iter` over chunks with adaptive sizing (4 chunks per thread, clamped to [100, 10000] paths per chunk). Chunk statistics are merged deterministically via `OnlineStats::merge()`

---

## Random Number Generation

### Philox PRNG (`PhiloxRng`)

Counter-based pseudo-random number generator (Philox 4×32-10). Supports deterministic stream splitting — each path gets `rng.split(path_id)` producing independent, reproducible streams regardless of execution order.

- Statistically high-quality output (passes BigCrush)
- Zero-state initialization from counter + key
- Parallel-safe by construction

### Sobol QMC (`SobolRng`)

Sobol low-discrepancy sequences with Owen scrambling for quasi-Monte Carlo integration. Provides faster convergence than pseudo-random sampling for smooth integrands (effective rate up to O(N⁻¹) vs O(N⁻¹/²)).

### Brownian Bridge (`BrownianBridge`)

Constructs Brownian paths by first sampling the terminal value, then successively filling in midpoints. When combined with Sobol sequences, this ensures the most important (lowest-index) Sobol dimensions control the largest-scale path features.

### Sobol-PCA (`sobol_pca`)

PCA ordering for Sobol-Brownian bridge generation. Computes effective dimension and transforms PCA-ordered draws into asset-space for correlated multi-asset simulation.

### Poisson Variates

`poisson_from_normal` and `poisson_inverse_cdf` generate Poisson-distributed variates from uniform/normal inputs, used by jump-diffusion processes (Merton, Bates).

---

## Stochastic Processes

| Process | SDE | Params | Dimensions |
|---------|-----|--------|------------|
| **GbmProcess** | dS = (r−q)S dt + σS dW | `GbmParams { rate, div_yield, vol }` | 1 state, 1 factor |
| **MultiGbmProcess** | Correlated multi-asset GBM | `Vec<GbmParams>` + correlation | N states, N factors |
| **BrownianProcess** | dX = μ dt + σ dW | `BrownianParams { mu, sigma }` | 1 state, 1 factor |
| **MultiBrownianProcess** | Correlated multi-dimensional BM | mus, sigmas, correlation | N states, N factors |
| **HestonProcess** | dS = (r−q)S dt + √v S dW₁ | `HestonParams` | 2 states, 2 factors |
| | dv = κ(θ−v)dt + ξ√v dW₂ | { kappa, theta, xi, rho, ... } | |
| **HullWhite1FProcess** | dr = κ[θ(t)−r]dt + σ dW | `HullWhite1FParams` | 1 state, 1 factor |
| **VasicekProcess** | dr = κ(θ−r)dt + σ dW | (constant θ) | 1 state, 1 factor |
| **CirProcess** | dv = κ(θ−v)dt + σ√v dW | `CirParams` | 1 state, 1 factor |
| **CirPlusPlusProcess** | CIR + deterministic shift φ(t) | CirProcess + shift curve | 1 state, 1 factor |
| **BatesProcess** | Heston + Merton compound jumps | `BatesParams` | 2 states, 2 factors + jumps |
| **MertonJumpProcess** | GBM + Poisson jumps | `MertonJumpParams` | 1 state, 1 factor + jumps |
| **SchwartzSmithProcess** | Two-factor commodity model | `SchwartzSmithParams` | 2 states, 2 factors |
| | dχ = −κχ dt + σ_χ dW₁ (OU) | | |
| | dξ = μ_ξ dt + σ_ξ dW₂ (ABM) | | |
| **GbmWithDividends** | GBM + discrete dividend jumps | GbmParams + `Vec<Dividend>` | 1 state, 1 factor |
| **MultiOuProcess** | Multi-dimensional OU | `MultiOuParams` | N states, N factors |
| **RevolvingCreditProcess** | Utilization + rate + credit | `RevolvingCreditProcessParams` | 3 states, 3 factors |

### Correlation

The `correlation` module provides:
- `cholesky_decomposition` — Cholesky factorization for correlation matrices
- `apply_correlation` — Transform independent normals Z into correlated draws via L·Z
- `build_correlation_matrix` — Construct correlation matrix from pairwise correlations

---

## Discretization Schemes

| Scheme | Process | Strong Order | Notes |
|--------|---------|:------------:|-------|
| **ExactGbm** | GBM | exact | Log-normal analytical transition |
| **ExactMultiGbm** | Multi-GBM (diagonal) | exact | Independent exact steps |
| **ExactMultiGbmCorrelated** | Multi-GBM (correlated) | exact | Cholesky correlation transform |
| **ExactGbmWithDividends** | GBM + dividends | exact | Exact GBM + deterministic dividend jumps |
| **ExactHullWhite1F** | Hull-White 1F | exact | Gaussian OU analytical transition |
| **ExactSchwartzSmith** | Schwartz-Smith | exact | Exact bivariate OU/ABM with correlation |
| **EulerMaruyama** | any | 0.5 | First-order explicit scheme |
| **LogEuler** | log-normal | 0.5 | Euler in log-space (ensures positivity) |
| **Milstein** | diagonal diffusion | 1.0 | Second-order for scalar/diagonal SDEs |
| **LogMilstein** | log-normal | 1.0 | Milstein in log-space |
| **QeHeston** | Heston | N/A | Andersen's quadratic-exponential for variance; log-Euler for spot |
| **QeCir** | CIR | N/A | Andersen QE for CIR-type processes |
| **JumpEuler** | Merton jump-diffusion | 0.5 | Euler + Poisson jump sampling |
| **RevolvingCreditDiscretization** | Revolving credit | mixed | OU + HW1F + CIR per factor |

### Choosing a scheme

- **Exact schemes** are preferred when available (no discretization error, any step size works).
- **Euler-Maruyama/LogEuler** is the default general-purpose fallback.
- **Milstein/LogMilstein** improves convergence for smooth diffusions with known derivatives.
- **QE schemes** are essential for Heston and CIR to avoid negative variance.

---

## Payoffs

### Vanilla

| Payoff | Formula |
|--------|---------|
| `EuropeanCall` | max(S_T − K, 0) × notional |
| `EuropeanPut` | max(K − S_T, 0) × notional |
| `Digital` | 1_{S_T > K} × notional (or put variant) |
| `Forward` | (S_T − K) × notional |

### Path-Dependent

| Payoff | Description |
|--------|-------------|
| `AsianCall` / `AsianPut` | Call/put on arithmetic or geometric average. Supports `AveragingMethod::Arithmetic` and `Geometric`. Geometric variant has a closed-form benchmark via `geometric_asian_call_closed_form`. |
| `BarrierOptionPayoff` | Up/Down × In/Out barrier options. `BarrierType` enum: `UpAndIn`, `UpAndOut`, `DownAndIn`, `DownAndOut`. |
| `Lookback` | Fixed-strike and floating-strike lookback options. `FloatingStrikeLookbackCall`, `FloatingStrikeLookbackPut`. |
| `BasketCall` / `BasketPut` | Weighted basket options. `BasketType` determines weighting. |
| `ExchangeOption` | Option to exchange one asset for another (Margrabe). `margrabe_exchange_option` provides closed-form benchmark. |

### Rates & Structured Products

| Payoff | Description |
|--------|-------------|
| `RatesPayoff` | Caps, floors, and swaps under Hull-White dynamics. `RatesPayoffType` selects cap/floor/swap. |
| `BermudanSwaptionPayoff` | Bermudan swaption with `SwapSchedule` and exercise dates. Priced via LSMC. |
| `CmsPayoff` | Constant maturity swap payoffs. `CmsType` selects variants. |

### Multi-Asset / FX

| Payoff | Description |
|--------|-------------|
| `QuantoCallPayoff` / `QuantoPutPayoff` | Quanto options with FX adjustment. |
| `FxBarrierCall` | FX barrier call option. |

### Exotic / Structured

| Payoff | Description |
|--------|-------------|
| `AutocallablePayoff` | Autocallable with observation dates, autocall barriers, coupons, knock-in put, capital protection, or participation. `FinalPayoffType` determines terminal behavior. |
| `CliquetCallPayoff` | Periodic reset cliquet with local/global caps and floors. Accumulates period returns R_i = S_i/S_{i−1} − 1. |
| `RangeAccrualPayoff` | Accrues notional for each fixing date where spot is within a specified range. |

### Credit

| Component | Description |
|-----------|-------------|
| `FirstPassageCalculator` | First-passage time default model. Tracks cumulative hazard Λ(t) = ∫λ(s)ds and triggers default when Λ(t) > E, where E ~ Exp(1). Hazard rate derived from credit spreads: λ = s/(1−R). |
| `DefaultEvent` | Enum: `NoDefault` or `DefaultOccurred { time, recovery_fraction }`. |

### Generic

`SimpleTerminalPayoff` wraps a closure for ad-hoc terminal payoffs without defining a new struct.

---

## Pricers

High-level pricing orchestrators that combine the engine, process, discretization, and payoff.

### `EuropeanPricer`

Prices European-style payoffs under GBM. Provides a simple API with `EuropeanPricerConfig` (num_paths, seed, use_parallel).

### `PathDependentPricer`

Prices path-dependent products with configurable time grid resolution. Supports:
- Sobol quasi-random sequences (`use_sobol`)
- Brownian bridge construction (`use_brownian_bridge`)
- Antithetic variance reduction (`antithetic`)
- Configurable steps per year (`steps_per_year`, `min_steps`)

### `LsmcPricer`

Longstaff-Schwartz Monte Carlo for American and Bermudan options. Implements backward induction with least-squares regression:

1. **Forward pass**: Simulate paths and store spot values at exercise dates
2. **Backward pass**: At each exercise date, regress continuation value on basis functions, compare with immediate exercise value
3. **Pricing pass**: Compute optimal exercise policy and present-value payoffs

Basis function choices:
- `PolynomialBasis` — {1, x, x², …, xⁿ}
- `LaguerreBasis` — Laguerre polynomials normalized by strike

Exercise value via `ImmediateExercise` trait (`AmericanCall`, `AmericanPut`).

### Swaption LSMC

Specialized LSMC for Bermudan swaptions under Hull-White dynamics, with swap rate computation utilities in `swap_rate_utils`.

---

## Greeks

Three methods for computing Monte Carlo sensitivities:

### Pathwise (Infinitesimal Perturbation Analysis)

Differentiates the payoff along the simulated path. Zero additional simulation cost but requires the payoff to be differentiable (not suitable for digital/barrier payoffs).

- `pathwise_delta_call` / `pathwise_delta_put` — ∂V/∂S
- `pathwise_vega` — ∂V/∂σ

### Likelihood Ratio Method (Score Function)

Uses the score function ∂log p/∂θ to compute sensitivities. Works for discontinuous payoffs (digitals, barriers) but has higher variance.

- `lrm_delta` — ∂V/∂S via likelihood ratio
- `lrm_vega` — ∂V/∂σ via likelihood ratio

### Finite Differences (CRN)

Bumps parameters and re-simulates with common random numbers to compute centered differences. Most general but requires 2–3× the simulation cost.

- `finite_diff_delta` — (V(S+ε) − V(S−ε)) / 2ε
- `finite_diff_gamma` — (V(S+ε) − 2V(S) + V(S−ε)) / ε²
- `finite_diff_vega` — (V(σ+ε) − V(σ−ε)) / 2ε

---

## Variance Reduction

### Antithetic Variates

Pairs each path (shocks Z) with its mirror (-Z). The negative correlation reduces variance for monotone payoffs. Integrated into `McEngine` via the `antithetic` flag, or standalone via `antithetic_price()`.

### Control Variates

Uses Black-Scholes closed-form as a control variate for GBM-based pricing. Functions `black_scholes_call` and `black_scholes_put` provide the analytical benchmarks.

### Moment Matching

Forces sample moments to match theoretical N(0,1) moments exactly:
- `match_standard_normal_moments` — adjusts mean to 0 and variance to 1
- `match_moments_per_step` — applies per time step column in a path matrix

### Importance Sampling

Exponential tilting to shift the sampling distribution toward rare events (deep OTM options, barrier hits, tail risks):
- `exponential_tilt(theta, z)` → `(tilted_z, likelihood_ratio)`
- `weighted_estimate(values, weights)` → `(mean, stderr)`
- `weighted_estimate_with_diagnostics(...)` → `ImportanceSamplingResult` with ESS monitoring

Effective Sample Size (ESS) diagnostics warn when ESS/N drops below 10%, indicating unreliable estimates.

---

## Barrier Corrections

### Brownian Bridge

`bridge_hit_probability(s_t, s_t_dt, barrier, sigma, dt)` computes the probability that the continuous path crosses a barrier between two discrete observations using the Brownian bridge formula:

```
p_hit ≈ exp(−2 · ln(S_t/B) · ln(S_{t+Δt}/B) / (σ²Δt))
```

`barrier_hit_check_with_bridge` uses a per-step uniform random draw (from `PathState.uniform_random()`) to probabilistically determine barrier hits.

### Gobet-Miri Continuity Correction

Adjusts the barrier level to reduce discretization bias when monitoring is discrete:
- Down barrier: B' = B · exp(−β · σ · √Δt)
- Up barrier: B' = B · exp(+β · σ · √Δt)

where β ≈ 0.5826 (Gobet-Miri optimal coefficient).

Also provides `half_step_adjusted_barrier` for a simpler half-step correction.

---

## Path Capture

The engine supports capturing full path data for visualization, debugging, and analysis.

### Configuration

```rust
let engine = McEngine::builder()
    .capture_all_paths()                    // capture every path
    .capture_sample_paths(100, 42)          // or sample 100 paths
    .path_capture(PathCaptureConfig::sample(100, 42).with_payoffs()) // with payoff values
    .build()?;
```

### Data structures

- **`PathPoint`** — Single point along a path: step, time, state vector, cashflows, optional payoff value
- **`SimulatedPath`** — Full path: sequence of `PathPoint`s, final discounted value, optional IRR
- **`PathDataset`** — Collection of paths with `ProcessParams` metadata
- **`CashflowType`** — `Principal`, `Interest`, `CommitmentFee`, `UsageFee`, `FacilityFee`, `UpfrontFee`, `Recovery`, `MarkToMarket`, `Other`

When paths are captured, the engine automatically computes additional statistics: median, P25/P75 percentiles, min/max, and IRR (via Newton's method from `finstack_core`).

---

## Seed Management

The `seed` module provides deterministic seed derivation for reproducible Greek calculations via finite differences:

```rust
let base_seed = seed::derive_seed(&instrument_id, "base");
let delta_up = seed::derive_seed_for_metric(&instrument_id, "delta", "up");
let delta_down = seed::derive_seed_for_metric(&instrument_id, "delta", "down");
```

Same instrument + same scenario always produces the same seed, enabling common-random-number Greeks across bumped scenarios.

---

## Feature Flags

| Flag | Enables |
|------|---------|
| `mc` | Heston, Hull-White, CIR, Bates, jump-diffusion, Schwartz-Smith processes; Sobol QMC; Poisson variates; Greeks; LSMC; path-dependent pricer; exotic payoffs (Asian, barrier, lookback, basket, autocallable, cliquet, range accrual, etc.); moment matching; importance sampling; barrier corrections |
| `parallel` | Rayon-based parallel path simulation |

Basic GBM pricing with Philox RNG, exact discretization, and European payoffs works without any feature flags.

---

## Usage Examples

### European Call under GBM

```rust
use finstack_valuations::instruments::common::models::monte_carlo::prelude::*;
use finstack_core::currency::Currency;

let engine = McEngine::builder()
    .num_paths(100_000)
    .seed(42)
    .uniform_grid(1.0, 252)
    .build()?;

let gbm = GbmProcess::with_params(0.03, 0.00, 0.20);
let disc = ExactGbm::new();
let payoff = EuropeanCall::new(100.0, 1.0, 252);
let rng = PhiloxRng::new(42);

let result = engine.price(&rng, &gbm, &disc, &[100.0], &payoff, Currency::USD, 1.0)?;
println!("Price: {} ± {:.4}", result.mean, result.stderr);
```

### Asian Option with Sobol QMC

```rust
let config = PathDependentPricerConfig {
    num_paths: 50_000,
    seed: 42,
    use_parallel: true,
    steps_per_year: 252,
    min_steps: 50,
    use_sobol: true,
    antithetic: false,
    use_brownian_bridge: true,
};

let pricer = PathDependentPricer::new(config);
let payoff = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, 252, 1.0);
let result = pricer.price(&gbm, &disc, &[100.0], &payoff, Currency::USD, df)?;
```

### American Put via LSMC

```rust
let exercise_dates: Vec<usize> = (1..=12).map(|m| m * 21).collect(); // monthly exercise
let config = LsmcConfig::new(50_000, exercise_dates).with_seed(42);
let exercise = AmericanPut { strike: 100.0 };
let basis = PolynomialBasis::new(3);

let result = LsmcPricer::price(&config, &gbm, &disc, &[100.0], &exercise, &basis, Currency::USD, df)?;
```

### Manual Path Simulation

```rust
let grid = TimeGrid::uniform(1.0, 252)?;
let gbm = GbmProcess::with_params(0.05, 0.02, 0.20);
let disc = ExactGbm::new();
let mut rng = PhiloxRng::new(42);

let mut x = vec![100.0];
let mut z = vec![0.0; gbm.num_factors()];
let mut work = vec![0.0; disc.work_size(&gbm)];

for step in 0..grid.num_steps() {
    rng.fill_std_normals(&mut z);
    disc.step(&gbm, grid.time(step), grid.dt(step), &mut x, &z, &mut work);
}
// x[0] is the terminal spot
```

### Path Capture with Cashflows

```rust
let engine = McEngine::builder()
    .num_paths(1_000)
    .seed(42)
    .uniform_grid(5.0, 60)
    .capture_sample_paths(100, 42)
    .build()?;

let result = engine.price_with_capture(
    &rng, &process, &disc, &initial_state, &payoff,
    Currency::USD, discount_factor, process_params,
)?;

if let Some(paths) = result.paths() {
    for path in &paths.paths {
        let cashflows = path.extract_cashflows();
        println!("Path {} IRR: {:?}", path.path_id, path.irr());
    }
}
```

---

## Adding New Components

### Adding a New Stochastic Process

1. Create `process/my_process.rs`
2. Define a params struct and implement `StochasticProcess`:

```rust
pub struct MyProcess { /* params */ }

impl StochasticProcess for MyProcess {
    fn dim(&self) -> usize { 1 }
    fn num_factors(&self) -> usize { 1 }

    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]) {
        out[0] = /* drift coefficient */;
    }

    fn diffusion(&self, t: f64, x: &[f64], out: &mut [f64]) {
        out[0] = /* diffusion coefficient */;
    }

    fn populate_path_state(&self, x: &[f64], state: &mut PathState) {
        state.set(state_keys::SPOT, x[0]);
    }
}
```

3. Add `pub mod my_process;` to `process/mod.rs`
4. Add to prelude in `mod.rs`
5. Implement `ProcessMetadata` if path capture is needed

### Adding a New Discretization Scheme

1. Create `discretization/my_scheme.rs`
2. Implement `Discretization<MyProcess>` (or make it generic):

```rust
pub struct MyScheme;

impl Discretization<MyProcess> for MyScheme {
    fn step(&self, process: &MyProcess, t: f64, dt: f64, x: &mut [f64], z: &[f64], work: &mut [f64]) {
        // Update x in-place
    }

    fn work_size(&self, _process: &MyProcess) -> usize { 0 }
}
```

3. Add to `discretization/mod.rs` and the prelude

### Adding a New Payoff

1. Create `payoff/my_payoff.rs`
2. Implement `Payoff` (must be `Clone + Send + Sync`):

```rust
#[derive(Clone)]
pub struct MyPayoff { /* config + internal state */ }

impl Payoff for MyPayoff {
    fn on_event(&mut self, state: &mut PathState) {
        let spot = state.spot().unwrap_or(0.0);
        // Accumulate observations, check conditions, record cashflows
        state.add_typed_cashflow(state.time, amount, CashflowType::Interest);
    }

    fn value(&self, currency: Currency) -> Money {
        Money::new(self.accumulated_value, currency)
    }

    fn reset(&mut self) {
        // Reset all path-specific state for the next simulation
    }

    fn on_path_start<R: RandomStream>(&mut self, rng: &mut R) {
        // Optional: draw per-path random variables
    }
}
```

3. Add to `payoff/mod.rs` and the prelude
4. Feature-gate under `#[cfg(feature = "mc")]` if the payoff is exotic

### Adding a New Variance Reduction Technique

1. Create `variance_reduction/my_technique.rs`
2. Implement as standalone functions or a struct that wraps the simulation loop
3. Add to `variance_reduction/mod.rs`

---

## Performance Notes

- **MC convergence**: Standard MC error decays as O(N⁻¹/²). Doubling paths halves the standard error.
- **QMC convergence**: Sobol sequences with Brownian bridge can achieve effective rates up to O(N⁻¹) for smooth integrands.
- **Parallelism**: Adaptive chunk sizing targets 4 chunks per thread. Chunks are clamped to [100, 10000] paths for good cache behavior.
- **Buffer reuse**: The engine pre-allocates state, shock, and workspace buffers and reuses them across paths (serial mode) or per-chunk (parallel mode).
- **Vectorized RNG**: `fill_u01` and `fill_std_normals` operate on slices, enabling SIMD-friendly access patterns.
- **Online statistics**: Welford's algorithm provides numerically stable mean/variance in a single pass with O(1) memory.
- **Auto-stopping**: The `target_ci_half_width` option terminates early once the 95% CI is tight enough, avoiding unnecessary computation.

---

## Academic References

### Core Methods

1. Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*. Springer.
   — Comprehensive treatment of MC simulation for derivatives pricing.

2. Kloeden, P. E. & Platen, E. (1992). *Numerical Solution of Stochastic Differential Equations*. Springer.
   — Euler-Maruyama, Milstein, and higher-order SDE discretization.

### Early Exercise

3. Longstaff, F. A. & Schwartz, E. S. (2001). "Valuing American Options by Simulation: A Simple Least-Squares Approach." *Review of Financial Studies*, 14(1), 113–147.
   — Least-squares Monte Carlo (LSMC) algorithm.

### Stochastic Volatility

4. Heston, S. L. (1993). "A Closed-Form Solution for Options with Stochastic Volatility with Applications to Bond and Currency Options." *Review of Financial Studies*, 6(2), 327–343.

5. Andersen, L. (2008). "Simple and Efficient Simulation of the Heston Stochastic Volatility Model." *Journal of Computational Finance*, 11(3), 1–42.
   — Quadratic-exponential (QE) scheme for Heston/CIR variance.

### Jump Processes

6. Merton, R. C. (1976). "Option Pricing When Underlying Stock Returns Are Discontinuous." *Journal of Financial Economics*, 3(1-2), 125–144.

7. Bates, D. S. (1996). "Jumps and Stochastic Volatility: Exchange Rate Processes Implicit in Deutsche Mark Options." *Review of Financial Studies*, 9(1), 69–107.

### Interest Rate Models

8. Hull, J. & White, A. (1990). "Pricing Interest-Rate-Derivative Securities." *Review of Financial Studies*, 3(4), 573–592.

9. Cox, J. C., Ingersoll, J. E. & Ross, S. A. (1985). "A Theory of the Term Structure of Interest Rates." *Econometrica*, 53(2), 385–407.
   — CIR process.

### Commodity Models

10. Schwartz, E. & Smith, J. E. (2000). "Short-Term Variations and Long-Term Dynamics in Commodity Prices." *Management Science*, 46(7), 893–911.

### Random Number Generation

11. Salmon, J. K. et al. (2011). "Parallel Random Numbers: As Easy as 1, 2, 3." *Proceedings of SC '11*.
    — Philox counter-based PRNG.

12. Joe, S. & Kuo, F. Y. (2003). "Remark on Algorithm 659: Implementing Sobol's Quasirandom Sequence Generator." *ACM Transactions on Mathematical Software*, 29(1), 49–57.

13. Owen, A. B. (1998). "Scrambling Sobol and Niederreiter-Xing Points." *Journal of Complexity*, 14(4), 466–489.

### Barrier Corrections

14. Gobet, E. & Miri, M. (2001). "Weak Approximation of Averaged Diffusion Processes." *Stochastic Processes and their Applications*.
    — Gobet-Miri continuity correction for discrete barrier monitoring.

### Variance Reduction

15. Kong, A. et al. (1994). "Sequential Imputations and Bayesian Missing Data Problems." *Journal of the American Statistical Association*, 89(425), 278–288.
    — ESS diagnostics for importance sampling.

### Credit Risk

16. Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.). Chapter 24.

17. Brigo, D. & Mercurio, F. (2006). *Interest Rate Models — Theory and Practice* (2nd ed.). Chapter 21.
