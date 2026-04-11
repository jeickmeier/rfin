"""Performance analytics: returns, drawdowns, risk metrics, benchmarks.

Bindings for the ``finstack-analytics`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import analytics as _analytics

# Types
PeriodStats = _analytics.PeriodStats
BetaResult = _analytics.BetaResult
GreeksResult = _analytics.GreeksResult
RollingGreeks = _analytics.RollingGreeks
MultiFactorResult = _analytics.MultiFactorResult
DrawdownEpisode = _analytics.DrawdownEpisode
LookbackReturns = _analytics.LookbackReturns
RollingSharpe = _analytics.RollingSharpe
RollingSortino = _analytics.RollingSortino
RollingVolatility = _analytics.RollingVolatility
RuinDefinition = _analytics.RuinDefinition
RuinModel = _analytics.RuinModel
RuinEstimate = _analytics.RuinEstimate
BenchmarkAlignmentPolicy = _analytics.BenchmarkAlignmentPolicy
Performance = _analytics.Performance

# Aggregation
group_by_period = _analytics.group_by_period
period_stats = _analytics.period_stats

# Benchmark
align_benchmark = _analytics.align_benchmark
align_benchmark_with_policy = _analytics.align_benchmark_with_policy
calc_beta = _analytics.calc_beta
greeks = _analytics.greeks
rolling_greeks = _analytics.rolling_greeks
tracking_error = _analytics.tracking_error
information_ratio = _analytics.information_ratio
r_squared = _analytics.r_squared
up_capture = _analytics.up_capture
down_capture = _analytics.down_capture
capture_ratio = _analytics.capture_ratio
batting_average = _analytics.batting_average
multi_factor_greeks = _analytics.multi_factor_greeks
treynor = _analytics.treynor
m_squared = _analytics.m_squared
m_squared_from_returns = _analytics.m_squared_from_returns

# Consecutive
count_consecutive = _analytics.count_consecutive

# Drawdown
to_drawdown_series = _analytics.to_drawdown_series
drawdown_details = _analytics.drawdown_details
avg_drawdown = _analytics.avg_drawdown
average_drawdown = _analytics.average_drawdown
max_drawdown = _analytics.max_drawdown
max_drawdown_from_returns = _analytics.max_drawdown_from_returns
max_drawdown_duration = _analytics.max_drawdown_duration
cdar = _analytics.cdar
ulcer_index = _analytics.ulcer_index
pain_index = _analytics.pain_index
calmar = _analytics.calmar
calmar_from_returns = _analytics.calmar_from_returns
recovery_factor = _analytics.recovery_factor
recovery_factor_from_returns = _analytics.recovery_factor_from_returns
martin_ratio = _analytics.martin_ratio
martin_ratio_from_returns = _analytics.martin_ratio_from_returns
sterling_ratio = _analytics.sterling_ratio
sterling_ratio_from_returns = _analytics.sterling_ratio_from_returns
burke_ratio = _analytics.burke_ratio
pain_ratio = _analytics.pain_ratio
pain_ratio_from_returns = _analytics.pain_ratio_from_returns

# Returns
simple_returns = _analytics.simple_returns
clean_returns = _analytics.clean_returns
excess_returns = _analytics.excess_returns
convert_to_prices = _analytics.convert_to_prices
rebase = _analytics.rebase
comp_sum = _analytics.comp_sum
comp_total = _analytics.comp_total

# Risk metrics — return-based
cagr = _analytics.cagr
cagr_from_periods = _analytics.cagr_from_periods
mean_return = _analytics.mean_return
volatility = _analytics.volatility
sharpe = _analytics.sharpe
downside_deviation = _analytics.downside_deviation
sortino = _analytics.sortino
geometric_mean = _analytics.geometric_mean
omega_ratio = _analytics.omega_ratio
gain_to_pain = _analytics.gain_to_pain
modified_sharpe = _analytics.modified_sharpe
estimate_ruin = _analytics.estimate_ruin

# Risk metrics — rolling
rolling_sharpe = _analytics.rolling_sharpe
rolling_sortino = _analytics.rolling_sortino
rolling_volatility = _analytics.rolling_volatility
rolling_sharpe_values = _analytics.rolling_sharpe_values
rolling_sortino_values = _analytics.rolling_sortino_values
rolling_volatility_values = _analytics.rolling_volatility_values

# Risk metrics — tail
value_at_risk = _analytics.value_at_risk
expected_shortfall = _analytics.expected_shortfall
parametric_var = _analytics.parametric_var
cornish_fisher_var = _analytics.cornish_fisher_var
skewness = _analytics.skewness
kurtosis = _analytics.kurtosis
tail_ratio = _analytics.tail_ratio
outlier_win_ratio = _analytics.outlier_win_ratio
outlier_loss_ratio = _analytics.outlier_loss_ratio

__all__: list[str] = [
    "BenchmarkAlignmentPolicy",
    "BetaResult",
    "DrawdownEpisode",
    "GreeksResult",
    "LookbackReturns",
    "MultiFactorResult",
    "Performance",
    "PeriodStats",
    "RollingGreeks",
    "RollingSharpe",
    "RollingSortino",
    "RollingVolatility",
    "RuinDefinition",
    "RuinEstimate",
    "RuinModel",
    "align_benchmark",
    "align_benchmark_with_policy",
    "average_drawdown",
    "avg_drawdown",
    "batting_average",
    "burke_ratio",
    "cagr",
    "cagr_from_periods",
    "calc_beta",
    "calmar",
    "calmar_from_returns",
    "capture_ratio",
    "cdar",
    "clean_returns",
    "comp_sum",
    "comp_total",
    "convert_to_prices",
    "cornish_fisher_var",
    "count_consecutive",
    "down_capture",
    "downside_deviation",
    "drawdown_details",
    "estimate_ruin",
    "excess_returns",
    "expected_shortfall",
    "gain_to_pain",
    "geometric_mean",
    "greeks",
    "group_by_period",
    "information_ratio",
    "kurtosis",
    "m_squared",
    "m_squared_from_returns",
    "martin_ratio",
    "martin_ratio_from_returns",
    "max_drawdown",
    "max_drawdown_duration",
    "max_drawdown_from_returns",
    "mean_return",
    "modified_sharpe",
    "multi_factor_greeks",
    "omega_ratio",
    "outlier_loss_ratio",
    "outlier_win_ratio",
    "pain_index",
    "pain_ratio",
    "pain_ratio_from_returns",
    "parametric_var",
    "period_stats",
    "r_squared",
    "rebase",
    "recovery_factor",
    "recovery_factor_from_returns",
    "rolling_greeks",
    "rolling_sharpe",
    "rolling_sharpe_values",
    "rolling_sortino",
    "rolling_sortino_values",
    "rolling_volatility",
    "rolling_volatility_values",
    "sharpe",
    "simple_returns",
    "skewness",
    "sortino",
    "sterling_ratio",
    "sterling_ratio_from_returns",
    "tail_ratio",
    "to_drawdown_series",
    "tracking_error",
    "treynor",
    "ulcer_index",
    "up_capture",
    "value_at_risk",
    "volatility",
]
