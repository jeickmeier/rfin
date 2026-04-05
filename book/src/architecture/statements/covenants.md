# Covenants

Covenants are financial tests applied to statement outputs. The statements
crate supports both maintenance covenants (tested periodically) and incurrence
covenants (tested on specific events).

## Covenant Types

| Type | Description | Example |
|------|-------------|--------|
| Leverage | Debt / EBITDA ratio | Max 4.0x |
| Interest Coverage | EBITDA / Interest | Min 2.0x |
| Fixed Charge | (EBITDA - CapEx) / Fixed Charges | Min 1.25x |
| Minimum Liquidity | Cash + Revolver availability | Min $10M |
| Maximum CapEx | Annual capital expenditure limit | Max $5M |
| Debt/Equity | Total debt / equity ratio | Max 2.0x |

## Definition

```python
from finstack.statements import Covenant, CovenantType

leverage_cov = Covenant(
    name="max_leverage",
    covenant_type=CovenantType.MAINTENANCE,
    numerator="total_debt",
    denominator="ebitda",
    threshold=4.0,
    direction="max",       # ratio must be <= threshold
    test_frequency="quarterly",
)

ic_cov = Covenant(
    name="min_interest_coverage",
    covenant_type=CovenantType.MAINTENANCE,
    numerator="ebitda",
    denominator="interest_expense",
    threshold=2.0,
    direction="min",       # ratio must be >= threshold
    test_frequency="quarterly",
)
```

## Breach Detection

```python
result = stmt.evaluate()
covenant_results = result.test_covenants([leverage_cov, ic_cov])

for cov_result in covenant_results:
    status = "PASS" if cov_result.passed else "BREACH"
    print(f"{cov_result.name}: {cov_result.ratio:.2f}x [{status}]")
```

## Cure Mechanics

Covenant breaches can trigger cure rights (equity injection to restore
compliance) or waivers. The framework tracks breach history and cure
period windows.
