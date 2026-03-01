"""Multi-dimensional solvers for calibration problems."""

from __future__ import annotations

from typing import Callable, Sequence

class LmTerminationReason:
    """Termination reason for the Levenberg-Marquardt solver.

    Indicates why the solver stopped iterating. Compare the
    :attr:`LmStats.termination_reason` against the class attributes.

    Examples
    --------
        >>> from finstack.core.math.solver_multi import LmTerminationReason
        >>> reason = LmTerminationReason.CONVERGED_RESIDUAL_NORM
    """

    CONVERGED_RESIDUAL_NORM: LmTerminationReason
    """Residual norm fell below the configured tolerance."""
    CONVERGED_RELATIVE_REDUCTION: LmTerminationReason
    """Relative residual reduction fell below the configured tolerance."""
    CONVERGED_GRADIENT: LmTerminationReason
    """Gradient norm fell below the configured tolerance."""
    STEP_TOO_SMALL: LmTerminationReason
    """Parameter update step became smaller than ``min_step_size``."""
    MAX_ITERATIONS: LmTerminationReason
    """Solver exhausted the iteration budget."""
    NUMERICAL_FAILURE: LmTerminationReason
    """Solver encountered an unrecoverable numerical failure."""

    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...

class LmStats:
    """Solver statistics for diagnostics and monitoring.

    Provides detailed information about solver convergence behaviour
    including iteration counts, residual norms, and the termination reason.
    """

    @property
    def iterations(self) -> int:
        """Number of accepted LM iterations."""
        ...

    @property
    def residual_evals(self) -> int:
        """Total residual evaluations performed (including Jacobian probes)."""
        ...

    @property
    def jacobian_evals(self) -> int:
        """Total Jacobian evaluations performed."""
        ...

    @property
    def termination_reason(self) -> LmTerminationReason:
        """Reason why the solver terminated."""
        ...

    @property
    def final_residual_norm(self) -> float:
        """Final residual norm when termination occurred."""
        ...

    @property
    def final_step_norm(self) -> float:
        """Norm of the final accepted (or attempted) step."""
        ...

    @property
    def lambda_final(self) -> float:
        """Final damping parameter (lambda) at termination."""
        ...

    @property
    def lambda_bound_hits(self) -> int:
        """Number of times lambda hit the upper or lower bound."""
        ...

    def __repr__(self) -> str: ...

class LmSolution:
    """Solution vector plus solver statistics.

    Returned by :meth:`LevenbergMarquardtSolver.solve_system_with_stats`
    to provide both the solved parameters and diagnostic information.
    """

    @property
    def params(self) -> list[float]:
        """Solved parameter vector."""
        ...

    @property
    def stats(self) -> LmStats:
        """Detailed solver diagnostics."""
        ...

    def __repr__(self) -> str: ...

class LevenbergMarquardtSolver:
    """Damped least-squares solver for non-linear calibration."""

    def __init__(
        self,
        *,
        tolerance: float | None = ...,
        max_iterations: int | None = ...,
        lambda_init: float | None = ...,
        lambda_factor: float | None = ...,
        fd_step: float | None = ...,
        min_step_size: float | None = ...,
    ) -> None: ...
    @property
    def tolerance(self) -> float: ...
    @tolerance.setter
    def tolerance(self, value: float) -> None: ...
    @property
    def max_iterations(self) -> int: ...
    @max_iterations.setter
    def max_iterations(self, value: int) -> None: ...
    @property
    def lambda_init(self) -> float: ...
    @lambda_init.setter
    def lambda_init(self, value: float) -> None: ...
    @property
    def lambda_factor(self) -> float: ...
    @lambda_factor.setter
    def lambda_factor(self, value: float) -> None: ...
    @property
    def fd_step(self) -> float: ...
    @fd_step.setter
    def fd_step(self, value: float) -> None: ...
    @property
    def min_step_size(self) -> float: ...
    @min_step_size.setter
    def min_step_size(self, value: float) -> None: ...
    def minimize(
        self,
        objective: Callable[[Sequence[float]], float],
        initial: Sequence[float],
        bounds: Sequence[tuple[float, float]] | None = ...,
    ) -> list[float]: ...
    def solve_system(
        self,
        residuals: Callable[[Sequence[float]], Sequence[float]],
        initial: Sequence[float],
        n_residuals: int,
    ) -> list[float]:
        """Solve system of equations using Levenberg-Marquardt.

        Parameters
        ----------
        residuals : callable
            Function that takes params and returns residuals.
        initial : Sequence[float]
            Initial parameter guess.
        n_residuals : int
            Number of residuals (equations) in the system.

        Returns
        -------
        list[float]
            Parameter vector that minimizes ||residuals(params)||^2.
        """
        ...

    def solve_system_with_stats(
        self,
        residuals: Callable[[Sequence[float]], Sequence[float]],
        initial: Sequence[float],
        n_residuals: int,
    ) -> LmSolution:
        """Solve system of equations and return full diagnostics.

        Like :meth:`solve_system`, but returns an :class:`LmSolution`
        containing both the solved parameters and an :class:`LmStats`
        object with convergence diagnostics.

        Parameters
        ----------
        residuals : callable
            Function that takes params and returns residuals.
        initial : Sequence[float]
            Initial parameter guess.
        n_residuals : int
            Number of residuals (equations) in the system.

        Returns
        -------
        LmSolution
            Solution object with ``params`` and ``stats`` attributes.
        """
        ...

    def __repr__(self) -> str: ...

__all__ = [
    "LevenbergMarquardtSolver",
    "LmTerminationReason",
    "LmStats",
    "LmSolution",
]
