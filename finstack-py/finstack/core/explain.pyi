"""Explainability components: options and trace containers.

This module exposes the explanation infrastructure for capturing
execution traces during financial computations.

Examples
--------
    >>> from finstack.core.explain import ExplainOpts, ExplanationTrace, TraceEntry
    >>> opts = ExplainOpts.enabled()
    >>> trace = ExplanationTrace("calibration")
    >>> trace.push(
    ...     TraceEntry.calibration_iteration(
    ...         iteration=0, residual=0.005, knots_updated=["2025-01-15"], converged=False
    ...     ),
    ...     max_entries=100
    ... )
"""

from typing import Any, Optional


class ExplainOpts:
    """Opt-in configuration for generating explanation traces.

    ExplainOpts controls whether detailed execution traces are captured
    during financial computations. Traces provide transparency into how
    values are calculated, which is useful for debugging, auditing, and
    explainability.

    Explanation traces can be memory-intensive, so they are opt-in and
    can be capped with max_entries.

    Parameters
    ----------
    enabled : bool, optional
        Whether explanation tracing is enabled (default: False).
    max_entries : int, optional
        Maximum number of trace entries to capture (default: None).

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
    :class:`TraceEntry`: Domain-specific trace entries
    """

    DISABLED: "ExplainOpts"
    """Pre-built disabled options (zero overhead)."""

    def __init__(
        self, enabled: bool = False, max_entries: Optional[int] = None
    ) -> None: ...

    @classmethod
    def enabled(cls) -> "ExplainOpts":
        """Create an enabled ExplainOpts instance with default limits.
        
        Default limit is 1000 entries to prevent unbounded memory growth.
        
        Returns
        -------
        ExplainOpts
            Enabled options with max_entries=1000.
        """
        ...

    @property
    def is_enabled(self) -> bool:
        """Whether tracing is enabled."""
        ...

    @property
    def max_entries(self) -> Optional[int]:
        """Maximum number of trace entries, if set."""
        ...

    def __repr__(self) -> str: ...


class TraceEntry:
    """Domain-specific trace entry for explainability output.
    
    Use class methods to create entries for different computation types:
    - :meth:`calibration_iteration`: Calibration solver iteration details
    - :meth:`cashflow_pv`: Cashflow present value breakdown
    - :meth:`waterfall_step`: Structured credit waterfall step
    - :meth:`computation_step`: Generic computation step
    - :meth:`jacobian`: Jacobian sensitivity matrix
    
    Examples
    --------
        >>> from finstack.core.explain import TraceEntry
        >>> entry = TraceEntry.calibration_iteration(
        ...     iteration=0, residual=0.005, knots_updated=["2025-01-15"], converged=False
        ... )
        >>> entry.kind
        'calibration_iteration'
    """

    @classmethod
    def calibration_iteration(
        cls,
        iteration: int,
        residual: float,
        knots_updated: list[str],
        converged: bool,
    ) -> "TraceEntry":
        """Create a calibration iteration entry.
        
        Parameters
        ----------
        iteration : int
            Iteration number (0-based).
        residual : float
            Objective function residual.
        knots_updated : list[str]
            Knot points that were updated (date strings).
        converged : bool
            Whether convergence was achieved.
            
        Returns
        -------
        TraceEntry
            Calibration iteration entry.
        """
        ...

    @classmethod
    def cashflow_pv(
        cls,
        date: str,
        cashflow_amount: float,
        cashflow_currency: str,
        discount_factor: float,
        pv_amount: float,
        pv_currency: str,
        curve_id: str,
    ) -> "TraceEntry":
        """Create a cashflow present value entry.
        
        Parameters
        ----------
        date : str
            Cashflow payment date (ISO8601).
        cashflow_amount : float
            Cashflow amount.
        cashflow_currency : str
            Cashflow currency code.
        discount_factor : float
            Discount factor applied.
        pv_amount : float
            Present value of this cashflow.
        pv_currency : str
            PV currency code.
        curve_id : str
            Discount curve ID used.
            
        Returns
        -------
        TraceEntry
            Cashflow PV entry.
        """
        ...

    @classmethod
    def waterfall_step(
        cls,
        period: int,
        step_name: str,
        cash_in_amount: float,
        cash_in_currency: str,
        cash_out_amount: float,
        cash_out_currency: str,
        shortfall_amount: Optional[float] = None,
        shortfall_currency: Optional[str] = None,
    ) -> "TraceEntry":
        """Create a waterfall step entry.
        
        Parameters
        ----------
        period : int
            Period index.
        step_name : str
            Step name/description.
        cash_in_amount : float
            Cash inflow amount.
        cash_in_currency : str
            Cash inflow currency code.
        cash_out_amount : float
            Cash outflow amount.
        cash_out_currency : str
            Cash outflow currency code.
        shortfall_amount : float, optional
            Shortfall amount if any.
        shortfall_currency : str, optional
            Shortfall currency code.
            
        Returns
        -------
        TraceEntry
            Waterfall step entry.
        """
        ...

    @classmethod
    def computation_step(
        cls,
        name: str,
        description: str,
        metadata: Optional[dict[str, Any]] = None,
    ) -> "TraceEntry":
        """Create a generic computation step entry.
        
        Parameters
        ----------
        name : str
            Step name.
        description : str
            Step description.
        metadata : dict, optional
            Arbitrary metadata (JSON-serializable).
            
        Returns
        -------
        TraceEntry
            Computation step entry.
        """
        ...

    @classmethod
    def jacobian(
        cls,
        row_labels: list[str],
        col_labels: list[str],
        sensitivity_matrix: list[list[float]],
    ) -> "TraceEntry":
        """Create a Jacobian sensitivity matrix entry.
        
        Parameters
        ----------
        row_labels : list[str]
            Row labels (Instrument IDs).
        col_labels : list[str]
            Column labels (Curve Point Dates/Tenors).
        sensitivity_matrix : list[list[float]]
            Sensitivity matrix (Rows x Cols).
            
        Returns
        -------
        TraceEntry
            Jacobian entry.
        """
        ...

    @property
    def kind(self) -> str:
        """Get the entry kind (e.g., 'calibration_iteration', 'cashflow_pv')."""
        ...


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

    Parameters
    ----------
    trace_type : str
        Type of trace (e.g., "calibration", "pricing", "waterfall").

    Examples
    --------
        >>> from finstack.core.explain import ExplanationTrace, TraceEntry
        >>> trace = ExplanationTrace("calibration")
        >>> trace.push(
        ...     TraceEntry.calibration_iteration(0, 0.005, ["2025-01-15"], False),
        ...     max_entries=100
        ... )
        >>> trace.trace_type
        'calibration'

    Notes
    -----
    - Traces are only generated when ExplainOpts.enabled() is used
    - Entries are domain-specific (calibration, pricing, waterfall)
    - Traces can be truncated if max_entries exceeded
    - JSON serialization for persistence and sharing

    See Also
    --------
    :class:`ExplainOpts`: Trace configuration
    :class:`TraceEntry`: Domain-specific trace entries
    """

    def __init__(self, trace_type: str) -> None:
        """Create a new empty trace of the given type.
        
        Parameters
        ----------
        trace_type : str
            Type of trace (e.g., "calibration", "pricing", "waterfall").
        """
        ...

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

    def push(self, entry: TraceEntry, max_entries: Optional[int] = None) -> None:
        """Add an entry to the trace, respecting size limits.
        
        If max_entries is reached, marks the trace as truncated.
        
        Parameters
        ----------
        entry : TraceEntry
            Entry to add.
        max_entries : int, optional
            Maximum entries cap (overrides trace-level setting).
        """
        ...

    def to_json(self) -> str:
        """Serialize the full trace to a JSON string (pretty-printed).
        
        Returns
        -------
        str
            Pretty-printed JSON representation.
        """
        ...

    def __repr__(self) -> str: ...


__all__ = [
    "ExplainOpts",
    "ExplanationTrace",
    "TraceEntry",
]
