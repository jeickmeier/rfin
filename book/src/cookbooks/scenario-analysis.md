# Scenario Analysis

This cookbook covers defining market shocks, running stress tests, and
comparing results across scenarios.

## 1. Simple Parallel Rate Shock

```python
from finstack.scenarios import ScenarioSpec, OperationSpec
from finstack.valuations.pricer import standard_registry

registry = standard_registry()

# Rates +100bp scenario
rates_up = ScenarioSpec(
    name="Rates +100bp",
    operations=[
        OperationSpec.parallel_shift("USD-OIS", 0.01),
        OperationSpec.parallel_shift("USD-SOFR-3M", 0.01),
    ],
)

# Apply and reprice
shocked_market = rates_up.apply(base_market)
base_pv = registry.value(bond, "discounting", base_market, as_of)
shock_pv = registry.value(bond, "discounting", shocked_market, as_of)
print(f"P&L impact: {(shock_pv - base_pv)}")
```

## 2. Curve Twist (Steepener)

```python
steepener = ScenarioSpec(
    name="2s10s Steepener +50bp",
    operations=[
        OperationSpec.bucketed_shift("USD-OIS", [
            ("2y", -0.0025),   # 2Y down 25bp
            ("10y", 0.0025),   # 10Y up 25bp
        ]),
    ],
)
```

## 3. Credit Widening

```python
credit_stress = ScenarioSpec(
    name="Credit Widening +50bp",
    operations=[
        OperationSpec.parallel_shift("ACME-HZD", 0.005),
        OperationSpec.parallel_shift("CDX-IG-HZD", 0.005),
    ],
)
```

## 4. Multi-Factor Stress

Combine rate, credit, and equity shocks:

```python
crisis = ScenarioSpec(
    name="Crisis Scenario",
    operations=[
        OperationSpec.parallel_shift("USD-OIS", -0.01),     # rates down
        OperationSpec.parallel_shift("ACME-HZD", 0.01),     # credit wider
        OperationSpec.vol_parallel_shift("EQ-VOL", 0.10),   # vol up
        OperationSpec.equity_shock("SPX", -0.20),            # equity down
    ],
)
```

## 5. Batch Scenario Execution

```python
from finstack.scenarios import run_scenarios

scenarios = [
    ScenarioSpec("Rates +100", [OperationSpec.parallel_shift("USD-OIS", 0.01)]),
    ScenarioSpec("Rates -100", [OperationSpec.parallel_shift("USD-OIS", -0.01)]),
    ScenarioSpec("Credit +50", [OperationSpec.parallel_shift("ACME-HZD", 0.005)]),
    ScenarioSpec("Vol +5",     [OperationSpec.vol_parallel_shift("EQ-VOL", 0.05)]),
]

results = run_scenarios(portfolio, base_market, as_of, scenarios)

print(f"{'Scenario':<20} {'P&L':>15}")
print("-" * 35)
for name, pnl in results.items():
    print(f"{name:<20} {pnl:>15}")
```

## 6. Historical Scenario Replay

```python
# Replay the March 2020 rate shock
march_2020 = ScenarioSpec(
    name="March 2020 Replay",
    operations=[
        OperationSpec.parallel_shift("USD-OIS", -0.015),
        OperationSpec.parallel_shift("ACME-HZD", 0.02),
        OperationSpec.vol_parallel_shift("EQ-VOL", 0.30),
        OperationSpec.equity_shock("SPX", -0.34),
    ],
)
```
