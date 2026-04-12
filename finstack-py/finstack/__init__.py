"""Finstack: Python bindings for the Rust finstack quantitative-finance toolkit.

The public API mirrors the Rust umbrella crate structure exactly.
Import subpackages by domain::

    from finstack import core, analytics, valuations

Submodules are loaded lazily — importing ``finstack`` does not pull in every
domain, which reduces cold-start time in CLIs, notebooks, and serverless
contexts.
"""

from __future__ import annotations

import importlib as _importlib

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

_SUBMODULES: frozenset[str] = frozenset(__all__)


def __getattr__(name: str):
    if name in _SUBMODULES:
        mod = _importlib.import_module(f".{name}", __name__)
        globals()[name] = mod
        return mod
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
