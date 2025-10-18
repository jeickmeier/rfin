"""Foreign exchange bindings.

Provides FX rate management and conversion policies
for multi-currency calculations.
"""

from typing import List, Tuple, Optional, Union
from datetime import date
from ..currency import Currency

class FxConversionPolicy:
    """FX conversion policy for cross-currency calculations.
    
    Available policies:
    - NoConversion: No conversion (error if currencies differ)
    - ConvertToBase: Convert to base currency
    - ConvertToQuote: Convert to quote currency
    - ConvertToPivot: Convert to pivot currency
    """
    
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

# FX conversion policy constants
NoConversion: FxConversionPolicy
ConvertToBase: FxConversionPolicy
ConvertToQuote: FxConversionPolicy
ConvertToPivot: FxConversionPolicy

class FxConfig:
    """FX configuration for rate management.
    
    Parameters
    ----------
    pivot_currency : Currency, optional
        Pivot currency for triangulation.
    enable_triangulation : bool, optional
        Enable automatic triangulation.
    cache_capacity : int, optional
        Cache capacity for rates.
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
    """FX rate matrix for currency conversions.
    
    Parameters
    ----------
    config : FxConfig, optional
        FX configuration.
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
    """Get FX rate between currencies.
    
    Parameters
    ----------
    from_currency : Currency
        Source currency.
    to_currency : Currency
        Target currency.
    on : str or date
        Date for the rate.
    policy : str or FxConversionPolicy, optional
        Conversion policy.
        
    Returns
    -------
    FxRateResult
        FX rate result.
    """
    
    def cache_stats(self) -> Tuple[int, int]: ...
    """Get cache statistics.
    
    Returns
    -------
    Tuple[int, int]
        (hits, misses) cache statistics.
    """
    
    def __repr__(self) -> str: ...
