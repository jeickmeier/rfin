"""Mathematical utilities from finstack-core.

This module aggregates bindings for:
- distributions: binomial probabilities, logarithms, and Beta sampling
- integration: Simpson/trapezoidal rules and Gauss-Legendre/Hermite quadrature
- solver: Newton and Brent root finders
- solver_multi: Levenberg-Marquardt calibration helpers
- linalg: Cholesky decomposition and correlation helpers
- stats: Means, variances, covariances, realized variance
- special_functions: Normal CDF/PDF, Erf
- summation: Kahan, pairwise, and stable sums
- random: SimpleRng and Box-Muller transforms
- interp: interpolation and extrapolation styles
"""

from . import distributions
from . import integration
from . import interp
from . import linalg
from . import random
from . import solver
from . import solver_multi
from . import special_functions
from . import stats
from . import summation

__all__ = [
    "distributions",
    "integration",
    "interp",
    "linalg",
    "random",
    "solver",
    "solver_multi",
    "special_functions",
    "stats",
    "summation",
]
