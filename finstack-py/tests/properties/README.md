# Property Tests for Finstack Python Bindings

This directory contains property-based tests using [Hypothesis](https://hypothesis.readthedocs.io/) to verify invariants and mathematical properties of the finstack Python API.

## Overview

Property tests complement unit tests by automatically generating hundreds of test cases to verify that certain properties hold across a wide range of inputs. This helps catch edge cases and ensures mathematical correctness.

## Test Modules

### 1. Currency Safety Properties (`test_currency_safety_properties.py`)

**Invariants Tested:**
- **Same-currency operations never raise**: Adding/subtracting Money with same currency always succeeds
- **Cross-currency operations raise**: Adding/subtracting Money with different currencies always raises
- **Currency preservation**: Operations preserve currency through transformations
- **Commutativity**: `a + b = b + a`
- **Associativity**: `(a + b) + c = a + (b + c)`
- **Identity elements**: `a + 0 = a`, `a * 1 = a`
- **Inverse elements**: `a - a = 0`, `-(-a) = a`
- **Distributive property**: `k*(a + b) = k*a + k*b`
- **Immutability**: Operations don't mutate original instances

**Why These Matter:**
Currency safety is critical in financial systems. These tests ensure that cross-currency arithmetic errors are caught at runtime and that Money operations follow mathematical group/field properties.

### 2. Pricing Determinism Properties (`test_pricing_determinism_properties.py`)

**Invariants Tested:**
- **Pricing reproducibility**: Pricing the same instrument twice yields identical results
- **Order independence**: Order of pricing multiple instruments doesn't affect individual results
- **Market context reusability**: Same market context can be used for multiple pricings
- **Curve insertion order independence**: Order of curve insertion doesn't affect market state
- **Iteration stability**: Multiple iterations produce identical results
- **Metrics determinism**: Metrics computation is deterministic

**Why These Matter:**
Determinism is essential for reproducible risk reports, trade confirmations, and regulatory compliance. Non-deterministic pricing would make debugging impossible and violate audit requirements.

### 3. Curve Reversibility Properties (`test_curve_reversibility_properties.py`)

**Invariants Tested:**
- **Bump reversibility**: Bumping curve up then down restores original
- **Symmetric bumps cancel**: `bump(+x)` then `bump(-x)` is identity
- **Multiple cycle stability**: Multiple bump-unbump cycles don't accumulate errors
- **Commutativity**: Sequential bumps are additive: `bump(a) then bump(b) = bump(a+b)`
- **Identity**: Bumping by zero is identity operation
- **Structure preservation**: Bumping preserves curve structure (dates, id)
- **Inverse existence**: Every bump has an inverse

**Why These Matter:**
Curve bumping is fundamental to risk calculations (DV01, CS01). These tests ensure that bump operations are numerically stable and that risk calculations can be trusted.

### 4. Monotonicity Properties (`test_monotonicity_properties.py`)

**Invariants Tested:**
- **Discount rate monotonicity**: Higher discount rate → lower PV
- **Coupon rate monotonicity**: Higher coupon → higher PV
- **Maturity monotonicity**: Longer maturity → more accrued interest
- **Bump monotonicity**: Larger positive bump → lower PV
- **Notional scaling**: PV scales proportionally with notional
- **Discount factor monotonicity**: DF decreases as rate increases
- **Transitivity**: If `rate_a < rate_b < rate_c`, then `PV(a) > PV(b) > PV(c)`

**Why These Matter:**
Monotonicity relationships are fundamental to fixed income pricing. Violations indicate numerical errors or incorrect implementations that could lead to arbitrage or mispricing.

## Running the Tests

### Run all property tests:
```bash
pytest finstack-py/tests/properties/ -v
```

### Run specific test module:
```bash
pytest finstack-py/tests/properties/test_currency_safety_properties.py -v
```

### Run with more examples (stress testing):
```bash
pytest finstack-py/tests/properties/ -v --hypothesis-profile=stress
```

### Run with specific random seed (for reproducibility):
```bash
pytest finstack-py/tests/properties/ -v --hypothesis-seed=12345
```

### Run and show generated examples:
```bash
pytest finstack-py/tests/properties/ -v --hypothesis-show-statistics
```

## Hypothesis Configuration

The tests use the following default settings:
- **max_examples**: 30-50 per test (can be increased for stress testing)
- **deadline**: None (allows tests to run without time limits)
- **suppress_health_check**: Disabled for most tests

## Test Coverage Statistics

Each test module generates:
- **Currency Safety**: 25+ property tests, 100+ examples per test
- **Pricing Determinism**: 15+ property tests, 30-50 examples per test
- **Curve Reversibility**: 15+ property tests, 20-30 examples per test
- **Monotonicity**: 15+ property tests, 20-50 examples per test

**Total**: 70+ property tests generating thousands of test cases automatically.

## Debugging Failed Property Tests

When a property test fails, Hypothesis provides:

1. **Minimal failing example**: Automatically shrinks input to smallest case that fails
2. **Reproducible seed**: Rerun with `--hypothesis-seed=<seed>` to reproduce
3. **Full stack trace**: Shows exact assertion that failed

Example failure output:
```
Falsifying example: test_same_currency_addition_never_raises(
    money_pair=(Money(1e+100, USD), Money(-1e+100, USD))
)
```

## Best Practices

1. **Use `assume()` to filter inputs**: Skip invalid cases early
2. **Set reasonable bounds**: Avoid extreme values that cause numerical instability
3. **Check relative errors**: Use relative tolerance for floating-point comparisons
4. **Document invariants**: Each test should have clear docstring explaining what property is tested
5. **Keep tests fast**: Use `max_examples` to balance coverage vs speed

## Common Patterns

### Generating test data:
```python
@st.composite
def money_strategy(draw, currency_code=None):
    if currency_code is None:
        currency_code = draw(major_currencies)
    amount = draw(amounts)
    currency = Currency(currency_code)
    return Money(amount, currency)
```

### Testing with tolerance:
```python
@given(same_currency_pair())
def test_addition_commutative(self, money_pair):
    m1, m2 = money_pair
    result1 = m1 + m2
    result2 = m2 + m1
    assert abs(result1.amount - result2.amount) < 1e-10
```

### Filtering invalid inputs:
```python
@given(money_strategy(), non_zero_amounts)
def test_scalar_division_preserves_currency(self, money, divisor):
    assume(abs(divisor) > 1e-10)  # Avoid division by zero
    result = money / divisor
    assert result.currency.code == money.currency.code
```

## Integration with CI/CD

Property tests are part of the standard test suite and run automatically on:
- Pull requests
- Main branch commits
- Scheduled nightly builds (with increased `max_examples`)

## Future Enhancements

Planned additions:
- **Statement evaluation properties**: Determinism, commutativity of node evaluation
- **Scenario composition properties**: Associativity, identity
- **Portfolio aggregation properties**: Commutativity, currency safety
- **Option pricing properties**: Put-call parity, moneyness relationships

## References

- [Hypothesis Documentation](https://hypothesis.readthedocs.io/)
- [Property-Based Testing](https://fsharpforfunandprofit.com/posts/property-based-testing/)
- [QuickCheck Paper](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf)

## Contributing

When adding new property tests:
1. Identify mathematical invariants that should hold
2. Write clear test docstrings explaining the property
3. Use appropriate Hypothesis strategies for input generation
4. Set reasonable `max_examples` (start with 30-50)
5. Add test to appropriate module or create new module if needed
6. Update this README with new tests
