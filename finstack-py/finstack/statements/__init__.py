"""Statements bindings (Rust).

This package is a thin re-export of the Rust extension module.
No runtime monkeypatching or compatibility shims are applied.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
import sys as _sys
import types as _types
from typing import cast

from finstack import finstack as _finstack

_rust_statements = _finstack.statements

for _name in dir(_rust_statements):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_statements, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr


class Report(ABC):
    """Shared reporting surface implemented by concrete statement report types."""

    @abstractmethod
    def to_string(self) -> str: ...

    @abstractmethod
    def print(self) -> None: ...

    def to_markdown(self) -> str:
        return self.to_string()


for _report_name in ("PLSummaryReport", "CreditAssessmentReport", "DebtSummaryReport"):
    _report_cls = globals().get(_report_name)
    if isinstance(_report_cls, type):
        Report.register(_report_cls)

globals()["Report"] = Report

_analysis_mod = globals().get("analysis")
if isinstance(_analysis_mod, _types.ModuleType):
    _analysis_mod.__dict__["Report"] = Report
    _analysis_all = _analysis_mod.__dict__.get("__all__")
    if isinstance(_analysis_all, list) and "Report" not in _analysis_all:
        _analysis_mod.__dict__["__all__"] = [*cast(list[str], _analysis_all), "Report"]

__all__ = [name for name in globals() if not name.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]
