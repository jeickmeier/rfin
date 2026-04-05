# Path Generation

Path generation discretizes a continuous-time SDE into discrete time steps
and generates sample paths.

## Random Number Generators

| RNG | Type | Properties |
|-----|------|------------|
| `Philox` | Counter-based | Parallel-safe, reproducible, fast |
| `Sobol` | Quasi-random | Low-discrepancy, better convergence |
| `MersenneTwister` | PRNG | Standard MT19937 |

### Philox (Default)

Philox is a counter-based RNG ideal for parallel simulation:
- Deterministic given (seed, counter)
- No sequential dependency between outputs
- Statistically independent sub-streams

```python
from finstack.monte_carlo import PhiloxRng

rng = PhiloxRng(seed=42)
```

### Sobol Sequences

Quasi-random sequences fill the space more uniformly than pseudo-random,
yielding faster convergence ($O(1/N)$ vs $O(1/\sqrt{N})$):

```python
from finstack.monte_carlo import SobolRng

rng = SobolRng(dimension=252)  # one dimension per time step
```

## Variance Reduction

### Antithetic Variates

For each path with increments $Z_i$, generate a mirror path with $-Z_i$.
This halves the number of random draws needed and reduces variance for
monotonic payoffs.

```python
engine = MonteCarloEngine(
    process=GBM(spot=100.0, vol=0.20, rate=0.05),
    n_paths=50_000,        # generates 100K paths (50K + 50K antithetic)
    antithetic=True,
)
```

### Control Variates

Use a correlated instrument with a known analytical price to reduce variance:

```python
engine = MonteCarloEngine(
    process=gbm,
    n_paths=100_000,
    control_variate="geometric",  # geometric Asian has closed form
)
```

## Multi-Asset Correlation

For basket/multi-asset products, correlated paths use Cholesky decomposition:

$$\mathbf{Z}_{\text{correlated}} = L \cdot \mathbf{Z}_{\text{independent}}$$

where $L$ is the lower-triangular Cholesky factor of the correlation matrix.

```python
from finstack.monte_carlo import MultiAssetGBM
import numpy as np

corr = np.array([
    [1.0, 0.6, 0.3],
    [0.6, 1.0, 0.5],
    [0.3, 0.5, 1.0],
])

process = MultiAssetGBM(
    spots=[100.0, 200.0, 50.0],
    vols=[0.20, 0.25, 0.30],
    rate=0.05,
    correlation=corr,
)
```
