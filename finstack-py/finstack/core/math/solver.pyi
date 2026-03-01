"""Numerical solver bindings.

Provides root finding algorithms including Newton's method
and Brent's method.
"""

from __future__ import annotations
from typing import Callable

class NewtonSolver:
    """Newton's method for root finding.

    NewtonSolver uses Newton-Raphson iteration to find roots of functions.
    It requires the function to be differentiable and uses finite differences
    to approximate derivatives when analytical derivatives are not available.

    Newton's method converges quadratically near the root but requires a
    good initial guess. Use BrentSolver for more robust root finding.

    Examples
    --------
    Find yield to maturity:

        >>> from finstack.core.math.solver import NewtonSolver
        >>> def price_error(yield_rate):
        ...     # Calculate bond price at yield_rate
        ...     calculated_price = 100.0 / (1 + yield_rate)  # Simple example
        ...     market_price = 95.0
        ...     return calculated_price - market_price
        >>> solver = NewtonSolver(tolerance=1e-6, max_iterations=100, fd_step=1e-6)
        >>> ytm = solver.solve(price_error, initial_guess=0.03)

    Notes
    -----
    - Requires good initial guess for convergence
    - Uses finite differences for derivative approximation
    - Faster convergence than Brent when near root
    - May fail if derivative is zero or function is not smooth

    See Also
    --------
    :class:`BrentSolver`: More robust root finding method
    """

    def __init__(
        self,
        tolerance: float | None = None,
        max_iterations: int | None = None,
        fd_step: float | None = None,
    ) -> None: ...
    @property
    def tolerance(self) -> float:
        """Get the convergence tolerance.

        Returns
        -------
        float
            Tolerance.
        """
        ...

    def set_tolerance(self, value: float) -> None:
        """Set the convergence tolerance.

        Parameters
        ----------
        value : float
            New tolerance.
        """
        ...

    @property
    def max_iterations(self) -> int:
        """Get the maximum iterations.

        Returns
        -------
        int
            Maximum iterations.
        """
        ...

    def set_max_iterations(self, value: int) -> None:
        """Set the maximum iterations.

        Parameters
        ----------
        value : int
            New maximum iterations.
        """
        ...

    @property
    def fd_step(self) -> float:
        """Get the finite difference step.

        Returns
        -------
        float
            FD step size.
        """
        ...

    def set_fd_step(self, value: float) -> None:
        """Set the finite difference step.

        Parameters
        ----------
        value : float
            New FD step size.
        """
        ...

    def solve(self, func: Callable[[float], float], initial_guess: float) -> float:
        """Solve for root using Newton's method.

        Finds the root of a function f(x) = 0 using Newton-Raphson iteration.
        The method uses the derivative (computed via finite differences) to
        converge to the root.

        Parameters
        ----------
        func : Callable[[float], float]
            Function to find root of. Must be differentiable and return
            f(x) where we want to find x such that f(x) = 0.
        initial_guess : float
            Initial guess for the root. Should be close to the actual root
            for best convergence.

        Returns
        -------
        float
            Root value where func(root) ≈ 0 (within tolerance).

        Raises
        ------
        ValueError
            If convergence fails (max_iterations exceeded, derivative is zero,
            or function is not converging).

        Examples
        --------
            >>> def f(x):
            ...     return x**2 - 4.0  # Find sqrt(4) = 2
            >>> solver = NewtonSolver(tolerance=1e-8)
            >>> root = solver.solve(f, initial_guess=1.5)
            >>> print(f"Root: {root:.6f}")
            Root: 2.000000
        """
        ...

    def __repr__(self) -> str: ...

class BrentSolver:
    """Brent's method for robust root finding.

    BrentSolver combines bisection, secant, and inverse quadratic interpolation
    to find roots of functions. It is more robust than Newton's method and
    doesn't require derivatives, making it suitable for functions that are
    not smooth or when derivatives are expensive to compute.

    Brent's method is guaranteed to converge if a bracket [a, b] exists where
    f(a) and f(b) have opposite signs. It automatically expands the bracket
    if needed.

    Examples
    --------
    Find yield to maturity (robust method):

        >>> from finstack.core.math.solver import BrentSolver
        >>> def price_error(yield_rate):
        ...     calculated_price = 100.0 / (1 + yield_rate)  # Simple example
        ...     market_price = 95.0
        ...     return calculated_price - market_price
        >>> solver = BrentSolver(
        ...     tolerance=1e-8,
        ...     max_iterations=100,
        ...     bracket_expansion=1.5,
        ...     initial_bracket_size=0.5,  # ±50% range
        ... )
        >>> ytm = solver.solve(price_error, initial_guess=0.03)

    Notes
    -----
    - More robust than Newton's method
    - Doesn't require derivatives
    - Automatically brackets the root
    - Slower convergence than Newton but more reliable

    See Also
    --------
    :class:`NewtonSolver`: Faster method when derivatives are available
    """

    def __init__(
        self,
        tolerance: float | None = None,
        max_iterations: int | None = None,
        bracket_expansion: float | None = None,
        initial_bracket_size: float | None = None,
    ) -> None: ...
    @property
    def tolerance(self) -> float:
        """Get the convergence tolerance.

        Returns
        -------
        float
            Tolerance.
        """
        ...

    def set_tolerance(self, value: float) -> None:
        """Set the convergence tolerance.

        Parameters
        ----------
        value : float
            New tolerance.
        """
        ...

    @property
    def max_iterations(self) -> int:
        """Get the maximum iterations.

        Returns
        -------
        int
            Maximum iterations.
        """
        ...

    def set_max_iterations(self, value: int) -> None:
        """Set the maximum iterations.

        Parameters
        ----------
        value : int
            New maximum iterations.
        """
        ...

    @property
    def bracket_expansion(self) -> float:
        """Get the bracket expansion factor.

        Returns
        -------
        float
            Bracket expansion factor.
        """
        ...

    def set_bracket_expansion(self, value: float) -> None:
        """Set the bracket expansion factor.

        Parameters
        ----------
        value : float
            New bracket expansion factor.
        """
        ...

    @property
    def initial_bracket_size(self) -> float | None:
        """Get the initial bracket size.

        Returns
        -------
        float or None
            Initial bracket size.
        """
        ...

    def set_initial_bracket_size(self, value: float | None) -> None:
        """Set the initial bracket size.

        Parameters
        ----------
        value : float or None
            New initial bracket size.
        """
        ...

    def solve(self, func: Callable[[float], float], initial_guess: float) -> float:
        """Solve for root using Brent's method.

        Finds the root of a function f(x) = 0 using Brent's algorithm, which
        combines bisection, secant, and inverse quadratic interpolation. The
        method automatically brackets the root and is robust to non-smooth functions.

        Parameters
        ----------
        func : Callable[[float], float]
            Function to find root of. Returns f(x) where we want to find x
            such that f(x) = 0. The function should be continuous.
        initial_guess : float
            Initial guess for the root. The solver will expand around this
            guess to find a bracket [a, b] where f(a) and f(b) have opposite signs.

        Returns
        -------
        float
            Root value where func(root) ≈ 0 (within tolerance).

        Raises
        ------
        ValueError
            If convergence fails (unable to bracket root, max_iterations exceeded,
            or function has no root in the searchable range).

        Examples
        --------
            >>> def f(x):
            ...     return x**3 - 8.0  # Find cube root of 8 = 2
            >>> solver = BrentSolver(tolerance=1e-8)
            >>> root = solver.solve(f, initial_guess=1.0)
            >>> print(f"Root: {root:.6f}")
            Root: 2.000000
        """
        ...

    def __repr__(self) -> str: ...
