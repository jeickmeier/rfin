"""Market data components mirroring finstack-core: curves, surfaces, FX, scalars, and aggregation context.

This module provides comprehensive market data handling:
- Term Structures: Discount, forward, hazard, inflation curves
- Surfaces: Volatility surfaces for various asset classes
- FX: Foreign exchange rates and conversion policies
- Scalars: Market prices and time series
- Context: Market data aggregation and management
"""

from . import context
from . import dividends
from . import fx
from . import interp
from . import scalars
from . import surfaces
from . import term_structures

__all__ = [
    "context",
    "dividends",
    "fx", 
    "interp",
    "scalars",
    "surfaces",
    "term_structures",
]
