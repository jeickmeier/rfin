"""Standalone Monte Carlo path generation for visualization and analysis."""

from typing import Optional, Dict, List
from finstack.valuations.mc_paths import PathDataset

class MonteCarloPathGenerator:
    """
    Standalone Monte Carlo path generator.

    This class generates Monte Carlo paths without pricing, useful for:
    - Pure process visualization
    - Process validation
    - Understanding stochastic dynamics
    - Educational purposes
    """

    def __init__(self) -> None: ...
    def generate_gbm_paths(
        self,
        initial_spot: float,
        r: float,
        q: float,
        sigma: float,
        time_to_maturity: float,
        num_steps: int,
        num_paths: int,
        capture_mode: str = "all",
        sample_count: Optional[int] = None,
        seed: int = 42,
    ) -> PathDataset:
        """
        Generate paths for a Geometric Brownian Motion process.

        Args:
            initial_spot: Initial spot price
            r: Risk-free rate (annual)
            q: Dividend/foreign rate (annual)
            sigma: Volatility (annual)
            time_to_maturity: Time horizon in years
            num_steps: Number of time steps
            num_paths: Total number of paths to simulate
            capture_mode: 'all' to capture all paths, or 'sample' with count
            sample_count: Number of paths to capture (if mode='sample')
            seed: Random seed for reproducibility

        Returns:
            PathDataset with generated paths

        Example:
            >>> generator = MonteCarloPathGenerator()
            >>> paths = generator.generate_gbm_paths(
            ...     initial_spot=100.0,
            ...     r=0.05,
            ...     q=0.02,
            ...     sigma=0.2,
            ...     time_to_maturity=1.0,
            ...     num_steps=252,
            ...     num_paths=1000,
            ...     capture_mode="sample",
            ...     sample_count=100,
            ...     seed=42,
            ... )
            >>> print(f"Generated {paths.num_captured()} paths")
        """
        ...

    def generate_paths(
        self,
        process_type: str,
        process_params: Dict[str, float],
        initial_state: List[float],
        time_to_maturity: float,
        num_steps: int,
        num_paths: int,
        capture_mode: str = "all",
        sample_count: Optional[int] = None,
        seed: int = 42,
    ) -> PathDataset:
        """
        Generate paths with custom parameters (advanced).

        This is a lower-level interface for advanced users who want full control.

        Args:
            process_type: Currently only 'gbm' supported
            process_params: Dictionary of process parameters
            initial_state: List of initial state values
            time_to_maturity: Time horizon in years
            num_steps: Number of time steps
            num_paths: Total number of paths to simulate
            capture_mode: 'all' or 'sample'
            sample_count: Number of paths to capture (if mode='sample')
            seed: Random seed

        Returns:
            PathDataset with generated paths

        Example:
            >>> generator = MonteCarloPathGenerator()
            >>> paths = generator.generate_paths(
            ...     process_type="gbm",
            ...     process_params={"r": 0.05, "q": 0.02, "sigma": 0.2},
            ...     initial_state=[100.0],
            ...     time_to_maturity=1.0,
            ...     num_steps=252,
            ...     num_paths=1000,
            ... )
        """
        ...

__all__ = ["MonteCarloPathGenerator"]
