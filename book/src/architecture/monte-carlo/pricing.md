# Monte Carlo Pricing

Monte Carlo pricing evaluates expected payoffs by averaging over simulated
paths, discounted to present value:

$$V_0 = DF(0, T) \cdot \frac{1}{N} \sum_{i=1}^{N} \text{Payoff}(\text{path}_i)$$

## Pricing Workflow

```python
from finstack.monte_carlo import MonteCarloEngine, GBM
from finstack.valuations.instruments import BarrierOption

# 1. Define the process
process = GBM(spot=100.0, vol=0.20, rate=0.05)

# 2. Configure the engine
engine = MonteCarloEngine(
    process=process,
    n_paths=500_000,
    n_steps=252,
    maturity=1.0,
    seed=42,
    antithetic=True,
)

# 3. Price via the pricer registry
result = registry.price_with_metrics(
    barrier_option, "monte_carlo", market, as_of,
    mc_engine=engine,
    metrics=["delta", "gamma", "vega"],
)

print(f"Price: {result.npv}")
print(f"MC StdErr: {result.get('mc_stderr')}")
```

## Convergence Monitoring

The engine uses Welford's online algorithm for stable mean/variance
accumulation. Convergence can be monitored:

| Metric | Description |
|--------|-------------|
| `mc_stderr` | Standard error of the MC estimate |
| `mc_paths` | Number of paths used |
| `mc_confidence_lo` | 95% confidence interval lower bound |
| `mc_confidence_hi` | 95% confidence interval upper bound |

## Greeks via Finite Difference

MC Greeks are computed by bumping inputs and re-simulating:

- **Delta**: Bump spot ±0.5%, average price change
- **Gamma**: Second-order finite difference on spot
- **Vega**: Bump vol +1%, price change

All bumped simulations use the same random seed for variance reduction.

## Path-Dependent Products

Products that depend on the entire path (not just terminal value):

| Product | Path Dependency |
|---------|----------------|
| Asian option | Average price over monitoring dates |
| Barrier option | Whether barrier was crossed |
| Lookback | Max/min price observed |
| Autocallable | Early redemption on observation dates |
| Cliquet | Forward-starting option resets |

## Performance

- Paths are generated and evaluated in parallel (Rayon)
- Philox RNG enables lock-free parallel generation
- Welford accumulator avoids storing all paths in memory
- Typical throughput: ~1M paths/sec for single-asset GBM
