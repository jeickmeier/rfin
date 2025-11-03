"""Core financial primitives and utilities.

This module provides the fundamental building blocks for financial computations:
- Currency: ISO-4217 currency metadata and identification
- Money: Currency-tagged monetary amounts with safe arithmetic
- Dates: Calendar, day count, and period utilities
- Market Data: Curves, surfaces, FX, and market context
- Math: Distributions, integration, and numerical solvers
- Config: Global rounding and scaling policies
"""

from . import cashflow
from . import config
from . import currency
from . import dates
from . import market_data
from . import math
from . import money
from . import utils

__all__ = [
    "cashflow",
    "config",
    "currency",
    "dates",
    "market_data",
    "math",
    "money",
    "utils",
]
