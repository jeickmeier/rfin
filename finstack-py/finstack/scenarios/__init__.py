"""Scenarios bindings (Rust).

This package re-exports the Rust extension module types for scenario
specification, composition, and execution.
"""

from __future__ import annotations

from finstack import finstack as _finstack
from finstack.valuations.common import InstrumentType as _InstrumentType

from .error import Error, Result

_rust = _finstack.scenarios

ApplicationReport = _rust.ApplicationReport
Compounding = _rust.Compounding
CurveKind = _rust.CurveKind
ExecutionContext = _rust.ExecutionContext
OperationSpec = _rust.OperationSpec
RateBindingSpec = _rust.RateBindingSpec
RollForwardReport = _rust.RollForwardReport
ScenarioEngine = _rust.ScenarioEngine
ScenarioSpec = _rust.ScenarioSpec
TenorMatchMode = _rust.TenorMatchMode
TimeRollMode = _rust.TimeRollMode
InstrumentType = _InstrumentType
VolSurfaceKind = _rust.VolSurfaceKind

__all__ = [
    "ApplicationReport",
    "Compounding",
    "CurveKind",
    "Error",
    "ExecutionContext",
    "InstrumentType",
    "OperationSpec",
    "RateBindingSpec",
    "Result",
    "RollForwardReport",
    "ScenarioEngine",
    "ScenarioSpec",
    "TenorMatchMode",
    "TimeRollMode",
    "VolSurfaceKind",
]
