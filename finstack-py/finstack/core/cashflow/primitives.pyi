"""Cashflow primitive bindings.

Provides basic cashflow types and amortization specifications
for financial instrument modeling.
"""

from typing import List, Tuple, Optional, Union
import datetime
from datetime import date
from ..money import Money

class CFKind:
    """Cashflow kind enumeration for categorizing payments.

    CFKind categorizes cashflows by their nature, which is important for
    reporting, analytics, and cashflow aggregation. Different kinds may
    be treated differently in calculations (e.g., PIK vs cash payments).

    Available kinds:
    - Fixed: Fixed-rate coupon or payment
    - Floating: Floating-rate payment (requires reset date)
    - PIK: Payment-in-kind (non-cash payment)
    - Amortization: Principal amortization payment
    - PrincipalExchange: Principal exchange (initial or final)
    - Fee: Fee payment (upfront, periodic, or exit fees)

    Notes
    -----
    - Kinds are used for cashflow categorization and reporting
    - Floating cashflows require reset_date
    - PIK cashflows represent non-cash payments
    - PrincipalExchange is used for initial/final principal exchanges

    See Also
    --------
    :class:`CashFlow`: Cashflow structure
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
    """A single cashflow event with date, amount, and metadata.

    CashFlow represents a single payment or receipt in a cashflow schedule.
    It includes the payment date, amount (as Money for currency safety),
    cashflow kind (fixed, floating, amortization, etc.), and optional
    accrual and reset date information.

    Cashflows are used to build payment schedules for bonds, loans, swaps,
    and other instruments. They support currency-safe arithmetic and can
    be aggregated into schedules.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.cashflow import CashFlow, CFKind
        >>> from finstack.core.money import Money
        >>> from finstack.core.currency import Currency
        >>> flow = CashFlow(
        ...     date=date(2025, 6, 30), amount=Money(25_000, Currency("USD")), kind=CFKind.FIXED, accrual_factor=0.5
        ... )
        >>> (flow.kind.name, flow.amount.currency.code)
        ('FIXED', 'USD')

    Notes
    -----
    - Amount must be non-zero (validated)
    - Currency is preserved via Money type
    - Accrual factor is used for yield calculations
    - Reset date is used for floating cashflows

    See Also
    --------
    :class:`CFKind`: Cashflow kind enumeration
    :class:`Money`: Currency-safe monetary amounts
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

    def date(self) -> datetime.date: ...
    """Get the cashflow date.
    
    Returns
    -------
    date
        Cashflow date.
    """

    def reset_date(self) -> Optional[datetime.date]: ...
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

    def to_tuple(self) -> Tuple[datetime.date, Money, CFKind, float, Optional[datetime.date]]: ...
    """Convert to tuple representation.
    
    Returns
    -------
    Tuple[datetime.date, Money, CFKind, float, Optional[datetime.date]]
        (payment_date, amount, kind, accrual_factor, reset_date).
    """

    def __repr__(self) -> str: ...
