# P&L Attribution

P&L attribution decomposes the change in portfolio value between two dates
into risk factor contributions.

## Decomposition

The total P&L is attributed to:

$$\Delta V = \underbrace{\text{Carry}}_{\text{time}} + \underbrace{\text{Roll-down}}_{\text{curve shape}} + \underbrace{\text{Rate \Delta}}_{\text{DV01}} + \underbrace{\text{Spread \Delta}}_{\text{CS01}} + \underbrace{\text{Vol \Delta}}_{\text{Vega}} + \underbrace{\text{Residual}}_{\text{unexplained}}$$

### Components

| Component | Description |
|-----------|-------------|
| **Carry** | Time value: accrued interest + funding cost |
| **Roll-down** | P&L from the curve term structure ("riding the curve") |
| **Rate delta** | P&L from parallel + non-parallel rate moves |
| **Spread delta** | P&L from credit spread changes |
| **Vega** | P&L from implied volatility changes |
| **FX** | P&L from currency moves |
| **Residual** | Higher-order / unexplained |

## Methodology

Attribution uses a sequential re-pricing approach:

1. **T₀ valuation**: Price with yesterday's market + yesterday's positions
2. **New trade effect**: Add/remove trades (trade P&L)
3. **Carry step**: Advance time by 1 day, keep old curves
4. **Roll-down step**: Roll curves forward by 1 day using old rates
5. **Rate move**: Apply today's rate levels
6. **Spread move**: Apply today's credit spreads
7. **Vol move**: Apply today's vol surface
8. **FX move**: Apply today's FX rates
9. **T₁ valuation**: Final value = sum of all steps

The difference at each step gives the attributed P&L.

## Usage

```python
from finstack.valuations.attribution import attribute_pnl

result = attribute_pnl(
    portfolio,
    market_t0, market_t1,
    as_of_t0, as_of_t1,
)

print(result.carry)       # Carry P&L
print(result.rolldown)    # Roll-down P&L
print(result.rate_delta)  # Rate move P&L
print(result.spread_delta)# Spread move P&L
print(result.total)       # Total P&L (should match mark-to-market)
print(result.residual)    # Unexplained
```

## Aggregation

Attribution results aggregate cleanly across:
- **Instrument level**: Per-position P&L breakdown
- **Book/portfolio level**: Sum across positions
- **Factor level**: Total rate P&L, total spread P&L, etc.
