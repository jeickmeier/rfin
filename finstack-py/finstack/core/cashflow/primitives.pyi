"""Cashflow primitive bindings.

Provides basic cashflow types and amortization specifications
for financial instrument modeling.
"""

from typing import List, Tuple, Optional, Union
from datetime import date
from ..money import Money

class CFKind:
    """Cashflow kind enumeration.

    Available kinds:
    - Fixed: Fixed cashflow
    - Floating: Floating cashflow
    - PIK: Payment-in-kind
    - Amortization: Principal amortization
    - PrincipalExchange: Principal exchange
    - Fee: Fee payment
    """

    @classmethod
    def from_name(cls, name: str) -> CFKind: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Kind name (case-insensitive).
        
    Returns
    -------
    CFKind
        Cashflow kind instance.
    """

    @property
    def name(self) -> str: ...
    """Get the kind name.
    
    Returns
    -------
    str
        Human-readable kind name.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

# Cashflow kind constants
Fixed: CFKind
Floating: CFKind
PIK: CFKind
Amortization: CFKind
PrincipalExchange: CFKind
Fee: CFKind

class CashFlow:
    """A single cashflow event.

    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Cashflow amount.
    kind : CFKind
        Cashflow kind.
    accrual_factor : float, optional
        Accrual factor.
    reset_date : str or date, optional
        Reset date for floating cashflows.
    """

    def validate(self) -> None: ...
    """Validate cashflow amount and fields.
    
    Raises
    ------
    ValueError
        If the cashflow amount is zero.
    """

    @property
    def kind(self) -> CFKind: ...
    """Get the cashflow kind.
    
    Returns
    -------
    CFKind
        Cashflow kind.
    """

    def date(self) -> date: ...
    """Get the cashflow date.
    
    Returns
    -------
    date
        Cashflow date.
    """

    def reset_date(self) -> Optional[date]: ...
    """Get the reset date.
    
    Returns
    -------
    date or None
        Reset date if applicable.
    """

    @property
    def amount(self) -> Money: ...
    """Get the cashflow amount.
    
    Returns
    -------
    Money
        Cashflow amount.
    """

    @property
    def accrual_factor(self) -> float: ...
    """Get the accrual factor.
    
    Returns
    -------
    float
        Accrual factor.
    """

    def set_accrual_factor(self, value: float) -> None: ...
    """Set the accrual factor.
    
    Parameters
    ----------
    value : float
        New accrual factor.
    """

    def to_tuple(self) -> Tuple[date, Money, CFKind, float, Optional[date]]: ...
    """Convert to tuple representation.
    
    Returns
    -------
    Tuple[date, Money, CFKind, float, Optional[date]]
        (date, amount, kind, accrual_factor, reset_date).
    """

    def __repr__(self) -> str: ...
