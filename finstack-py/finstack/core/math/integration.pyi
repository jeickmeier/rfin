# flake8: noqa: PYI021
def adaptive_quadrature(func, a, b, tol, max_depth):
    """
    Adaptive Simpson quadrature (alias of `adaptive_simpson`).

    Args:
        func (Callable[[float], float]): Callable evaluated at requested points.
        a (float): Lower bound of the integration interval.
        b (float): Upper bound of the integration interval.
        tol (float): Target absolute error tolerance.
        max_depth (int): Maximum recursion depth for refinement.

    Returns:
        float: Integral estimate identical to `adaptive_simpson`.
    """

def adaptive_simpson(func, a, b, tol, max_depth):
    """
    Adaptive Simpson integration with automatic refinement.

    Args:
        func (Callable[[float], float]): Callable evaluated at requested points.
        a (float): Lower bound of the integration interval.
        b (float): Upper bound of the integration interval.
        tol (float): Target absolute error tolerance.
        max_depth (int): Maximum recursion depth controlling refinement.

    Returns:
        float: Integral estimate satisfying the tolerance when possible.
    """

def gauss_legendre_integrate(func, a, b, order):
    """
    Gauss-Legendre quadrature on ``[a, b]`` with fixed order.

    Args:
        func (Callable[[float], float]): Function evaluated at node locations.
        a (float): Lower integration bound.
        b (float): Upper integration bound.
        order (int): Supported quadrature order (2, 4, 8, or 16).

    Returns:
        float: Integral approximation over ``[a, b]``.
    """

def gauss_legendre_integrate_adaptive(func, a, b, order, tol, max_depth):
    """
    Adaptive Gauss-Legendre quadrature with panel refinement.

    Args:
        func (Callable[[float], float]): Function to integrate.
        a (float): Lower bound of the integration domain.
        b (float): Upper bound of the integration domain.
        order (int): Base quadrature order (2, 4, 8, or 16).
        tol (float): Error tolerance governing panel refinement.
        max_depth (int): Maximum number of recursive refinements.

    Returns:
        float: Integral approximation with adaptive panel splitting.
    """

def gauss_legendre_integrate_composite(func, a, b, order, panels):
    """
    Composite Gauss-Legendre quadrature with multiple panels.

    Args:
        func (Callable[[float], float]): Function evaluated for each sub-interval.
        a (float): Lower bound.
        b (float): Upper bound.
        order (int): Individual panel quadrature order.
        panels (int): Number of sub-intervals to tile across ``[a, b]``.

    Returns:
        float: Integrated value across the full interval.
    """

def simpson_rule(func, a, b, intervals):
    """
    Simpson's composite rule for integrating a callable on ``[a, b]``.

    Args:
        func (Callable[[float], float]): Function to evaluate at grid points.
        a (float): Lower integration bound.
        b (float): Upper integration bound.
        intervals (int): Even number of sub-intervals used by Simpson's rule.

    Returns:
        float: Integral estimate across ``[a, b]``.

    Raises:
        ValueError: If ``intervals`` is zero or odd.
    """

def trapezoidal_rule(func, a, b, intervals):
    """
    Trapezoidal rule for integrating a callable on ``[a, b]``.

    Args:
        func (Callable[[float], float]): Function evaluated at grid points.
        a (float): Lower bound of the integration interval.
        b (float): Upper bound of the integration interval.
        intervals (int): Number of sub-intervals to apply.

    Returns:
        float: Integral approximation from the trapezoidal rule.
    """
