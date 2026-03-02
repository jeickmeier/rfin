"""Monte Carlo estimation types."""

from __future__ import annotations

class Estimate:
    """Monte Carlo estimation result.

    Contains point estimate, uncertainty quantification, and metadata
    about the simulation run.
    """

    def __init__(
        self,
        mean: float,
        stderr: float,
        ci_95_lower: float,
        ci_95_upper: float,
        num_paths: int,
    ) -> None:
        """Create a new estimate.

        Args:
            mean: Point estimate (mean)
            stderr: Standard error of the mean
            ci_95_lower: Lower bound of 95% confidence interval
            ci_95_upper: Upper bound of 95% confidence interval
            num_paths: Number of paths simulated
        """
        ...

    @property
    def mean(self) -> float:
        """Point estimate (mean)."""
        ...

    @property
    def stderr(self) -> float:
        """Standard error of the mean."""
        ...

    @property
    def ci_95(self) -> tuple[float, float]:
        """95% confidence interval as (lower, upper)."""
        ...

    @property
    def num_paths(self) -> int:
        """Number of paths simulated."""
        ...

    @property
    def std_dev(self) -> float | None:
        """Sample standard deviation (if available)."""
        ...

    @property
    def median(self) -> float | None:
        """Median value (if available)."""
        ...

    @property
    def percentile_25(self) -> float | None:
        """25th percentile (if available)."""
        ...

    @property
    def percentile_75(self) -> float | None:
        """75th percentile (if available)."""
        ...

    @property
    def min(self) -> float | None:
        """Minimum value (if available)."""
        ...

    @property
    def max(self) -> float | None:
        """Maximum value (if available)."""
        ...

    def relative_stderr(self) -> float:
        """Relative standard error (stderr / |mean|)."""
        ...

    def cv(self) -> float | None:
        """Coefficient of variation (std_dev / |mean|)."""
        ...

    def ci_half_width(self) -> float:
        """Half-width of the 95% confidence interval."""
        ...

    def iqr(self) -> float | None:
        """Interquartile range (IQR) if percentiles are available."""
        ...

    def range(self) -> float | None:
        """Range (max - min) if available."""
        ...

class ConvergenceDiagnostics:
    """Convergence diagnostics for Monte Carlo simulation."""

    def __init__(self) -> None:
        """Create empty diagnostics."""
        ...

    @property
    def stderr_decay_rate(self) -> float | None:
        """Stderr decay rate (should be ~-0.5 for standard MC)."""
        ...

    @property
    def effective_sample_size(self) -> int | None:
        """Effective sample size (for weighted samples)."""
        ...

    @property
    def variance_reduction_factor(self) -> float | None:
        """Variance reduction factor (vs. baseline)."""
        ...

__all__ = ["Estimate", "ConvergenceDiagnostics"]
