"""Statements bindings (Rust).

Thin re-export of the ``finstack.statements`` Rust extension module with one
shared :class:`Report` abstract base class so concrete report types from both
this package and the underlying Rust ``analysis`` submodule satisfy a single
reporting surface.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import cast

from finstack import finstack as _finstack
from finstack._binding_exports import export_rust_members, set_public_all

export_rust_members(globals(), _finstack.statements, package_name=__name__)


class Report(ABC):
    """Shared reporting surface implemented by concrete statement report types."""

    @abstractmethod
    def to_string(self) -> str:
        """Render the report as plain text."""
        raise NotImplementedError

    @abstractmethod
    def print(self) -> None:
        """Write the report to standard output."""
        raise NotImplementedError

    def to_markdown(self) -> str:
        """Render the report using the default markdown-friendly text form."""
        return self.to_string()


for _report_name in ("PLSummaryReport", "CreditAssessmentReport", "DebtSummaryReport"):
    _report_cls = globals().get(_report_name)
    if isinstance(_report_cls, type):
        Report.register(_report_cls)

# Expose Report from the Rust analysis submodule as well, so callers that
# import via ``finstack.statements.analysis`` see the same abstract base.
_rust_analysis = _finstack.statements.analysis
_rust_analysis.__dict__["Report"] = Report
_rust_analysis_all = _rust_analysis.__dict__.get("__all__")
if isinstance(_rust_analysis_all, list) and "Report" not in _rust_analysis_all:
    _rust_analysis.__dict__["__all__"] = [*cast(list[str], _rust_analysis_all), "Report"]

set_public_all(globals(), helper_names={"ABC", "abstractmethod", "cast", "annotations"})
