"""Analytics package — canonical path for the finstack-analytics crate.

Provides the :class:`Performance` class for computing returns, drawdowns,
risk metrics, and benchmark-relative statistics from price data.

The :mod:`finstack.analytics.expr` sub-module exposes the same metrics as
native Polars expression plugins for use inside ``.select()``,
``.with_columns()``, and other Polars contexts.

Note:
    ``finstack.core.analytics`` remains importable as a compatibility shim.
"""

from __future__ import annotations
from . import expr as expr
from .performance import (
    BetaResult,
    DrawdownEpisodeDict,
    GreeksResult,
    LookbackResult,
    MultiFactorResult,
    Performance,
    PeriodStatsResult,
)

__all__ = [
    "BetaResult",
    "DrawdownEpisodeDict",
    "GreeksResult",
    "LookbackResult",
    "MultiFactorResult",
    "Performance",
    "PeriodStatsResult",
    "expr",
]
