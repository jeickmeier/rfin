# Credit Migration Models (JLT / CreditMetrics-Style Transition Matrices)

## Overview

Add credit migration modeling to `finstack-core` as a new `credit/migration` module. Phase 1 delivers time-homogeneous continuous-time Markov chain (CTMC) infrastructure: transition matrix validation, generator matrix extraction via matrix logarithm, matrix exponentiation for arbitrary time horizons, and single-obligor rating path simulation via Gillespie's algorithm.

Credit migration models underpin portfolio credit risk (CreditMetrics, KMV), CVA/XVA calculations, CLO tranche pricing, and stress testing. The module provides the mathematical primitives that downstream crates (`correlation`, `monte_carlo`, `portfolio`, `scenarios`) build on.

## Scope

### In Scope (Phase 1)

- `RatingScale`: flexible ordered state set (standard ratings, notched, with/without NR, or custom)
- `TransitionMatrix`: validated row-stochastic N×N matrix with time horizon
- `GeneratorMatrix`: validated continuous-time intensity matrix Q
- Generator extraction: matrix logarithm via eigendecomposition with Kreinin-Sidenius post-processing
- Matrix exponentiation: scaling-and-squaring with Padé approximation + eigendecomposition for small matrices
- CTMC path simulation: Gillespie's competing exponentials algorithm
- Batch simulation with empirical transition matrix estimation
- Round-trip validation: exp(Q) ≈ P within configurable tolerance
- Integration with existing `CreditRating` enum via convenience constructors

### Deferred (Phase 2+)

- Time-inhomogeneous generators (economic-state-conditional migration)
- Full JLT model with stochastic migration intensities driven by latent factors
- Correlated multi-obligor simulation (CreditMetrics factor model)
- Calibration from raw rating history data (cohort/duration methods)
- Built-in historical transition matrices (data licensing concerns)
- Through-the-cycle vs point-in-time matrix adjustment
- Rating momentum / non-Markov effects

## Architecture

### Module Layout

```
finstack/core/src/
  credit/
    mod.rs                  — Module docs, re-exports
    migration/
      mod.rs                — Submodule docs, re-exports
      scale.rs              — RatingScale type
      matrix.rs             — TransitionMatrix type, validation, accessors
      generator.rs          — GeneratorMatrix type, extraction from P via matrix log
      projection.rs         — Matrix exponentiation: P(t) = exp(Qt)
      simulation.rs         — CTMC path simulation (Gillespie), batch simulation
```

### Dependency Flow

```
RatingScale ← TransitionMatrix ← GeneratorMatrix ← Projection
                                                   ← Simulation
```

- `RatingScale` is standalone (no math dependencies)
- `TransitionMatrix` owns a `RatingScale` and a `DMatrix<f64>`
- `GeneratorMatrix` owns a `RatingScale` and a `DMatrix<f64>`; constructed from a `TransitionMatrix` or directly
- `projection` operates on `GeneratorMatrix` → produces `TransitionMatrix`
- `simulation` operates on `GeneratorMatrix` → produces `RatingPath`

### External Dependencies

All already in the workspace:

| Crate | Usage |
|-------|-------|
| `nalgebra` | `DMatrix<f64>`, eigendecomposition, matrix arithmetic |
| `rand` | `Rng` trait for simulation |
| `serde` | Serialization of all public types |
| `thiserror` | Error variants |

No new dependencies required.

## Core Types

### `RatingScale`

An ordered set of state labels defining the dimensions and label-to-index mapping for transition matrices.

```rust
/// An ordered set of states defining a transition matrix's row/column layout.
///
/// States are identified by string labels for flexibility across rating
/// granularities (coarse, notched, with/without NR). The scale defines
/// which index is the absorbing default state (if any).
///
/// # Examples
///
/// ```
/// use finstack_core::credit::migration::RatingScale;
///
/// let scale = RatingScale::standard(); // [AAA, AA, A, BBB, BB, B, CCC, CC, C, D]
/// assert_eq!(scale.n_states(), 10);
/// assert_eq!(scale.index_of("BBB"), Some(3));
/// assert_eq!(scale.default_state(), Some(9)); // D
/// ```
pub struct RatingScale {
    labels: Vec<String>,
    index_map: HashMap<String, usize>,
    default_state: Option<usize>,
}
```

**Preset constructors:**

| Method | States | Default |
|--------|--------|---------|
| `standard()` | AAA, AA, A, BBB, BB, B, CCC, CC, C, D | D (index 9) |
| `standard_with_nr()` | AAA, AA, A, BBB, BB, B, CCC, CC, C, NR, D | D (index 10) |
| `notched()` | AAA, AA+, AA, AA-, A+, A, A-, BBB+, BBB, BBB-, BB+, BB, BB-, B+, B, B-, CCC+, CCC, CCC-, CC, C, D | D (index 21) |
| `custom(labels)` | User-provided | Last state |
| `custom_with_default(labels, default_label)` | User-provided | Specified label |

**Validation:**
- At least 2 states
- No duplicate labels
- Default label (if specified) must exist in labels

### `TransitionMatrix`

A validated N×N row-stochastic matrix representing transition probabilities over a fixed time horizon.

```rust
/// Row-stochastic transition matrix for a discrete-state Markov chain.
///
/// Entry (i, j) is the probability of transitioning from state i to state j
/// over the matrix's time horizon. All entries are in [0, 1] and each row
/// sums to 1.
///
/// # Construction
///
/// ```
/// use finstack_core::credit::migration::{TransitionMatrix, RatingScale};
///
/// let scale = RatingScale::standard();
/// // 10×10 row-major data (rows sum to 1)
/// let data: Vec<f64> = /* ... */;
/// let matrix = TransitionMatrix::new(scale, &data, 1.0)?;
/// ```
pub struct TransitionMatrix {
    data: DMatrix<f64>,
    horizon: f64,
    scale: RatingScale,
}
```

**Public API:**

```rust
impl TransitionMatrix {
    /// Construct from row-major data with validation.
    pub fn new(scale: RatingScale, data: &[f64], horizon: f64) -> Result<Self>;

    /// Transition probability P(from → to).
    pub fn probability(&self, from: &str, to: &str) -> Result<f64>;

    /// Transition probability by index.
    pub fn probability_by_index(&self, from: usize, to: usize) -> f64;

    /// Row of transition probabilities from a given state.
    pub fn row(&self, from: &str) -> Result<&[f64]>;

    /// The underlying matrix.
    pub fn as_matrix(&self) -> &DMatrix<f64>;

    /// Time horizon in years.
    pub fn horizon(&self) -> f64;

    /// The rating scale.
    pub fn scale(&self) -> &RatingScale;

    /// Number of states.
    pub fn n_states(&self) -> usize;

    /// Multiply two transition matrices (must share the same scale).
    /// P(s+t) = P(s) × P(t) for time-homogeneous chains.
    pub fn compose(&self, other: &TransitionMatrix) -> Result<TransitionMatrix>;

    /// Default probability vector: column corresponding to the default state.
    pub fn default_probabilities(&self) -> Option<Vec<f64>>;
}
```

**Validation on construction:**
- Data length = n_states²
- All entries in [0.0, 1.0]
- Each row sums to 1.0 (tolerance: 1e-8)
- Horizon > 0
- If default state is set, its row must be absorbing: [0, ..., 0, 1]

### `GeneratorMatrix`

The continuous-time intensity matrix Q where off-diagonal entries are transition rates and rows sum to 0.

```rust
/// Continuous-time generator (intensity) matrix for a CTMC.
///
/// Off-diagonal entry q_ij (i ≠ j) is the instantaneous rate of transitioning
/// from state i to state j. Diagonal entry q_ii = -Σ_{j≠i} q_ij so rows
/// sum to zero.
pub struct GeneratorMatrix {
    data: DMatrix<f64>,
    scale: RatingScale,
}
```

**Public API:**

```rust
impl GeneratorMatrix {
    /// Construct directly from row-major data with validation.
    pub fn new(scale: RatingScale, data: &[f64]) -> Result<Self>;

    /// Extract generator from an annual transition matrix via matrix logarithm.
    pub fn from_transition_matrix(matrix: &TransitionMatrix) -> Result<Self>;

    /// Transition intensity q_ij.
    pub fn intensity(&self, from: &str, to: &str) -> Result<f64>;

    /// Total exit rate from a state: -q_ii.
    pub fn exit_rate(&self, state: &str) -> Result<f64>;

    /// The underlying matrix.
    pub fn as_matrix(&self) -> &DMatrix<f64>;

    /// The rating scale.
    pub fn scale(&self) -> &RatingScale;
}
```

**Validation:**
- Off-diagonal entries >= 0
- Diagonal entries <= 0
- Each row sums to 0 (tolerance: 1e-8)
- If default state is set, its row must be zero (absorbing)

## Algorithms

### Generator Extraction (Matrix Logarithm)

Extract Q from P where P = exp(Q).

**Primary algorithm: Eigendecomposition**

1. Compute eigendecomposition: P = V Λ V⁻¹ via `nalgebra`
2. For each eigenvalue λ_k:
   - If λ_k <= 0: return `NoValidGenerator` error (P has no real generator)
   - Otherwise: ln(λ_k)
3. Reconstruct: Q = V diag(ln λ₁, ..., ln λ_n) V⁻¹
4. Take real part (imaginary components cancel for real P, but floating-point may leave residuals)
5. **Kreinin-Sidenius post-processing:**
   - For each off-diagonal entry q_ij < 0: set q_ij = 0
   - Recompute diagonal: q_ii = -Σ_{j≠i} q_ij
6. Validate round-trip: ||exp(Q) - P||_∞ < tolerance (default 1e-6)

**Fallback: Padé approximant of matrix logarithm**

If eigendecomposition fails (non-diagonalizable matrix), use inverse scaling and squaring with Padé approximation of log(I + X) for X = P - I.

**References:**
- Israel, R., Rosenthal, J., & Wei, J. (2001). "Finding Generators for Markov Chains via Empirical Transition Matrices." *Mathematical Finance*, 11(2), 245-265.
- Kreinin, A., & Sidenius, J. (2001). "Regularization Algorithms for Transition Matrices." *Algo Research Quarterly*, 4(1/2), 23-40.

### Matrix Exponentiation

Compute P(t) = exp(Qt) for arbitrary time horizon t.

**Algorithm 1: Eigendecomposition (default for N ≤ 20)**

1. Compute eigendecomposition: Q = V Λ V⁻¹
2. P(t) = V diag(exp(λ₁t), ..., exp(λ_nt)) V⁻¹
3. Post-process: clamp negative entries to 0, re-normalize rows

Efficient for small matrices (all credit rating scales qualify). O(N³) one-time decomposition, then O(N²) per time horizon.

**Algorithm 2: Scaling and squaring with Padé (for N > 20 or user request)**

1. Compute s = max(0, ceil(log2(||Qt||_∞ / θ))) where θ is the Padé threshold
2. Compute r_13(Qt / 2^s) using [13/13] Padé approximation
3. Square s times: P = r^(2^s)
4. Post-process: clamp negatives, re-normalize rows

**References:**
- Higham, N. J. (2005). "The Scaling and Squaring Method for the Matrix Exponential Revisited." *SIAM Journal on Matrix Analysis and Applications*, 26(4), 1179-1193.
- Moler, C., & Van Loan, C. (2003). "Nineteen Dubious Ways to Compute the Exponential of a Matrix, Twenty-Five Years Later." *SIAM Review*, 45(1), 3-49.

**Public API:**

```rust
/// Compute P(t) = exp(Q * t) for the given time horizon.
pub fn project(generator: &GeneratorMatrix, t: f64) -> Result<TransitionMatrix>;

/// Compute P(t) using eigendecomposition (explicit algorithm choice).
pub fn project_eigen(generator: &GeneratorMatrix, t: f64) -> Result<TransitionMatrix>;

/// Compute P(t) using scaling-and-squaring with Padé (explicit algorithm choice).
pub fn project_pade(generator: &GeneratorMatrix, t: f64) -> Result<TransitionMatrix>;
```

### CTMC Path Simulation (Gillespie's Algorithm)

Simulate an individual obligor's rating trajectory as a continuous-time Markov chain.

**Algorithm: Competing exponentials**

```
Given: generator Q, initial state s₀, time horizon T
Set t = 0, s = s₀
Loop:
  1. Exit rate: λ = -q_{ss}
  2. If λ ≈ 0 (absorbing state): record (t, s), break
  3. Draw holding time: τ ~ Exp(λ)  [i.e., τ = -ln(U₁)/λ, U₁ ~ Uniform(0,1)]
  4. If t + τ > T: stay in s until T, break
  5. Jump probabilities: p_j = q_{sj} / λ for j ≠ s
  6. Draw next state j from Categorical(p₁, ..., p_n) using U₂ ~ Uniform(0,1)
  7. Record transition (t + τ, j)
  8. Set t = t + τ, s = j
```

**Output type:**

```rust
/// A simulated rating trajectory: sequence of (time, state_index) pairs.
pub struct RatingPath {
    transitions: Vec<(f64, usize)>,
    horizon: f64,
    scale: RatingScale,
}

impl RatingPath {
    /// State at time t (piecewise constant, right-continuous).
    pub fn state_at(&self, t: f64) -> usize;

    /// State label at time t.
    pub fn label_at(&self, t: f64) -> &str;

    /// Whether default occurred.
    pub fn defaulted(&self) -> bool;

    /// Time of default (if any).
    pub fn default_time(&self) -> Option<f64>;

    /// Number of transitions (excluding initial state).
    pub fn n_transitions(&self) -> usize;

    /// All transition events.
    pub fn transitions(&self) -> &[(f64, usize)];
}
```

**Batch simulation:**

```rust
/// Simulator for generating rating paths from a generator matrix.
pub struct MigrationSimulator {
    generator: GeneratorMatrix,
    horizon: f64,
}

impl MigrationSimulator {
    pub fn new(generator: GeneratorMatrix, horizon: f64) -> Result<Self>;

    /// Simulate n_paths independent paths from initial_state.
    pub fn simulate<R: Rng>(
        &self,
        initial_state: usize,
        n_paths: usize,
        rng: &mut R,
    ) -> Vec<RatingPath>;

    /// Estimate transition matrix from simulation (all initial states).
    pub fn empirical_matrix<R: Rng>(
        &self,
        n_paths_per_state: usize,
        rng: &mut R,
    ) -> TransitionMatrix;
}
```

## Error Handling

All errors funnel through a `MigrationError` enum that integrates with `crate::Error`:

```rust
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("matrix is not square: {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },

    #[error("matrix dimension {actual} does not match rating scale size {expected}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("row {row} sums to {sum}, expected {expected} (tolerance {tol})")]
    RowSumViolation { row: usize, sum: f64, expected: f64, tol: f64 },

    #[error("entry ({row},{col}) = {value} is outside [{min},{max}]")]
    EntryOutOfRange { row: usize, col: usize, value: f64, min: f64, max: f64 },

    #[error("generator extraction failed: eigenvalue {index} = {value} is non-positive")]
    NoValidGenerator { index: usize, value: f64 },

    #[error("round-trip error ||exp(Q)-P||_inf = {error} exceeds tolerance {tolerance}")]
    RoundTripError { error: f64, tolerance: f64 },

    #[error("state '{label}' not found in rating scale")]
    UnknownState { label: String },

    #[error("rating scale must have at least 2 states")]
    InsufficientStates,

    #[error("duplicate state label '{label}'")]
    DuplicateLabel { label: String },

    #[error("absorbing state {state} has non-zero off-diagonal entries")]
    NonAbsorbingDefault { state: usize },

    #[error("horizon must be positive, got {0}")]
    InvalidHorizon(f64),
}
```

## Testing Strategy

### Unit Tests

1. **RatingScale:** preset constructors, custom construction, label lookup, validation failures (duplicates, empty)

2. **TransitionMatrix:**
   - 2×2 identity: P(from=0, to=0) = 1.0
   - Validation: reject non-square, negative entries, non-summing rows, non-absorbing default
   - Compose: P × I = P, P(1) × P(1) ≈ P(2) for known matrices

3. **GeneratorMatrix / extraction:**
   - 2×2 known case: P = [[0.9, 0.1], [0.0, 1.0]] → Q = [[ln(0.9), -ln(0.9)], [0, 0]]
   - Round-trip: extract Q from P, verify exp(Q) ≈ P
   - Kreinin-Sidenius correction: verify negative off-diagonals are clamped
   - Error case: matrix with zero eigenvalue → `NoValidGenerator`

4. **Matrix exponentiation:**
   - exp(0) = I
   - Semi-group: exp(Q·s) × exp(Q·t) ≈ exp(Q·(s+t))
   - Row-stochastic: all rows sum to 1.0, all entries in [0, 1]
   - Eigen vs Padé produce same result for small matrices

5. **Simulation:**
   - Absorbing state: all paths from D stay in D
   - Convergence: empirical matrix → analytical matrix as n_paths → ∞ (statistical test with tolerance)
   - Deterministic seed: same seed produces same path
   - Single-state chain: trivial case

### Reference Test

Use the following stylized 7×7 annual transition matrix (representative of investment-grade dynamics from academic literature):

| From\To | AAA   | AA    | A     | BBB   | BB    | B     | D     |
|---------|-------|-------|-------|-------|-------|-------|-------|
| AAA     | 0.9081| 0.0833| 0.0068| 0.0006| 0.0012| 0.0000| 0.0000|
| AA      | 0.0070| 0.9065| 0.0779| 0.0064| 0.0006| 0.0014| 0.0002|
| A       | 0.0009| 0.0227| 0.9105| 0.0552| 0.0074| 0.0026| 0.0007|
| BBB     | 0.0002| 0.0033| 0.0595| 0.8693| 0.0530| 0.0117| 0.0030|
| BB      | 0.0003| 0.0014| 0.0067| 0.0773| 0.8053| 0.0884| 0.0206|
| B       | 0.0000| 0.0011| 0.0024| 0.0043| 0.0648| 0.8346| 0.0928|
| D       | 0.0000| 0.0000| 0.0000| 0.0000| 0.0000| 0.0000| 1.0000|

Verify: generator extraction, 6-month and 5-year projections, simulation convergence.

## References

- Israel, R., Rosenthal, J., & Wei, J. (2001). "Finding Generators for Markov Chains via Empirical Transition Matrices." *Mathematical Finance*, 11(2), 245-265.
- Kreinin, A., & Sidenius, J. (2001). "Regularization Algorithms for Transition Matrices." *Algo Research Quarterly*, 4(1/2), 23-40.
- Jarrow, R. A., Lando, D., & Turnbull, S. M. (1997). "A Markov Model for the Term Structure of Credit Risk Spreads." *Review of Financial Studies*, 10(2), 481-523.
- Higham, N. J. (2005). "The Scaling and Squaring Method for the Matrix Exponential Revisited." *SIAM Journal on Matrix Analysis and Applications*, 26(4), 1179-1193.
- Moler, C., & Van Loan, C. (2003). "Nineteen Dubious Ways to Compute the Exponential of a Matrix, Twenty-Five Years Later." *SIAM Review*, 45(1), 3-49.
- Lando, D., & Skodeberg, T. M. (2002). "Analyzing Rating Transitions and Rating Drift with Continuous Observations." *Journal of Banking & Finance*, 26(2-3), 423-444.
- Gupton, G. M., Finger, C. C., & Bhatia, M. (1997). *CreditMetrics — Technical Document*. J.P. Morgan.
