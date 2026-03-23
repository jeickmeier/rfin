"""Compatibility shim — canonical path is :mod:`finstack.analytics`.

.. deprecated::
    Import from ``finstack.analytics`` instead of ``finstack.core.analytics``.
    This module will be removed in a future release.
"""

from __future__ import annotations

import warnings as _warnings

_warnings.warn(
    "finstack.core.analytics is deprecated. Use finstack.analytics instead.",
    DeprecationWarning,
    stacklevel=2,
)

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

from finstack import finstack as _finstack  # type: ignore[reportAttributeAccessIssue]

from finstack.analytics import expr as expr  # noqa: E402  # canonical path

_rust_analytics = _finstack.core.analytics  # type: ignore[unresolved-attribute]

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
