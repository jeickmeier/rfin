"""Deterministic scenario capability for stress testing and what-if analysis.

This module provides tools for applying shocks to market data and financial
statement forecasts, enabling deterministic scenario analysis with stable
composition and priority-based conflict resolution.
"""

from .enums import CurveKind, VolSurfaceKind, TenorMatchMode
from .spec import OperationSpec, ScenarioSpec
from .reports import ApplicationReport, RollForwardReport
from .engine import ExecutionContext, ScenarioEngine

__all__ = [
    "CurveKind",
    "VolSurfaceKind", 
    "TenorMatchMode",
    "OperationSpec",
    "ScenarioSpec",
    "ApplicationReport",
    "RollForwardReport",
    "ExecutionContext",
    "ScenarioEngine",
]
