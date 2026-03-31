"""Type stubs for standalone return computation functions."""

from __future__ import annotations

def clean_returns(returns: list[float]) -> list[float]:
    """Clean a return series: replace infinities with NaN and strip trailing NaN values."""
    ...

def simple_returns(prices: list[float]) -> list[float]:
    """Simple (percentage-change) returns from a price series."""
    ...

def comp_sum(returns: list[float]) -> list[float]:
    """Cumulative compounded returns: ``(1 + r).cumprod() - 1``."""
    ...

def comp_total(returns: list[float]) -> float:
    """Total compounded return over the full series."""
    ...

def convert_to_prices(returns: list[float], base: float) -> list[float]:
    """Convert simple returns back to a price series."""
    ...

def rebase(prices: list[float], base: float) -> list[float]:
    """Rebase a price series so the first value equals ``base``."""
    ...

def excess_returns(
    returns: list[float],
    rf: list[float],
    nperiods: float | None = None,
) -> list[float]:
    """Excess returns: portfolio returns minus risk-free returns."""
    ...
