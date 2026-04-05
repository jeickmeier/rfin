# Portfolio

The `finstack-portfolio` crate manages entities, positions, instruments,
and their aggregated valuation and risk.

## Data Model

```text
Portfolio
├── Entity (legal entity / fund)
│   └── Position
│       ├── instrument: Instrument
│       ├── notional: Money
│       ├── direction: Long | Short
│       └── metadata: HashMap<String, String>
└── MarketContext (shared curves, surfaces, FX)
```

## PortfolioBuilder

```python
from finstack.portfolio import PortfolioBuilder, Position, Entity
from finstack.valuations.instruments import Bond
from finstack.core.money import Money
from datetime import date

portfolio = PortfolioBuilder() \
    .add_entity(Entity("FUND_A")) \
    .add_position(Position(
        entity="FUND_A",
        instrument=bond,
        notional=Money(10_000_000, "USD"),
        direction="long",
    )) \
    .add_position(Position(
        entity="FUND_A",
        instrument=swap,
        notional=Money(25_000_000, "USD"),
        direction="receive_fixed",
    )) \
    .build()
```

## Detail Pages

- [Valuation](valuation.md) — Portfolio-level pricing engine
- [Grouping](grouping.md) — Book structure, netting sets, aggregation
- [Optimization](optimization.md) — Portfolio optimization, constraints
