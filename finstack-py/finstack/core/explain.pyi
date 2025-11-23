"""Explainability components: options and trace containers.

This module exposes the explanation infrastructure for capturing
execution traces during financial computations.
"""

from typing import Any

class ExplainOpts:
    """Opt-in configuration for generating explanation traces.

    Controls whether detailed execution traces are captured during computation.

    Parameters
    ----------
    enabled : bool
        Whether explanation tracing is enabled.
    max_entries : int, optional
        Maximum number of trace entries (caps memory usage).
    """

    DISABLED: ExplainOpts

    def __init__(self, enabled: bool = False, max_entries: int | None = None) -> None: ...
    @classmethod
    def enabled(cls) -> ExplainOpts:
        """Create an enabled ExplainOpts instance."""
        ...

    @property
    def is_enabled(self) -> bool:
        """Whether tracing is enabled."""
        ...

    @property
    def max_entries(self) -> int | None:
        """Maximum number of trace entries, if set."""
        ...

    def __repr__(self) -> str: ...

class ExplanationTrace:
    """Container for detailed execution traces of financial computations.

    Traces are organized by type (calibration, pricing, waterfall) and contain
    a sequence of domain-specific entries.
    """

    @property
    def trace_type(self) -> str:
        """Type of trace (e.g., 'calibration', 'pricing')."""
        ...

    @property
    def truncated(self) -> bool:
        """Whether the trace was truncated due to size limits."""
        ...

    @property
    def entries(self) -> list[dict[str, Any]]:
        """List of trace entries as dictionaries."""
        ...

    def to_json(self) -> str:
        """Serialize the full trace to a JSON string (pretty-printed)."""
        ...

    def __repr__(self) -> str: ...

__all__ = [
    "ExplainOpts",
    "ExplanationTrace",
]
