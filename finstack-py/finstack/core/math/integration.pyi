"""Numerical integration bindings.

Provides various numerical integration methods including
Simpson's rule, Gauss-Legendre quadrature, and adaptive methods.
"""

from typing import Callable

class GaussHermiteQuadrature:
    """Gauss-Hermite quadrature for infinite integrals.

    Provides high-order quadrature rules for integrals of the form:
    ∫ f(x) * exp(-x²) dx from -∞ to +∞
    """

    @classmethod
    def order_5(cls) -> GaussHermiteQuadrature: ...
    """Create 5th order Gauss-Hermite quadrature."""

    @classmethod
    def order_7(cls) -> GaussHermiteQuadrature: ...
    """Create 7th order Gauss-Hermite quadrature."""

    @classmethod
    def order_10(cls) -> GaussHermiteQuadrature: ...
    """Create 10th order Gauss-Hermite quadrature."""

    @property
    def order(self) -> int: ...
    """Get the quadrature order.
    
    Returns
    -------
    int
        Quadrature order.
    """

    @property
    def points(self) -> List[float]: ...
    """Get the quadrature points.
    
    Returns
    -------
    List[float]
        Quadrature points.
    """

    @property
    def weights(self) -> List[float]: ...
    """Get the quadrature weights.
    
    Returns
    -------
    List[float]
        Quadrature weights.
    """

    def integrate(self, func: Callable[[float], float]) -> float: ...
    """Integrate a function using Gauss-Hermite quadrature.
    
    Parameters
    ----------
    func : Callable[[float], float]
        Function to integrate.
        
    Returns
    -------
    float
        Integral value.
    """

    def integrate_adaptive(self, func: Callable[[float], float], tolerance: float) -> float: ...
    """Integrate with adaptive tolerance.
    
    Parameters
    ----------
    func : Callable[[float], float]
        Function to integrate.
    tolerance : float
        Integration tolerance.
        
    Returns
    -------
    float
        Integral value.
    """

    def __repr__(self) -> str: ...

def simpson_rule(func: Callable[[float], float], a: float, b: float, intervals: int) -> float: ...

"""Simpson's rule integration.

Parameters
----------
func : Callable[[float], float]
    Function to integrate.
a : float
    Lower bound.
b : float
    Upper bound.
intervals : int
    Number of intervals.

Returns
-------
float
    Integral value.
"""

def adaptive_simpson(
    func: Callable[[float], float],
    a: float,
    b: float,
    tol: float,
    max_depth: int,
) -> float: ...

"""Adaptive Simpson's rule integration.

Parameters
----------
func : Callable[[float], float]
    Function to integrate.
a : float
    Lower bound.
b : float
    Upper bound.
tol : float
    Tolerance.
max_depth : int
    Maximum recursion depth.

Returns
-------
float
    Integral value.
"""

"""Gauss-Legendre quadrature integration.

Parameters
----------
func : Callable[[float], float]
    Function to integrate.
a : float
    Lower bound.
b : float
    Upper bound.
order : int
    Quadrature order.

Returns
-------
float
    Integral value.
"""

def gauss_legendre_integrate_composite(
    func: Callable[[float], float],
    a: float,
    b: float,
    order: int,
    panels: int,
) -> float: ...

"""Composite Gauss-Legendre quadrature integration.

Parameters
----------
func : Callable[[float], float]
    Function to integrate.
a : float
    Lower bound.
b : float
    Upper bound.
order : int
    Quadrature order.
panels : int
    Number of panels.

Returns
-------
float
    Integral value.
"""

def gauss_legendre_integrate_adaptive(
    func: Callable[[float], float],
    a: float,
    b: float,
    order: int,
    tol: float,
    max_depth: int,
) -> float: ...

"""Adaptive Gauss-Legendre quadrature integration.

Parameters
----------
func : Callable[[float], float]
    Function to integrate.
a : float
    Lower bound.
b : float
    Upper bound.
order : int
    Quadrature order.
tol : float
    Tolerance.
max_depth : int
    Maximum recursion depth.

Returns
-------
float
    Integral value.
"""

def trapezoidal_rule(
    func: Callable[[float], float],
    a: float,
    b: float,
    intervals: int,
) -> float: ...

"""Trapezoidal rule integration.

Parameters
----------
func : Callable[[float], float]
    Function to integrate.
a : float
    Lower bound.
b : float
    Upper bound.
intervals : int
    Number of intervals.

Returns
-------
float
    Integral value.
"""
