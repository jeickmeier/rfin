# PDE / Finite Difference Solvers Design

## Overview

Add a complete PDE/finite difference infrastructure to rfin as a new `finstack/pde` crate. Phase 1 delivers 1D Crank-Nicolson with non-uniform meshing, barrier support, and American exercise via penalty method. Phase 2 extends to 2D ADI for Heston, local-stochastic vol, and convertible bonds.

PDE methods are the workhorse for 1D-2D problems where Monte Carlo is overkill. Barrier options, convertible bonds, and American options all benefit from faster convergence and free Greeks on the solution grid.

## Architecture: Operator-Based PDE Framework

The design composes PDE problems from spatial operators and time-stepping schemes:

```
PdeProblem (coefficients + boundary conditions + domain)
  -> SpatialOperator (discretizes PDE into tridiagonal matrix on a Grid)
    -> ThetaStepper (advances solution in time using the operator)
      -> PenaltyExercise (enforces American/Bermudan constraint post-step)
```

This maps naturally to how FD methods work mathematically: a 1D problem has one spatial operator; a 2D ADI problem splits into directional operators that each reuse the same tridiagonal infrastructure.

### Key design decisions

- **New crate `finstack/pde`** parallel to `finstack/monte_carlo`. PDE infrastructure (grids, solvers, theta schemes) lives here; instrument-specific boundary conditions and payoffs stay in `finstack/valuations`.
- **nalgebra** for linear algebra where needed (already a dependency). Thomas algorithm hand-rolled for the hot path.
- **Own `PdeProblem` trait**, not reusing `StochasticProcess` from MC. A Feynman-Kac bridge converts between representations. PDE coefficients (`a`, `b`, `c`, `f`) are more natural than drift/diffusion for FD work, and the trait needs source terms, reaction terms, and boundary conditions that don't exist on the MC trait.
- **Penalty method** for American exercise (not PSOR). Simpler, works naturally with all theta schemes, no inner iteration tuning.
- **Phased rollout**: 1D first (validate infrastructure), then 2D ADI.

## Crate Structure

```
finstack/pde/src/
  lib.rs
  grid/
    mod.rs
    uniform.rs          # Uniform 1D grid
    nonuniform.rs       # Non-uniform 1D grid (sinh-based concentration)
    grid2d.rs           # Tensor product of two 1D grids (Phase 2)
  operator/
    mod.rs
    tridiag.rs          # 1D tridiagonal spatial operator
    adi.rs              # 2D ADI operator splitting (Phase 2)
  boundary/
    mod.rs
    conditions.rs       # Dirichlet, Neumann, linear boundary
  stepper/
    mod.rs
    theta.rs            # Theta scheme (explicit/implicit/CN)
    rannacher.rs        # Rannacher time-stepping (implicit start + CN)
    craig_sneyd.rs      # Craig-Sneyd ADI (Phase 2)
  exercise/
    mod.rs
    penalty.rs          # Penalty method for American/Bermudan
  problem.rs            # PdeProblem1D / PdeProblem2D traits
  solver.rs             # Top-level Solver1D (and Solver2D in Phase 2)
  results.rs            # PDE solution output type
  bridge.rs             # Feynman-Kac helpers (BlackScholesPde, LocalVolPde)
```

## Core Trait: PdeProblem1D

Coefficients for a 1D convection-diffusion-reaction PDE:

```
du/dt = a(x,t) * d2u/dx2 + b(x,t) * du/dx + c(x,t) * u + f(x,t)
```

```rust
pub trait PdeProblem1D {
    /// Diffusion coefficient a(x,t) — e.g., 0.5 * sigma^2 in log-spot
    fn diffusion(&self, x: f64, t: f64) -> f64;

    /// Convection coefficient b(x,t) — e.g., (r - q - 0.5*sigma^2) in log-spot
    fn convection(&self, x: f64, t: f64) -> f64;

    /// Reaction coefficient c(x,t) — e.g., -r for discounting
    fn reaction(&self, x: f64, t: f64) -> f64;

    /// Source term f(x,t) — default zero
    fn source(&self, _x: f64, _t: f64) -> f64 { 0.0 }

    /// Terminal condition u(x, T) = payoff(x)
    fn terminal_condition(&self, x: f64) -> f64;

    /// Lower boundary condition at x_min
    fn lower_boundary(&self, t: f64) -> BoundaryCondition;

    /// Upper boundary condition at x_max
    fn upper_boundary(&self, t: f64) -> BoundaryCondition;

    /// Hint: are coefficients time-independent? Enables operator caching.
    fn is_time_homogeneous(&self) -> bool { false }
}
```

Working in log-spot coordinates (`x = ln S`) is preferred:
- Coefficients simplify (constant diffusion for flat vol BS)
- Grid spacing maps to relative price moves
- Payoff kinks at `K` map to `ln K`

## Grid Infrastructure

### Grid1D

```rust
pub struct Grid1D {
    points: Vec<f64>,  // sorted, non-uniform spacing allowed
}
```

Construction methods:
- `Grid1D::uniform(x_min, x_max, n)` — evenly spaced
- `Grid1D::sinh_concentrated(x_min, x_max, n, concentration_point, intensity)` — sinh-based mesh concentration near a point (strike, barrier)
- `Grid1D::from_points(Vec<f64>)` — user-supplied arbitrary grid

**Sinh transformation:** `x = c + d * sinh(alpha * (xi - xi_0))` where `xi` is uniform on [0,1] and `c` is the concentration point. Parameter `alpha` controls intensity (smaller = more concentration). Analytically invertible, smooth spacing transitions.

**Barrier alignment:** Grid construction places nodes exactly on barrier levels to avoid interpolation error at barriers, which is the dominant FD error source for barrier options.

### Grid2D (Phase 2)

Tensor product of two `Grid1D`s — one for spot/log-spot, one for variance. Each axis gets its own concentration: spot axis near strike, variance axis near long-run variance theta.

## Spatial Operator: Tridiagonal System

### TridiagOperator

```rust
pub struct TridiagOperator {
    lower: Vec<f64>,   // sub-diagonal
    main: Vec<f64>,    // main diagonal
    upper: Vec<f64>,   // super-diagonal
    n: usize,          // number of interior points
}
```

### Non-uniform FD stencils

For grid spacing `h_i = x[i] - x[i-1]`, second-order accurate on non-uniform grids:

Second derivative:
```
d2u/dx2 ~ 2/(h_i + h_{i+1}) * [u[i+1]/h_{i+1} - u[i]*(1/h_i + 1/h_{i+1}) + u[i-1]/h_i]
```

First derivative: central difference weighted for non-uniform spacing.

### Assembly and solve

```rust
impl TridiagOperator {
    /// Assemble from PDE coefficients at time t
    fn assemble(problem: &dyn PdeProblem1D, grid: &Grid1D, t: f64) -> Self;

    /// Apply: y = A * u (matrix-vector product)
    fn apply(&self, u: &[f64], out: &mut [f64]);

    /// Solve: (I - theta * dt * A) * u_new = rhs via Thomas algorithm
    fn solve_implicit(&self, theta: f64, dt: f64, rhs: &[f64], out: &mut [f64]);
}
```

Thomas algorithm is O(n) with no allocation beyond the operator. This is the hot inner loop.

### Boundary condition incorporation

Boundary conditions modify the first and last rows of the tridiagonal system:

```rust
pub enum BoundaryCondition {
    /// u = g(t): known value, neighbor terms shift to RHS
    Dirichlet(f64),
    /// du/dx = g(t): ghost node elimination, modifies stencil
    Neumann(f64),
    /// d2u/dx2 = 0: linear extrapolation from interior (standard far-field)
    Linear,
}
```

- **Dirichlet**: row becomes `[0, 1, 0]`, RHS = `g(t)`. Known value terms from neighbors shift to RHS.
- **Neumann**: ghost node elimination modifies the stencil at the boundary.
- **Linear**: extrapolation from interior. Standard for far-field option pricing where gamma vanishes.

## Time Stepping: Theta Scheme

### Theta parameter

- `theta = 0.0`: fully explicit (forward Euler) — conditionally stable, debugging only
- `theta = 0.5`: Crank-Nicolson — second-order in time, the workhorse
- `theta = 1.0`: fully implicit (backward Euler) — first-order, unconditionally stable

### TimeStepper trait

Both `ThetaStepper` and `RannacherStepper` implement a common trait:

```rust
pub trait TimeStepper {
    /// Advance the solution one step backward in time.
    /// The stepper calls TridiagOperator::assemble and ::solve_implicit internally.
    fn step(
        &self,
        problem: &dyn PdeProblem1D,
        grid: &Grid1D,
        u: &mut [f64],
        t_from: f64,
        t_to: f64,
        step_index: usize,
    );

    /// Total number of time steps
    fn n_steps(&self) -> usize;

    /// Time levels (may be non-uniform)
    fn time_levels(&self, maturity: f64) -> Vec<f64>;
}
```

The stepper orchestrates the theta scheme and calls into the operator for the tridiagonal solve. The operator itself is a low-level building block — it assembles coefficients and solves `(I - alpha * A) * x = rhs` without knowing about theta or time-stepping logic.

### The time step

At each step from `t_{n+1}` to `t_n` (backward from maturity):

```
(I - theta * dt * A_n) * u_n = (I + (1-theta) * dt * A_{n+1}) * u_{n+1}
                              + dt * [theta * f_n + (1-theta) * f_{n+1}]
```

```rust
pub struct ThetaStepper {
    theta: f64,
    n_steps: usize,
}

impl ThetaStepper {
    pub fn crank_nicolson(n_steps: usize) -> Self;
    pub fn implicit(n_steps: usize) -> Self;
    pub fn explicit(n_steps: usize) -> Self;
    pub fn custom(theta: f64, n_steps: usize) -> Self;
}
```

### Rannacher smoothing

Crank-Nicolson oscillates near payoff discontinuities (digitals, barrier knock-outs). Fix: run 2-4 fully implicit steps at the start (near terminal condition), then switch to CN.

```rust
pub struct RannacherStepper {
    implicit_steps: usize,  // typically 2-4
    theta: f64,             // for remaining steps, usually 0.5
}
```

### Non-uniform time steps

The stepper accepts a `Vec<f64>` of time levels. Needed for:
- Barrier options: concentrate near observation dates
- Bermudan/American: align with exercise dates
- Discrete dividends: align with dividend dates

### Operator caching

For time-homogeneous problems (`is_time_homogeneous() == true`), the tridiagonal operator is assembled once and reused. For local vol / time-varying rates, it's reassembled at each step.

## American Exercise: Penalty Method

### Mechanism

After solving the linear system at each time step, enforce `u >= payoff` via a penalty term added to the main diagonal:

```
(I - theta * dt * A - dt * P) * u_n = RHS
```

Where `P` is diagonal: `P_ii = lambda` if `u_i < payoff(x_i)`, else `0`. Lambda is large (`~1e8/dt`), making constraint violation extremely expensive.

```rust
pub struct PenaltyExercise {
    penalty_factor: f64,               // lambda scaling (default 1e8)
    payoff_values: Vec<f64>,           // intrinsic value at each grid node
    exercise_type: ExerciseType,
}

pub enum ExerciseType {
    /// Exercisable at every time step
    American,
    /// Exercisable only at specified times
    Bermudan { exercise_times: Vec<f64> },
}
```

### Integration with stepper

At exercise-eligible time steps:
1. Solve the standard theta step to get preliminary `u`
2. Identify nodes where `u_i < payoff(x_i)`
3. Add lambda to main diagonal at those nodes, adjust RHS
4. Re-solve (one iteration usually suffices; optionally 2-3 for convergence)

For Bermudan: penalty active only at steps aligned with exercise dates. Non-uniform time grid ensures exact alignment.

### Exercise boundary output

The solver reports the early exercise boundary `S*(t)` as a byproduct — the spot level where continuation value equals intrinsic value at each time step.

## Top-Level Solver

### Solver1D

```rust
pub struct Solver1D {
    grid: Grid1D,
    stepper: Box<dyn TimeStepper>,
    exercise: Option<PenaltyExercise>,
}

impl Solver1D {
    pub fn solve(&self, problem: &dyn PdeProblem1D) -> PdeSolution;
}
```

Builder pattern:
```rust
Solver1D::builder()
    .grid(Grid1D::sinh_concentrated(-5.0, 5.0, 200, 0.0, 0.1))
    .crank_nicolson(100)
    .rannacher(4)
    .american(payoff_values)
    .build()
```

### PdeSolution

```rust
pub struct PdeSolution {
    pub grid: Grid1D,
    pub values: Vec<f64>,                         // solution at t=0
    pub exercise_boundary: Option<Vec<(f64, f64)>>, // (t, S*) pairs
    pub theta_used: f64,
    pub n_time_steps: usize,
}

impl PdeSolution {
    /// Interpolate solution at a specific point (e.g., current spot)
    pub fn interpolate(&self, x: f64) -> f64;

    /// Delta via finite difference on the solution grid
    pub fn delta(&self, x: f64) -> f64;

    /// Gamma via finite difference on the solution grid
    pub fn gamma(&self, x: f64) -> f64;
}
```

Greeks come free from the PDE grid: delta and gamma are finite differences on the already-computed solution vector.

## Integration with Pricer Registry

### New ModelKey variants

```rust
pub enum ModelKey {
    // ... existing variants ...
    PdeCrankNicolson1D = 40,  // 1D finite difference
    PdeAdi2D = 41,            // 2D ADI (Phase 2)
}
```

Feature-gated behind a `pde` feature flag (mirroring the `mc` pattern).

### Registered pricers

Phase 1:
- `(EquityOption, PdeCrankNicolson1D)` — European/American equity options
- `(BarrierOption, PdeCrankNicolson1D)` — knock-in/knock-out barriers
- `(FxOption, PdeCrankNicolson1D)` — FX vanillas
- `(FxBarrierOption, PdeCrankNicolson1D)` — FX barriers

Phase 2:
- `(EquityOption, PdeAdi2D)` — Heston stochastic vol
- `(Convertible, PdeAdi2D)` — convertible bonds

Each pricer lives in `finstack/valuations`, translating instrument parameters into a `PdeProblem1D` and calling the solver. Same pattern as MC pricers translating into `StochasticProcess` + `Payoff`.

### Feynman-Kac bridge helpers

Utility structs in `finstack/pde` for common setups:

```rust
/// Black-Scholes PDE in log-spot coordinates
pub struct BlackScholesPde {
    pub sigma: f64,  // or Fn(f64, f64) -> f64 for local vol
    pub rate: f64,
    pub dividend: f64,
    pub payoff: Box<dyn Fn(f64) -> f64>,
}

impl PdeProblem1D for BlackScholesPde { ... }

/// Local vol PDE using Dupire surface
pub struct LocalVolPde { ... }

impl PdeProblem1D for LocalVolPde { ... }
```

## Phase 2: 2D ADI for Heston/LSV

### PdeProblem2D trait

```rust
pub trait PdeProblem2D {
    fn diffusion_xx(&self, x: f64, v: f64, t: f64) -> f64;   // 0.5*v
    fn diffusion_vv(&self, x: f64, v: f64, t: f64) -> f64;   // 0.5*xi^2*v
    fn cross_diffusion(&self, x: f64, v: f64, t: f64) -> f64; // rho*xi*v
    fn convection_x(&self, x: f64, v: f64, t: f64) -> f64;    // r - q - 0.5*v
    fn convection_v(&self, x: f64, v: f64, t: f64) -> f64;    // kappa*(theta - v)
    fn reaction(&self, x: f64, v: f64, t: f64) -> f64;         // -r
    fn terminal_condition(&self, x: f64, v: f64) -> f64;
    fn boundary_x_lower(&self, v: f64, t: f64) -> BoundaryCondition;
    fn boundary_x_upper(&self, v: f64, t: f64) -> BoundaryCondition;
    fn boundary_v_lower(&self, x: f64, t: f64) -> BoundaryCondition;
    fn boundary_v_upper(&self, x: f64, t: f64) -> BoundaryCondition;
}
```

### ADI schemes

Each sweep is a tridiagonal solve along one axis, reusing `TridiagOperator` and Thomas algorithm from Phase 1.

- **Douglas-Rachford** — simple first-order splitting, good baseline
- **Craig-Sneyd** — handles mixed derivative (correlation) explicitly, second-order, standard production scheme for Heston
- **Hundsdorfer-Verwer** — alternative with better stability for some parameter regimes

The cross-derivative term is always treated explicitly via four-point stencil on the tensor-product grid.

### Heston grid

- Spot axis: `Grid1D::sinh_concentrated` near strike
- Variance axis: `Grid1D::sinh_concentrated` near theta (long-run variance), `v_min = 0`, `v_max ~ 5*theta` to `10*theta`. The `v = 0` boundary requires special treatment (Feller condition).

### Phase 1 reuse

- `TridiagOperator` reused for each directional sweep
- `Grid1D` reused for each axis of the tensor-product grid
- `BoundaryCondition` enum works per-axis
- Thomas algorithm is the inner loop of every ADI sweep
- `PdeSolution` generalizes to 2D by storing a matrix of values

## Phasing Summary

**Phase 1 (1D):**
1. `finstack/pde` crate scaffold, `Cargo.toml`, feature flags
2. `Grid1D` (uniform, sinh-concentrated, from-points)
3. `PdeProblem1D` trait, `BoundaryCondition` enum
4. `TridiagOperator` with non-uniform stencils and Thomas algorithm
5. `ThetaStepper` (explicit, implicit, CN) and `RannacherStepper`
6. `PenaltyExercise` for American/Bermudan
7. `Solver1D` with builder, `PdeSolution` with interpolation and Greeks
8. `BlackScholesPde` and `LocalVolPde` bridge helpers
9. `ModelKey::PdeCrankNicolson1D` and pricer registrations in valuations
10. Tests: convergence vs analytical BS, barrier accuracy, American put vs binomial

**Phase 2 (2D ADI):**
1. `Grid2D` tensor product
2. `PdeProblem2D` trait
3. ADI steppers (Douglas-Rachford, Craig-Sneyd, Hundsdorfer-Verwer)
4. Cross-derivative explicit treatment
5. `Solver2D`, `HestonPde` bridge
6. `ModelKey::PdeAdi2D` and pricer registrations
7. Tests: convergence vs Heston Fourier, convertible bond pricing
