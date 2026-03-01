"""Deterministic scenario toolkit for stress testing and what-if analysis.

The :mod:`finstack.scenarios` package mirrors the Rust ``finstack-scenarios`` crate
and exposes **data-only** specifications plus a lightweight execution engine.
Bindings are pass-through: all logic runs in Rust for reproducibility and speed.

Highlights
----------
- Market shocks (FX, curves, vol surfaces, base correlation)
- Statement adjustments (percent/assign)
- Instrument shocks (price/spread by type or attributes)
- Structured credit correlation/factor shocks
- Time roll-forward with carry/theta reporting
- JSON serialization for persistence and auditability
"""

from __future__ import annotations
from .enums import CurveKind, VolSurfaceKind, TenorMatchMode
from .spec import Compounding, OperationSpec, RateBindingSpec, ScenarioSpec
from .reports import ApplicationReport, RollForwardReport
from .engine import ExecutionContext, ScenarioEngine

__all__ = [
    "CurveKind",
    "VolSurfaceKind",
    "TenorMatchMode",
    "Compounding",
    "RateBindingSpec",
    "OperationSpec",
    "ScenarioSpec",
    "ApplicationReport",
    "RollForwardReport",
    "ExecutionContext",
    "ScenarioEngine",
]
