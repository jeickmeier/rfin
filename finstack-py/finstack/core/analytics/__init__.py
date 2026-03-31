"""Compatibility shim — canonical path is :mod:`finstack.analytics`.

.. deprecated::
    Import from ``finstack.analytics`` instead of ``finstack.core.analytics``.
    This module will be removed in a future release.
"""

from __future__ import annotations

from typing import TYPE_CHECKING
import warnings as _warnings

_warnings.warn(
    "finstack.core.analytics is deprecated. Use finstack.analytics instead.",
    DeprecationWarning,
    stacklevel=2,
)

if TYPE_CHECKING:
    from .performance import (
        BetaResult as BetaResult,
        DrawdownEpisodeDict as DrawdownEpisodeDict,
        GreeksResult as GreeksResult,
        LookbackResult as LookbackResult,
        MultiFactorResult as MultiFactorResult,
        PeriodStatsResult as PeriodStatsResult,
    )

from finstack import finstack as _finstack  # noqa: E402  # type: ignore[reportAttributeAccessIssue]
from finstack.analytics import expr as expr  # noqa: E402  # canonical path

_rust_analytics = _finstack.core.analytics  # type: ignore[unresolved-attribute]

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
    "BetaResult",
    "DrawdownEpisodeDict",
    "GreeksResult",
    "LookbackResult",
    "MultiFactorResult",
    "Performance",
    "PeriodStatsResult",
    "expr",
    # Returns
    "clean_returns",
    "simple_returns",
    "comp_sum",
    "comp_total",
    "convert_to_prices",
    "rebase",
    "excess_returns",
    # Drawdown
    "to_drawdown_series",
    "max_drawdown",
    "max_drawdown_from_returns",
    "average_drawdown",
    "calmar",
    "calmar_from_returns",
    "pain_index",
    "ulcer_index",
    "cdar",
    "recovery_factor",
    "recovery_factor_from_returns",
    "martin_ratio",
    "martin_ratio_from_returns",
    "sterling_ratio",
    "sterling_ratio_from_returns",
    "burke_ratio",
    "pain_ratio",
    "pain_ratio_from_returns",
    # Risk metrics
    "cagr_from_periods",
    "mean_return",
    "volatility",
    "sharpe",
    "sortino",
    "downside_deviation",
    "value_at_risk",
    "expected_shortfall",
    "parametric_var",
    "cornish_fisher_var",
    "skewness",
    "kurtosis",
    "geometric_mean",
    "omega_ratio",
    "gain_to_pain",
    "tail_ratio",
    "modified_sharpe",
    # Benchmark
    "tracking_error",
    "information_ratio",
    "r_squared",
    "calc_beta",
    "greeks",
    "up_capture",
    "down_capture",
    "capture_ratio",
    "batting_average",
    "treynor",
    "m_squared",
    # Lookback
    "mtd_select",
    "qtd_select",
    "ytd_select",
    "fytd_select",
    # Consecutive
    "count_consecutive_positive",
    "count_consecutive_negative",
    "count_consecutive_above",
    "count_consecutive_below",
    # Aggregation
    "group_by_period",
    "period_stats",
    "grouped_returns",
]
