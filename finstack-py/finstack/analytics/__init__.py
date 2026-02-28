"""Performance analytics module.

Provides the :class:`Performance` class for computing returns, drawdowns,
risk metrics, and benchmark-relative statistics from price data.

The :mod:`finstack.analytics.expr` sub-module exposes the same metrics as
native Polars expression plugins for use inside ``.select()``,
``.with_columns()``, and other Polars contexts.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .performance import (
        BetaResult as BetaResult,
        DrawdownEpisodeDict as DrawdownEpisodeDict,
        GreeksResult as GreeksResult,
        LookbackResult as LookbackResult,
        MultiFactorResult as MultiFactorResult,
        PeriodStatsResult as PeriodStatsResult,
    )

from finstack.finstack import analytics as _rust_analytics  # type: ignore[reportAttributeAccessIssue]

from . import expr as expr

Performance = _rust_analytics.Performance

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
