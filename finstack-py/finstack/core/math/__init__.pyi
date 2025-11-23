"""Mathematical utilities from finstack-core.

This module aggregates bindings for:
- distributions: binomial probabilities, logarithms, and Beta sampling
- integration: Simpson/trapezoidal rules and Gauss-Legendre/Hermite quadrature
- solver: Newton and Brent root finders
- linalg: Cholesky decomposition and correlation helpers
- stats: Means, variances, covariances, realized variance
- special_functions: Normal CDF/PDF, Erf
- summation: Kahan, pairwise, and stable sums
- random: SimpleRng and Box-Muller transforms
"""

from . import distributions
from . import integration
from . import solver
from . import linalg
from . import stats
from . import special_functions
from . import summation
from . import random

__all__ = [
    "distributions",
    "integration",
    "solver",
    "linalg",
    "stats",
    "special_functions",
    "summation",
    "random",
]

