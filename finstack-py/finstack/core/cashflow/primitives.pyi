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
    
    @classmethod
    def fixed(
        cls,
        date: Union[str, date],
        amount: Money,
        accrual_factor: float,
    ) -> CashFlow: ...
    """Create a fixed cashflow.
    
    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Cashflow amount.
    accrual_factor : float
        Accrual factor.
        
    Returns
    -------
    CashFlow
        Fixed cashflow.
    """
    
    @classmethod
    def floating(
        cls,
        date: Union[str, date],
        amount: Money,
        reset_date: Optional[Union[str, date]],
        accrual_factor: float,
    ) -> CashFlow: ...
    """Create a floating cashflow.
    
    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Cashflow amount.
    reset_date : str or date, optional
        Reset date.
    accrual_factor : float
        Accrual factor.
        
    Returns
    -------
    CashFlow
        Floating cashflow.
    """
    
    @classmethod
    def pik(
        cls,
        date: Union[str, date],
        amount: Money,
    ) -> CashFlow: ...
    """Create a PIK cashflow.
    
    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Cashflow amount.
        
    Returns
    -------
    CashFlow
        PIK cashflow.
    """
    
    @classmethod
    def amortization(
        cls,
        date: Union[str, date],
        amount: Money,
    ) -> CashFlow: ...
    """Create an amortization cashflow.
    
    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Amortization amount.
        
    Returns
    -------
    CashFlow
        Amortization cashflow.
    """
    
    @classmethod
    def principal_exchange(
        cls,
        date: Union[str, date],
        amount: Money,
    ) -> CashFlow: ...
    """Create a principal exchange cashflow.
    
    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Exchange amount.
        
    Returns
    -------
    CashFlow
        Principal exchange cashflow.
    """
    
    @classmethod
    def fee(
        cls,
        date: Union[str, date],
        amount: Money,
    ) -> CashFlow: ...
    """Create a fee cashflow.
    
    Parameters
    ----------
    date : str or date
        Cashflow date.
    amount : Money
        Fee amount.
        
    Returns
    -------
    CashFlow
        Fee cashflow.
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

class AmortizationSpec:
    """Amortization specification for principal payments.
    
    Parameters
    ----------
    None
        Use class methods to create specific types.
    """
    
    @classmethod
    def none(cls) -> AmortizationSpec: ...
    """No amortization: principal remains until redemption.
    
    Returns
    -------
    AmortizationSpec
        No amortization spec.
    """
    
    @classmethod
    def linear_to(cls, final_notional: Money) -> AmortizationSpec: ...
    """Linear amortization to final notional.
    
    Parameters
    ----------
    final_notional : Money
        Final notional amount.
        
    Returns
    -------
    AmortizationSpec
        Linear amortization spec.
    """
    
    @classmethod
    def step_remaining(
        cls,
        schedule: List[Tuple[Union[str, date], Money]],
    ) -> AmortizationSpec: ...
    """Step amortization with remaining notional.
    
    Parameters
    ----------
    schedule : List[Tuple[str or date, Money]]
        (date, remaining_notional) pairs.
        
    Returns
    -------
    AmortizationSpec
        Step amortization spec.
    """
    
    @classmethod
    def percent_per_period(cls, pct: float) -> AmortizationSpec: ...
    """Percentage amortization per period.
    
    Parameters
    ----------
    pct : float
        Percentage per period.
        
    Returns
    -------
    AmortizationSpec
        Percentage amortization spec.
    """
    
    @classmethod
    def custom_principal(
        cls,
        items: List[Tuple[Union[str, date], Money]],
    ) -> AmortizationSpec: ...
    """Custom principal amortization.
    
    Parameters
    ----------
    items : List[Tuple[str or date, Money]]
        (date, principal_amount) pairs.
        
    Returns
    -------
    AmortizationSpec
        Custom amortization spec.
    """
    
    def __repr__(self) -> str: ...
