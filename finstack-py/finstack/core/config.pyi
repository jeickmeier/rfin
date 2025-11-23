"""Configuration bindings for rounding policies and currency scales.

Provides a Python-facing FinstackConfig to manage global rounding behavior
and per-currency decimal scales for both ingestion and presentation. Use this
to control how Money values are parsed and formatted throughout analyses.
Also exposes a RoundingMode enum with common strategies (bankers, floor,
ceil, toward/away from zero).
"""

from typing import Dict, Optional
from .currency import Currency

class RoundingMode:
    """Rounding strategy for decimal arithmetic.

    Available modes:
    - Bankers: Round to nearest even (default)
    - Floor: Round toward negative infinity
    - Ceil: Round toward positive infinity
    - TowardZero: Round toward zero
    - AwayFromZero: Round away from zero
    """

    @classmethod
    def from_name(cls, name: str) -> RoundingMode: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Rounding mode name (case-insensitive).
        
    Returns
    -------
    RoundingMode
        Rounding mode instance.
        
    Raises
    ------
    ValueError
        If name is not recognized.
    """

    @property
    def name(self) -> str: ...
    """Get the mode name.
    
    Returns
    -------
    str
        Human-readable mode name.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

# Rounding mode constants
Bankers: RoundingMode
Floor: RoundingMode
Ceil: RoundingMode
TowardZero: RoundingMode
AwayFromZero: RoundingMode

class FinstackConfig:
    """Global configuration for rounding policies and currency decimal scales.

    FinstackConfig controls how monetary values are rounded during ingestion
    and formatted during output. It also manages per-currency decimal place
    settings, allowing fine-grained control over precision for different
    currencies.

    The configuration is immutable once set, but can be copied and modified
    to create new configurations. Default settings use Bankers rounding and
    ISO-4217 standard decimal places.

    Parameters
    ----------
    None
        Construct via ``FinstackConfig()`` to use default rounding rules
        (Bankers rounding, ISO-4217 decimal places).

    Returns
    -------
    FinstackConfig
        Configuration instance that can be reused across money operations.

    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> cfg = FinstackConfig()
        >>> cfg.set_rounding_mode("floor")
        >>> cfg.set_ingest_scale("JPY", 4)
        >>> cfg.set_output_scale("USD", 4)
        >>> print((cfg.rounding_mode.name, cfg.ingest_scale("JPY"), cfg.output_scale("USD")))
        ('floor', 4, 4)

    Notes
    -----
    - Configuration changes affect all subsequent operations using that config
    - Default rounding mode is Bankers (round to nearest even)
    - Default decimal scales follow ISO-4217 standard
    - Use :meth:`copy` to create independent configurations
    - Ingest scale controls precision when creating Money from floats
    - Output scale controls precision when formatting Money to strings

    See Also
    --------
    :class:`RoundingMode`: Available rounding strategies
    :class:`Money`: Money formatting with configuration
    """

    def __init__(self) -> None: ...
    def copy(self) -> FinstackConfig: ...
    """Create a copy of this configuration.
    
    Returns
    -------
    FinstackConfig
        Independent copy of the configuration.
    """

    @property
    def rounding_mode(self) -> RoundingMode: ...
    """Get the current rounding mode.
    
    Returns
    -------
    RoundingMode
        Active rounding strategy.
    """

    def set_rounding_mode(self, mode: Union[str, RoundingMode]) -> None: ...
    """Set the global rounding mode for decimal arithmetic.
    
    Parameters
    ----------
    mode : str or RoundingMode
        New rounding mode. Can be a string (case-insensitive) or a
        :class:`RoundingMode` instance. Valid strings: "bankers", "floor",
        "ceil", "toward_zero", "away_from_zero".
        
    Raises
    ------
    ValueError
        If the mode string is not recognized.
        
    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> cfg = FinstackConfig()
        >>> cfg.set_rounding_mode("ceil")
        >>> cfg.rounding_mode.name
        'ceil'
    """

    def ingest_scale(self, currency: Union[str, Currency]) -> int: ...
    """Get the ingest scale for a currency.
    
    Parameters
    ----------
    currency : str or Currency
        Currency to query.
        
    Returns
    -------
    int
        Number of decimal places for ingest.
    """

    def set_ingest_scale(self, currency: Union[str, Currency], decimals: int) -> None: ...
    """Set the number of decimal places used when creating Money from floats.
    
    The ingest scale controls how many decimal places are preserved when
    converting a float to a Money value. This affects precision during
    monetary operations.
    
    Parameters
    ----------
    currency : str or Currency
        Currency to configure (e.g., "USD", "JPY").
    decimals : int
        Number of decimal places to preserve (must be >= 0).
        
    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> cfg = FinstackConfig()
        >>> cfg.set_ingest_scale("JPY", 4)
        >>> cfg.ingest_scale("JPY")
        4
    """

    def output_scale(self, currency: Union[str, Currency]) -> int: ...
    """Get the output scale for a currency.
    
    Parameters
    ----------
    currency : str or Currency
        Currency to query.
        
    Returns
    -------
    int
        Number of decimal places for output.
    """

    def set_output_scale(self, currency: Union[str, Currency], decimals: int) -> None: ...
    """Set the number of decimal places used when formatting Money to strings.
    
    The output scale controls how many decimal places are shown when converting
    a Money value to a string representation. This affects display formatting
    but does not change the underlying precision.
    
    Parameters
    ----------
    currency : str or Currency
        Currency to configure (e.g., "USD", "JPY").
    decimals : int
        Number of decimal places to display (must be >= 0).
        
    Examples
    --------
        >>> from finstack.core.config import FinstackConfig
        >>> from finstack.core.money import Money
        >>> cfg = FinstackConfig()
        >>> cfg.set_output_scale("USD", 4)
        >>> Money(100.123456, "USD").format_with_config(cfg)
        'USD 100.1235'
    """
