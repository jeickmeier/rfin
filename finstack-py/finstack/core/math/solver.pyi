"""Numerical solver bindings.

Provides root finding algorithms including Newton's method
and Brent's method.
"""

from typing import Callable, Optional


class NewtonSolver:
    """Newton's method for root finding.

    Parameters
    ----------
    tolerance : float, optional
        Convergence tolerance.
    max_iterations : int, optional
        Maximum number of iterations.
    fd_step : float, optional
        Finite difference step size.
    """

    def __init__(
        self,
        tolerance: Optional[float] = None,
        max_iterations: Optional[int] = None,
        fd_step: Optional[float] = None,
    ) -> None: ...

    @property
    def tolerance(self) -> float: ...
    """Get the convergence tolerance.

    Returns
    -------
    float
        Tolerance.
    """

    def set_tolerance(self, value: float) -> None: ...
    """Set the convergence tolerance.

    Parameters
    ----------
    value : float
        New tolerance.
    """

    @property
    def max_iterations(self) -> int: ...
    """Get the maximum iterations.

    Returns
    -------
    int
        Maximum iterations.
    """

    def set_max_iterations(self, value: int) -> None: ...
    """Set the maximum iterations.

    Parameters
    ----------
    value : int
        New maximum iterations.
    """

    @property
    def fd_step(self) -> float: ...
    """Get the finite difference step.

    Returns
    -------
    float
        FD step size.
    """

    def set_fd_step(self, value: float) -> None: ...
    """Set the finite difference step.

    Parameters
    ----------
    value : float
        New FD step size.
    """

    def solve(self, func: Callable[[float], float], initial_guess: float) -> float: ...
    """Solve for root using Newton's method.

    Parameters
    ----------
    func : Callable[[float], float]
        Function to find root of.
    initial_guess : float
        Initial guess.

    Returns
    -------
    float
        Root value.

    Raises
    ------
    ValueError
        If convergence fails.
    """

    def __repr__(self) -> str: ...


class BrentSolver:
    """Brent's method for root finding.

    Parameters
    ----------
    tolerance : float, optional
        Convergence tolerance.
    max_iterations : int, optional
        Maximum number of iterations.
    bracket_expansion : float, optional
        Bracket expansion factor.
    initial_bracket_size : float, optional
        Initial bracket size.
    """

    def __init__(
        self,
        tolerance: Optional[float] = None,
        max_iterations: Optional[int] = None,
        bracket_expansion: Optional[float] = None,
        initial_bracket_size: Optional[float] = None,
    ) -> None: ...

    @property
    def tolerance(self) -> float: ...
    """Get the convergence tolerance.

    Returns
    -------
    float
        Tolerance.
    """

    def set_tolerance(self, value: float) -> None: ...
    """Set the convergence tolerance.

    Parameters
    ----------
    value : float
        New tolerance.
    """

    @property
    def max_iterations(self) -> int: ...
    """Get the maximum iterations.

    Returns
    -------
    int
        Maximum iterations.
    """

    def set_max_iterations(self, value: int) -> None: ...
    """Set the maximum iterations.

    Parameters
    ----------
    value : int
        New maximum iterations.
    """

    @property
    def bracket_expansion(self) -> float: ...
    """Get the bracket expansion factor.

    Returns
    -------
    float
        Bracket expansion factor.
    """

    def set_bracket_expansion(self, value: float) -> None: ...
    """Set the bracket expansion factor.

    Parameters
    ----------
    value : float
        New bracket expansion factor.
    """

    @property
    def initial_bracket_size(self) -> Optional[float]: ...
    """Get the initial bracket size.

    Returns
    -------
    float or None
        Initial bracket size.
    """

    def set_initial_bracket_size(self, value: Optional[float]) -> None: ...
    """Set the initial bracket size.

    Parameters
    ----------
    value : float or None
        New initial bracket size.
    """

    def solve(self, func: Callable[[float], float], initial_guess: float) -> float: ...
    """Solve for root using Brent's method.

    Parameters
    ----------
    func : Callable[[float], float]
        Function to find root of.
    initial_guess : float
        Initial guess.

    Returns
    -------
    float
        Root value.

    Raises
    ------
    ValueError
        If convergence fails.
    """

    def __repr__(self) -> str: ...

