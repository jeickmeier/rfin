"""Polars expression plugins for finstack analytics.

Thin Python wrappers that register Rust-compiled expression plugins with Polars.
Each function returns a ``pl.Expr`` that can be used inside ``.select()``,
``.with_columns()``, ``.filter()``, etc.

Example::

    import polars as pl
    from finstack.core.analytics.expr import sharpe, simple_returns

    returns = prices.select(simple_returns("AAPL").alias("aapl_ret"))
    risk = returns.select(sharpe("aapl_ret", freq="daily").alias("sharpe"))
"""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import polars as pl
from polars.plugins import register_plugin_function

if TYPE_CHECKING:
    from polars._typing import IntoExpr

_PLUGIN_PATH = Path(__file__).parent.parent.parent

# ── Helpers ──


def _to_expr(expr: IntoExpr) -> pl.Expr:
    if isinstance(expr, str):
        return pl.col(expr)
    if isinstance(expr, pl.Expr):
        return expr
    raise TypeError(f"expected str or pl.Expr, got {type(expr).__name__}")


# ── Tier 1: Scalar risk metrics ──


def sharpe(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    risk_free: float = 0.0,
) -> pl.Expr:
    """Sharpe ratio (annualized return - risk-free) / annualized volatility."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_sharpe",
        args=_to_expr(expr),
        kwargs={"freq": freq, "risk_free": risk_free},
        is_elementwise=False,
        returns_scalar=True,
    )


def sortino(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Sortino ratio: penalises only downside volatility."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_sortino",
        args=_to_expr(expr),
        kwargs={"freq": freq},
        is_elementwise=False,
        returns_scalar=True,
    )


def volatility(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    annualize: bool = True,
) -> pl.Expr:
    """Volatility (standard deviation), optionally annualized."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_volatility",
        args=_to_expr(expr),
        kwargs={"freq": freq, "annualize": annualize},
        is_elementwise=False,
        returns_scalar=True,
    )


def mean_return(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    annualize: bool = True,
) -> pl.Expr:
    """Arithmetic mean return, optionally annualized."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_mean_return",
        args=_to_expr(expr),
        kwargs={"freq": freq, "annualize": annualize},
        is_elementwise=False,
        returns_scalar=True,
    )


def cagr(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Compound annual growth rate.

    Uses ``len(returns) / ann_factor`` to derive the holding period.
    Returns NaN for series with fewer than 2 elements.
    """
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_cagr",
        args=_to_expr(expr),
        kwargs={"freq": freq},
        is_elementwise=False,
        returns_scalar=True,
    )


def calmar(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Calmar ratio: CAGR / |max drawdown|."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_calmar",
        args=_to_expr(expr),
        kwargs={"freq": freq},
        is_elementwise=False,
        returns_scalar=True,
    )


def max_drawdown(expr: IntoExpr) -> pl.Expr:
    """Maximum drawdown (most negative value in the drawdown series)."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_max_drawdown",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def geometric_mean(expr: IntoExpr) -> pl.Expr:
    """Geometric mean return per period."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_geometric_mean",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def downside_deviation(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    annualize: bool = True,
) -> pl.Expr:
    """Downside deviation (semi-standard deviation below zero)."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_downside_deviation",
        args=_to_expr(expr),
        kwargs={"freq": freq, "annualize": annualize},
        is_elementwise=False,
        returns_scalar=True,
    )


def skewness(expr: IntoExpr) -> pl.Expr:
    """Fisher-corrected sample skewness."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_skewness",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def kurtosis(expr: IntoExpr) -> pl.Expr:
    """Fisher-corrected excess kurtosis."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_kurtosis",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def value_at_risk(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Historical Value-at-Risk at the given confidence level."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_value_at_risk",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def expected_shortfall(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Expected Shortfall (CVaR) at the given confidence level."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_expected_shortfall",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def parametric_var(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Parametric (Gaussian) Value-at-Risk."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_parametric_var",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def cornish_fisher_var(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Cornish-Fisher adjusted VaR (accounts for skewness and kurtosis)."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_cornish_fisher_var",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def ulcer_index(expr: IntoExpr) -> pl.Expr:
    """Ulcer index: RMS of the drawdown series."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_ulcer_index",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def pain_index(expr: IntoExpr) -> pl.Expr:
    """Pain index: mean absolute drawdown."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_pain_index",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def omega_ratio(expr: IntoExpr, *, threshold: float = 0.0) -> pl.Expr:
    """Omega ratio: probability-weighted gain-to-loss above threshold."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_omega_ratio",
        args=_to_expr(expr),
        kwargs={"threshold": threshold},
        is_elementwise=False,
        returns_scalar=True,
    )


def gain_to_pain(expr: IntoExpr) -> pl.Expr:
    """Gain-to-pain ratio: total return / total absolute losses."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_gain_to_pain",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def tail_ratio(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Tail ratio: |upper quantile| / |lower quantile|."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_tail_ratio",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def outlier_win_ratio(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Fraction of returns above the upper quantile (outlier wins)."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_outlier_win_ratio",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def outlier_loss_ratio(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Fraction of returns below the lower quantile (outlier losses)."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_outlier_loss_ratio",
        args=_to_expr(expr),
        kwargs={"confidence": confidence},
        is_elementwise=False,
        returns_scalar=True,
    )


def risk_of_ruin(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Probability of total loss under a simplified normal model."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_risk_of_ruin",
        args=_to_expr(expr),
        kwargs={"freq": freq},
        is_elementwise=False,
        returns_scalar=True,
    )


def recovery_factor(expr: IntoExpr) -> pl.Expr:
    """Recovery factor: total return / |max drawdown|."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_recovery_factor",
        args=_to_expr(expr),
        is_elementwise=False,
        returns_scalar=True,
    )


def martin_ratio(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Martin ratio (Ulcer Performance Index): CAGR / Ulcer Index."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_martin_ratio",
        args=_to_expr(expr),
        kwargs={"freq": freq},
        is_elementwise=False,
        returns_scalar=True,
    )


def sterling_ratio(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    risk_free: float = 0.0,
) -> pl.Expr:
    """Sterling ratio: (CAGR - Rf) / |avg drawdown|."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_sterling_ratio",
        args=_to_expr(expr),
        kwargs={"freq": freq, "risk_free": risk_free},
        is_elementwise=False,
        returns_scalar=True,
    )


def burke_ratio(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    risk_free: float = 0.0,
) -> pl.Expr:
    """Burke ratio: (CAGR - Rf) / RMS of drawdowns."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_burke_ratio",
        args=_to_expr(expr),
        kwargs={"freq": freq, "risk_free": risk_free},
        is_elementwise=False,
        returns_scalar=True,
    )


def pain_ratio(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    risk_free: float = 0.0,
) -> pl.Expr:
    """Pain ratio: (CAGR - Rf) / Pain Index."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_pain_ratio",
        args=_to_expr(expr),
        kwargs={"freq": freq, "risk_free": risk_free},
        is_elementwise=False,
        returns_scalar=True,
    )


def modified_sharpe(
    expr: IntoExpr,
    *,
    freq: str = "daily",
    risk_free: float = 0.0,
    confidence: float = 0.95,
) -> pl.Expr:
    """Modified Sharpe ratio: excess return / |Cornish-Fisher VaR|."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_modified_sharpe",
        args=_to_expr(expr),
        kwargs={"confidence": confidence, "freq": freq, "risk_free": risk_free},
        is_elementwise=False,
        returns_scalar=True,
    )


# ── Tier 2: Series transforms ──


def simple_returns(expr: IntoExpr) -> pl.Expr:
    """Simple (percentage-change) returns from a price series."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_simple_returns",
        args=_to_expr(expr),
        is_elementwise=False,
    )


def cumulative_returns(expr: IntoExpr) -> pl.Expr:
    """Cumulative compounded returns: ``(1+r).cumprod() - 1``."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_cumulative_returns",
        args=_to_expr(expr),
        is_elementwise=False,
    )


def drawdown_series(expr: IntoExpr) -> pl.Expr:
    """Drawdown series: per-period drawdown depth from peak."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_drawdown_series",
        args=_to_expr(expr),
        is_elementwise=False,
    )


def rebase(expr: IntoExpr, *, base: float = 100.0) -> pl.Expr:
    """Rebase a price series so the first value equals ``base``."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_rebase",
        args=_to_expr(expr),
        kwargs={"base": base},
        is_elementwise=False,
    )


# ── Tier 3: Two-input benchmark metrics ──


def tracking_error(
    portfolio: IntoExpr,
    benchmark: IntoExpr,
    *,
    freq: str = "daily",
    annualize: bool = True,
) -> pl.Expr:
    """Tracking error: annualized volatility of active returns."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_tracking_error",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        kwargs={"freq": freq, "annualize": annualize},
        is_elementwise=False,
        returns_scalar=True,
    )


def information_ratio(
    portfolio: IntoExpr,
    benchmark: IntoExpr,
    *,
    freq: str = "daily",
    annualize: bool = True,
) -> pl.Expr:
    """Information ratio: annualized active return / tracking error."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_information_ratio",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        kwargs={"freq": freq, "annualize": annualize},
        is_elementwise=False,
        returns_scalar=True,
    )


def r_squared(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """R-squared: proportion of variance explained by benchmark."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_r_squared",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        is_elementwise=False,
        returns_scalar=True,
    )


def beta(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """OLS beta of portfolio vs benchmark."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_beta",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        is_elementwise=False,
        returns_scalar=True,
    )


def up_capture(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Up-market capture ratio."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_up_capture",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        is_elementwise=False,
        returns_scalar=True,
    )


def down_capture(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Down-market capture ratio."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_down_capture",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        is_elementwise=False,
        returns_scalar=True,
    )


def capture_ratio(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Capture ratio: up capture / down capture."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_capture_ratio",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        is_elementwise=False,
        returns_scalar=True,
    )


def batting_average(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Batting average: fraction of periods outperforming benchmark."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_batting_average",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        is_elementwise=False,
        returns_scalar=True,
    )


def m_squared(
    portfolio: IntoExpr,
    benchmark: IntoExpr,
    *,
    freq: str = "daily",
    risk_free: float = 0.0,
) -> pl.Expr:
    """M-squared (Modigliani-Modigliani) risk-adjusted return."""
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_m_squared",
        args=[_to_expr(portfolio), _to_expr(benchmark)],
        kwargs={"freq": freq, "risk_free": risk_free},
        is_elementwise=False,
        returns_scalar=True,
    )


# ── Tier 4: Rolling metrics ──


def rolling_sharpe(
    expr: IntoExpr,
    *,
    window: int = 60,
    freq: str = "daily",
    risk_free: float = 0.0,
) -> pl.Expr:
    """Rolling Sharpe ratio over a sliding window.

    Leading values (before the first full window) are NaN.
    """
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_rolling_sharpe",
        args=_to_expr(expr),
        kwargs={"window": window, "freq": freq, "risk_free": risk_free},
        is_elementwise=False,
    )


def rolling_sortino(
    expr: IntoExpr,
    *,
    window: int = 60,
    freq: str = "daily",
) -> pl.Expr:
    """Rolling Sortino ratio over a sliding window.

    Leading values (before the first full window) are NaN.
    """
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_rolling_sortino",
        args=_to_expr(expr),
        kwargs={"window": window, "freq": freq},
        is_elementwise=False,
    )


def rolling_volatility(
    expr: IntoExpr,
    *,
    window: int = 60,
    freq: str = "daily",
) -> pl.Expr:
    """Rolling annualized volatility over a sliding window.

    Leading values (before the first full window) are NaN.
    """
    return register_plugin_function(
        plugin_path=_PLUGIN_PATH,
        function_name="expr_rolling_volatility",
        args=_to_expr(expr),
        kwargs={"window": window, "freq": freq},
        is_elementwise=False,
    )


__all__ = [
    "batting_average",
    "beta",
    "burke_ratio",
    "cagr",
    "calmar",
    "capture_ratio",
    "cornish_fisher_var",
    "cumulative_returns",
    "down_capture",
    "downside_deviation",
    "drawdown_series",
    "expected_shortfall",
    "gain_to_pain",
    "geometric_mean",
    "information_ratio",
    "kurtosis",
    "m_squared",
    "martin_ratio",
    "max_drawdown",
    "mean_return",
    "modified_sharpe",
    "omega_ratio",
    "outlier_loss_ratio",
    "outlier_win_ratio",
    "pain_index",
    "pain_ratio",
    "parametric_var",
    "r_squared",
    "rebase",
    "recovery_factor",
    "risk_of_ruin",
    # Tier 4: Rolling metrics
    "rolling_sharpe",
    "rolling_sortino",
    "rolling_volatility",
    # Tier 1: Scalar risk metrics
    "sharpe",
    # Tier 2: Series transforms
    "simple_returns",
    "skewness",
    "sortino",
    "sterling_ratio",
    "tail_ratio",
    # Tier 3: Benchmark metrics
    "tracking_error",
    "ulcer_index",
    "up_capture",
    "value_at_risk",
    "volatility",
]
