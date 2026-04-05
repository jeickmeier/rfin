# Expressions

The analytics expression engine provides a DSL for defining computed metrics
that can reference other statement/analytics nodes.

## StmtExpr Types

The expression language supports these node types:

| Expression | Description | Example |
|-----------|-------------|--------|
| `input(value)` | Literal value | `StmtExpr.input(100_000)` |
| `sum(nodes)` | Sum of referenced nodes | `StmtExpr.sum(["a", "b", "c"])` |
| `diff(a, b)` | Difference a − b | `StmtExpr.diff("revenue", "cogs")` |
| `pct_of(node, pct)` | Percentage of a node | `StmtExpr.pct_of("revenue", 0.21)` |
| `ratio(num, den)` | Ratio of two nodes | `StmtExpr.ratio("debt", "ebitda")` |
| `min_of(a, b)` | Minimum of two nodes | `StmtExpr.min_of("cash", "payment")` |
| `max_of(a, b)` | Maximum of two nodes | `StmtExpr.max_of("floor", "value")` |
| `clamp(node, lo, hi)` | Clamp to range | `StmtExpr.clamp("rate", 0.0, 0.10)` |
| `if_then(cond, t, f)` | Conditional | `StmtExpr.if_then("test", "a", "b")` |
| `residual()` | Remainder after prior allocations | `StmtExpr.residual()` |
| `lookup(key)` | External data lookup | `StmtExpr.lookup("macro::gdp")` |

## Evaluation

Expressions are evaluated via topological sort of the DAG:

```python
from finstack.statements import StatementBuilder, StmtExpr

stmt = StatementBuilder() \
    .add("revenue", StmtExpr.input(100_000_000)) \
    .add("cogs", StmtExpr.pct_of("revenue", -0.60)) \
    .add("gross_profit", StmtExpr.sum(["revenue", "cogs"])) \
    .add("gross_margin", StmtExpr.ratio("gross_profit", "revenue")) \
    .build()

result = stmt.evaluate()
print(result["gross_profit"])  # 40,000,000
print(result["gross_margin"])  # 0.40
```

## Cycle Detection

The DAG enforces acyclicity at build time. Circular references produce a
clear error message identifying the cycle path.

## Normalization

Statement values can be normalized for comparison:

```python
normalized = stmt.normalize(
    base_node="revenue",  # express everything as % of revenue
)
print(normalized["cogs"])          # -0.60
print(normalized["gross_profit"])  # 0.40
```
