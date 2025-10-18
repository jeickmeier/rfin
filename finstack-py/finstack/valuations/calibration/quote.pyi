"""Quote classes for calibration."""

from typing import Optional, Any
from datetime import date

class QuoteType:
    """Quote type enumeration."""
    
    # Class attributes
    PRICE: QuoteType
    YIELD: QuoteType
    SPREAD: QuoteType
    VOLATILITY: QuoteType
    
    @classmethod
    def from_name(cls, name: str) -> QuoteType:
        """Create quote type from name."""
        ...
    
    @property
    def name(self) -> str:
        """Quote type name."""
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Quote:
    """Quote for calibration."""
    
    def __init__(
        self,
        instrument_id: str,
        value: float,
        quote_type: QuoteType,
        as_of: date,
        market_data: Optional[Any] = None
    ) -> None:
        """Create a quote.
        
        Args:
            instrument_id: Instrument identifier
            value: Quote value
            quote_type: Type of quote
            as_of: Quote date
            market_data: Optional market data
        """
        ...
    
    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...
    
    @property
    def value(self) -> float:
        """Quote value."""
        ...
    
    @property
    def quote_type(self) -> QuoteType:
        """Quote type."""
        ...
    
    @property
    def as_of(self) -> date:
        """Quote date."""
        ...
    
    @property
    def market_data(self) -> Optional[Any]:
        """Optional market data."""
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
