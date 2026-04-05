# Waterfalls

A waterfall is a sequential cash distribution model used in structured finance
(CLOs, project finance, securitizations).

## Waterfall Model

Cash flows through a priority-ordered sequence of tranches:

```text
Available Cash
  │
  ├─ 1. Senior Fees (trustee, admin)
  ├─ 2. Class A Interest
  ├─ 3. Class A Principal
  ├─ 4. Class B Interest
  ├─ 5. Class B Principal
  ├─ 6. IC/OC Test Diversions
  ├─ 7. Equity Tranche (residual)
  └─ 8. Incentive Management Fee
```

## Construction

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
        name="class_a_principal",
        amount=StmtExpr.min_of("available_principal", "class_a_target"),
        priority=3,
    )) \
    .add_node(WaterfallNode(
        name="equity",
        amount=StmtExpr.residual(),  # whatever is left
        priority=99,
    )) \
    .build()
```

## Coverage Tests

Waterfalls often include interest coverage (IC) and overcollateralization (OC)
tests that can divert cash from junior to senior tranches:

```python
waterfall.add_test(CoverageTest(
    name="oc_test_a",
    ratio=StmtExpr.ratio("collateral_par", "class_a_balance"),
    trigger=1.20,                    # 120% OC ratio
    diversion_target="class_a_principal",
    diversion_pct=1.0,               # 100% diversion on breach
))
```

## Evaluation

The waterfall is evaluated period by period:

```python
results = waterfall.evaluate(
    periods=60,                      # 60 monthly periods
    collateral_cashflows=cf_schedule, # from underlying assets
)

for period in results.periods:
    print(f"Period {period.idx}: Equity CF = {period['equity']}")
```
