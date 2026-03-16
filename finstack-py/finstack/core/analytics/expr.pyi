"""Type stubs for finstack.core.analytics.expr — Polars expression plugins."""

from __future__ import annotations
from polars._typing import IntoExpr
import polars as pl

# ── Tier 1: Scalar risk metrics ──

def sharpe(expr: IntoExpr, *, freq: str = "daily", risk_free: float = 0.0) -> pl.Expr:
    """Sharpe ratio (annualized return - risk-free) / annualized volatility."""
    ...

def sortino(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Sortino ratio: penalises only downside volatility."""
    ...

def volatility(expr: IntoExpr, *, freq: str = "daily", annualize: bool = True) -> pl.Expr:
    """Volatility (standard deviation), optionally annualized."""
    ...

def mean_return(expr: IntoExpr, *, freq: str = "daily", annualize: bool = True) -> pl.Expr:
    """Arithmetic mean return, optionally annualized."""
    ...

def cagr(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Compound annual growth rate from period-based annualization."""
    ...

def calmar(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Calmar ratio: CAGR / |max drawdown|."""
    ...

def max_drawdown(expr: IntoExpr) -> pl.Expr:
    """Maximum drawdown (most negative value in the drawdown series)."""
    ...

def geometric_mean(expr: IntoExpr) -> pl.Expr:
    """Geometric mean return per period."""
    ...

def downside_deviation(expr: IntoExpr, *, freq: str = "daily", annualize: bool = True) -> pl.Expr:
    """Downside deviation (semi-standard deviation below zero)."""
    ...

def skewness(expr: IntoExpr) -> pl.Expr:
    """Fisher-corrected sample skewness."""
    ...

def kurtosis(expr: IntoExpr) -> pl.Expr:
    """Fisher-corrected excess kurtosis."""
    ...

def value_at_risk(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Historical Value-at-Risk at the given confidence level."""
    ...

def expected_shortfall(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Expected Shortfall (CVaR) at the given confidence level."""
    ...

def parametric_var(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Parametric (Gaussian) Value-at-Risk."""
    ...

def cornish_fisher_var(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Cornish-Fisher adjusted VaR (accounts for skewness and kurtosis)."""
    ...

def ulcer_index(expr: IntoExpr) -> pl.Expr:
    """Ulcer index: RMS of the drawdown series."""
    ...

def pain_index(expr: IntoExpr) -> pl.Expr:
    """Pain index: mean absolute drawdown."""
    ...

def omega_ratio(expr: IntoExpr, *, threshold: float = 0.0) -> pl.Expr:
    """Omega ratio: probability-weighted gain-to-loss above threshold."""
    ...

def gain_to_pain(expr: IntoExpr) -> pl.Expr:
    """Gain-to-pain ratio: total return / total absolute losses."""
    ...

def tail_ratio(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Tail ratio: |upper quantile| / |lower quantile|."""
    ...

def outlier_win_ratio(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Fraction of returns above the upper quantile (outlier wins)."""
    ...

def outlier_loss_ratio(expr: IntoExpr, *, confidence: float = 0.95) -> pl.Expr:
    """Fraction of returns below the lower quantile (outlier losses)."""
    ...

def estimate_ruin(
    expr: IntoExpr,
    *,
    definition: str = "drawdown_breach",
    threshold: float = 0.2,
    horizon_periods: int = 252,
    n_paths: int = 10000,
    block_size: int = 5,
    seed: int = 42,
    confidence_level: float = 0.95,
) -> pl.Expr:
    """Estimate ruin probability from empirical returns under an explicit ruin definition."""
    ...

def recovery_factor(expr: IntoExpr) -> pl.Expr:
    """Recovery factor: total return / |max drawdown|."""
    ...

def martin_ratio(expr: IntoExpr, *, freq: str = "daily") -> pl.Expr:
    """Martin ratio (Ulcer Performance Index): CAGR / Ulcer Index."""
    ...

def sterling_ratio(expr: IntoExpr, *, freq: str = "daily", risk_free: float = 0.0) -> pl.Expr:
    """Sterling ratio: (CAGR - Rf) / |avg drawdown|."""
    ...

def burke_ratio(expr: IntoExpr, *, freq: str = "daily", risk_free: float = 0.0) -> pl.Expr:
    """Burke ratio: (CAGR - Rf) / RMS of drawdowns."""
    ...

def pain_ratio(expr: IntoExpr, *, freq: str = "daily", risk_free: float = 0.0) -> pl.Expr:
    """Pain ratio: (CAGR - Rf) / Pain Index."""
    ...

def modified_sharpe(
    expr: IntoExpr, *, freq: str = "daily", risk_free: float = 0.0, confidence: float = 0.95
) -> pl.Expr:
    """Modified Sharpe ratio: excess return / |Cornish-Fisher VaR|."""
    ...

# ── Tier 2: Series transforms ──

def simple_returns(expr: IntoExpr) -> pl.Expr:
    """Simple (percentage-change) returns from a price series."""
    ...

def cumulative_returns(expr: IntoExpr) -> pl.Expr:
    """Cumulative compounded returns."""
    ...

def drawdown_series(expr: IntoExpr) -> pl.Expr:
    """Drawdown series: per-period drawdown depth from peak."""
    ...

def rebase(expr: IntoExpr, *, base: float = 100.0) -> pl.Expr:
    """Rebase a price series so the first value equals ``base``."""
    ...

# ── Tier 3: Two-input benchmark metrics ──

def tracking_error(portfolio: IntoExpr, benchmark: IntoExpr, *, freq: str = "daily", annualize: bool = True) -> pl.Expr:
    """Tracking error: annualized volatility of active returns."""
    ...

def information_ratio(
    portfolio: IntoExpr, benchmark: IntoExpr, *, freq: str = "daily", annualize: bool = True
) -> pl.Expr:
    """Information ratio: annualized active return / tracking error."""
    ...

def r_squared(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """R-squared: proportion of variance explained by benchmark."""
    ...

def beta(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """OLS beta of portfolio vs benchmark."""
    ...

def up_capture(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Up-market capture ratio."""
    ...

def down_capture(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Down-market capture ratio."""
    ...

def capture_ratio(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Capture ratio: up capture / down capture."""
    ...

def batting_average(portfolio: IntoExpr, benchmark: IntoExpr) -> pl.Expr:
    """Batting average: fraction of periods outperforming benchmark."""
    ...

def m_squared(portfolio: IntoExpr, benchmark: IntoExpr, *, freq: str = "daily", risk_free: float = 0.0) -> pl.Expr:
    """M-squared (Modigliani-Modigliani) risk-adjusted return."""
    ...

# ── Tier 4: Rolling metrics ──

def rolling_sharpe(expr: IntoExpr, *, window: int = 60, freq: str = "daily", risk_free: float = 0.0) -> pl.Expr:
    """Rolling Sharpe ratio over a sliding window."""
    ...

def rolling_sortino(expr: IntoExpr, *, window: int = 60, freq: str = "daily") -> pl.Expr:
    """Rolling Sortino ratio over a sliding window."""
    ...

def rolling_volatility(expr: IntoExpr, *, window: int = 60, freq: str = "daily") -> pl.Expr:
    """Rolling annualized volatility over a sliding window."""
    ...
