# Automatic Adjoint Differentiation (AAD) Design Spec

**Date**: 2026-03-29
**Status**: Draft
**Phase**: 1 (MC engine first-order Greeks; designed for xVA extensibility)

## Problem

Only bump-and-revalue finite-difference Greeks exist (`finstack/monte_carlo/src/greeks/finite_diff.rs`), plus pathwise and likelihood-ratio methods. Computing a full set of Greeks on a 50-instrument portfolio via MC requires N bumps x full repricing. AAD reduces this to ~3-4x a single pricing pass. This is table stakes at any modern quant desk.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| AD strategy | Operator-overloading dual numbers | Idiomatic Rust, composable with existing trait system, no macro magic |
| Tape architecture | Arena-allocated flat tape (Approach A) | Cache-friendly, predictable memory, simple to checkpoint per MC path |
| Greek scope | First-order initially, xVA-extensible | Highest value deliverable; second-order via forward-over-reverse later |
| Parallelism | Thread-local tapes, merge adjoints | No lock contention; each thread owns its tape entirely |
| Trait strategy | New generic traits + blanket adapters | Existing f64 code untouched; new AD traits generic over `S: Scalar` |
| Build vs buy | Custom tape from scratch | Full control for MC-specific optimizations (checkpointing, arena alloc) |
| Scope | Core AD in `finstack/core`; MC integration in `finstack/monte_carlo` | Enables future Phase 2 extension to analytical/tree-based models |

## Architecture

### Layer 1: Core AD Types (`finstack/core/src/ad/`)

#### `Scalar` Trait

Abstraction over numeric types. `f64` gets a blanket impl (zero overhead for existing code).

```rust
pub trait Scalar: Copy + Clone + Send + Sync + PartialOrd
    + Add<Output=Self> + Sub<Output=Self> + Mul<Output=Self> + Div<Output=Self>
    + Neg<Output=Self> + From<f64>
{
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn sqrt(self) -> Self;
    fn powf(self, n: Self) -> Self;
    fn abs(self) -> Self;
    fn max(self, other: Self) -> Self;
    fn min(self, other: Self) -> Self;
    fn value(self) -> f64;  // Extract primal for branching/comparisons
}
```

#### `Tape` -- Arena-Allocated Flat Tape

```rust
pub struct Tape {
    ops: Vec<TapeEntry>,
    values: Vec<f64>,
    adjoints: Vec<f64>,
    checkpoint: usize,
}

struct TapeEntry {
    op: Op,
    args: [u32; 2],  // Indices into values vec (u32 = 4B node capacity)
    out: u32,
}

enum Op {
    Add, Sub, Mul, Div, Neg,
    Exp, Ln, Sqrt, Powf, Abs,
    Max, Min,   // Non-smooth: subgradient (1 if left >= right, else 0)
    Const,      // Leaf node
}
```

Memory layout: ~12 bytes per `TapeEntry` (1 byte op + 3 padding + 4+4 args + 4 out, or packed tighter). A typical MC path with 252 steps and ~10 ops per step = ~2,520 entries = ~30 KB per tape. Well within L1 cache.

#### `AFloat` -- Active Float

```rust
#[derive(Copy, Clone)]
pub struct AFloat {
    index: u32,
    tape: *const Tape,  // Non-owning pointer to thread-local tape
}

impl Scalar for AFloat { ... }
impl Add for AFloat { ... }  // Records TapeEntry on each op
// ... all arithmetic ops
```

Raw pointer to thread-local tape avoids `Rc<RefCell<>>` overhead on every arithmetic op. Safety: tape lifetime is scoped by the engine -- tape is always alive for the duration of pricing pass.

#### Reverse Sweep

```rust
impl Tape {
    /// Create a new active variable (input parameter).
    pub fn var(&mut self, value: f64) -> AFloat;

    /// Create a constant (not differentiated).
    pub fn constant(&mut self, value: f64) -> AFloat;

    /// Run reverse-mode AD. Sets adjoint of `output` to 1.0, walks backward.
    pub fn reverse(&mut self, output: AFloat);

    /// Extract adjoint (gradient) for a registered variable.
    pub fn adjoint(&self, var: AFloat) -> f64;

    /// Save tape position for later rewind (per-path checkpointing).
    pub fn checkpoint(&mut self) -> usize;

    /// Rewind tape to checkpoint, clearing ops and adjoints beyond it.
    pub fn rewind(&mut self, cp: usize);

    /// Clear all adjoints without clearing ops (for multiple reverse sweeps).
    pub fn zero_adjoints(&mut self);
}
```

### Layer 2: MC AD Traits (`finstack/monte_carlo/src/ad_traits.rs`)

Parallel trait hierarchy generic over `S: Scalar`:

```rust
pub trait StochasticProcessAD<S: Scalar>: Send + Sync {
    fn dim(&self) -> usize;
    fn num_factors(&self) -> usize { self.dim() }
    fn drift(&self, t: f64, x: &[S], out: &mut [S]);
    fn diffusion(&self, t: f64, x: &[S], out: &mut [S]);
    fn is_diagonal(&self) -> bool { true }

    /// Register process parameters as active tape variables.
    fn register_params(&self, tape: &mut Tape) -> Vec<(Sensitivity, AFloat)>;

    /// Reconstruct self with active parameters from tape registration.
    fn with_active_params(&self, params: &[(Sensitivity, AFloat)]) -> Self
    where Self: Sized;
}

pub trait DiscretizationAD<S: Scalar, P: StochasticProcessAD<S>>: Send + Sync {
    fn step(&self, process: &P, t: f64, dt: f64, x: &mut [S], z: &[S], work: &mut [S]);
    fn work_size(&self, process: &P) -> usize { process.dim() }
}

pub trait PayoffAD<S: Scalar>: Send + Sync + Clone {
    fn on_event(&mut self, spot: S, time: f64, step: usize);
    fn value(&self) -> S;   // Returns S, not Money
    fn reset(&mut self);
}
```

**Key differences from existing traits:**
- Generic over `S: Scalar` instead of hardcoded `f64`
- `PayoffAD::on_event` takes `spot: S` directly (PathState is f64-only)
- `PayoffAD::value` returns `S` not `Money` (currency wrapping after adjoint extraction)

#### Blanket Adapters

Existing f64 implementations auto-implement the AD traits:

```rust
impl<P: StochasticProcess> StochasticProcessAD<f64> for P {
    fn dim(&self) -> usize { StochasticProcess::dim(self) }
    fn drift(&self, t: f64, x: &[f64], out: &mut [f64]) {
        StochasticProcess::drift(self, t, x, out)
    }
    fn diffusion(&self, t: f64, x: &[f64], out: &mut [f64]) {
        StochasticProcess::diffusion(self, t, x, out)
    }
    fn register_params(&self, _tape: &mut Tape) -> Vec<(Sensitivity, AFloat)> { vec![] }
    fn with_active_params(&self, _: &[(Sensitivity, AFloat)]) -> Self
    where Self: Sized { /* clone self unchanged */ }
}
```

### Layer 3: AAD Engine (`finstack/monte_carlo/src/ad_engine.rs`)

#### Sensitivity Specification

```rust
pub enum Sensitivity {
    Delta,           // dV/dS0
    Vega,            // dV/dsigma
    Rho,             // dV/dr
    Theta,           // dV/dT
    DividendDelta,   // dV/dq
    Custom(String),  // Named parameter for extensibility
}
```

#### Per-Path AAD Flow

```
For each path:
  1. tape.checkpoint()
  2. Register inputs: S0, sigma, r, q as AFloat tape variables
  3. Process creates AD-aware copy via with_active_params()
  4. Forward pass: simulate with AFloat arithmetic
     - disc.step(process, t, dt, &mut x_ad, &z_ad, &mut work_ad)
     - payoff.on_event(x_ad[0], t, step)
  5. terminal = payoff.value() * discount_factor_ad
  6. tape.reverse(terminal)
  7. Extract: delta_path = tape.adjoint(S0), vega_path = tape.adjoint(sigma), ...
  8. Accumulate into per-Greek OnlineStats
  9. tape.rewind(checkpoint)
```

#### Parallel Execution

```
price_parallel_aad():
  chunks.par_iter():
    let mut tape = Tape::new();              // Thread-local, reused across paths
    let mut greek_stats: Vec<OnlineStats>;   // Per-Greek accumulator
    for path_id in chunk:
      let cp = tape.checkpoint();
      // ... AAD flow above ...
      for (i, greek) in greeks.iter().enumerate():
        greek_stats[i].update(tape.adjoint(param_vars[i]));
      tape.rewind(cp);
    return greek_stats
  // Merge OnlineStats across threads via .merge()
```

No lock contention. Each thread owns its tape. `OnlineStats::merge()` already exists in the codebase for parallel aggregation.

#### Results

```rust
pub struct AadGreeks {
    pub price: MoneyEstimate,
    pub greeks: Vec<GreekEstimate>,
}

pub struct GreekEstimate {
    pub sensitivity: Sensitivity,
    pub value: f64,
    pub stderr: f64,
    pub ci_95: (f64, f64),
}
```

Each Greek gets its own `OnlineStats` accumulator, providing proper standard errors and confidence intervals on Greeks (not just on price).

#### Public API

```rust
let result = aad_engine.price_with_greeks(
    &rng, &gbm_process, &exact_gbm, &[spot],
    &call_payoff, Currency::USD, df,
    &[Sensitivity::Delta, Sensitivity::Vega, Sensitivity::Rho],
)?;

println!("Price: {}", result.price.mean);
println!("Delta: {:.6} +/- {:.6}", result.greeks[0].value, result.greeks[0].stderr);
```

### Module Layout

```
finstack/core/src/ad/
  mod.rs          -- Public API, re-exports
  scalar.rs       -- Scalar trait, f64 blanket impl
  tape.rs         -- Tape, TapeEntry, Op enum, reverse sweep
  afloat.rs       -- AFloat type, operator overloading

finstack/monte_carlo/src/
  ad_traits.rs    -- StochasticProcessAD, DiscretizationAD, PayoffAD
  ad_engine.rs    -- AadEngine, price_with_greeks orchestrator
  ad_process/
    mod.rs
    gbm.rs        -- GbmProcessAD (AFloat impl)
  ad_disc/
    mod.rs
    exact_gbm.rs  -- ExactGbmAD (AFloat impl)
  ad_payoff/
    mod.rs
    vanilla.rs    -- EuropeanCallAD, EuropeanPutAD
  greeks/
    mod.rs        -- Updated to re-export aad
    aad.rs        -- AAD Greek entry point
    finite_diff.rs
    pathwise.rs
    lrm.rs
```

## xVA Extensibility (Phase 2, not implemented now)

The design supports future portfolio-level AAD:
1. Multiple instruments on the same tape (register all parameters once, price all, single reverse sweep)
2. `Sensitivity::Custom(String)` for arbitrary risk factor naming
3. `Tape::reverse_from(node)` for selective backprop (CVA vs MVA sensitivity)
4. Core AD in `finstack/core` enables analytical model Greeks (Black-Scholes, bond pricing) in Phase 2

## Testing Strategy

1. **Unit tests for core AD**: verify tape records correctly, reverse sweep produces correct adjoints for known functions (e.g. `f(x) = x^2` => `df/dx = 2x`)
2. **Finite-diff validation**: for each Greek, compare AAD result against finite-diff bump-and-revalue. Agreement within MC noise + bump error tolerance.
3. **Analytical benchmarks**: European call delta/vega/rho from AAD vs Black-Scholes closed-form. Must agree within MC standard error.
4. **Performance benchmark**: AAD pricing pass vs single f64 pricing pass. Target: < 4x cost multiplier for full gradient.
5. **Thread-safety**: parallel AAD produces identical results to serial AAD (deterministic RNG splitting).

## Performance Characteristics

- **Cost multiplier**: ~3-4x a single pricing pass for all first-order Greeks simultaneously
- **Memory per thread**: ~30 KB tape for 252-step path (~2,500 ops)
- **Tape rewind**: O(1) truncation per path, no allocation after warmup
- **vs finite-diff**: For N Greeks, AAD is O(1) vs O(N) full repricings. Break-even at N=4, dominant advantage at N >= 10.
