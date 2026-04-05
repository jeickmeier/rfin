# Portfolio Valuation

This cookbook builds a multi-asset portfolio and runs aggregated valuation
with risk metrics.

## 1. Build Portfolio

```python
from finstack.portfolio import PortfolioBuilder, Position, Entity
from finstack.valuations.instruments import Bond, InterestRateSwap, CreditDefaultSwap
from finstack.core.money import Money
from datetime import date

as_of = date(2025, 1, 15)

# Create instruments
bond = Bond.builder("ACME-5Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.045) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .build()

swap = InterestRateSwap.builder("IRS_5Y") \
    .money(Money(25_000_000, "USD")) \
    .side("receive_fixed") \
    .fixed_rate(0.0400) \
    .start(date(2025, 1, 17)) \
    .maturity(date(2030, 1, 17)) \
    .disc_id("USD-OIS") \
    .fwd_id("USD-SOFR-3M") \
    .build()

cds = CreditDefaultSwap.buy_protection(
    "CDS-ACME-5Y",
    Money(10_000_000, "USD"),
    spread_bp=200.0,
    start_date=date(2025, 3, 20),
    maturity=date(2030, 3, 20),
    discount_curve="USD-OIS",
    credit_curve="ACME-HZD",
)

portfolio = PortfolioBuilder() \
    .add_entity(Entity("FUND_A")) \
    .add_position(Position("FUND_A", bond, "long")) \
    .add_position(Position("FUND_A", swap, "receive_fixed")) \
    .add_position(Position("FUND_A", cds, "buy_protection")) \
    .build()
```

## 2. Run Valuation

```python
from finstack.portfolio import value_portfolio

results = value_portfolio(
    portfolio, market, as_of,
    metrics=["dv01", "cs01", "bucketed_dv01"],
)

print(f"Total NPV: {results.total_npv}")
```

## 3. Per-Position Results

```python
for pos_id, result in results.position_results.items():
    print(f"{pos_id}:")
    print(f"  NPV:  {result.npv}")
    print(f"  DV01: {result.get('dv01'):.2f}")
    print(f"  CS01: {result.get('cs01'):.2f}")
```

## 4. Aggregated Risk

```python
print(f"Portfolio DV01: {results.aggregate('dv01'):.2f}")
print(f"Portfolio CS01: {results.aggregate('cs01'):.2f}")

# Bucketed DV01 across all positions
for key, value in sorted(results.aggregate_bucketed('bucketed_dv01').items()):
    print(f"  {key}: {value:.2f}")
```

## 5. Multi-Currency Portfolio

```python
results = value_portfolio(
    portfolio, market, as_of,
    reporting_currency="USD",
)
print(f"Total NPV (USD): {results.total_npv}")
```
