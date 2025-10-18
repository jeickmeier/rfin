"""Mathematical utilities from finstack-core (distributions, integration, solvers).

This module aggregates bindings for common mathematical routines:
- distributions: binomial probabilities and related logarithms
- integration: Simpson/trapezoidal rules and Gauss-Legendre/Hermite quadrature
- solver: Newton, Brent, and a hybrid strategy for root finding
All functions accept Python callables where appropriate and return floats.
"""

from . import distributions
from . import integration
from . import solver

__all__ = [
    "distributions",
    "integration",
    "solver",
]
