# P&L Attribution

This cookbook decomposes daily P&L into carry, roll-down, rate moves, spread
moves, and residual.

## 1. Setup

```python
from finstack.valuations.attribution import attribute_pnl
from datetime import date

t0 = date(2025, 1, 14)
t1 = date(2025, 1, 15)
```

## 2. Run Attribution

```python
result = attribute_pnl(
    portfolio,
    market_t0, market_t1,  # yesterday's and today's market data
    t0, t1,
)

print(f"Carry:        {result.carry}")
print(f"Roll-down:    {result.rolldown}")
print(f"Rate Δ:       {result.rate_delta}")
print(f"Spread Δ:     {result.spread_delta}")
print(f"Vol Δ:        {result.vol_delta}")
print(f"FX:           {result.fx_delta}")
print(f"Residual:     {result.residual}")
print(f"Total:        {result.total}")
```

## 3. Per-Position Attribution

```python
for pos_id, pos_result in result.position_results.items():
    print(f"\n{pos_id}:")
    print(f"  Carry:      {pos_result.carry}")
    print(f"  Rate Δ:     {pos_result.rate_delta}")
    print(f"  Spread Δ:   {pos_result.spread_delta}")
    print(f"  Total:      {pos_result.total}")
```

## 4. Multi-Day Attribution

```python
from datetime import timedelta

cumulative = {}
for i in range(5):  # 5 business days
    t0 = date(2025, 1, 13) + timedelta(days=i)
    t1 = t0 + timedelta(days=1)

    result = attribute_pnl(
        portfolio,
        markets[t0], markets[t1],
        t0, t1,
    )

    for component in ["carry", "rate_delta", "spread_delta", "total"]:
        cumulative[component] = cumulative.get(component, 0) + getattr(result, component).amount

print("\nWeekly Attribution:")
for component, value in cumulative.items():
    print(f"  {component}: {value:,.2f}")
```

## 5. Validate Against Mark-to-Market

```python
mtm_t0 = value_portfolio(portfolio, market_t0, t0).total_npv
mtm_t1 = value_portfolio(portfolio, market_t1, t1).total_npv
mtm_pnl = mtm_t1 - mtm_t0

print(f"MTM P&L:         {mtm_pnl}")
print(f"Attribution Total: {result.total}")
print(f"Residual:        {result.residual}")
# Residual should be small (unexplained)
```
