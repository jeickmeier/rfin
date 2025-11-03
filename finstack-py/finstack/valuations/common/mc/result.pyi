"""Monte Carlo result wrapper with optional path data."""

from typing import Optional, Tuple
from finstack.core import Money
from finstack.valuations.mc_paths import PathDataset

class MonteCarloResult:
    """Monte Carlo result with optional path data."""

    @property
    def estimate(self) -> Money:
        """Get the statistical estimate (mean value)."""
        ...

    @property
    def stderr(self) -> float:
        """Get the standard error."""
        ...

    @property
    def ci_95(self) -> Tuple[Money, Money]:
        """Get the 95% confidence interval as a tuple (lower, upper)."""
        ...

    @property
    def num_paths(self) -> int:
        """Get the number of paths used for the estimate."""
        ...

    @property
    def paths(self) -> Optional[PathDataset]:
        """Get the captured paths dataset (if available)."""
        ...

    def has_paths(self) -> bool:
        """Check if paths were captured."""
        ...

    def num_captured_paths(self) -> int:
        """Get the number of captured paths."""
        ...

    def mean(self) -> Money:
        """Get just the mean estimate as Money."""
        ...

    def relative_stderr(self) -> float:
        """Get the relative standard error (stderr / mean)."""
        ...

__all__ = ["MonteCarloResult"]
