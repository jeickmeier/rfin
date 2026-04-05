# Scenarios

The `finstack-scenarios` crate provides a declarative scenario engine for
stress testing and what-if analysis. Scenarios are defined as sequences of
market data transformations.

## Architecture

```text
ScenarioSpec → [OperationSpec, ...] → ExecutionContext → MarketContext'
```

1. A **ScenarioSpec** contains a list of operations to apply
2. Each **OperationSpec** describes a single market data transformation
3. The **ExecutionContext** applies operations to a base `MarketContext`
4. The result is a new `MarketContext` with shocked market data

## Operation Types

The scenario engine supports 20+ operation types:

### Curve Operations

| Operation | Description |
|-----------|-------------|
| `ParallelShift` | Shift all knots by a fixed amount |
| `BucketedShift` | Shift specific tenor knots |
| `Twist` | Steepening/flattening (pivot + slope) |
| `Butterfly` | Body vs wings deformation |
| `ReplaceCurve` | Swap entire curve with a new one |
| `KeyRateShift` | Shift a single key rate |

### Vol Operations

| Operation | Description |
|-----------|-------------|
| `VolParallelShift` | Shift entire vol surface |
| `VolBucketShift` | Shift specific expiry/strike |
| `VolSkewShift` | Tilt the vol smile |
| `VolTermShift` | Shift vol term structure |

### Macro Operations

| Operation | Description |
|-----------|-------------|
| `FxShock` | Shock FX rates |
| `EquityShock` | Shock equity spot prices |
| `CommodityShock` | Shock commodity prices |
| `InflationShock` | Shock inflation expectations |
| `RecoveryRateShock` | Shock recovery assumptions |

## Python Example

```python
from finstack.scenarios import ScenarioSpec, OperationSpec

# Define a rates stress test
rates_up_100 = ScenarioSpec(
    name="Rates +100bp",
    operations=[
        OperationSpec.parallel_shift("USD-OIS", 0.01),
        OperationSpec.parallel_shift("USD-SOFR-3M", 0.01),
    ],
)

# Apply scenario
shocked_market = rates_up_100.apply(base_market)

# Re-price under scenario
result_base = registry.value(instrument, "discounting", base_market, as_of)
result_shock = registry.value(instrument, "discounting", shocked_market, as_of)
pnl_impact = result_shock - result_base
```

## Scenario Templates

Pre-built scenario templates for common stress tests:

| Template | Description |
|----------|-------------|
| `rates_parallel(bp)` | Parallel rates shock |
| `rates_steepener(bp)` | 2s10s steepening |
| `rates_flattener(bp)` | 2s10s flattening |
| `credit_widening(bp)` | Parallel credit spread widening |
| `equity_crash(pct)` | Equity spot down + vol up |
| `fx_depreciation(pair, pct)` | FX depreciation scenario |

## Batch Execution

Run multiple scenarios efficiently:

```python
from finstack.scenarios import run_scenarios

results = run_scenarios(
    portfolio,
    market,
    as_of,
    scenarios=[
        rates_up_100,
        rates_down_100,
        credit_widening_50,
        equity_crash_20,
    ],
)

for name, pnl in results.items():
    print(f"{name}: {pnl}")
```
