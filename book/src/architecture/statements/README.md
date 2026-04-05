# Statements

The `finstack-statements` crate models financial statements as directed acyclic
graphs (DAGs), with support for covenants and forecasting.

## Core Concept

A financial statement is a DAG where each node is a line item and edges
represent computational dependencies:

```text
Revenue ──────────────────────┐
COGS ─────────────────────────┤
                              ├─ Gross Profit
SGA ──────────────────────────┤
D&A ──────────────────────────┤
                              ├─ EBITDA
Interest ─────────────────────┤
Tax ──────────────────────────┤
                              └─ Net Income
```

Nodes are defined using a DSL (expression language) that references other nodes:

```python
from finstack.statements import StatementBuilder, StmtExpr

stmt = StatementBuilder() \
    .add("revenue", StmtExpr.input(100_000_000)) \
    .add("cogs", StmtExpr.input(-60_000_000)) \
    .add("gross_profit", StmtExpr.sum(["revenue", "cogs"])) \
    .add("sga", StmtExpr.input(-15_000_000)) \
    .add("ebitda", StmtExpr.sum(["gross_profit", "sga"])) \
    .add("interest", StmtExpr.input(-5_000_000)) \
    .add("tax", StmtExpr.pct_of("ebitda", -0.21)) \
    .add("net_income", StmtExpr.sum(["ebitda", "interest", "tax"])) \
    .build()

result = stmt.evaluate()
print(result["net_income"])  # computed from the DAG
```

## Detail Pages

- [Waterfalls](waterfalls.md) — Waterfall engine, node graphs, cash distribution
- [Covenants](covenants.md) — Covenant definitions, monitoring, breach detection
- [Forecasting](forecasting.md) — Financial forecasting, adjustments, scenarios
