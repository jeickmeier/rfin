"""Scenarios bindings (Rust).

Data-only scenario specification and a lightweight execution engine. All
logic runs in Rust; this package is a thin Python surface over
``finstack.finstack.scenarios``.
"""

from __future__ import annotations

from typing import Any

from finstack import FinstackError, finstack as _finstack
from finstack.valuations.common import InstrumentType

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
VolSurfaceKind = _rust.VolSurfaceKind

# The Rust crate reports errors via the shared FinstackError type. Result is
# kept as a free-form alias for forward compatibility with Rust's Result<T, E>.
Error = FinstackError
type Result = Any

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
