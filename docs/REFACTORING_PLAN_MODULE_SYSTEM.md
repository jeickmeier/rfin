# Refactoring Plan: Module System Simplification

## Current State Analysis

- Dynamic module registration in `/finstack-py/finstack/__init__.py` lines 30-69
- Hybrid module setup in `/finstack-py/finstack/core/__init__.py` lines 75-96
- Complex `_walk_and_register_nested` function creating unpredictable behavior

## Proposed New Structure

### 1. Eliminate Dynamic Registration

```python
# New /finstack-py/finstack/__init__.py
"""Finstack: Deterministic Financial Computation Library for Python."""

# Direct imports - no dynamic registration
from .core import *
from .valuations import *
from .statements import *
from .scenarios import *
from .portfolio import *

# Explicit re-exports for convenience
from .core.currency import Currency
from .core.money import Money
from .core.market_data.term_structures import DiscountCurve

__all__ = [
    # Core modules
    "core",
    "valuations",
    "statements",
    "scenarios",
    "portfolio",
    # Convenience re-exports
    "Currency",
    "Money",
    "DiscountCurve",
]
```

### 2. Simplified Core Module

```python
# New /finstack-py/finstack/core/__init__.py
"""Core module wrapper - explicit re-exports only."""

# Import Rust extension
try:
    from finstack import finstack as _finstack
    _rust_core = _finstack.core
except ImportError as e:
    raise ImportError(
        "Failed to import finstack Rust extension. "
        "Please ensure the package is properly built."
    ) from e

# Explicit re-exports - no loops or hasattr checks
from . import expr_helpers

# Re-export all Rust submodules explicitly
currency = _rust_core.currency
money = _rust_core.money
dates = _rust_core.dates
market_data = _rust_core.market_data
math = _rust_core.math
types = _rust_core.types
expr = _rust_core.expr
config = _rust_core.config
explain = _rust_core.explain
cashflow = _rust_core.cashflow
volatility = _rust_core.volatility

# Re-export key classes directly
Currency = currency.Currency
Money = money.Money
# ... other key exports

__all__ = [
    "currency",
    "money",
    "dates",
    "market_data",
    "math",
    "types",
    "expr",
    "config",
    "explain",
    "cashflow",
    "volatility",
    "expr_helpers",
    # Direct exports
    "Currency",
    "Money",
    # ... other direct exports
]
```

### 3. Migration Strategy

1. **Phase 1**: Create new explicit imports alongside existing system
2. **Phase 2**: Update all internal imports to use explicit paths
3. **Phase 3**: Remove dynamic registration code
4. **Phase 4**: Clean up unused helper functions

## Benefits

- Predictable import behavior
- Better IDE support
- Easier debugging
- Clearer dependency graph
- Faster import times
