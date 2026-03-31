"""Type stubs for standalone risk-metric functions."""

from __future__ import annotations

def cagr_from_periods(returns: list[float], ann_factor: float) -> float:
    """Compound annual growth rate from a return series using a period-based factor."""
    ...

def mean_return(
    returns: list[float], annualize: bool = True, ann_factor: float = 252.0
) -> float:
    """Mean return, optionally annualized."""
    ...

def volatility(
    returns: list[float], annualize: bool = True, ann_factor: float = 252.0
) -> float:
    """Volatility (standard deviation of returns), optionally annualized."""
    ...

def sharpe(
    ann_return: float, ann_vol: float, risk_free_rate: float = 0.0
) -> float:
    """Sharpe ratio from pre-computed annualized return and volatility."""
    ...

def sortino(
    returns: list[float], annualize: bool = True, ann_factor: float = 252.0
) -> float:
    """Sortino ratio (downside-risk-adjusted return)."""
    ...

def downside_deviation(
    returns: list[float],
    mar: float = 0.0,
    annualize: bool = False,
    ann_factor: float = 252.0,
) -> float:
    """Downside deviation below a minimum acceptable return."""
    ...

def value_at_risk(
    returns: list[float],
    confidence: float = 0.95,
    ann_factor: float | None = None,
) -> float:
    """Historical Value-at-Risk."""
    ...

def expected_shortfall(
    returns: list[float],
    confidence: float = 0.95,
    ann_factor: float | None = None,
) -> float:
    """Expected shortfall (CVaR)."""
    ...

def parametric_var(
    returns: list[float],
    confidence: float = 0.95,
    ann_factor: float | None = None,
) -> float:
    """Parametric (Gaussian) VaR."""
    ...

def cornish_fisher_var(
    returns: list[float],
    confidence: float = 0.95,
    ann_factor: float | None = None,
) -> float:
    """Cornish-Fisher adjusted VaR."""
    ...

def skewness(returns: list[float]) -> float:
    """Skewness of a return series."""
    ...

def kurtosis(returns: list[float]) -> float:
    """Excess kurtosis of a return series."""
    ...

def geometric_mean(returns: list[float]) -> float:
    """Geometric mean return per period."""
    ...

def omega_ratio(returns: list[float], threshold: float = 0.0) -> float:
    """Omega ratio: probability-weighted gain-to-loss ratio."""
    ...

def gain_to_pain(returns: list[float]) -> float:
    """Gain-to-pain ratio."""
    ...

def tail_ratio(returns: list[float], confidence: float = 0.95) -> float:
    """Tail ratio: upper quantile / |lower quantile|."""
    ...

def modified_sharpe(
    returns: list[float],
    risk_free_rate: float = 0.0,
    confidence: float = 0.95,
    ann_factor: float = 252.0,
) -> float:
    """Modified Sharpe ratio using Cornish-Fisher VaR."""
    ...
