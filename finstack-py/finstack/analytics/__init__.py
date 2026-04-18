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
beta = _analytics.beta
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

# Drawdown
to_drawdown_series = _analytics.to_drawdown_series
drawdown_details = _analytics.drawdown_details
mean_episode_drawdown = _analytics.mean_episode_drawdown
mean_drawdown = _analytics.mean_drawdown
max_drawdown = _analytics.max_drawdown
max_drawdown_duration = _analytics.max_drawdown_duration
cdar = _analytics.cdar
ulcer_index = _analytics.ulcer_index
pain_index = _analytics.pain_index
calmar = _analytics.calmar
recovery_factor = _analytics.recovery_factor
martin_ratio = _analytics.martin_ratio
sterling_ratio = _analytics.sterling_ratio
burke_ratio = _analytics.burke_ratio
pain_ratio = _analytics.pain_ratio

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

# VaR backtesting — types
KupiecResult = _analytics.KupiecResult
ChristoffersenResult = _analytics.ChristoffersenResult
TrafficLightResult = _analytics.TrafficLightResult
BacktestResult = _analytics.BacktestResult

# VaR backtesting — functions
classify_breaches = _analytics.classify_breaches
kupiec_test = _analytics.kupiec_test
christoffersen_test = _analytics.christoffersen_test
traffic_light = _analytics.traffic_light
run_backtest = _analytics.run_backtest

# GARCH volatility models
GarchFit = _analytics.GarchFit
GarchParams = _analytics.GarchParams
fit_garch11 = _analytics.fit_garch11
fit_egarch11 = _analytics.fit_egarch11
fit_gjr_garch11 = _analytics.fit_gjr_garch11
garch11_forecast = _analytics.garch11_forecast
ljung_box = _analytics.ljung_box
arch_lm = _analytics.arch_lm
aic = _analytics.aic
bic = _analytics.bic
hqic = _analytics.hqic

# Comparable company analysis
percentile_rank = _analytics.percentile_rank
z_score = _analytics.z_score
peer_stats = _analytics.peer_stats
regression_fair_value = _analytics.regression_fair_value
compute_multiple = _analytics.compute_multiple
score_relative_value = _analytics.score_relative_value

__all__: list[str] = [
    "BacktestResult",
    "BenchmarkAlignmentPolicy",
    "BetaResult",
    "ChristoffersenResult",
    "DrawdownEpisode",
    "GarchFit",
    "GarchParams",
    "GreeksResult",
    "KupiecResult",
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
    "TrafficLightResult",
    "aic",
    "align_benchmark",
    "arch_lm",
    "batting_average",
    "beta",
    "bic",
    "burke_ratio",
    "cagr",
    "cagr_from_periods",
    "calmar",
    "capture_ratio",
    "cdar",
    "christoffersen_test",
    "classify_breaches",
    "clean_returns",
    "comp_sum",
    "comp_total",
    "compute_multiple",
    "convert_to_prices",
    "cornish_fisher_var",
    "down_capture",
    "downside_deviation",
    "drawdown_details",
    "estimate_ruin",
    "excess_returns",
    "expected_shortfall",
    "fit_egarch11",
    "fit_garch11",
    "fit_gjr_garch11",
    "gain_to_pain",
    "garch11_forecast",
    "geometric_mean",
    "greeks",
    "group_by_period",
    "hqic",
    "information_ratio",
    "kupiec_test",
    "kurtosis",
    "ljung_box",
    "m_squared",
    "martin_ratio",
    "max_drawdown",
    "max_drawdown_duration",
    "mean_drawdown",
    "mean_episode_drawdown",
    "mean_return",
    "modified_sharpe",
    "multi_factor_greeks",
    "omega_ratio",
    "outlier_loss_ratio",
    "outlier_win_ratio",
    "pain_index",
    "pain_ratio",
    "parametric_var",
    "peer_stats",
    "percentile_rank",
    "period_stats",
    "r_squared",
    "rebase",
    "recovery_factor",
    "regression_fair_value",
    "rolling_greeks",
    "rolling_sharpe",
    "rolling_sharpe_values",
    "rolling_sortino",
    "rolling_sortino_values",
    "rolling_volatility",
    "rolling_volatility_values",
    "run_backtest",
    "score_relative_value",
    "sharpe",
    "simple_returns",
    "skewness",
    "sortino",
    "sterling_ratio",
    "tail_ratio",
    "to_drawdown_series",
    "tracking_error",
    "traffic_light",
    "treynor",
    "ulcer_index",
    "up_capture",
    "value_at_risk",
    "volatility",
    "z_score",
]
