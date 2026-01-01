# Consistent Error Handling Design

## Current Issues

- Generic error messages without context
- Inconsistent error types across modules
- Missing helpful suggestions for users

## Proposed Error Handling System

### 1. Centralized Error Definitions

```python
# New file: /finstack-py/finstack/errors.py
"""Centralized error definitions for finstack."""

from __future__ import annotations
from typing import Any, Optional, Dict, List
import sys

class FinstackError(Exception):
    """Base class for all finstack errors."""

    def __init__(
        self,
        message: str,
        context: Optional[str] = None,
        suggestions: Optional[List[str]] = None,
        details: Optional[Dict[str, Any]] = None
    ):
        super().__init__(message)
        self.context = context
        self.suggestions = suggestions or []
        self.details = details or {}

    def __str__(self) -> str:
        msg = super().__str__()
        if self.context:
            msg = f"{self.context}: {msg}"
        if self.suggestions:
            msg += f"\nSuggestions:\n  - " + "\n  - ".join(self.suggestions)
        return msg

class CurrencyError(FinstackError):
    """Raised for currency-related errors."""

    def __init__(self, code: str, **kwargs):
        message = f"Invalid currency code: '{code}'"
        suggestions = [
            f"Use a valid ISO 4217 code (e.g., 'USD', 'EUR', 'JPY')",
            "Check Currency.all() for available currencies"
        ]
        super().__init__(message, suggestions=suggestions, **kwargs)

class MoneyError(FinstackError):
    """Raised for money-related errors."""

    def __init__(self, message: str, currencies: Optional[List[str]] = None, **kwargs):
        if currencies:
            message += f" (currencies: {', '.join(currencies)})"
        super().__init__(message, **kwargs)

class MarketDataError(FinstackError):
    """Raised for market data related errors."""

    def __init__(self, message: str, curve_id: Optional[str] = None, **kwargs):
        context = f"Curve '{curve_id}'" if curve_id else None
        super().__init__(message, context=context, **kwargs)

class ValidationError(FinstackError):
    """Raised for validation errors."""
    pass

class ConfigurationError(FinstackError):
    """Raised for configuration errors."""
    pass
```

### 2. Error Handling Utilities

```python
# New file: /finstack-py/finstack/_error_utils.py
"""Internal utilities for consistent error handling."""

from typing import Any, Callable, TypeVar, Optional
import functools
from .errors import FinstackError

T = TypeVar('T')

def handle_errors(
    error_type: type[FinstackError],
    context: Optional[str] = None,
    reraise: bool = True
):
    """Decorator for consistent error handling."""
    def decorator(func: Callable[..., T]) -> Callable[..., T]:
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            try:
                return func(*args, **kwargs)
            except FinstackError:
                # Re-raise finstack errors as-is
                if reraise:
                    raise
                return None  # type: ignore
            except ValueError as e:
                # Convert ValueError to appropriate FinstackError
                if reraise:
                    raise error_type(str(e), context=context) from e
                return None  # type: ignore
            except TypeError as e:
                if reraise:
                    raise error_type(f"Invalid type: {e}", context=context) from e
                return None  # type: ignore
            except Exception as e:
                if reraise:
                    raise error_type(f"Unexpected error: {e}", context=context) from e
                return None  # type: ignore
        return wrapper
    return decorator

def validate_currency_code(code: str) -> str:
    """Validate and normalize currency code."""
    if not isinstance(code, str):
        raise CurrencyError(f"Currency code must be a string, got {type(code).__name__}")

    code = code.upper().strip()
    if len(code) != 3:
        raise CurrencyError(
            f"Currency code must be 3 characters, got '{code}'"
        )

    return code

def validate_positive_number(value: Any, name: str) -> float:
    """Validate a positive number."""
    try:
        num = float(value)
    except (TypeError, ValueError):
        raise ValidationError(f"{name} must be a number, got {value}")

    if num < 0:
        raise ValidationError(f"{name} must be positive, got {num}")

    return num
```

### 3. Rust Error Integration

```rust
// Update /finstack-py/src/errors.rs
use pyo3::exceptions::{PyValueError, PyTypeError};
use pyo3::prelude::*;
use std::fmt;

pub fn core_to_py(err: finstack_core::Error) -> PyErr {
    match err {
        finstack_core::Error::UnknownCurrency(code) => {
            PyValueError::new_err(format!(
                "Unknown currency '{}'. Use a valid ISO 4217 code. Available currencies: USD, EUR, GBP, JPY, ...",
                code
            ))
        }
        finstack_core::Error::CurrencyMismatch { expected, actual } => {
            PyValueError::new_err(format!(
                "Currency mismatch: expected {}, got {}. Use Money.convert() for cross-currency operations.",
                expected, actual
            ))
        }
        finstack_core::Error::MissingCurve(id) => {
            PyValueError::new_err(format!(
                "Missing curve '{}'. Insert it into MarketContext before use.",
                id
            ))
        }
        // ... other error mappings
    }
}

// Result extension for better error handling
pub trait PyResultExt<T> {
    fn context(self, context: &str) -> PyResult<T>;
}

impl<T> PyResultExt<T> for PyResult<T> {
    fn context(self, context: &str) -> PyResult<T> {
        self.map_err(|e| {
            PyValueError::new_err(format!("{}: {}", context, e))
        })
    }
}
```

### 4. Usage Examples

```python
# Updated /finstack-py/src/core/currency.rs (Python wrapper)
from .._error_utils import validate_currency_code, handle_errors
from ..errors import CurrencyError

@handle_errors(CurrencyError, "Currency creation")
def from_code(code: str) -> Currency:
    """Create currency with validated code."""
    validated_code = validate_currency_code(code)
    return _rust_core.currency.Currency(validated_code)

# Updated /finstack-py/src/core/money.rs
@handle_errors(MoneyError, "Money arithmetic")
def add(self, other: Money) -> Money:
    """Add with consistent error handling."""
    if self.currency != other.currency:
        raise MoneyError(
            "Cannot add money with different currencies",
            currencies=[self.currency.code, other.currency.code],
            suggestions=[
                "Convert to same currency first",
                "Use Money.convert() for currency conversion"
            ]
        )
    return _rust_core.money.Money.add(self, other)
```

### 5. Testing Error Handling

```python
# New file: /finstack-py/tests/test_error_handling.py
"""Test consistent error handling."""

def test_currency_error_formatting():
    """Test that currency errors include helpful suggestions."""
    with pytest.raises(CurrencyError) as exc_info:
        Currency("INVALID")

    error = exc_info.value
    assert "Invalid currency code" in str(error)
    assert "Use a valid ISO 4217 code" in str(error)
    assert "Currency.all()" in str(error)

def test_error_context():
    """Test error context is preserved."""
    with pytest.raises(MoneyError) as exc_info:
        # Operation that fails
        pass

    assert "Money arithmetic" in str(exc_info.value)
```

## Implementation Steps

1. Create centralized error definitions
2. Implement error handling utilities
3. Update Rust error mappings
4. Refactor existing code to use new errors
5. Add comprehensive error tests
6. Update documentation with error examples

## Benefits

- Consistent error experience across all modules
- Helpful error messages with suggestions
- Better debugging for users
- Easier maintenance of error handling
