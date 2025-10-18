"""SABR calibrator classes."""

from typing import List, Optional, Any
from .config import CalibrationConfig
from .quote import Quote
from .report import CalibrationReport

class SabrCalibrator:
    """SABR calibrator."""
    
    def __init__(self, config: Optional[CalibrationConfig] = None) -> None:
        """Create a SABR calibrator.
        
        Args:
            config: Optional calibration configuration
        """
        ...
    
    def calibrate(
        self, 
        quotes: List[Quote], 
        instruments: List[Any]
    ) -> CalibrationReport:
        """Calibrate using quotes and instruments.
        
        Args:
            quotes: List of quotes
            instruments: List of instruments
            
        Returns:
            CalibrationReport: Calibration results
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
