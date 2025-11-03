"""Calibration report classes."""

from typing import Dict, List, Optional, Any
from datetime import date

class CalibrationReport:
    """Calibration report."""

    def __init__(
        self,
        success: bool,
        iterations: int,
        final_error: float,
        parameters: Dict[str, float],
        quotes: List[Any],
        as_of: date,
    ) -> None:
        """Create a calibration report.

        Args:
            success: Whether calibration succeeded
            iterations: Number of iterations
            final_error: Final error value
            parameters: Calibrated parameters
            quotes: Input quotes
            as_of: Calibration date
        """
        ...

    @property
    def success(self) -> bool:
        """Whether calibration succeeded."""
        ...

    @property
    def iterations(self) -> int:
        """Number of iterations."""
        ...

    @property
    def final_error(self) -> float:
        """Final error value."""
        ...

    @property
    def parameters(self) -> Dict[str, float]:
        """Calibrated parameters."""
        ...

    @property
    def quotes(self) -> List[Any]:
        """Input quotes."""
        ...

    @property
    def as_of(self) -> date:
        """Calibration date."""
        ...

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
