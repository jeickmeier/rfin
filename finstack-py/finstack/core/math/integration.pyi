# flake8: noqa: PYI021
from collections.abc import Callable
from typing import Self

class GaussHermiteQuadrature:
    @classmethod
    def order_5(cls) -> Self: ...

    @classmethod
    def order_7(cls) -> Self: ...

    @classmethod
    def order_10(cls) -> Self: ...

    @property
    def order(self) -> int: ...

    @property
    def points(self) -> list[float]: ...

    @property
    def weights(self) -> list[float]: ...

    def integrate(self, func: Callable[[float], float]) -> float: ...

    def integrate_adaptive(self, func: Callable[[float], float], tolerance: float) -> float: ...

    def __repr__(self) -> str: ...


def simpson_rule(func: Callable[[float], float], a: float, b: float, intervals: int) -> float: ...

def adaptive_simpson(func: Callable[[float], float], a: float, b: float, tol: float, max_depth: int) -> float: ...

def adaptive_quadrature(func: Callable[[float], float], a: float, b: float, tol: float, max_depth: int) -> float: ...

def gauss_legendre_integrate(func: Callable[[float], float], a: float, b: float, order: int) -> float: ...

def gauss_legendre_integrate_composite(
    func: Callable[[float], float],
    a: float,
    b: float,
    order: int,
    panels: int,
) -> float: ...

def gauss_legendre_integrate_adaptive(
    func: Callable[[float], float],
    a: float,
    b: float,
    order: int,
    tol: float,
    max_depth: int,
) -> float: ...

def trapezoidal_rule(func: Callable[[float], float], a: float, b: float, intervals: int) -> float: ...

__all__ = [
    "GaussHermiteQuadrature",
    "simpson_rule",
    "adaptive_simpson",
    "adaptive_quadrature",
    "gauss_legendre_integrate",
    "gauss_legendre_integrate_composite",
    "gauss_legendre_integrate_adaptive",
    "trapezoidal_rule",
]
