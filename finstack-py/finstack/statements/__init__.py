"""Statements bindings (Rust).

This package re-exports the Rust extension module.  Two submodules —
``analysis`` and ``templates`` — are deprecated; use
``finstack.statements_analytics.*`` instead.  A :class:`DeprecationWarning`
is emitted on first import of either deprecated path.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
import sys as _sys
import types as _types
from typing import cast

from finstack import finstack as _finstack

_rust_statements = _finstack.statements

# Submodules that have a Python shim with DeprecationWarning — exclude from
# the Rust-registration loop so the Python import machinery finds the shim.
_DEPRECATED_SUBMODULES = frozenset({"analysis", "templates"})

for _name in dir(_rust_statements):
    if _name.startswith("_") or _name in _DEPRECATED_SUBMODULES:
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

# Inject Report into the Rust analysis module directly so that canonical
# callers (finstack.statements_analytics.analysis) can also access it.
_rust_analysis = _finstack.statements.analysis
_rust_analysis.__dict__["Report"] = Report
_rust_analysis_all = _rust_analysis.__dict__.get("__all__")
if isinstance(_rust_analysis_all, list) and "Report" not in _rust_analysis_all:
    _rust_analysis.__dict__["__all__"] = [*cast(list[str], _rust_analysis_all), "Report"]

_HELPER_NAMES = frozenset({"ABC", "abstractmethod", "cast", "annotations"})
__all__ = [  # pyright: ignore[reportUnsupportedDunderAll]
    name for name in globals() if not name.startswith("_") and name not in _HELPER_NAMES
]
del _HELPER_NAMES
