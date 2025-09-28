"""Demonstrate the finstack.core.math integration and solver bindings.

Run this script after building the extension (e.g., `uv run maturin develop`).
It prints a few numerical integration results and root-finding examples that
mirror the Rust core capabilities, giving analysts a quick feel for the Python
API surface.
"""

from __future__ import annotations

import math
from typing import Callable

import finstack

# ---------------------------------------------------------------------------
# Integration helpers
# ---------------------------------------------------------------------------

def integrate_standard_normal(func: Callable[[float], float]) -> float:
    """Evaluate E[f(X)] for X ~ N(0, 1) using Gauss-Hermite quadrature."""

    quad = finstack.core.math.integration.GaussHermiteQuadrature.order_7()
    return quad.integrate(func)

def gauss_legendre_demo() -> float:
    """Approximate ∫ cos(x) dx over [0, π/2] with Gauss-Legendre nodes."""

    return finstack.core.math.integration.gauss_legendre_integrate(
        math.cos,
        0.0,
        math.pi / 2.0,
        order=8,
    )

def adaptive_simpson_demo() -> float:
    """Integrate sin(10x)/(1+x^2) over [0, 1] with adaptive Simpson."""

    return finstack.core.math.integration.adaptive_simpson(
        lambda x: math.sin(10.0 * x) / (1.0 + x * x),
        0.0,
        1.0,
        tol=1e-8,
        max_depth=12,
    )

# ---------------------------------------------------------------------------
# Solver helpers
# ---------------------------------------------------------------------------

def newton_solve_sqrt2() -> float:
    """Find sqrt(2) by solving x^2 - 2 = 0 with Newton's method."""

    solver = finstack.core.math.solver.NewtonSolver(
        tolerance=1e-12, max_iterations=50, fd_step=1e-8
    )
    return solver.solve(lambda x: x * x - 2.0, initial_guess=1.0)

def brent_bisection_cos() -> float:
    """Locate a root of cos(x) - x near zero using Brent's method."""

    solver = finstack.core.math.solver.BrentSolver(
        tolerance=1e-12,
        max_iterations=100,
        bracket_expansion=2.0,
        initial_bracket_size=None,
    )
    return solver.solve(lambda x: math.cos(x) - x, initial_guess=0.5)


def hybrid_polynomial() -> float:
    """Use the hybrid solver to find a root of x^3 - x - 1 ≈ 0."""

    solver = finstack.core.math.solver.HybridSolver(
        tolerance=1e-12,
        max_iterations=100,
    )
    return solver.solve(lambda x: x * x * x - x - 1.0, initial_guess=1.0)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    print("=== finstack.core.math integration showcases ===")
    print("E[X^2] under N(0,1):", integrate_standard_normal(lambda x: x * x))
    print(
        "∫₀^{π/2} cos(x) dx (Gauss-Legendre order 8):",
        gauss_legendre_demo(),
    )
    print("Adaptive Simpson ∫ sin(10x)/(1+x²) dx over [0,1]:", adaptive_simpson_demo())

    print("\n=== finstack.core.math solver showcases ===")
    print("Newton root for x^2 - 2:", newton_solve_sqrt2())
    print("Brent root for cos(x) - x:", brent_bisection_cos())
    print("Hybrid root for x^3 - x - 1:", hybrid_polynomial())


if __name__ == "__main__":
    main()
