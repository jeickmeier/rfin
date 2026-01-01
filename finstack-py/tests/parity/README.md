# Parity Tests

This directory contains comprehensive parity tests that verify the Python bindings produce identical results to the Rust implementation.

## Structure

- `test_core_parity.py` - Core module (currency, money, dates, market data, math, expr)
- `test_valuations_parity.py` - Valuations module (instruments, pricing, metrics)
- `test_statements_parity.py` - Statements module (model building, evaluation)
- `test_scenarios_parity.py` - Scenarios module (spec, engine, DSL)
- `test_portfolio_parity.py` - Portfolio module (entities, positions, valuation)

## Test Philosophy

### What is Parity?

Parity tests verify that:

1. **Identical inputs produce identical outputs** - Rust and Python yield the same numeric results
2. **API completeness** - All public Rust APIs are exposed in Python
3. **Behavioral consistency** - Edge cases and error handling match across languages

### Tolerance Levels

- **Deterministic operations** (arithmetic, date calculations): `abs(python_result - rust_result) < 1e-10`
- **Floating-point operations** (pricing, discounting): `abs(python_result - rust_result) < 1e-8`
- **Monte Carlo simulations** (with fixed seed): `abs(python_result - rust_result) < 1e-6`

### Test Pattern

Each parity test follows this structure:

```python
def test_feature_parity():
    """Test that Python and Rust produce identical results for feature X."""
    # 1. Create inputs (same values for both languages)
    input_value = 100.0

    # 2. Execute operation in Python
    python_result = python_api(input_value)

    # 3. Define expected result (from Rust golden values or analytical calculation)
    expected_result = 105.0  # Known correct answer

    # 4. Assert parity within tolerance
    assert abs(python_result - expected_result) < 1e-10
```

## Running Tests

Run all parity tests:

```bash
pytest tests/parity/ -v
```

Run specific module:

```bash
pytest tests/parity/test_core_parity.py -v
pytest tests/parity/test_valuations_parity.py -v
```

Run with coverage:

```bash
pytest tests/parity/ --cov=finstack --cov-report=html
```

## Adding New Parity Tests

When adding a new Python binding:

1. Add test to appropriate module file
2. Use known-good values from Rust tests or analytical calculations
3. Document tolerance requirements
4. Include edge cases (zero, negative, boundary values)
5. Test error handling matches Rust behavior

## CI/CD Integration

These tests are run:

- On every commit to main branch
- Before releasing new versions
- After updating Rust core library

Parity test failures block releases to ensure behavioral consistency.
