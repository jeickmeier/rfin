"""Performance analytics module.

Provides the :class:`Performance` class for computing returns, drawdowns,
risk metrics, and benchmark-relative statistics from price data.

Standalone functions operate on plain ``list[float]`` slices for lightweight
analytics without constructing a ``Performance`` object.

The :mod:`finstack.core.analytics.expr` sub-module exposes the same metrics as
native Polars expression plugins for use inside ``.select()``,
``.with_columns()``, and other Polars contexts.
"""

from __future__ import annotations
from . import expr as expr
from .performance import (
    BetaResult,
    DrawdownEpisodeDict,
    GreeksResult,
    LookbackResult,
    MultiFactorResult,
    Performance,
    PeriodStatsResult,
)

from finstack.analytics.returns import (
    clean_returns as clean_returns,
    comp_sum as comp_sum,
    comp_total as comp_total,
    convert_to_prices as convert_to_prices,
    excess_returns as excess_returns,
    rebase as rebase,
    simple_returns as simple_returns,
)
from finstack.analytics.drawdown import (
    average_drawdown as average_drawdown,
    burke_ratio as burke_ratio,
    calmar as calmar,
    calmar_from_returns as calmar_from_returns,
    cdar as cdar,
    martin_ratio as martin_ratio,
    martin_ratio_from_returns as martin_ratio_from_returns,
    max_drawdown as max_drawdown,
    max_drawdown_from_returns as max_drawdown_from_returns,
    pain_index as pain_index,
    pain_ratio as pain_ratio,
    pain_ratio_from_returns as pain_ratio_from_returns,
    recovery_factor as recovery_factor,
    recovery_factor_from_returns as recovery_factor_from_returns,
    sterling_ratio as sterling_ratio,
    sterling_ratio_from_returns as sterling_ratio_from_returns,
    to_drawdown_series as to_drawdown_series,
    ulcer_index as ulcer_index,
)
from finstack.analytics.risk_metrics import (
    cagr_from_periods as cagr_from_periods,
    cornish_fisher_var as cornish_fisher_var,
    downside_deviation as downside_deviation,
    expected_shortfall as expected_shortfall,
    gain_to_pain as gain_to_pain,
    geometric_mean as geometric_mean,
    kurtosis as kurtosis,
    mean_return as mean_return,
    modified_sharpe as modified_sharpe,
    omega_ratio as omega_ratio,
    parametric_var as parametric_var,
    sharpe as sharpe,
    skewness as skewness,
    sortino as sortino,
    tail_ratio as tail_ratio,
    value_at_risk as value_at_risk,
    volatility as volatility,
)
from finstack.analytics.benchmark import (
    batting_average as batting_average,
    calc_beta as calc_beta,
    capture_ratio as capture_ratio,
    down_capture as down_capture,
    greeks as greeks,
    information_ratio as information_ratio,
    m_squared as m_squared,
    r_squared as r_squared,
    tracking_error as tracking_error,
    treynor as treynor,
    up_capture as up_capture,
)
from finstack.analytics.lookback import (
    fytd_select as fytd_select,
    mtd_select as mtd_select,
    qtd_select as qtd_select,
    ytd_select as ytd_select,
)
from finstack.analytics.consecutive import (
    count_consecutive_above as count_consecutive_above,
    count_consecutive_below as count_consecutive_below,
    count_consecutive_negative as count_consecutive_negative,
    count_consecutive_positive as count_consecutive_positive,
)
from finstack.analytics.aggregation import (
    group_by_period as group_by_period,
    grouped_returns as grouped_returns,
    period_stats as period_stats,
)

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
