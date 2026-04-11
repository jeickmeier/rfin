"""Finstack: Python bindings for the Rust finstack quantitative-finance toolkit.

The public API mirrors the Rust umbrella crate structure exactly.
Import subpackages by domain::

    from finstack import core, analytics, valuations
"""

from __future__ import annotations

from . import (
    analytics,
    core,
    correlation,
    margin,
    monte_carlo,
    portfolio,
    scenarios,
    statements,
    statements_analytics,
    valuations,
)

__all__ = [
    "analytics",
    "core",
    "correlation",
    "margin",
    "monte_carlo",
    "portfolio",
    "scenarios",
    "statements",
    "statements_analytics",
    "valuations",
]
