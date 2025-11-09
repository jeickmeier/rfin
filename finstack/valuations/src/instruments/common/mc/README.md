# Monte Carlo (mc) — Simulation Primitives

Primitives for Monte Carlo simulation: stochastic processes, discretization schemes, time grids, RNG (PRNG/QMC), basic online statistics, and generic estimates. This module is pricing‑agnostic and reusable across use cases. Pricing components (payoffs, pricers, Greeks, variance reduction, capture engine) live under `instruments/common/models/monte_carlo`.

## What this module provides

- Deterministic, reproducible RNGs (Philox PRNG; Sobol QMC with Owen scrambling)
- Stochastic processes (GBM, Heston, Hull‑White/Vasicek, CIR/CIR++, Bates, Jump‑diffusion, Brownian additive (1D/multi‑D), Multi‑OU)
- Discretization schemes (Exact, Euler/Log‑Euler, Milstein/Log‑Milstein, QE‑Heston, QE‑CIR, Jump‑Euler, Exact HW1F, Exact Schwartz‑Smith)
- Time grids on year‑fractions (`TimeGrid`)
- Online statistics and confidence intervals (`OnlineStats`)
- Generic estimation envelopes (`Estimate`, `ConvergenceDiagnostics`)
- Path data structs for capture/visualization (`PathPoint`, `SimulatedPath`, `PathDataset`, `ProcessParams`)

Not included here: payoffs, money, pricers, Greeks, and pricing engines — these are in `models/monte_carlo`.

## Quick start (driving primitives directly)

```rust
use finstack_valuations::instruments::common::mc::prelude::*;

// 1) Build a time grid (1y, 252 steps)
let grid = TimeGrid::uniform(1.0, 252)?;

// 2) Define a process and a discretization (1D GBM, exact stepping)
let gbm = GbmProcess::with_params(0.05, 0.02, 0.20);
let disc = ExactGbm::new();

// 3) RNG for shocks (deterministic Philox stream per path)
let mut rng = PhiloxRng::new(42);

// 4) Evolve a single path state manually
let mut x = vec![100.0]; // state vector [spot]
let mut z = vec![0.0; gbm.num_factors()];
let mut work = vec![0.0; disc.work_size(&gbm)];

for step in 0..grid.num_steps() {
    // draw standard normals and take one step
    rng.fill_std_normals(&mut z);
    disc.step(&gbm, grid.time(step), grid.dt(step), &mut x, &z, &mut work);
}

// 5) Accumulate estimate with OnlineStats as needed
let mut stats = OnlineStats::new();
stats.update(x[0]);
let est = Estimate::new(stats.mean(), stats.stderr(), stats.ci_95(), stats.count());
println!("estimate: {}", est);
# finstack_core::Result::Ok(())
```

To price instruments (payoffs, currencies, capture results), use `instruments::common::models::monte_carlo::prelude`.

## Architecture and dependencies

- Uses math utilities from `finstack_core::math` (special functions, linalg, stats)
- RNG and correlation helpers compose with processes and discretizations
- All time is on year‑fractions; convert dates in the instrument layer via `finstack_core::dates`
- Types here are numeric and pricing‑agnostic; money and currency appear only in the pricing layer

## Module structure

```
mc/
├── traits.rs             // RandomStream, StochasticProcess, Discretization, PathState
├── time_grid.rs          // Year‑fraction grids
├── online_stats.rs       // OnlineStats, required_samples
├── estimate.rs           // Estimate, ConvergenceDiagnostics
├── paths.rs              // PathPoint, SimulatedPath, PathDataset, ProcessParams
├── rng/
│   ├── philox.rs         // Philox PRNG
│   ├── sobol.rs          // Sobol QMC (Owen scrambling)
│   ├── poisson.rs        // Poisson sampling (feature‑gated)
│   ├── brownian_bridge.rs
│   ├── sobol_pca.rs
│   └── transforms.rs
├── process/
│   ├── brownian.rs, gbm.rs, gbm_dividends.rs, heston.rs, cir.rs, ou.rs, multi_ou.rs,
│   ├── schwartz_smith.rs, jump_diffusion.rs, bates.rs
│   ├── correlation.rs    // re‑exports from core::math::linalg
│   └── metadata.rs       // ProcessMetadata → ProcessParams
└── discretization/
    ├── exact.rs, exact_gbm_dividends.rs, exact_hw1f.rs, schwartz_smith.rs
    ├── euler.rs, milstein.rs
    ├── qe_heston.rs, qe_cir.rs
    └── jump_euler.rs
```

## Design principles

- Determinism: counter‑based RNGs; reproducibility across threads and runs
- Separation of concerns: simulation primitives here; pricing lives in `models/monte_carlo`
- Numerical stability: Welford statistics; Cholesky for correlation; QE schemes where appropriate
- Clear statistical naming: GBM, Heston, CIR, Euler, Milstein, QE, etc.

## Performance notes

- MC error decays as O(N^{-1/2}); QMC often improves effective rates for smooth integrands
- Prefer vectorized generation and pre‑allocated work buffers for high throughput
- Parallel performance depends on path count and chunk sizing upstream

## Compatibility

Module aliases for internal use:
- `mc::results` → `mc::estimate`
- `mc::stats` → `mc::online_stats`
- `mc::path_data` → `mc::paths`

These aliases are maintained for compatibility with `models/monte_carlo` and should be accessed through the `prelude` module in external code.

## Feature flags

- `mc` enables Monte Carlo features in this crate. Build with: `--features mc`

## References

1) Salmon et al. (2011): Philox counter‑based RNG
2) Owen (1998): Scrambling for Sobol sequences
3) Andersen (2008): QE scheme for Heston
4) Kloeden & Platen (1992): SDE discretization methods