# Exotic Options

This cookbook covers pricing exotic derivatives using Monte Carlo simulation.

## 1. Barrier Option

```python
from finstack.valuations.instruments import BarrierOption
from finstack.monte_carlo import MonteCarloEngine, GBM
from finstack.core.money import Money
from datetime import date

as_of = date(2025, 1, 15)

barrier = BarrierOption.builder("BARRIER-DO-CALL") \
    .notional(Money(100_000, "USD")) \
    .strike(100.0) \
    .barrier(85.0) \
    .barrier_type("down_and_out") \
    .option_type("call") \
    .expiry(date(2026, 1, 15)) \
    .disc_id("USD-OIS") \
    .vol_surface("EQ-VOL") \
    .build()

engine = MonteCarloEngine(
    process=GBM(spot=100.0, vol=0.20, rate=0.05),
    n_paths=500_000,
    n_steps=252,
    maturity=1.0,
    seed=42,
    antithetic=True,
)

result = registry.price_with_metrics(
    barrier, "monte_carlo", market, as_of,
    mc_engine=engine,
    metrics=["delta", "gamma", "vega", "mc_stderr"],
)

print(f"Price:     {result.npv}")
print(f"Delta:     {result.get('delta'):.4f}")
print(f"MC StdErr: {result.get('mc_stderr'):.4f}")
```

## 2. Autocallable Note

```python
from finstack.valuations.instruments import Autocallable

autocall = Autocallable.builder("AUTOCALL-SPX-1Y") \
    .notional(Money(1_000_000, "USD")) \
    .underlying("SPX") \
    .autocall_barrier(1.05) \
    .coupon_barrier(0.95) \
    .coupon_rate(0.08) \
    .ki_barrier(0.65) \
    .observation_dates([
        date(2025, 4, 15), date(2025, 7, 15),
        date(2025, 10, 15), date(2026, 1, 15),
    ]) \
    .maturity(date(2026, 1, 15)) \
    .disc_id("USD-OIS") \
    .vol_surface("SPX-VOL") \
    .build()

result = registry.price_with_metrics(
    autocall, "monte_carlo", market, as_of,
    mc_engine=engine,
)
print(f"Price: {result.npv}")
```

## 3. Asian Option with Control Variates

```python
from finstack.valuations.instruments import AsianOption

asian = AsianOption.builder("ASIAN-AVG-CALL") \
    .notional(Money(100_000, "USD")) \
    .strike(100.0) \
    .option_type("call") \
    .averaging("arithmetic") \
    .expiry(date(2026, 1, 15)) \
    .disc_id("USD-OIS") \
    .vol_surface("EQ-VOL") \
    .build()

result = registry.price_with_metrics(
    asian, "monte_carlo", market, as_of,
    mc_engine=MonteCarloEngine(
        process=GBM(spot=100.0, vol=0.20, rate=0.05),
        n_paths=200_000, n_steps=252, maturity=1.0,
        seed=42, control_variate="geometric",
    ),
)
print(f"Asian Call Price: {result.npv}")
```

## 4. Convergence Study

```python
for n in [10_000, 50_000, 100_000, 500_000]:
    engine = MonteCarloEngine(
        process=GBM(spot=100.0, vol=0.20, rate=0.05),
        n_paths=n, n_steps=252, maturity=1.0, seed=42,
    )
    result = registry.price_with_metrics(
        barrier, "monte_carlo", market, as_of,
        mc_engine=engine, metrics=["mc_stderr"],
    )
    print(f"{n:>10,}: Price={result.npv.amount:.4f}, "
          f"StdErr={result.get('mc_stderr'):.6f}")
```
