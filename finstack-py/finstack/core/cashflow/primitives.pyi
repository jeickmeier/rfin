"""Cashflow primitive bindings.

Provides basic cashflow types and classification enums
for financial instrument modeling.
"""

from typing import Optional, Union, ClassVar
import datetime
from datetime import date
from ..money import Money

class CFKind:
    """Cashflow kind enumeration for categorizing payments.

    CFKind categorizes cashflows by their nature, which is important for
    reporting, analytics, and cashflow aggregation. Different kinds may
    be treated differently in calculations (e.g., PIK vs cash payments).

    Available kinds include (non-exhaustive):
    - FIXED: Fixed-rate coupon cashflow
    - FLOAT_RESET: Floating-rate reset (index fixing)
    - FEE / COMMITMENT_FEE / USAGE_FEE / FACILITY_FEE: Fee cashflows
    - NOTIONAL / AMORTIZATION / PREPAYMENT: Principal flows
    - PIK: Payment-in-kind interest capitalization
    - DEFAULTED_NOTIONAL / RECOVERY: Credit event flows
    - INITIAL_MARGIN_* / VARIATION_MARGIN_*: Margin/collateral flows

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

    FIXED: ClassVar["CFKind"]
    FLOAT_RESET: ClassVar["CFKind"]
    FEE: ClassVar["CFKind"]
    COMMITMENT_FEE: ClassVar["CFKind"]
    USAGE_FEE: ClassVar["CFKind"]
    FACILITY_FEE: ClassVar["CFKind"]
    NOTIONAL: ClassVar["CFKind"]
    PIK: ClassVar["CFKind"]
    AMORTIZATION: ClassVar["CFKind"]
    PREPAYMENT: ClassVar["CFKind"]
    REVOLVING_DRAW: ClassVar["CFKind"]
    REVOLVING_REPAYMENT: ClassVar["CFKind"]
    DEFAULTED_NOTIONAL: ClassVar["CFKind"]
    RECOVERY: ClassVar["CFKind"]
    STUB: ClassVar["CFKind"]
    INITIAL_MARGIN_POST: ClassVar["CFKind"]
    INITIAL_MARGIN_RETURN: ClassVar["CFKind"]
    VARIATION_MARGIN_RECEIVE: ClassVar["CFKind"]
    VARIATION_MARGIN_PAY: ClassVar["CFKind"]
    MARGIN_INTEREST: ClassVar["CFKind"]
    COLLATERAL_SUBSTITUTION_IN: ClassVar["CFKind"]
    COLLATERAL_SUBSTITUTION_OUT: ClassVar["CFKind"]

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
