"""Analytics package — canonical path for the finstack-analytics crate.

Provides :class:`Performance` for computing returns, drawdowns, risk metrics,
and benchmark-relative statistics from price data.

Standalone functions operate on plain ``list[float]`` slices for lightweight
analytics without constructing a ``Performance`` object.

The :mod:`finstack.analytics.expr` sub-module exposes the same metrics as
native Polars expression plugins for use inside ``.select()``,
``.with_columns()``, and other Polars contexts.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .performance import (
        BetaResult as BetaResult,
        DrawdownEpisodeDict as DrawdownEpisodeDict,
        GreeksResult as GreeksResult,
        LookbackResult as LookbackResult,
        MultiFactorResult as MultiFactorResult,
        PeriodStatsResult as PeriodStatsResult,
    )

from finstack import finstack as _finstack  # type: ignore[reportAttributeAccessIssue]

from . import expr as expr

_rust_analytics = _finstack.analytics  # type: ignore[unresolved-attribute]

Performance = _rust_analytics.Performance

# ── Returns ──
clean_returns = _rust_analytics.clean_returns
simple_returns = _rust_analytics.simple_returns
comp_sum = _rust_analytics.comp_sum
comp_total = _rust_analytics.comp_total
convert_to_prices = _rust_analytics.convert_to_prices
rebase = _rust_analytics.rebase
excess_returns = _rust_analytics.excess_returns

# ── Drawdown ──
to_drawdown_series = _rust_analytics.to_drawdown_series
max_drawdown = _rust_analytics.max_drawdown
max_drawdown_from_returns = _rust_analytics.max_drawdown_from_returns
average_drawdown = _rust_analytics.average_drawdown
calmar = _rust_analytics.calmar
calmar_from_returns = _rust_analytics.calmar_from_returns
pain_index = _rust_analytics.pain_index
ulcer_index = _rust_analytics.ulcer_index
cdar = _rust_analytics.cdar
recovery_factor = _rust_analytics.recovery_factor
recovery_factor_from_returns = _rust_analytics.recovery_factor_from_returns
martin_ratio = _rust_analytics.martin_ratio
martin_ratio_from_returns = _rust_analytics.martin_ratio_from_returns
sterling_ratio = _rust_analytics.sterling_ratio
sterling_ratio_from_returns = _rust_analytics.sterling_ratio_from_returns
burke_ratio = _rust_analytics.burke_ratio
pain_ratio = _rust_analytics.pain_ratio
pain_ratio_from_returns = _rust_analytics.pain_ratio_from_returns

# ── Risk metrics ──
cagr_from_periods = _rust_analytics.cagr_from_periods
mean_return = _rust_analytics.mean_return
volatility = _rust_analytics.volatility
sharpe = _rust_analytics.sharpe
sortino = _rust_analytics.sortino
downside_deviation = _rust_analytics.downside_deviation
value_at_risk = _rust_analytics.value_at_risk
expected_shortfall = _rust_analytics.expected_shortfall
parametric_var = _rust_analytics.parametric_var
cornish_fisher_var = _rust_analytics.cornish_fisher_var
skewness = _rust_analytics.skewness
kurtosis = _rust_analytics.kurtosis
geometric_mean = _rust_analytics.geometric_mean
omega_ratio = _rust_analytics.omega_ratio
gain_to_pain = _rust_analytics.gain_to_pain
tail_ratio = _rust_analytics.tail_ratio
modified_sharpe = _rust_analytics.modified_sharpe

# ── Benchmark ──
tracking_error = _rust_analytics.tracking_error
information_ratio = _rust_analytics.information_ratio
r_squared = _rust_analytics.r_squared
calc_beta = _rust_analytics.calc_beta
greeks = _rust_analytics.greeks
up_capture = _rust_analytics.up_capture
down_capture = _rust_analytics.down_capture
capture_ratio = _rust_analytics.capture_ratio
batting_average = _rust_analytics.batting_average
treynor = _rust_analytics.treynor
m_squared = _rust_analytics.m_squared

# ── Lookback ──
mtd_select = _rust_analytics.mtd_select
qtd_select = _rust_analytics.qtd_select
ytd_select = _rust_analytics.ytd_select
fytd_select = _rust_analytics.fytd_select

# ── Consecutive ──
count_consecutive_positive = _rust_analytics.count_consecutive_positive
count_consecutive_negative = _rust_analytics.count_consecutive_negative
count_consecutive_above = _rust_analytics.count_consecutive_above
count_consecutive_below = _rust_analytics.count_consecutive_below

# ── Aggregation ──
group_by_period = _rust_analytics.group_by_period
period_stats = _rust_analytics.period_stats
grouped_returns = _rust_analytics.grouped_returns

__all__ = [
    # Performance class & result types
    "BetaResult",
    "DrawdownEpisodeDict",
    "GreeksResult",
    "LookbackResult",
    "MultiFactorResult",
    "Performance",
    "PeriodStatsResult",
    "average_drawdown",
    "batting_average",
    "burke_ratio",
    # Risk metrics
    "cagr_from_periods",
    "calc_beta",
    "calmar",
    "calmar_from_returns",
    "capture_ratio",
    "cdar",
    # Returns
    "clean_returns",
    "comp_sum",
    "comp_total",
    "convert_to_prices",
    "cornish_fisher_var",
    "count_consecutive_above",
    "count_consecutive_below",
    "count_consecutive_negative",
    # Consecutive
    "count_consecutive_positive",
    "down_capture",
    "downside_deviation",
    "excess_returns",
    "expected_shortfall",
    "expr",
    "fytd_select",
    "gain_to_pain",
    "geometric_mean",
    "greeks",
    # Aggregation
    "group_by_period",
    "grouped_returns",
    "information_ratio",
    "kurtosis",
    "m_squared",
    "martin_ratio",
    "martin_ratio_from_returns",
    "max_drawdown",
    "max_drawdown_from_returns",
    "mean_return",
    "modified_sharpe",
    # Lookback
    "mtd_select",
    "omega_ratio",
    "pain_index",
    "pain_ratio",
    "pain_ratio_from_returns",
    "parametric_var",
    "period_stats",
    "qtd_select",
    "r_squared",
    "rebase",
    "recovery_factor",
    "recovery_factor_from_returns",
    "sharpe",
    "simple_returns",
    "skewness",
    "sortino",
    "sterling_ratio",
    "sterling_ratio_from_returns",
    "tail_ratio",
    # Drawdown
    "to_drawdown_series",
    # Benchmark
    "tracking_error",
    "treynor",
    "ulcer_index",
    "up_capture",
    "value_at_risk",
    "volatility",
    "ytd_select",
]
