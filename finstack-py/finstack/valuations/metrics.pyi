"""Metric identifiers and registry helpers for finstack valuations."""

from typing import List, Any, Union
from .common import InstrumentType

class MetricId:
    """Strongly typed metric identifier for risk and return calculations.

    MetricId provides type-safe access to financial metrics computed during
    instrument valuation. It supports both standard metrics (DV01, CS01, yield,
    spread, Greeks, etc.) and custom metrics defined by users.

    Standard metrics cover:
    - **Risk metrics**: DV01, CS01, Theta, BucketedDV01, BucketedCS01
    - **Yield metrics**: YTM, YTW, yield
    - **Spread metrics**: Z-spread, OAS, I-spread, discount margin, ASW spread
    - **Pricing metrics**: Clean price, dirty price, accrued interest
    - **Duration metrics**: Modified duration, Macaulay duration, convexity
    - **Options Greeks**: Delta, Gamma, Vega, Theta, Rho, and higher-order Greeks
    - **Credit metrics**: Par spread, risky PV01, protection leg PV, default probability

    Examples
    --------
    Create metric identifiers:

        >>> from finstack.valuations.metrics import MetricId
        >>> # Standard metrics
        >>> dv01 = MetricId.from_name("dv01")
        >>> ytm = MetricId.from_name("ytm")
        >>> z_spread = MetricId.from_name("z_spread")
        >>> print(dv01.name)
        'dv01'

    List available metrics:

        >>> from finstack.valuations.metrics import MetricId
        >>> standard = MetricId.standard_names()
        >>> ("dv01" in standard, "cs01" in standard, "ytm" in standard)
        (True, True, True)

    Notes
    -----
    - Metric names are snake_case (e.g., "dv01", "z_spread", "clean_price")
    - Use standard_names() to see all available metrics
    - Custom metrics can be created for user-defined calculations
    - Metric availability depends on instrument type

    See Also
    --------
    :class:`MetricRegistry`: Check metric availability for instruments
    :meth:`PricerRegistry.price_with_metrics`: Request metrics during pricing
    :class:`ValuationResult`: Access computed metrics in results
    """

    @classmethod
    def from_name(cls, name: str) -> MetricId: ...
    """Parse a metric identifier, falling back to a custom metric when unknown.

    Creates a MetricId from a string name. If the name matches a standard
    metric, returns the standard identifier. Otherwise, creates a custom
    metric identifier.

    Parameters
    ----------
    name : str
        Metric label in snake_case (e.g., "pv", "dv01", "ytm", "z_spread").
        Common metrics:
        - Risk: "dv01", "cs01", "theta", "bucketed_dv01", "bucketed_cs01"
        - Yield: "ytm", "ytw", "yield"
        - Spread: "z_spread", "oas", "i_spread", "discount_margin", "asw_spread"
        - Pricing: "clean_price", "dirty_price", "accrued_interest"
        - Duration: "duration_modified", "duration_macaulay", "convexity"
        - Greeks: "delta", "gamma", "vega", "rho"

    Returns
    -------
    MetricId
        Metric identifier corresponding to the name. If name is not a
        standard metric, creates a custom metric identifier.

    Raises
    ------
    ValueError
        If name is empty or invalid.

    Examples
    --------
        >>> dv01 = MetricId.from_name("dv01")
        >>> print(dv01.name)
        'dv01'
        >>> 
        >>> ytm = MetricId.from_name("ytm")
        >>> print(ytm.name)
        'ytm'
        >>> 
        >>> # Custom metric
        >>> custom = MetricId.from_name("my_custom_metric")
        >>> print(custom.name)
        'my_custom_metric'
    """

    @property
    def name(self) -> str: ...
    """Snake-case name of the metric.

    Returns
    -------
    str
        Metric label in snake_case format (e.g., "dv01", "z_spread", "ytm").
        Suitable for use as dictionary keys in ValuationResult.measures.

    Examples
    --------
        >>> from finstack.valuations.metrics import MetricId
        >>> metric = MetricId.from_name("dv01")
        >>> metric.name
        'dv01'
    """

    @classmethod
    def standard_names(cls) -> List[str]: ...
    """List of all standard metric identifiers bundled with finstack.

    Returns a comprehensive list of all built-in metric identifiers available
    in finstack. This includes risk metrics, yield metrics, spread metrics,
    pricing metrics, duration metrics, and options Greeks.

    Returns
    -------
    List[str]
        List of all standard metric names in snake_case format. Includes:
        - Risk: "dv01", "cs01", "theta", "bucketed_dv01", "bucketed_cs01", "pv01"
        - Yield: "ytm", "ytw"
        - Spread: "z_spread", "oas", "i_spread", "discount_margin", "asw_spread"
        - Pricing: "clean_price", "dirty_price", "accrued_interest", "accrued"
        - Duration: "duration_modified", "duration_macaulay", "convexity"
        - Greeks: "delta", "gamma", "vega", "rho", "foreign_rho"
        - Credit: "par_spread", "risky_pv01", "protection_leg_pv", "premium_leg_pv"
        - Options: "implied_vol", "vanna", "volga", "veta", "charm", "color", "speed"
        - And many more...

    Examples
    --------
        >>> from finstack.valuations.metrics import MetricId
        >>> standard = MetricId.standard_names()
        >>> ("dv01" in standard, "cs01" in standard, "ytm" in standard)
        (True, True, True)

    Notes
    -----
    - Standard metrics are available for all supported instrument types
    - Metric availability may vary by instrument (use MetricRegistry to check)
    - Custom metrics are not included in this list
    - Metric names are stable and suitable for serialization

    See Also
    --------
    :class:`MetricRegistry`: Check which metrics are available for specific instruments
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __richcmp__(self, other: object, op: int) -> object: ...

class MetricRegistry:
    """Registry of metric calculators with applicability filtering.

    MetricRegistry manages which metrics are available and which instruments
    they support. Use it to discover available metrics for specific instrument
    types and to check if a metric can be computed for a given instrument.

    The standard registry includes metrics for all major instrument types:
    bonds, swaps, options, credit instruments, and more. Metrics are registered
    with applicability filters to ensure they're only computed for supported
    instruments.

    Examples
    --------
    Use standard registry:

        >>> from finstack.valuations.metrics import MetricRegistry
        >>> from finstack.valuations.common import InstrumentType
        >>> registry = MetricRegistry.standard()
        >>> bond_metrics = {m.name for m in registry.metrics_for_instrument(InstrumentType.BOND)}
        >>> (
        ...     registry.has_metric("dv01"),
        ...     "dv01" in bond_metrics,
        ...     "ytm" in bond_metrics,
        ... )
        (True, True, True)

    Check metric applicability:

        >>> from finstack.valuations.metrics import MetricRegistry
        >>> from finstack.valuations.common import InstrumentType
        >>> registry = MetricRegistry.standard()
        >>> (
        ...     registry.is_applicable("dv01", InstrumentType.BOND),
        ...     registry.is_applicable("delta", InstrumentType.EQUITY_OPTION),
        ...     registry.is_applicable("dv01", InstrumentType.EQUITY_OPTION),
        ... )
        (True, True, False)

    Notes
    -----
    - Standard registry includes all built-in finstack metrics
    - Metrics are filtered by instrument type for correctness
    - Use metrics_for_instrument() to discover available metrics
    - Use is_applicable() to check before requesting metrics

    See Also
    --------
    :class:`MetricId`: Metric identifiers
    :class:`InstrumentType`: Instrument type enumeration
    :meth:`PricerRegistry.price_with_metrics`: Request metrics during pricing
    """

    def __init__(self) -> None: ...
    """Create an empty registry instance.

    Creates a new MetricRegistry without any pre-registered metrics.
    Use this for custom metric registrations or testing.

    Returns
    -------
    MetricRegistry
        Empty registry ready for custom metric registration.

    Examples
    --------
        >>> custom = MetricRegistry()
        >>> print(len(custom.available_metrics()))
        0
        >>> 
        >>> # Register custom metrics here...
    """

    @classmethod
    def standard(cls) -> MetricRegistry: ...
    """Create a registry populated with all finstack standard metrics.

    Returns a MetricRegistry pre-populated with all standard finstack metrics
    and their applicability filters. This is the registry used by default in
    PricerRegistry.

    Returns
    -------
    MetricRegistry
        Registry containing the complete default metric set, including:
        - Risk metrics (DV01, CS01, Theta, etc.)
        - Yield metrics (YTM, YTW)
        - Spread metrics (Z-spread, OAS, I-spread, etc.)
        - Pricing metrics (clean price, dirty price, accrued)
        - Duration metrics (modified duration, Macaulay duration, convexity)
        - Options Greeks (Delta, Gamma, Vega, Rho, etc.)
        - Credit metrics (par spread, risky PV01, etc.)

    Examples
    --------
        >>> registry = MetricRegistry.standard()
        >>> print(registry.has_metric("dv01"))
        True
        >>> print(registry.has_metric("ytm"))
        True
        >>> print(registry.has_metric("delta"))
        True
        >>> 
        >>> # Get all available metrics
        >>> all_metrics = registry.available_metrics()
        >>> print(len(all_metrics))
        50+

    Notes
    -----
    - Standard registry is sufficient for most use cases
    - Metrics are registered with instrument type filters
    - Use metrics_for_instrument() to see which metrics apply to your instrument
    """

    def available_metrics(self) -> List[MetricId]: ...
    """All metric identifiers currently registered.

    Returns a list of all metrics registered in this registry, regardless
    of instrument type applicability.

    Returns
    -------
    List[MetricId]
        List of all registered metric identifiers. For standard registry,
        this includes all built-in finstack metrics.

    Examples
    --------
        >>> registry = MetricRegistry.standard()
        >>> metrics = registry.available_metrics()
        >>> print(len(metrics))
        50+
        >>> 
        >>> # Get metric names
        >>> names = [m.name for m in metrics]
        >>> print("dv01" in names)
        True
    """

    def metrics_for_instrument(self, instrument_type: Union[InstrumentType, str]) -> List[MetricId]: ...
    """Metrics applicable to the supplied instrument type.

    Returns a filtered list of metrics that can be computed for the specified
    instrument type. This is useful for discovering which metrics are available
    for your instrument.

    Parameters
    ----------
    instrument_type : InstrumentType or str
        Instrument type enumeration (e.g., InstrumentType.BOND) or string
        label (e.g., "bond", "InterestRateSwap").

    Returns
    -------
    List[MetricId]
        List of metrics that can be computed for the instrument type.
        Empty list if no metrics are applicable.

    Raises
    ------
    ValueError
        If the instrument type cannot be parsed.

    Examples
    --------
        >>> from finstack.valuations.common import InstrumentType
        >>> 
        >>> registry = MetricRegistry.standard()
        >>> 
        >>> # Get metrics for bonds
        >>> bond_metrics = registry.metrics_for_instrument(InstrumentType.BOND)
        >>> print([m.name for m in bond_metrics])
        ['pv', 'dv01', 'cs01', 'ytm', 'ytw', 'clean_price', 'dirty_price', ...]
        >>> 
        >>> # Get metrics for equity options
        >>> option_metrics = registry.metrics_for_instrument(InstrumentType.EQUITY_OPTION)
        >>> print([m.name for m in option_metrics])
        ['pv', 'delta', 'gamma', 'vega', 'theta', 'rho', 'implied_vol', ...]
        >>> 
        >>> # Get metrics for swaps
        >>> swap_metrics = registry.metrics_for_instrument("InterestRateSwap")
        >>> print([m.name for m in swap_metrics])
        ['pv', 'dv01', 'annuity', 'par_rate', 'pv_fixed', 'pv_float', ...]

    Notes
    -----
    - Returns only metrics that are applicable to the instrument type
    - Some metrics (e.g., "pv", "theta") apply to all instruments
    - Instrument-specific metrics (e.g., "ytm" for bonds, "delta" for options)
      are filtered appropriately
    """

    def is_applicable(self, metric: Union[MetricId, str], instrument_type: Union[InstrumentType, str]) -> bool: ...
    """Test whether metric applies to the provided instrument type.

    Checks if a specific metric can be computed for a given instrument type.
    Useful for validating metric requests before calling price_with_metrics().

    Parameters
    ----------
    metric : MetricId or str
        Metric identifier or snake_case name (e.g., "dv01", "ytm", "delta").
    instrument_type : InstrumentType or str
        Instrument type enumeration or string label.

    Returns
    -------
    bool
        True if the metric can be computed for the instrument type,
        False otherwise.

    Raises
    ------
    ValueError
        If the metric or instrument type cannot be parsed.

    Examples
    --------
        >>> from finstack.valuations.common import InstrumentType
        >>> 
        >>> registry = MetricRegistry.standard()
        >>> 
        >>> # DV01 applies to bonds
        >>> print(registry.is_applicable("dv01", InstrumentType.BOND))
        True
        >>> 
        >>> # YTM applies to bonds
        >>> print(registry.is_applicable("ytm", InstrumentType.BOND))
        True
        >>> 
        >>> # Delta applies to options, not bonds
        >>> print(registry.is_applicable("delta", InstrumentType.EQUITY_OPTION))
        True
        >>> print(registry.is_applicable("delta", InstrumentType.BOND))
        False
        >>> 
        >>> # Theta applies to all instruments
        >>> print(registry.is_applicable("theta", InstrumentType.BOND))
        True
        >>> print(registry.is_applicable("theta", InstrumentType.EQUITY_OPTION))
        True

    Notes
    -----
    - Universal metrics (e.g., "pv", "theta") return True for all instruments
    - Instrument-specific metrics return True only for supported types
    - Use this to validate metric requests before pricing
    """

    def has_metric(self, metric: Union[MetricId, str]) -> bool: ...
    """Determine whether the registry contains metric.

    Checks if a metric is registered in the registry, regardless of
    instrument type applicability.

    Parameters
    ----------
    metric : MetricId or str
        Metric identifier or snake_case name (e.g., "dv01", "ytm").

    Returns
    -------
    bool
        True if the metric is registered, False otherwise.

    Raises
    ------
    ValueError
        If the metric name cannot be parsed.

    Examples
    --------
        >>> registry = MetricRegistry.standard()
        >>> 
        >>> print(registry.has_metric("dv01"))
        True
        >>> print(registry.has_metric("ytm"))
        True
        >>> print(registry.has_metric("custom_metric"))
        False
    """

    def clone(self) -> MetricRegistry:
        """Clone the registry for experimentation without mutating the original.

        Returns:
            MetricRegistry: Shallow clone of the current registry.

        Examples:
            >>> cloned = MetricRegistry.standard().clone()
            >>> cloned.has_metric("pv")
            True
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
