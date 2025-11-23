"""Market data components mirroring finstack-core: curves, surfaces, FX, scalars, and aggregation context.

This module provides comprehensive market data handling:
- Term Structures: Discount, forward, hazard, inflation curves
- Surfaces: Volatility surfaces for various asset classes
- FX: Foreign exchange rates and conversion policies
- Scalars: Market prices and time series
- Context: Market data aggregation and management
- Volatility: Volatility conventions and pricing models
- Bumps: Scenario specification helpers
- Diff: Market movement measurement utilities

Note: Interpolation types have moved to finstack.core.math.interp
"""

from . import bumps
from . import context
from . import diff
from . import dividends
from . import fx
from . import scalars
from . import surfaces
from . import term_structures
from . import volatility

__all__ = [
    "bumps",
    "context",
    "diff",
    "dividends",
    "fx",
    "scalars",
    "surfaces",
    "term_structures",
    "volatility",
]
