"""Type stubs for standalone drawdown functions."""

from __future__ import annotations

def to_drawdown_series(returns: list[float]) -> list[float]:
    """Compute a drawdown series from simple returns."""
    ...

def max_drawdown(drawdown: list[float]) -> float:
    """Maximum drawdown depth from a pre-computed drawdown series."""
    ...

def max_drawdown_from_returns(returns: list[float]) -> float:
    """Maximum drawdown computed directly from a returns series."""
    ...

def average_drawdown(drawdown: list[float]) -> float:
    """Average drawdown depth across all periods."""
    ...

def calmar(cagr_val: float, max_dd: float) -> float:
    """Calmar ratio: CAGR / |max drawdown|."""
    ...

def calmar_from_returns(returns: list[float], ann_factor: float) -> float:
    """Calmar ratio computed directly from a returns series."""
    ...

def pain_index(drawdown: list[float]) -> float:
    """Pain index: mean absolute drawdown."""
    ...

def ulcer_index(drawdown: list[float]) -> float:
    """Ulcer index: root-mean-square of drawdown depths."""
    ...

def cdar(drawdown: list[float], confidence: float) -> float:
    """Conditional Drawdown at Risk (CDaR)."""
    ...

def recovery_factor(total_return: float, max_dd: float) -> float:
    """Recovery factor: total return / |max drawdown|."""
    ...

def recovery_factor_from_returns(returns: list[float]) -> float:
    """Recovery factor computed directly from a returns series."""
    ...

def martin_ratio(cagr_val: float, ulcer: float) -> float:
    """Martin ratio: CAGR / Ulcer Index."""
    ...

def martin_ratio_from_returns(returns: list[float], ann_factor: float) -> float:
    """Martin ratio computed directly from a returns series."""
    ...

def sterling_ratio(cagr_val: float, avg_dd: float, risk_free_rate: float) -> float:
    """Sterling ratio: (CAGR - Rf) / |avg drawdown|."""
    ...

def sterling_ratio_from_returns(
    returns: list[float], ann_factor: float, risk_free_rate: float
) -> float:
    """Sterling ratio computed directly from a returns series."""
    ...

def burke_ratio(
    cagr_val: float, dd_episodes: list[float], risk_free_rate: float
) -> float:
    """Burke ratio: (CAGR - Rf) / RMS of worst drawdowns."""
    ...

def pain_ratio(cagr_val: float, pain: float, risk_free_rate: float) -> float:
    """Pain ratio: (CAGR - Rf) / Pain Index."""
    ...

def pain_ratio_from_returns(
    returns: list[float], ann_factor: float, risk_free_rate: float
) -> float:
    """Pain ratio computed directly from a returns series."""
    ...
