'"""Multi-dimensional solvers for calibration problems."""'

from __future__ import annotations

from typing import Callable, Iterable, Sequence

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
    ) -> list[float]: ...

__all__ = ["LevenbergMarquardtSolver"]
