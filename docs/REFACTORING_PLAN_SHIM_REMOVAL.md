# Shim Classes Removal Plan

## Current Issues

- Incomplete shim implementations in `/finstack-py/finstack/core/__init__.py` lines 57-216
- Type incompatibility between shims and Rust implementations
- Fallback behavior masks import errors

## Analysis of Current Shims

### 1. Currency Shim (lines 59-82)

```python
# Current problematic shim
if "Currency" not in globals():
    class Currency:
        def __init__(self, code: str) -> None:
            # Only supports 4 currencies!
            mapping = {
                "USD": (840, "US Dollar", 2),
                "EUR": (978, "Euro", 2),
                "GBP": (826, "Pound Sterling", 2),
                "JPY": (392, "Japanese Yen", 0),
            }
```

**Problems:**
- Only supports 4 currencies vs 170+ in Rust
- Missing methods and properties
- Creates inconsistent API

### 2. Money Shim (lines 87-120)

```python
# Missing key methods like format(), convert(), etc.
class Money:
    def __init__(self, amount: float, currency: Currency) -> None:
        # Basic implementation only
```

**Problems:**
- No formatting support
- Missing currency conversion
- No configuration support

### 3. DayCount Shim (lines 127-147)

**Problems:**
- Simplified calculation logic
- Missing calendar integration
- Different behavior from Rust version

## Removal Strategy

### Option 1: Complete Removal (Recommended)

```python
# New /finstack-py/finstack/core/__init__.py
"""Core module wrapper - requires Rust extension."""

# Import the Rust module - fail fast if not available
try:
    from finstack import finstack as _finstack
    _rust_core = _finstack.core
except ImportError as e:
    raise ImportError(
        "Failed to import finstack Rust extension.\n"
        "Please install the package with: pip install finstack[pkg]\n"
        "Or build from source with: maturin develop\n"
        "\nOriginal error: " + str(e)
    ) from e

# No shims - direct re-exports only
from . import expr_helpers

# Direct re-exports
currency = _rust_core.currency
money = _rust_core.money
# ... etc
```

### Option 2: Development Mode Fallback

```python
# For development/testing only
import os
if os.getenv("FINSTACK_DEV_MODE"):
    # Use minimal shims for type checking
    from ._dev_shims import Currency, Money, DayCount
else:
    # Require real implementation
    from finstack import finstack as _finstack
    Currency = _finstack.core.currency.Currency
    # ... etc
```

### Option 3: Separate Package

Create `finstack-stubs` package with mock implementations for type checking only.

## Recommended Implementation: Option 1

### Phase 1: Documentation and Warnings

```python
# Add deprecation warnings
import warnings

if "Currency" not in globals():
    warnings.warn(
        "Using fallback Currency shim. This will be removed in v0.9. "
        "Please ensure the Rust extension is properly installed.",
        DeprecationWarning,
        stacklevel=2
    )
    # Keep current shim for now
```

### Phase 2: Clear Error Messages

```python
# Replace shims with helpful errors
def _missing_implementation(name: str):
    def _raise_error(*args, **kwargs):
        raise ImportError(
            f"{name} is not available. "
            f"Please install the finstack Rust extension.\n"
            f"Run: pip install --force-reinstall finstack"
        )
    return _raise_error

if "Currency" not in globals():
    Currency = _missing_implementation("Currency")
```

### Phase 3: Complete Removal

```python
# Final version - no fallbacks
try:
    from finstack import finstack as _finstack
except ImportError as e:
    raise ImportError(
        "The finstack Rust extension is required.\n"
        "Installation instructions:\n"
        "1. Users: pip install finstack\n"
        "2. Developers: cd finstack-py && maturin develop\n"
        "3. Docker: Use the official finstack image\n"
        "\nIf you're seeing this error after installation, "
        "please report an issue at: https://github.com/finstack/finstack/issues"
    ) from e
```

## Migration Guide for Users

### Before (with shims)

```python
# This worked but was limited
from finstack import Currency
c = Currency("USD")  # Only worked for 4 currencies
```

### After (no shims)

```python
# Clear error message guides users
from finstack import Currency
# ImportError: The finstack Rust extension is required...
```

## Benefits of Removal

1. **Clear failure mode** - Users know immediately if installation is broken
2. **No API inconsistency** - Same behavior everywhere
3. **Simpler codebase** - Less maintenance overhead
4. **Better documentation** - Can document actual API without caveats
5. **Faster imports** - No hasattr checks or conditional logic

## Implementation Timeline

- **v0.8.0**: Add deprecation warnings
- **v0.8.5**: Replace with helpful errors
- **v0.9.0**: Complete removal

## Testing Strategy

```python
# Test that shims are gone
def test_no_shims():
    """Test that fallback shims are not present."""
    with pytest.raises(ImportError):
        # Mock failed import
        with patch('finstack.finstack', side_effect=ImportError):
            from finstack import Currency

# Test error messages are helpful
def test_helpful_import_error():
    """Test that import errors guide users."""
    with pytest.raises(ImportError) as exc_info:
        # Force import failure
        from finstack import NonExistentModule

    assert "Installation instructions" in str(exc_info.value)
    assert "pip install finstack" in str(exc_info.value)
```

## Rollback Plan

If removal causes too many issues:
1. Reintroduce shims with clear "development only" labeling
2. Create separate `finstack-mock` package for testing
3. Provide better installation documentation
