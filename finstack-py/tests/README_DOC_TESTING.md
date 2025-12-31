# Testing Documentation Examples

This directory contains tools for testing code examples in Python documentation.

## Approaches

### 1. Simple Manual Testing (`test_doc_examples_simple.py`)

**Recommended for most use cases.**

This file contains manually curated tests for representative examples from the documentation. It's simpler, more maintainable, and easier to debug than automatic extraction.

**Usage:**

```bash
# Run from finstack-py directory
pytest tests/test_doc_examples_simple.py -v
```

**Benefits:**

- Easy to maintain
- Can test complete workflows
- Easy to debug failures
- Can skip examples that require complex setup

**Adding new examples:**

1. Add a test method to the appropriate test class
2. Copy the example code from the docstring
3. Add assertions to verify the example works
4. Run the test to ensure it passes

### 2. Automatic Extraction (`test_doc_examples.py`)

**Experimental - may need refinement.**

This file automatically extracts all `>>>` code blocks from `.pyi` files and attempts to run them as tests.

**Usage:**

```bash
# Run from finstack-py directory
pytest tests/test_doc_examples.py -v
```

**Limitations:**

- Examples must be complete and runnable
- May miss examples that require setup code
- Can't handle examples that are intentionally incomplete
- May have issues with imports and context
- Some examples may be skipped if they require complex setup or missing dependencies

**How it works:**

1. Scans all `.pyi` files in `finstack/`
2. Extracts code blocks marked with `>>>` prompts
3. Attempts to execute each block
4. Reports failures

## Best Practices for Doc Examples

When writing examples in docstrings:

1. **Make examples complete and runnable:**

   ```python
   >>> from finstack.core import Currency, Money
   >>> usd = Currency("USD")
   >>> amount = Money(1_000_000, usd)
   >>> amount.amount
   1000000.0
   ```

2. **Include necessary imports:**

   ```python
   >>> from finstack.statements import ModelBuilder
   >>> from finstack.core.dates import PeriodId
   >>> from finstack.statements.types.value import AmountOrScalar
   ```

3. **Show expected output when helpful:**

   ```python
   >>> result = some_function()
   >>> result.value
   42.0
   ```

4. **For incomplete examples, use comments:**

   ```python
   >>> # ... setup code ...
   >>> result = compute_value(data)
   >>> # ... more processing ...
   ```

5. **Test your examples:**
   - Copy the example code
   - Run it in a Python REPL
   - Add it to `test_doc_examples_simple.py` if it's a key example

## Running Tests

### Run all doc example tests

```bash
pytest tests/test_doc_examples*.py -v
```

### Run specific test

```bash
pytest tests/test_doc_examples_simple.py::TestCoreExamples::test_currency_example -v
```

### Run with coverage

```bash
pytest tests/test_doc_examples*.py --cov=finstack --cov-report=html
```

## CI Integration

Add to your CI pipeline:

```yaml
- name: Test documentation examples
  run: |
    cd finstack-py
    pytest tests/test_doc_examples_simple.py -v
```

## Troubleshooting

### All tests skipped

If all tests in `test_doc_examples.py` are skipped:

- Check that `finstack` is properly installed: `make python-dev` or `maturin develop`
- Verify the extraction is finding examples (check the test output)
- Some examples may be marked as incomplete if they end with `...` or have syntax errors

### Import errors

- Ensure `finstack` is installed: `make python-dev` or `maturin develop`
- Check that you're in the `finstack-py` directory
- The test automatically provides common imports, but some examples may need additional imports

### Example failures

- Check if the example requires setup code not shown
- Verify the API hasn't changed
- Check if the example is intentionally incomplete
- Some examples may fail if they require external data or complex setup

### Missing examples

- Not all examples are tested (by design)
- Focus on testing key examples that users are likely to copy
- Add examples to `test_doc_examples_simple.py` as needed
- The automatic extraction may miss examples that don't follow the `>>>` prompt pattern
