# Monte Carlo

The `finstack-monte-carlo` crate provides a simulation engine for pricing
path-dependent derivatives and running risk simulations.

## Architecture

```text
Process (SDE model)
  │
  ├─ RNG (Philox / Sobol / MersenneTwister)
  │
  ├─ Path Generator (discretization + variance reduction)
  │
  ├─ Payoff Evaluator (per-path payoff computation)
  │
  └─ Statistics Accumulator (Welford online mean/variance)
```

## Supported Processes

| Process | SDE | Use Case |
|---------|-----|----------|
| GBM | $dS = \mu S\,dt + \sigma S\,dW$ | Equity, FX |
| Heston | $dS = \mu S\,dt + \sqrt{v}S\,dW_1$, $dv = \kappa(\theta-v)dt + \xi\sqrt{v}dW_2$ | Equity with stochastic vol |
| Local Vol | $dS = \mu S\,dt + \sigma(S,t)S\,dW$ | Calibrated to vol surface |
| SABR | $dF = \alpha F^\beta\,dW_1$, $d\alpha = \nu\alpha\,dW_2$ | Rates, FX |
| Hull-White | $dr = (\theta(t) - ar)dt + \sigma\,dW$ | Short rate model |
| CIR | $dr = \kappa(\theta - r)dt + \sigma\sqrt{r}\,dW$ | Short rate (positive) |
| Vasicek | $dr = \kappa(\theta - r)dt + \sigma\,dW$ | Short rate (Gaussian) |
| Multi-Asset GBM | Correlated GBM with Cholesky | Baskets, worst-of |
| Jump Diffusion | GBM + Poisson jumps | Equity with jumps |
| Variance Gamma | Subordinated Brownian motion | Heavy tails |
| Normal Inverse Gaussian | NIG process | Flexible tail behavior |

## Quick Example

```python
from finstack.monte_carlo import MonteCarloEngine, GBM

engine = MonteCarloEngine(
    process=GBM(spot=100.0, vol=0.20, rate=0.05),
    n_paths=100_000,
    n_steps=252,
    maturity=1.0,
    seed=42,
)

paths = engine.simulate()  # shape: (100_000, 252)
print(f"Mean final: {paths[:, -1].mean():.2f}")
```

## Detail Pages

- [Path Generation](path-generation.md) — SDE models, antithetic variates, Sobol
- [Pricing](pricing.md) — MC pricing, convergence monitoring, path capture
