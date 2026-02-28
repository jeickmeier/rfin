"""Performance analytics module.

Provides the :class:`Performance` class for computing returns, drawdowns,
risk metrics, and benchmark-relative statistics from price data.
"""

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
    "Performance",
    "BetaResult",
    "GreeksResult",
    "MultiFactorResult",
    "DrawdownEpisodeDict",
    "LookbackResult",
    "PeriodStatsResult",
]
