# Monte Carlo

This cookbook covers Monte Carlo engine setup, path generation, convergence,
and variance reduction.

## 1. Basic GBM Simulation

```python
from finstack.monte_carlo import MonteCarloEngine, GBM

engine = MonteCarloEngine(
    process=GBM(spot=100.0, vol=0.20, rate=0.05),
    n_paths=100_000,
    n_steps=252,
    maturity=1.0,
    seed=42,
)

paths = engine.simulate()
print(f"Shape: {paths.shape}")          # (100000, 252)
print(f"Mean final: {paths[:, -1].mean():.2f}")
print(f"Std final:  {paths[:, -1].std():.2f}")
```

## 2. Heston Stochastic Volatility

```python
from finstack.monte_carlo import Heston

process = Heston(
    spot=100.0,
    v0=0.04,          # initial variance
    kappa=2.0,        # mean reversion speed
    theta=0.04,       # long-run variance
    xi=0.3,           # vol of vol
    rho=-0.7,         # correlation spot-vol
    rate=0.05,
)

engine = MonteCarloEngine(
    process=process,
    n_paths=200_000,
    n_steps=252,
    maturity=1.0,
    seed=42,
)
```

## 3. Variance Reduction

### Antithetic Variates

```python
engine = MonteCarloEngine(
    process=GBM(spot=100.0, vol=0.20, rate=0.05),
    n_paths=50_000,    # generates 100K effective paths
    n_steps=252,
    maturity=1.0,
    seed=42,
    antithetic=True,
)
```

### Sobol Quasi-Random

```python
from finstack.monte_carlo import SobolRng

engine = MonteCarloEngine(
    process=GBM(spot=100.0, vol=0.20, rate=0.05),
    n_paths=100_000,
    n_steps=252,
    maturity=1.0,
    rng=SobolRng(dimension=252),
)
```

## 4. Multi-Asset Correlated Paths

```python
from finstack.monte_carlo import MultiAssetGBM
import numpy as np

corr = np.array([[1.0, 0.6], [0.6, 1.0]])

process = MultiAssetGBM(
    spots=[100.0, 200.0],
    vols=[0.20, 0.25],
    rate=0.05,
    correlation=corr,
)

engine = MonteCarloEngine(
    process=process,
    n_paths=100_000,
    n_steps=252,
    maturity=1.0,
    seed=42,
)
paths = engine.simulate()  # shape: (100000, 252, 2)
```

## 5. Convergence Monitoring

```python
for n in [1_000, 10_000, 100_000, 1_000_000]:
    engine = MonteCarloEngine(
        process=GBM(spot=100.0, vol=0.20, rate=0.05),
        n_paths=n, n_steps=252, maturity=1.0, seed=42,
    )
    stats = engine.simulate_with_stats()
    print(f"{n:>10,}: mean={stats.mean:.4f}, "
          f"stderr={stats.stderr:.6f}")
```
