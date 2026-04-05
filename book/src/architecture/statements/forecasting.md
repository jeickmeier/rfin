# Forecasting

The statements crate supports multi-period financial forecasting by projecting
statement line items forward in time.

## Forecast Methods

| Method | Description |
|--------|-------------|
| `growth(rate)` | Constant growth rate per period |
| `mean_revert(target, speed)` | Mean-reversion to a target value |
| `seasonal(pattern)` | Seasonal adjustment factors |
| `linked(driver, elasticity)` | Linked to another line item |
| `scenario(values)` | Explicit per-period overrides |
| `macro_linked(factor, beta)` | Linked to macro factor (GDP, CPI) |

## Construction

```python
from finstack.statements import ForecastBuilder, ForecastMethod

forecast = ForecastBuilder(base_statement=stmt) \
    .set_method("revenue", ForecastMethod.growth(0.05)) \
    .set_method("cogs", ForecastMethod.linked("revenue", -0.60)) \
    .set_method("sga", ForecastMethod.growth(0.03)) \
    .set_method("interest", ForecastMethod.scenario(
        [-5_000_000, -4_800_000, -4_600_000, -4_400_000]
    )) \
    .periods(4) \
    .build()

results = forecast.evaluate()
for period, result in enumerate(results):
    print(f"Year {period + 1}: Revenue={result['revenue']:,.0f}, "
          f"EBITDA={result['ebitda']:,.0f}")
```

## Scenario Analysis

Run multiple forecast scenarios in parallel:

```python
from finstack.statements import ScenarioSet

scenarios = ScenarioSet()
scenarios.add("base", {"revenue": ForecastMethod.growth(0.05)})
scenarios.add("bull", {"revenue": ForecastMethod.growth(0.10)})
scenarios.add("bear", {"revenue": ForecastMethod.growth(-0.05)})

results = scenarios.evaluate(base_statement=stmt, periods=4)
for name, forecast in results.items():
    print(f"{name}: Year 4 EBITDA = {forecast[-1]['ebitda']:,.0f}")
```

## Integration with Covenants

Forecasts can be tested against covenants to project compliance:

```python
for period, result in enumerate(forecast_results):
    cov_results = result.test_covenants([leverage_cov])
    for cr in cov_results:
        status = "OK" if cr.passed else "BREACH"
        print(f"Year {period+1} {cr.name}: {cr.ratio:.2f}x [{status}]")
```
