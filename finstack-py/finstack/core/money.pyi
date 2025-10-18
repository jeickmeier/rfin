"""Money bindings: currency-tagged amounts with safe arithmetic.

This module exposes the Rust Money type to Python with currency safety:
arithmetic requires matching currencies and raises ValueError otherwise.
Formatting respects ISO minor units by default and can be customized via
FinstackConfig ingest/output scales.
"""

from typing import Union, Tuple
from .currency import Currency

class Money:
    """Represent a currency-tagged monetary amount with safe arithmetic semantics.
    
    Parameters
    ----------
    amount : float
        Scalar value expressed in minor units defined by currency.
    currency : str or Currency
        ISO code or Currency instance describing the legal tender.
        
    Returns
    -------
    Money
        Money wrapper supporting arithmetic, formatting, and tuple conversions.
    """
    
    def __init__(self, amount: float, currency: Union[str, Currency]) -> None: ...
    
    @classmethod
    def from_config(cls, amount: float, currency: Union[str, Currency], config: "FinstackConfig") -> Money: ...
    """Construct a money value using a configuration for ingest rounding.
    
    Parameters
    ----------
    amount : float
        Raw monetary value.
    currency : Currency or str
        Currency identifier.
    config : FinstackConfig
        Configuration controlling ingest rounding/scale.
        
    Returns
    -------
    Money
        Money instance respecting custom ingest rules.
        
    Examples
    --------
    >>> cfg = FinstackConfig()
    >>> cfg.set_ingest_scale("JPY", 4)
    >>> Money.from_config(123.4567, "JPY", cfg)
    """
    
    @classmethod
    def zero(cls, currency: Union[str, Currency]) -> Money: ...
    """Create a zero amount in the specified currency.
    
    Parameters
    ----------
    currency : Currency or str
        Currency for the zero amount.
        
    Returns
    -------
    Money
        Zero amount in the specified currency.
    """
    
    @classmethod
    def from_tuple(cls, value: Tuple[float, Currency]) -> Money: ...
    """Construct from a (amount, currency) tuple.
    
    Parameters
    ----------
    value : Tuple[float, Currency]
        (amount, currency) tuple.
        
    Returns
    -------
    Money
        Money instance from tuple representation.
    """
    
    @property
    def amount(self) -> float: ...
    """Get the numeric amount.
    
    Returns
    -------
    float
        Scalar value in the currency's minor units.
    """
    
    @property
    def currency(self) -> Currency: ...
    """Get the currency.
    
    Returns
    -------
    Currency
        Currency instance.
    """
    
    def to_tuple(self) -> Tuple[float, Currency]: ...
    """Convert to (amount, currency) tuple.
    
    Returns
    -------
    Tuple[float, Currency]
        Tuple representation.
    """
    
    def format(self) -> str: ...
    """Format as a human-readable string.
    
    Returns
    -------
    str
        Formatted string (e.g. "USD 125.50").
    """
    
    def format_with_config(self, config: "FinstackConfig") -> str: ...
    """Format using custom configuration.
    
    Parameters
    ----------
    config : FinstackConfig
        Configuration for formatting rules.
        
    Returns
    -------
    str
        Formatted string respecting config rules.
    """
    
    def checked_add(self, other: Money) -> Money: ...
    """Add another money amount (same currency required).
    
    Parameters
    ----------
    other : Money
        Money to add (must have same currency).
        
    Returns
    -------
    Money
        Sum of the two amounts.
        
    Raises
    ------
    ValueError
        If currencies don't match.
    """
    
    def checked_sub(self, other: Money) -> Money: ...
    """Subtract another money amount (same currency required).
    
    Parameters
    ----------
    other : Money
        Money to subtract (must have same currency).
        
    Returns
    -------
    Money
        Difference of the two amounts.
        
    Raises
    ------
    ValueError
        If currencies don't match.
    """
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __add__(self, other: Money) -> Money: ...
    def __radd__(self, other: Money) -> Money: ...
    def __sub__(self, other: Money) -> Money: ...
    def __rsub__(self, other: Money) -> Money: ...
    def __mul__(self, factor: float) -> Money: ...
    def __rmul__(self, factor: float) -> Money: ...
    def __truediv__(self, divisor: float) -> Money: ...
    def __rtruediv__(self, value: float) -> None: ...
    def __getnewargs__(self) -> Tuple[float, Currency]: ...
