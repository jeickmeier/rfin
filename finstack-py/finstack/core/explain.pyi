"""Explainability components: options and trace containers.

This module exposes the explanation infrastructure for capturing
execution traces during financial computations.
"""

from typing import Any

class ExplainOpts:
    """Opt-in configuration for generating explanation traces.

    ExplainOpts controls whether detailed execution traces are captured
    during financial computations. Traces provide transparency into how
    values are calculated, which is useful for debugging, auditing, and
    explainability.

    Explanation traces can be memory-intensive, so they are opt-in and
    can be capped with max_entries.

    Examples
    --------
        >>> from finstack.core.explain import ExplainOpts
        >>> opts = ExplainOpts(enabled=True, max_entries=100)
        >>> (opts.is_enabled, opts.max_entries)
        (True, 100)

    Notes
    -----
    - Traces are opt-in (disabled by default)
    - max_entries caps memory usage
    - Traces are useful for debugging and auditing
    - Different trace types: calibration, pricing, waterfall

    See Also
    --------
    :class:`ExplanationTrace`: Trace container
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

    ExplanationTrace holds a sequence of execution steps captured during
    computation. Traces are organized by type (calibration, pricing, waterfall)
    and provide transparency into how values are calculated.

    Traces are useful for:
    - Debugging calculation errors
    - Auditing financial models
    - Explaining results to stakeholders
    - Understanding calculation dependencies

    Notes
    -----
    - Traces are only generated when ExplainOpts.enabled() is used
    - Entries are domain-specific (calibration, pricing, waterfall)
    - Traces can be truncated if max_entries exceeded
    - JSON serialization for persistence and sharing

    See Also
    --------
    :class:`ExplainOpts`: Trace configuration
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
