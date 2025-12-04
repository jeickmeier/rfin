"""Core financial primitives and utilities.

This module provides the fundamental building blocks for financial computations:
- Currency: ISO-4217 currency metadata and identification
- Money: Currency-tagged monetary amounts with safe arithmetic
- Dates: Calendar, day count, and period utilities
- Market Data: Curves, surfaces, FX, and market context
- Math: Distributions, integration, and numerical solvers
- Config: Global rounding and scaling policies
- Explain: Explainability infrastructure for computation tracing
- Volatility: Volatility conventions and pricing models
- Expr: Expression engine for formula evaluation
- Types: Type-safe identifiers and rate helpers
"""

from . import cashflow
from . import config
from . import currency
from . import dates
from . import explain
from . import expr
from . import market_data
from . import math
from . import money
from . import types
from . import volatility

__all__ = [
    "cashflow",
    "config",
    "currency",
    "dates",
    "explain",
    "expr",
    "market_data",
    "math",
    "money",
    "types",
    "volatility",
]
