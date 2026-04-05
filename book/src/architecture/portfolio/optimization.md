# Optimization

Portfolio optimization finds the optimal allocation across instruments
subject to constraints.

## Risk Budgeting

Allocate risk budget across positions:

```python
from finstack.portfolio import optimize_risk_budget

result = optimize_risk_budget(
    portfolio,
    market,
    as_of,
    target_metric="dv01",
    budget=50_000.0,         # total DV01 budget in USD
    constraints={
        "max_position_pct": 0.20,  # max 20% of budget per position
        "min_position_pct": 0.01,  # min 1% of budget per position
    },
)

for pos_id, weight in result.weights.items():
    print(f"{pos_id}: {weight:.2%}")
```

## Hedge Optimization

Find the minimum-cost hedge for a target risk reduction:

```python
from finstack.portfolio import optimize_hedge

hedge = optimize_hedge(
    portfolio,
    market,
    as_of,
    target_metric="bucketed_dv01",
    hedge_instruments=[irs_2y, irs_5y, irs_10y, irs_30y],
    objective="minimize_notional",
)

for instrument, notional in hedge.trades.items():
    print(f"{instrument.id()}: {notional}")
```

## Constraint Types

| Constraint | Description |
|------------|-------------|
| Notional limits | Min/max notional per position |
| Risk limits | Max DV01, CS01, or Greeks per bucket |
| Concentration | Max exposure to single issuer/sector |
| Turnover | Max turnover from current portfolio |
| Currency | Limits by currency exposure |
