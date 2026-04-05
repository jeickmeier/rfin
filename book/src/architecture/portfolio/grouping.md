# Grouping

Positions can be grouped by various attributes for aggregated reporting.

## Aggregation Keys

Positions carry metadata that enables flexible grouping:

| Key | Example | Description |
|-----|---------|-------------|
| `entity` | `FUND_A` | Legal entity / fund |
| `book` | `TRADING` | Trading book |
| `strategy` | `RATES_RV` | Investment strategy |
| `currency` | `USD` | Position currency |
| `asset_class` | `credit` | Broad asset class |
| `sector` | `technology` | Industry sector |
| Custom | Any string | User-defined grouping |

## Group-By Operations

```python
results = value_portfolio(portfolio, market, as_of)

# Group by entity
by_entity = results.group_by("entity")
for entity, group in by_entity.items():
    print(f"{entity}: NPV={group.total_npv}")

# Group by asset class
by_class = results.group_by("asset_class")
for cls, group in by_class.items():
    print(f"{cls}: DV01={group.aggregate('dv01')}")
```

## Netting Sets

For counterparty risk and margin calculations, positions are organized into
netting sets — groups of trades that can be legally netted in case of default:

```python
portfolio = PortfolioBuilder() \
    .add_position(pos1, netting_set="NS_BANK_A") \
    .add_position(pos2, netting_set="NS_BANK_A") \
    .add_position(pos3, netting_set="NS_BANK_B") \
    .build()
```

Netting sets are used by the margin crate (SIMM) for calculating initial
margin at the netting set level.
