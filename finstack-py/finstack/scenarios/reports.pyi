"""Report types from scenario execution."""

from typing import List, Tuple, Optional
from datetime import date

class ApplicationReport:
    """Report describing what happened during scenario application.
    
    Attributes:
        operations_applied: Number of operations successfully applied
        warnings: Warnings generated during application (non-fatal)
        rounding_context: Rounding context stamp (for reproducibility metadata)
    """
    
    @property
    def operations_applied(self) -> int:
        """Number of operations successfully applied.
        
        Returns:
            int: Count of applied operations
        """
        ...
    
    @property
    def warnings(self) -> List[str]:
        """Warnings generated during application (non-fatal).
        
        Returns:
            list[str]: List of warning messages
        """
        ...
    
    @property
    def rounding_context(self) -> Optional[str]:
        """Rounding context stamp (for reproducibility metadata).
        
        Returns:
            str | None: Rounding context identifier if available
        """
        ...
    
    def __repr__(self) -> str: ...

class RollForwardReport:
    """Report from time roll-forward operation.
    
    Attributes:
        old_date: Original as-of date
        new_date: New as-of date after roll
        days: Number of days rolled forward
        instrument_carry: Per-instrument carry accrual
        instrument_mv_change: Per-instrument market value change
        total_carry: Total P&L from carry
        total_mv_change: Total P&L from market value changes
    """
    
    @property
    def old_date(self) -> date:
        """Original as-of date.
        
        Returns:
            date: Date before roll
        """
        ...
    
    @property
    def new_date(self) -> date:
        """New as-of date after roll.
        
        Returns:
            date: Date after roll
        """
        ...
    
    @property
    def days(self) -> int:
        """Number of days rolled forward.
        
        Returns:
            int: Day count
        """
        ...
    
    @property
    def instrument_carry(self) -> List[Tuple[str, float]]:
        """Per-instrument carry accrual.
        
        Returns:
            list[tuple[str, float]]: List of (instrument_id, carry) pairs
        """
        ...
    
    @property
    def instrument_mv_change(self) -> List[Tuple[str, float]]:
        """Per-instrument market value change.
        
        Returns:
            list[tuple[str, float]]: List of (instrument_id, mv_change) pairs
        """
        ...
    
    @property
    def total_carry(self) -> float:
        """Total P&L from carry.
        
        Returns:
            float: Total carry
        """
        ...
    
    @property
    def total_mv_change(self) -> float:
        """Total P&L from market value changes.
        
        Returns:
            float: Total market value change
        """
        ...
    
    def __repr__(self) -> str: ...
