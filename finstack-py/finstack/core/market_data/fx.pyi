"""Foreign exchange bindings for FX policies, configuration, and matrices."""

from typing import List, Tuple, Optional, Union
from datetime import date
from ..currency import Currency

class FxConversionPolicy:
    """FX conversion policy for cross-currency calculations.

    Available policies:
    - CASHFLOW_DATE: Use cashflow date
    - PERIOD_END: Use period end date
    - PERIOD_AVERAGE: Average over the period
    - CUSTOM: Application-defined
    """

    CASHFLOW_DATE: "FxConversionPolicy"
    PERIOD_END: "FxConversionPolicy"
    PERIOD_AVERAGE: "FxConversionPolicy"
    CUSTOM: "FxConversionPolicy"

    @classmethod
    def from_name(cls, name: str) -> FxConversionPolicy: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Policy name (case-insensitive).
        
    Returns
    -------
    FxConversionPolicy
        Policy instance.
    """

    @property
    def name(self) -> str: ...
    """Get the policy name.
    
    Returns
    -------
    str
        Human-readable policy name.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxConfig:
    """Configuration for FX matrix behavior and triangulation.

    FxConfig controls how an FxMatrix evaluates rates, particularly whether
    it can compute cross rates via triangulation when direct quotes are
    unavailable. It also sets the cache capacity for performance optimization.

    Parameters
    ----------
    pivot_currency : Currency, optional
        Currency used as an intermediate step for triangulation. When
        evaluating a cross rate (e.g., EUR/GBP), if no direct quote exists,
        the matrix can compute it via the pivot (EUR/USD and GBP/USD).
        Typically USD for most applications. Defaults to USD if not specified.
    enable_triangulation : bool, optional
        If True, allow the matrix to compute rates via the pivot currency
        when direct quotes are unavailable. If False, only direct quotes
        are used. Defaults to False.
    cache_capacity : int, optional
        Maximum number of evaluated rates to cache for performance.
        Defaults to 256. Higher values use more memory but may improve
        performance for repeated queries.

    Returns
    -------
    FxConfig
        Configuration instance to pass to :class:`FxMatrix` constructor.

    Examples
    --------
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.fx import FxConfig
        >>> cfg = FxConfig(pivot_currency=Currency("USD"), enable_triangulation=True, cache_capacity=16)
        >>> print((cfg.pivot_currency.code, cfg.enable_triangulation, cfg.cache_capacity))
        ('USD', True, 16)

    Notes
    -----
    - Triangulation requires quotes for both currencies against the pivot
    - Pivot currency defaults to USD if not specified
    - Cache capacity affects memory usage but not correctness
    - Configuration is immutable once passed to FxMatrix

    See Also
    --------
    :class:`FxMatrix`: FX matrix using this configuration
    """

    def __init__(
        self,
        pivot_currency: Optional[Currency] = None,
        enable_triangulation: Optional[bool] = None,
        cache_capacity: Optional[int] = None,
    ) -> None: ...
    @property
    def pivot_currency(self) -> Currency: ...
    """Get the pivot currency.
    
    Returns
    -------
    Currency
        Pivot currency.
    """

    @property
    def enable_triangulation(self) -> bool: ...
    """Check if triangulation is enabled.
    
    Returns
    -------
    bool
        True if triangulation is enabled.
    """

    @property
    def cache_capacity(self) -> int: ...
    """Get the cache capacity.
    
    Returns
    -------
    int
        Cache capacity.
    """

    def __repr__(self) -> str: ...

class FxRateResult:
    """Result of FX rate lookup.

    Attributes
    ----------
    rate : float
        FX rate.
    triangulated : bool
        Whether the rate was triangulated.
    """

    rate: float
    triangulated: bool

    def __init__(self, rate: float, triangulated: bool) -> None: ...

class FxMatrix:
    """Foreign exchange rate matrix for multi-currency calculations.

    FxMatrix stores direct FX quotes and can evaluate rates between any pair
    of currencies, optionally using triangulation through a pivot currency
    when direct quotes are unavailable. It is used by MarketContext for
    multi-currency portfolio valuations and risk calculations.

    The matrix stores quotes as (from_currency, to_currency, rate) pairs,
    where rate represents how many units of to_currency equal one unit of
    from_currency. For example, EUR/USD = 1.10 means 1 EUR = 1.10 USD.

    Parameters
    ----------
    config : FxConfig, optional
        Configuration controlling triangulation behavior and cache capacity.
        If None, uses default settings (no triangulation, 256 quote cache).

    Returns
    -------
    FxMatrix
        Mutable FX matrix ready to accept quotes via :meth:`set_quote` or
        :meth:`set_quotes`.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.fx import FxConfig, FxMatrix
        >>> fx = FxMatrix(config=FxConfig(pivot_currency=Currency("USD"), enable_triangulation=True))
        >>> fx.set_quote(Currency("EUR"), Currency("USD"), 1.10)
        >>> fx.set_quote(Currency("GBP"), Currency("USD"), 1.25)
        >>> result = fx.rate(Currency("EUR"), Currency("GBP"), date(2024, 1, 1))
        >>> print((round(result.rate, 4), result.triangulated))
        (0.88, True)

    Notes
    -----
    - FX quotes are stored as direct (from, to, rate) pairs
    - Rates are date-agnostic (spot rates); use MarketBump for time-dependent FX
    - Triangulation requires a pivot currency (typically USD)
    - The matrix caches evaluated rates for performance
    - Use :meth:`cache_stats` to monitor cache usage
    - Inverse rates are automatically available (1/rate)

    See Also
    --------
    :class:`FxConfig`: Configuration for triangulation and caching
    :class:`FxRateResult`: Result of rate queries with triangulation flag
    :class:`MarketContext`: Container for FX matrices
    :class:`MarketBump`: Time-dependent FX shifts for scenarios
    """

    def __init__(self, config: Optional[FxConfig] = None) -> None: ...
    def set_quote(
        self,
        from_currency: Currency,
        to_currency: Currency,
        rate: float,
    ) -> None: ...
    """Set a direct FX rate.
    
    Parameters
    ----------
    from_currency : Currency
        Source currency.
    to_currency : Currency
        Target currency.
    rate : float
        Exchange rate.
    """

    def set_quotes(self, quotes: List[Tuple[Currency, Currency, float]]) -> None: ...
    """Set multiple FX rates.
    
    Parameters
    ----------
    quotes : List[Tuple[Currency, Currency, float]]
        List of (from, to, rate) tuples.
    """

    def rate(
        self,
        from_currency: Currency,
        to_currency: Currency,
        on: Union[str, date],
        policy: Optional[Union[str, FxConversionPolicy]] = None,
    ) -> FxRateResult: ...
    """Evaluate the FX rate between two currencies on a given date.
    
    Returns the exchange rate to convert from_currency to to_currency. If a
    direct quote exists, it is used. Otherwise, if triangulation is enabled,
    the rate is computed via the pivot currency. The result includes a flag
    indicating whether triangulation was used.
    
    Parameters
    ----------
    from_currency : Currency
        Source currency (base currency of the rate).
    to_currency : Currency
        Target currency (quote currency of the rate).
    on : str or date
        Valuation date. Currently used for policy selection; actual rates
        are spot rates stored in the matrix.
    policy : str or FxConversionPolicy, optional
        FX conversion policy for cashflow timing. Defaults to cashflow_date.
        See :class:`FxConversionPolicy` for available policies.
        
    Returns
    -------
    FxRateResult
        Result containing:
        - rate: The exchange rate (how many to_currency per from_currency)
        - triangulated: True if the rate was computed via triangulation
        
    Raises
    ------
    ValueError
        If no direct quote exists and triangulation is disabled or fails.

    Notes
    -----
    - Rates are stored as spot rates (date-agnostic)
    - The 'on' parameter is used for policy selection, not rate lookup
    - Inverse rates are automatically available (querying USD/EUR when EUR/USD exists)
    - Triangulation requires both currencies to have quotes against the pivot
    - Use :meth:`cache_stats` to monitor cache usage
        
    See Also
    --------
    :class:`FxRateResult`: Result structure with triangulation flag
    :class:`FxConversionPolicy`: Cashflow timing policies
    """

    def cache_stats(self) -> Tuple[int, int]: ...
    """Get cache statistics as (stored_quotes, reserved_capacity)."""

    def __repr__(self) -> str: ...
