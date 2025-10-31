"""Monte Carlo path data structures for visualization and analysis."""

from typing import Optional, Dict, List, Tuple

class PathPoint:
    """A single point along a Monte Carlo path."""
    
    @property
    def step(self) -> int:
        """Get the step index."""
        ...
    
    @property
    def time(self) -> float:
        """Get the time in years."""
        ...
    
    @property
    def state_vars(self) -> Dict[str, float]:
        """Get state variables as a dictionary."""
        ...
    
    @property
    def payoff_value(self) -> Optional[float]:
        """Get the payoff value at this point (if captured)."""
        ...
    
    def get_var(self, key: str) -> Optional[float]:
        """Get a specific state variable by name."""
        ...
    
    def spot(self) -> Optional[float]:
        """Get the spot price (convenience method)."""
        ...
    
    def variance(self) -> Optional[float]:
        """Get the variance (convenience method)."""
        ...
    
    def short_rate(self) -> Optional[float]:
        """Get the short rate (convenience method)."""
        ...

class SimulatedPath:
    """A complete simulated Monte Carlo path."""
    
    @property
    def path_id(self) -> int:
        """Get the path ID."""
        ...
    
    @property
    def points(self) -> List[PathPoint]:
        """Get all points along the path."""
        ...
    
    @property
    def final_value(self) -> float:
        """Get the final discounted payoff value."""
        ...
    
    def num_steps(self) -> int:
        """Get the number of time steps."""
        ...
    
    def point(self, step: int) -> Optional[PathPoint]:
        """Get a specific point by step index."""
        ...
    
    def initial_point(self) -> Optional[PathPoint]:
        """Get the initial point."""
        ...
    
    def terminal_point(self) -> Optional[PathPoint]:
        """Get the terminal point."""
        ...
    
    def __len__(self) -> int: ...

class PathDataset:
    """Collection of simulated paths with metadata."""
    
    @property
    def paths(self) -> List[SimulatedPath]:
        """Get all captured paths."""
        ...
    
    @property
    def num_paths_total(self) -> int:
        """Get the total number of paths in the simulation."""
        ...
    
    @property
    def sampling_method(self) -> str:
        """Get the sampling method used."""
        ...
    
    def num_captured(self) -> int:
        """Get the number of captured paths."""
        ...
    
    def path(self, index: int) -> Optional[SimulatedPath]:
        """Get a specific path by index."""
        ...
    
    def is_complete(self) -> bool:
        """Check if all paths were captured."""
        ...
    
    def sampling_ratio(self) -> float:
        """Get the sampling ratio (captured / total)."""
        ...
    
    def state_var_keys(self) -> List[str]:
        """Get all state variable keys present in the dataset."""
        ...
    
    def to_dict(self) -> Dict[str, List]:
        """
        Convert to a long-format dictionary suitable for pandas DataFrame.
        
        Returns a dictionary with columns:
        - path_id: Path identifier
        - step: Time step index
        - time: Time in years
        - final_value: Final discounted payoff for this path
        - One column per state variable (e.g., 'spot', 'variance')
        - payoff_value: Optional payoff at each step (if captured)
        """
        ...
    
    def to_wide_dict(self, state_var: str) -> Dict[str, List]:
        """
        Convert to a wide-format dictionary (paths as columns).
        
        Args:
            state_var: Name of the state variable to extract (e.g., 'spot')
        
        Returns a dictionary with:
        - time: Time points (shared across all paths)
        - step: Step indices
        - path_0, path_1, ...: State variable values for each path
        """
        ...
    
    def __len__(self) -> int: ...

__all__ = ["PathPoint", "SimulatedPath", "PathDataset"]

