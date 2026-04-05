# Statement Modeling

This cookbook builds financial statement models, tests covenants, and runs
forecasts.

## 1. Build an Income Statement

```python
from finstack.statements import StatementBuilder, StmtExpr

stmt = StatementBuilder() \
    .add("revenue", StmtExpr.input(100_000_000)) \
    .add("cogs", StmtExpr.input(-60_000_000)) \
    .add("gross_profit", StmtExpr.sum(["revenue", "cogs"])) \
    .add("sga", StmtExpr.input(-15_000_000)) \
    .add("dna", StmtExpr.input(-5_000_000)) \
    .add("ebitda", StmtExpr.sum(["gross_profit", "sga"])) \
    .add("ebit", StmtExpr.sum(["ebitda", "dna"])) \
    .add("interest", StmtExpr.input(-5_000_000)) \
    .add("ebt", StmtExpr.sum(["ebit", "interest"])) \
    .add("tax", StmtExpr.pct_of("ebt", -0.21)) \
    .add("net_income", StmtExpr.sum(["ebt", "tax"])) \
    .build()

result = stmt.evaluate()
for node in ["revenue", "gross_profit", "ebitda", "ebit", "net_income"]:
    print(f"{node:>15}: {result[node]:>15,.0f}")
```

## 2. Covenant Testing

```python
from finstack.statements import Covenant

leverage = Covenant(
    name="max_leverage",
    numerator="total_debt",
    denominator="ebitda",
    threshold=4.0,
    direction="max",
)

ic = Covenant(
    name="min_interest_coverage",
    numerator="ebitda",
    denominator="interest",
    threshold=2.0,
    direction="min",
)

# Add debt to the statement
stmt_with_debt = StatementBuilder() \
    .extend(stmt) \
    .add("total_debt", StmtExpr.input(80_000_000)) \
    .build()

result = stmt_with_debt.evaluate()
cov_results = result.test_covenants([leverage, ic])

for cr in cov_results:
    status = "PASS" if cr.passed else "BREACH"
    print(f"{cr.name}: {cr.ratio:.2f}x [{status}]")
```

## 3. Multi-Year Forecast

```python
from finstack.statements import ForecastBuilder, ForecastMethod

forecast = ForecastBuilder(base_statement=stmt) \
    .set_method("revenue", ForecastMethod.growth(0.05)) \
    .set_method("cogs", ForecastMethod.linked("revenue", -0.60)) \
    .set_method("sga", ForecastMethod.growth(0.03)) \
    .set_method("dna", ForecastMethod.growth(0.02)) \
    .set_method("interest", ForecastMethod.scenario(
        [-5_000_000, -4_500_000, -4_000_000, -3_500_000]
    )) \
    .periods(4) \
    .build()

for i, period in enumerate(forecast.evaluate()):
    print(f"Year {i+1}: Revenue={period['revenue']:,.0f}, "
          f"EBITDA={period['ebitda']:,.0f}, "
          f"Net Income={period['net_income']:,.0f}")
```

## 4. Waterfall Model

```python
from finstack.statements import WaterfallBuilder, WaterfallNode

waterfall = WaterfallBuilder() \
    .add_node(WaterfallNode(
        name="senior_fees",
        amount=StmtExpr.input(50_000),
        priority=1,
    )) \
    .add_node(WaterfallNode(
        name="class_a_interest",
        amount=StmtExpr.pct_of("class_a_balance", 0.05 / 12),
        priority=2,
    )) \
    .add_node(WaterfallNode(
        name="equity",
        amount=StmtExpr.residual(),
        priority=99,
    )) \
    .build()

results = waterfall.evaluate(periods=60, collateral_cashflows=cfs)
```
