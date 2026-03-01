"""Time grid for Monte Carlo simulation."""

from __future__ import annotations

class TimeGrid:
    """Time grid for Monte Carlo simulation.

    Defines the discretization points in time from t=0 to t=T.
    Supports both uniform grids (equal spacing) and custom grids
    (irregular time points for finer resolution near important dates).
    """

    @staticmethod
    def uniform(t_max: float, num_steps: int) -> TimeGrid:
        """Create a uniform time grid from 0 to t_max with num_steps steps.

        Args:
            t_max: Final time in years (must be > 0)
            num_steps: Number of time steps (must be > 0)

        Returns:
            TimeGrid with equally spaced time points

        Example:
            Create a 1-year daily grid::

                from finstack.valuations.common.mc import TimeGrid

                grid = TimeGrid.uniform(1.0, 252)
                print(grid.num_steps)  # 252
                print(grid.t_max)     # 1.0
        """
        ...

    @staticmethod
    def from_times(times: list[float]) -> TimeGrid:
        """Create a custom time grid from explicit time points.

        Args:
            times: Monotonically increasing time points starting at 0.0

        Returns:
            TimeGrid with the specified time points

        Example:
            Create a quarterly grid::

                from finstack.valuations.common.mc import TimeGrid

                grid = TimeGrid.from_times([0.0, 0.25, 0.5, 0.75, 1.0])
                print(grid.num_steps)  # 4
        """
        ...

    @property
    def num_steps(self) -> int:
        """Number of time steps."""
        ...

    @property
    def t_max(self) -> float:
        """Total time span."""
        ...

    @property
    def times(self) -> list[float]:
        """All time points as a list."""
        ...

    @property
    def dts(self) -> list[float]:
        """All time step sizes as a list."""
        ...

    def time_at(self, step: int) -> float:
        """Get time at a specific step index."""
        ...

    def dt_at(self, step: int) -> float:
        """Get time step size at a specific step index."""
        ...

    def is_uniform(self) -> bool:
        """Check if grid is uniform (all dts equal within tolerance)."""
        ...

    def __len__(self) -> int: ...

__all__ = ["TimeGrid"]
