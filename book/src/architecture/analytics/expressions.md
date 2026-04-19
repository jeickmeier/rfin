# Expressions

`finstack` has one shared expression AST, but today it has two different
evaluation engines built on top of that AST.

## The split

### 1. `finstack_core::expr` — vector / column engine

This is the engine in `finstack/core/src/expr/`. It operates on column-oriented
numeric arrays:

- `Expr` and `Function` define the shared AST
- `CompiledExpr::new()` evaluates directly
- `CompiledExpr::with_planning()` builds a DAG-backed execution plan
- `SimpleContext` maps column names to positions
- `EvalOpts` controls optional cache budget and arena sizing

This engine is designed for:

- row-window functions such as `rolling_mean`, `rolling_std`, `ewm_mean`
- reducers that broadcast over a full column such as `quantile`
- efficient evaluation over in-memory vectors

### 2. `finstack_statements::evaluator::formula` — scalar / period engine

This is the engine in `finstack/statements/src/evaluator/formula.rs`. It
evaluates one statement node at a time inside a period-aware
`EvaluationContext`:

- values are resolved by node id, not column position
- historical lookbacks are re-evaluated over prior periods
- aggregate and time-aware functions (`ttm`, `ytd`, `qtd`, `fiscal_ytd`,
  `growth_rate`, etc.) live here
- evaluation happens inside the statements precedence contract:
  `Value > Forecast > Formula`

This engine is the one end users normally hit when they call statement or model
evaluation APIs.

## Why both exist

The two engines solve different problems:

- `core::expr` is optimized for numeric array execution and DAG reuse
- `statements::formula` is optimized for statement semantics, historical
  periods, and node-graph evaluation

They intentionally share the same AST so the statements layer can compile a DSL
into `Expr` values, but they do not share one evaluator implementation yet.

## Capability table

| Capability | `core::expr` | `statements::formula` | Notes |
| --- | --- | --- | --- |
| Arithmetic over literals / columns | Yes | Yes | Shared AST, different execution context |
| DAG planning / shared-subexpression reuse | Yes | No | `CompiledExpr::with_planning()` only |
| Row-window operations (`rolling_*`, `ewm_*`) | Yes | Yes | Statements re-evaluates across periods |
| Historical period functions (`ttm`, `ytd`, `qtd`, `fiscal_ytd`) | No | Yes | Rejected in `core::expr::eval_function_core()` |
| Statement-specific lookup / node resolution | No | Yes | Uses `EvaluationContext` |
| Column-oriented batch evaluation | Yes | No | `SimpleContext` + slices of `f64` |
| Statement precedence (`Value > Forecast > Formula`) | No | Yes | Load-bearing invariant in statements |

## What the shared AST buys us

The shared `Expr` / `Function` surface still gives useful consistency:

- one serialization format for expression trees
- one place to define function names and argument shapes
- one DSL compilation target from statements
- one future seam for extracting shared kernels

## What `core::expr` does not do

The `core::expr` engine does not currently own the full statement-language
semantics. In particular, these functions are intentionally rejected in
`core::expr` and must be evaluated in the statements layer:

- `sum`
- `mean`
- `ttm`
- `ytd`
- `qtd`
- `fiscal_ytd`
- `annualize`
- `annualize_rate`
- `coalesce`
- `growth_rate`

That split is enforced in `finstack/core/src/expr/eval_functions.rs`, where
unsupported variants return a validation error pointing callers at the
statements layer.

## Statements DSL examples

At the user-facing layer, statement expressions are still authored through the
DSL:

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

## Cycle detection

The shared expression DAG still enforces acyclicity at build time. Circular
references produce an error that identifies the cycle path.

## Normalization

Statement values can be normalized for comparison:

```python
normalized = stmt.normalize(
    base_node="revenue",
)
print(normalized["cogs"])          # -0.60
print(normalized["gross_profit"])  # 0.40
```
