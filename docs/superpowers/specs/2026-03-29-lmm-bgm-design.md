# LIBOR Market Model (LMM/BGM) Design Spec

## Summary

Add a 2-3 factor LIBOR Market Model (BGM) to the finstack crate with displaced diffusion, co-terminal swaption calibration, and Bermudan swaption pricing via LSMC. The implementation follows the established `StochasticProcess` + `Discretization` pattern used by Hull-White 1F and Heston.

## Scope

**In scope (Phase 1):**
- LMM stochastic process with SOFR forward rates
- 2-3 factor support with piecewise-constant instantaneous volatilities
- Displaced diffusion for negative-rate support
- Predictor-corrector discretization (Glasserman 2003)
- Co-terminal swaption calibration (vol stripping + PCA factor decomposition)
- Bermudan swaption pricing via LSMC with LMM dynamics
- `ModelKey::LmmMonteCarlo` and pricer registry integration

**Deferred (Phase 2):**
- CMS convexity adjustment via static replication (Hagan-style)
- SABR-LMM and stochastic-vol LMM extensions
- Exotic rate products (callable range accruals, TARNs, snowballs)
- Full swaption matrix calibration

## Architecture

### Approach

LMM is implemented as a new `StochasticProcess` with a dedicated `Discretization`, following the same pattern as `HestonProcess` + `QeHeston` and `HullWhite1FProcess` + `ExactHullWhite1F`. This plugs into the existing MC engine, LSMC framework, and pricer registry with no changes to core infrastructure.

### Component 1: LMM Process

**File:** `finstack/monte_carlo/src/process/lmm.rs`

**SDE (displaced diffusion, terminal measure T_N):**

```
dF_i(t) = mu_i(t, F) dt + (F_i(t) + d_i) * sum_k lambda_{i,k}(t) * dW_k(t)
```

where:
- `F_i(t)` = i-th SOFR forward rate for period `[T_i, T_{i+1}]`
- `d_i` = displacement (shift) for negative-rate support
- `lambda_{i,k}(t)` = factor loading of forward i on Brownian motion k
- `mu_i` = drift correction under terminal measure

**Drift under terminal measure T_N:**

```
mu_i(t, F) = -sum_{j=i+1}^{N-1} [tau_j * (F_j + d_j) / (1 + tau_j * F_j)] * rho_ij * sigma_i * sigma_j
```

where `rho_ij = lambda_i . lambda_j` (dot product of factor loadings) and `sigma_i = |lambda_i|`.

**Parameters:**

```rust
pub struct LmmParams {
    pub num_forwards: usize,
    pub num_factors: usize,              // 2 or 3
    pub tenors: Vec<f64>,                // T_0, T_1, ..., T_N (N+1 dates)
    pub accrual_factors: Vec<f64>,       // tau_i = T_{i+1} - T_i (length N)
    pub displacements: Vec<f64>,         // d_i per forward (length N)
    pub vol_times: Vec<f64>,             // Piecewise-constant vol breakpoints
    pub vol_values: Vec<Vec<[f64; 3]>>,  // lambda_i(t) per forward, per time period
    pub initial_forwards: Vec<f64>,      // F_i(0) from the curve (length N)
}
```

**Process struct:**

```rust
pub struct LmmProcess {
    params: LmmParams,
    // Pre-computed at construction:
    // - factor loading matrix per vol period
    // - correlation matrix rho_ij = lambda_i . lambda_j
}
```

**Trait implementation:**
- `dim()` = N (number of forward rates)
- `num_factors()` = K (2 or 3)
- `is_diagonal()` = false
- `drift()` computes terminal-measure drift correction for all alive forwards
- `diffusion()` returns N x K factor-loading matrix (row-major)
- `populate_path_state()` stores forward rates via `indexed_spot(i)` and computes the current numeraire ratio `P(t, T_k)/P(t, T_N)` from forwards for discounting

### Component 2: Predictor-Corrector Discretization

**File:** `finstack/monte_carlo/src/discretization/lmm_predictor_corrector.rs`

Implements `Discretization<LmmProcess>`. Follows Glasserman (2003) predictor-corrector scheme.

**Algorithm (one time step t -> t+dt):**

1. Generate K standard normals Z_1, ..., Z_K (provided by engine via `z` slice)
2. **Predictor:** Euler step with drift computed from current forwards F_i(t)
   - `F_i^{pred} = F_i(t) + mu_i(t, F(t)) * (F_i(t) + d_i) * dt + (F_i(t) + d_i) * sum_k lambda_{i,k}(t) * Z_k * sqrt(dt)`
3. **Corrector:** Recompute drift at predicted forwards, average with predictor drift
   - `mu_i^{avg} = (mu_i(t, F(t)) + mu_i(t, F^{pred})) / 2`
   - `F_i(t+dt) = F_i(t) + mu_i^{avg} * (F_i(t) + d_i) * dt + (F_i(t) + d_i) * sum_k lambda_{i,k}(t) * Z_k * sqrt(dt)`
4. **Floor:** `F_i(t+dt) = max(F_i(t+dt), -d_i)` (displaced diffusion floor)

**Optimization:** Only evolve "alive" forwards (those with T_i > t). Dead forwards are frozen at their last value.

**Struct:**

```rust
pub struct LmmPredictorCorrector {
    skip_dead_forwards: bool,  // default true
}

impl Discretization<LmmProcess> for LmmPredictorCorrector {
    fn step(&self, process: &LmmProcess, t: f64, dt: f64,
            x: &mut [f64], z: &[f64], work: &mut [f64]);
    fn work_size(&self, process: &LmmProcess) -> usize;
    // work buffer: N floats for predicted forwards + N floats for drift vectors
}
```

### Component 3: Co-Terminal Swaption Calibration

**File:** `finstack/valuations/src/calibration/lmm.rs`

Two-stage calibration to co-terminal swaption volatilities.

**Stage 1 -- Instantaneous volatility stripping:**

For each forward rate F_i, extract scalar instantaneous volatility sigma_i(t) from the co-terminal swaption exercisable at T_i. Uses Rebonato's approximate swaption volatility formula:

```
sigma_swap^2 ~= (1/T_ex) * sum_{i,j} w_i * w_j * rho_ij * integral_0^{T_ex} sigma_i(t) * sigma_j(t) dt
```

where w_i are annuity-weighted forward rate contributions to the swap rate. With piecewise-constant vols, the integral is analytic.

Strip iteratively from the shortest-expiry co-terminal swaption, since each depends on fewer unknowns than the next.

**Stage 2 -- Factor decomposition (PCA):**

1. Parameterize correlation: `rho_ij = exp(-beta * |T_i - T_j|)` (exponential decay)
2. Eigendecompose: `rho = V * Lambda * V^T`
3. Retain top K eigenvectors: factor loadings `epsilon_i = V_{i,1:K} * Lambda_{1:K}^{1/2}`
4. Scale: `lambda_{i,k}(t) = sigma_i(t) * epsilon_{i,k}`
5. Optionally calibrate beta (correlation decay) by minimizing repricing error across the co-terminal set via Levenberg-Marquardt (reuses existing LM solver from HW1F calibration)

**Public API:**

```rust
pub struct LmmCalibrationResult {
    pub params: LmmParams,
    pub report: CalibrationReport,  // Reuses existing type
}

pub fn calibrate_lmm_to_coterminal_swaptions(
    forwards: &[f64],
    discount_fn: &dyn Fn(f64) -> f64,
    tenors: &[f64],
    quotes: &[SwaptionQuote],       // Reuses existing type
    num_factors: usize,
    displacements: &[f64],
) -> Result<LmmCalibrationResult>;
```

### Component 4: Bermudan Swaption Pricing via LSMC

**File:** `finstack/valuations/src/instruments/rates/swaption/pricing/lmm_bermudan.rs`

Reuses the existing MC engine and LSMC framework.

**Pricing flow:**

1. Calibrate LMM to co-terminal swaptions (or accept pre-calibrated LmmParams)
2. Build `LmmProcess` from calibrated params + initial forwards
3. Configure `McEngine` with `LmmProcess` + `LmmPredictorCorrector`
4. Time grid aligned to exercise dates and forward fixing dates
5. Simulate paths; at each step, `populate_path_state` writes forward rates and numeraire
6. LSMC backward induction at each exercise date:
   - Exercise value: `V_exercise = sum_i tau_i * (F_i(t) - K) * P(t, T_{i+1})` (from forwards in PathState)
   - Continuation value: polynomial regression on basis functions
   - Exercise decision: `V_exercise > V_continuation`

**Payoff struct:**

```rust
#[derive(Clone)]
pub struct BermudanSwaptionLmmPayoff {
    exercise_dates: Vec<f64>,     // Exercise times (year fractions)
    strike: f64,                  // Fixed rate K
    payer: bool,                  // true = payer swaption
    num_forwards: usize,
    accrual_factors: Vec<f64>,    // tau_i
}

impl Payoff for BermudanSwaptionLmmPayoff {
    fn on_event(&mut self, state: &mut PathState);
    fn value(&self, currency: Currency) -> Money;
    fn reset(&mut self);
}
```

**LSMC basis functions:**
- Forward swap rate S(t)
- Annuity A(t)
- S(t)^2
- First 2-3 principal components of alive forward rates (yield curve shape)

**Numeraire handling:**
Simulation under terminal measure uses `P(t, T_N)` as numeraire. Payoff `on_event` accumulates values in numeraire units (divided by `P(t, T_N)`). The `discount_factor` passed to the MC engine is `P(0, T_N)`.

**Pricer integration:**

```rust
pub struct BermudanSwaptionLmmPricer {
    config: LmmPricerConfig,
}

impl Pricer for BermudanSwaptionLmmPricer {
    fn key(&self) -> PricerKey {
        (InstrumentType::Swaption, ModelKey::LmmMonteCarlo)
    }
    fn price_dyn(&self, instrument: &dyn Instrument, market: &MarketContext, as_of: Date)
        -> Result<ValuationResult>;
}
```

Adds `ModelKey::LmmMonteCarlo` variant to the existing enum and registers the pricer in `standard_registry()`.

## File Layout

**New files (4):**

| File | Purpose |
|------|---------|
| `finstack/monte_carlo/src/process/lmm.rs` | `LmmProcess`, `LmmParams`, `StochasticProcess` impl |
| `finstack/monte_carlo/src/discretization/lmm_predictor_corrector.rs` | `LmmPredictorCorrector`, `Discretization<LmmProcess>` impl |
| `finstack/valuations/src/calibration/lmm.rs` | Co-terminal swaption calibration (vol stripping + PCA) |
| `finstack/valuations/src/instruments/rates/swaption/pricing/lmm_bermudan.rs` | `BermudanSwaptionLmmPayoff`, `BermudanSwaptionLmmPricer` |

**Modified files (small edits):**

| File | Change |
|------|--------|
| `finstack/monte_carlo/src/process/mod.rs` | Add `pub mod lmm; pub use lmm::*;` |
| `finstack/monte_carlo/src/discretization/mod.rs` | Add `pub mod lmm_predictor_corrector; pub use ...;` |
| `finstack/valuations/src/calibration/mod.rs` | Add `pub mod lmm;` |
| `finstack/valuations/src/instruments/rates/swaption/pricing/mod.rs` | Add `pub mod lmm_bermudan;` |
| `finstack/valuations/src/instruments/common/models/mod.rs` (or wherever `ModelKey` lives) | Add `LmmMonteCarlo` variant |
| `finstack/valuations/src/pricer/` (registry) | Register `BermudanSwaptionLmmPricer` |

## Dependencies

No new external crates. Uses:
- `nalgebra` (already in workspace) for eigendecomposition in PCA
- Existing LM solver from calibration module
- Existing `SwaptionQuote`, `CalibrationReport`, `McEngine`, LSMC framework

## Testing Strategy

**Unit tests (in each new file):**
- Drift formula: verify against hand-computed values for 2-3 forward setup
- Predictor-corrector: single-step convergence, forward rate positivity with displacement
- Vol stripping: round-trip (strip vols -> Rebonato formula -> recover input swaption vols)
- PCA: verify factor loadings reproduce correlation matrix within truncation error

**Integration tests:**
- Calibrate to synthetic co-terminal swaptions -> price European swaption -> compare to Rebonato approximation (match within MC noise, ~1-2bp)
- Bermudan price >= European price (exercise premium non-negative)
- Convergence: Bermudan price stabilizes as num_paths increases (check CI narrows)

**Regression tests:**
- Known benchmark: 10Y Bermudan payer swaption (annually exercisable), compare to HW1F tree price as a sanity cross-check (not exact match, but same order of magnitude)

## References

- Brace, Gatarek, Musiela (1997) - "The Market Model of Interest Rate Dynamics"
- Glasserman (2003) - "Monte Carlo Methods in Financial Engineering", Ch. 7 (predictor-corrector)
- Rebonato (2002) - "Modern Pricing of Interest-Rate Derivatives", Ch. 8-9 (calibration)
- Andersen & Piterbarg (2010) - "Interest Rate Modeling", Vol. 2, Ch. 15-16
- Brigo & Mercurio (2006) - "Interest Rate Models", Ch. 6-7

## Phase 2 Roadmap (Not In Scope)

For reference, the natural extensions after Phase 1:

1. **CMS convexity adjustment** via Hagan-style static replication
2. **SABR-LMM** with per-forward SABR dynamics for smile
3. **SV-LMM** with common stochastic volatility driver
4. **Exotic products:** callable range accruals, TARNs, CMS spread options
5. **Full swaption matrix calibration** with weighted least-squares
6. **Co-initial calibration** as alternative to co-terminal
