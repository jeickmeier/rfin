"""Validation classes for calibration."""

from typing import List, Optional, Any, Dict
from datetime import date

class ValidationResult:
    """Validation result."""

    def __init__(self, valid: bool, errors: List[str], warnings: List[str], as_of: date) -> None:
        """Create a validation result.

        Args:
            valid: Whether validation passed
            errors: List of errors
            warnings: List of warnings
            as_of: Validation date
        """
        ...

    @property
    def valid(self) -> bool:
        """Whether validation passed."""
        ...

    @property
    def errors(self) -> List[str]:
        """List of errors."""
        ...

    @property
    def warnings(self) -> List[str]:
        """List of warnings."""
        ...

    @property
    def as_of(self) -> date:
        """Validation date."""
        ...

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
