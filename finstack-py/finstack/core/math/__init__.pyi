# flake8: noqa: PYI021
from . import distributions, integration, solver

binomial_probability = distributions.binomial_probability
log_binomial_coefficient = distributions.log_binomial_coefficient
log_factorial = distributions.log_factorial

GaussHermiteQuadrature = integration.GaussHermiteQuadrature
simpson_rule = integration.simpson_rule
adaptive_simpson = integration.adaptive_simpson
adaptive_quadrature = integration.adaptive_quadrature
gauss_legendre_integrate = integration.gauss_legendre_integrate
gauss_legendre_integrate_composite = integration.gauss_legendre_integrate_composite
gauss_legendre_integrate_adaptive = integration.gauss_legendre_integrate_adaptive
trapezoidal_rule = integration.trapezoidal_rule

NewtonSolver = solver.NewtonSolver
BrentSolver = solver.BrentSolver
HybridSolver = solver.HybridSolver

__all__ = [
    "distributions",
    "integration",
    "solver",
    "binomial_probability",
    "log_binomial_coefficient",
    "log_factorial",
    "GaussHermiteQuadrature",
    "simpson_rule",
    "adaptive_simpson",
    "adaptive_quadrature",
    "gauss_legendre_integrate",
    "gauss_legendre_integrate_composite",
    "gauss_legendre_integrate_adaptive",
    "trapezoidal_rule",
    "NewtonSolver",
    "BrentSolver",
    "HybridSolver",
]
