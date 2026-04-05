# Risk

The risk framework provides bump-based sensitivity computation, metric
aggregation, P&L attribution, and scenario analysis.

## Approach

Finstack computes risk via **bump-and-reprice**: perturb a market data input
(rate, spread, vol) and re-price the instrument. The difference yields the
sensitivity:

$$\text{DV01} = \frac{V(r + \Delta r) - V(r - \Delta r)}{2 \cdot \Delta r}$$

This works identically for all instrument types — no model-specific
differentiation is required. Bumps are applied to curves, surfaces, and
scalars through the scenario engine.

## Metric Keys

Metric keys are fully qualified strings using `::` separators:

```text
bucketed_dv01::USD-OIS::10y     # DV01 to 10Y knot of USD-OIS
cs01::ACME-HZD                  # CS01 to ACME hazard curve
pv01::usd_ois                   # PV01 to entire OIS curve
vega::AAPL::6m                  # Vega to 6M equity vol
delta                           # Equity delta (scalar)
theta                           # Time decay
```

Format: `metric_name::curve_or_surface_id[::tenor_or_bucket]`

See the [Metric Keys reference](../../reference/metric-keys.md) for a full catalog.

## ValuationResult

All metrics are returned in a `ValuationResult` container:

```python
result = registry.price_with_metrics(
    instrument, "discounting", market, as_of,
    metrics=["dv01", "cs01", "ytm", "duration_mod"],
)

npv = result.npv                          # Money
dv01 = result.get("dv01")                 # float
bucketed = result.get("bucketed_dv01")    # dict[str, float]
all_metrics = result.metrics              # dict[str, Any]
```

## Detail Pages

- [Metrics](metrics.md) — DV01, CS01, vega, Greeks, and custom metrics
- [Attribution](attribution.md) — Daily and period P&L decomposition
- [Scenarios](scenarios.md) — Scenario engine and stress testing
