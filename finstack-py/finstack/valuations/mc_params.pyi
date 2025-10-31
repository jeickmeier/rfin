"""Monte Carlo process parameters and metadata."""

from typing import Optional, Dict, List, Tuple

class ProcessParams:
    """Process parameters and metadata for Monte Carlo simulation."""
    
    @property
    def process_type(self) -> str:
        """Get the process type name."""
        ...
    
    @property
    def parameters(self) -> Dict[str, float]:
        """Get parameters as a dictionary."""
        ...
    
    @property
    def correlation(self) -> Optional[List[float]]:
        """Get correlation matrix as a flat list (row-major)."""
        ...
    
    @property
    def factor_names(self) -> List[str]:
        """Get factor names."""
        ...
    
    def get_param(self, key: str) -> Optional[float]:
        """Get a specific parameter by name."""
        ...
    
    def dim(self) -> Optional[int]:
        """Get the dimension (number of factors) from correlation matrix."""
        ...
    
    def correlation_matrix(self) -> Optional[List[List[float]]]:
        """
        Get correlation matrix as a 2D list (nested lists).
        
        Returns None if no correlation matrix is present.
        """
        ...
    
    def correlation_array(self) -> Optional[Tuple[List[float], Tuple[int, int]]]:
        """
        Get correlation matrix as a flat numpy-compatible list with shape info.
        
        Returns a tuple of (flat_data, shape) suitable for numpy.array(data).reshape(shape).
        Returns None if no correlation matrix is present.
        """
        ...

__all__ = ["ProcessParams"]

