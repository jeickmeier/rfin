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
    """Manage global rounding behaviour and currency decimal scales.

    Parameters
    ----------
    None
        Construct via FinstackConfig() to use default rounding rules.

    Returns
    -------
    FinstackConfig
        Configuration handle that can be reused across money formatting operations.
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
    """Set the rounding mode.
    
    Parameters
    ----------
    mode : str or RoundingMode
        New rounding mode.
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
    """Set the ingest scale for a currency.
    
    Parameters
    ----------
    currency : str or Currency
        Currency to configure.
    decimals : int
        Number of decimal places for ingest.
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
    """Set the output scale for a currency.
    
    Parameters
    ----------
    currency : str or Currency
        Currency to configure.
    decimals : int
        Number of decimal places for output.
    """
