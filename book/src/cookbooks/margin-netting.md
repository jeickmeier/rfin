# Margin & Netting

This cookbook covers ISDA SIMM margin calculations and netting set management.

## 1. ISDA SIMM Calculation

The `finstack-margin` crate implements ISDA SIMM v2.5 and v2.6:

```python
from finstack.margin import SimmCalculator, SimmVersion
from finstack.portfolio import value_portfolio

calculator = SimmCalculator(version=SimmVersion.V2_6)

# Compute SIMM from portfolio sensitivities
results = value_portfolio(
    portfolio, market, as_of,
    metrics=["dv01", "bucketed_dv01", "cs01", "vega"],
)

simm = calculator.calculate(
    sensitivities=results,
    netting_set="NS_BANK_A",
)

print(f"SIMM IM: {simm.total_margin}")
print(f"  Interest Rate: {simm.rate_margin}")
print(f"  Credit:        {simm.credit_margin}")
print(f"  Equity:        {simm.equity_margin}")
print(f"  FX:            {simm.fx_margin}")
print(f"  Commodity:     {simm.commodity_margin}")
```

## 2. SIMM Risk Classes

| Risk Class | Sensitivities Used |
|-----------|-------------------|
| Interest Rate | DV01, bucketed DV01, inflation delta |
| Credit (Qualifying) | CS01 for IG names |
| Credit (Non-Qualifying) | CS01 for HY/distressed |
| Equity | Equity delta, vega |
| FX | FX delta |
| Commodity | Commodity delta |

## 3. Netting Set Management

```python
from finstack.portfolio import PortfolioBuilder, Position

portfolio = PortfolioBuilder() \
    .add_position(pos1, netting_set="NS_BANK_A") \
    .add_position(pos2, netting_set="NS_BANK_A") \
    .add_position(pos3, netting_set="NS_BANK_B") \
    .build()

# Calculate SIMM per netting set
for ns in portfolio.netting_sets():
    ns_portfolio = portfolio.filter(netting_set=ns)
    simm = calculator.calculate(
        sensitivities=value_portfolio(ns_portfolio, market, as_of),
        netting_set=ns,
    )
    print(f"{ns}: {simm.total_margin}")
```

## 4. Margin Decomposition

```python
# Decompose margin by risk factor
decomp = simm.decomposition()

for bucket, contribution in decomp.items():
    print(f"  {bucket}: {contribution:,.0f}")
```

## 5. What-If Analysis

Test the margin impact of a new trade:

```python
# Current margin
current_simm = calculator.calculate(sensitivities=current_results)

# Add a new trade
new_portfolio = portfolio.add_position(new_trade)
new_results = value_portfolio(new_portfolio, market, as_of)
new_simm = calculator.calculate(sensitivities=new_results)

print(f"Current SIMM:  {current_simm.total_margin}")
print(f"New SIMM:      {new_simm.total_margin}")
print(f"Margin Impact: {new_simm.total_margin - current_simm.total_margin}")
```
