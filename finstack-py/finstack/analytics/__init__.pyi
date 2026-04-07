"""Analytics package — canonical path for the finstack-analytics crate.

Provides the :class:`Performance` class for computing returns, drawdowns,
risk metrics, and benchmark-relative statistics from price data.

Standalone functions operate on plain ``list[float]`` slices for lightweight
analytics without constructing a ``Performance`` object.

The :mod:`finstack.analytics.expr` sub-module exposes the same metrics as
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
from .returns import (
    clean_returns,
    comp_sum,
    comp_total,
    convert_to_prices,
    excess_returns,
    rebase,
    simple_returns,
)
from .drawdown import (
    average_drawdown,
    burke_ratio,
    calmar,
    calmar_from_returns,
    cdar,
    martin_ratio,
    martin_ratio_from_returns,
    max_drawdown,
    max_drawdown_from_returns,
    pain_index,
    pain_ratio,
    pain_ratio_from_returns,
    recovery_factor,
    recovery_factor_from_returns,
    sterling_ratio,
    sterling_ratio_from_returns,
    to_drawdown_series,
    ulcer_index,
)
from .risk_metrics import (
    cagr_from_periods,
    cornish_fisher_var,
    downside_deviation,
    expected_shortfall,
    gain_to_pain,
    geometric_mean,
    kurtosis,
    mean_return,
    modified_sharpe,
    omega_ratio,
    parametric_var,
    sharpe,
    skewness,
    sortino,
    tail_ratio,
    value_at_risk,
    volatility,
)
from .benchmark import (
    batting_average,
    calc_beta,
    capture_ratio,
    down_capture,
    greeks,
    information_ratio,
    m_squared,
    r_squared,
    tracking_error,
    treynor,
    up_capture,
)
from .lookback import (
    fytd_select,
    mtd_select,
    qtd_select,
    ytd_select,
)
from .consecutive import (
    count_consecutive_above,
    count_consecutive_below,
    count_consecutive_negative,
    count_consecutive_positive,
)
from .aggregation import (
    group_by_period,
    grouped_returns,
    period_stats,
)

__all__ = [
    # Performance class & result types
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
