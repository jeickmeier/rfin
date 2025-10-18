"""Money-market deposit with simple interest accrual."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class Deposit:
    """Money-market deposit with simple interest accrual.
    
    Examples:
        >>> deposit = Deposit(
        ...     "dep_001",
        ...     Money("USD", 1_000_000),
        ...     date(2024, 1, 2),
        ...     date(2024, 2, 2),
        ...     DayCount("act_360"),
        ...     "usd_discount"
        ... )
        >>> deposit.quote_rate
        None
    """
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        end: date,
        day_count: DayCount,
        discount_curve: str,
        quote_rate: Optional[float] = None
    ) -> None:
        """Create a deposit with explicit start/end dates and optional quoted rate.
        
        Args:
            instrument_id: Instrument identifier or string-like object.
            notional: Notional principal amount as Money.
            start: Start date for interest accrual.
            end: End date for the deposit.
            day_count: Day-count convention label or object.
            discount_curve: Discount curve identifier used for valuation.
            quote_rate: Optional quoted simple rate in decimal form.
            
        Returns:
            Deposit: Configured deposit instrument ready for pricing.
            
        Raises:
            ValueError: If identifiers or dates cannot be parsed.
            RuntimeError: When the underlying Rust builder encounters invalid input.
        """
        ...
    
    @property
    def instrument_id(self) -> str:
        """Instrument identifier.
        
        Returns:
            str: Unique identifier assigned to the instrument.
        """
        ...
    
    @property
    def notional(self) -> Money:
        """Underlying notional amount.
        
        Returns:
            Money: Notional amount wrapped in Money.
        """
        ...
    
    @property
    def start(self) -> date:
        """Start date of the deposit period.
        
        Returns:
            datetime.date: Start date for interest accrual.
        """
        ...
    
    @property
    def end(self) -> date:
        """End date of the deposit period.
        
        Returns:
            datetime.date: Maturity date for the deposit.
        """
        ...
    
    @property
    def day_count(self) -> DayCount:
        """Day-count convention used for accrual.
        
        Returns:
            DayCount: Day-count convention wrapper.
        """
        ...
    
    @property
    def quote_rate(self) -> Optional[float]:
        """Optional quoted simple rate.
        
        Returns:
            float | None: Quoted rate in decimal form when supplied.
        """
        ...
    
    @property
    def discount_curve(self) -> str:
        """Discount curve identifier used for valuation.
        
        Returns:
            str: Discount curve identifier.
        """
        ...
    
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type enum (InstrumentType.DEPOSIT).
        
        Returns:
            InstrumentType: Enumeration value identifying the instrument family.
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
