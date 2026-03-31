"""Type stubs for standalone benchmark-relative analytics functions."""

from __future__ import annotations
from .performance import BetaResult, GreeksResult

def tracking_error(
    returns: list[float],
    benchmark_returns: list[float],
    annualize: bool = True,
    ann_factor: float = 252.0,
) -> float:
    """Tracking error between a portfolio and benchmark."""
    ...

def information_ratio(
    returns: list[float],
    benchmark_returns: list[float],
    annualize: bool = True,
    ann_factor: float = 252.0,
) -> float:
    """Information ratio: excess return / tracking error."""
    ...

def r_squared(returns: list[float], benchmark_returns: list[float]) -> float:
    """R-squared: correlation squared between portfolio and benchmark."""
    ...

def calc_beta(
    portfolio: list[float], benchmark_returns: list[float]
) -> BetaResult:
    """Beta with confidence interval via OLS regression."""
    ...

def greeks(
    returns: list[float],
    benchmark_returns: list[float],
    ann_factor: float = 252.0,
) -> GreeksResult:
    """Greeks: alpha, beta, and R-squared from OLS regression."""
    ...

def up_capture(returns: list[float], benchmark_returns: list[float]) -> float:
    """Up-market capture ratio."""
    ...

def down_capture(returns: list[float], benchmark_returns: list[float]) -> float:
    """Down-market capture ratio."""
    ...

def capture_ratio(returns: list[float], benchmark_returns: list[float]) -> float:
    """Capture ratio: up capture / down capture."""
    ...

def batting_average(
    returns: list[float], benchmark_returns: list[float]
) -> float:
    """Batting average: fraction of periods portfolio beats benchmark."""
    ...

def treynor(
    ann_return: float, risk_free_rate: float = 0.0, beta: float = 1.0
) -> float:
    """Treynor ratio: excess return per unit of systematic risk."""
    ...

def m_squared(
    ann_return: float,
    ann_vol: float,
    bench_vol: float,
    risk_free_rate: float = 0.0,
) -> float:
    """M-squared (Modigliani-Modigliani) measure."""
    ...
