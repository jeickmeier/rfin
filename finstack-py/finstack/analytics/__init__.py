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
CagrBasis = _analytics.CagrBasis
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
PnlExplanation = _analytics.PnlExplanation
MultiModelComparison = _analytics.MultiModelComparison

# VaR backtesting — functions
classify_breaches = _analytics.classify_breaches
kupiec_test = _analytics.kupiec_test
christoffersen_test = _analytics.christoffersen_test
traffic_light = _analytics.traffic_light
run_backtest = _analytics.run_backtest
rolling_var_forecasts = _analytics.rolling_var_forecasts
compare_var_backtests = _analytics.compare_var_backtests
pnl_explanation = _analytics.pnl_explanation
mtd_select = _analytics.mtd_select
qtd_select = _analytics.qtd_select
ytd_select = _analytics.ytd_select
fytd_select = _analytics.fytd_select

# GARCH volatility models
VarianceForecast = _analytics.VarianceForecast
GarchFit = _analytics.GarchFit
GarchParams = _analytics.GarchParams
fit_garch11 = _analytics.fit_garch11
fit_egarch11 = _analytics.fit_egarch11
fit_gjr_garch11 = _analytics.fit_gjr_garch11
forecast_garch_fit = _analytics.forecast_garch_fit
ljung_box = _analytics.ljung_box
arch_lm = _analytics.arch_lm
aic = _analytics.aic
bic = _analytics.bic
hqic = _analytics.hqic

__all__: list[str] = [
    "BacktestResult",
    "BenchmarkAlignmentPolicy",
    "BetaResult",
    "CagrBasis",
    "ChristoffersenResult",
    "DrawdownEpisode",
    "GarchFit",
    "GarchParams",
    "GreeksResult",
    "KupiecResult",
    "LookbackReturns",
    "MultiFactorResult",
    "MultiModelComparison",
    "Performance",
    "PeriodStats",
    "PnlExplanation",
    "RollingGreeks",
    "RollingSharpe",
    "RollingSortino",
    "RollingVolatility",
    "RuinDefinition",
    "RuinEstimate",
    "RuinModel",
    "TrafficLightResult",
    "VarianceForecast",
    "aic",
    "align_benchmark",
    "arch_lm",
    "batting_average",
    "beta",
    "bic",
    "burke_ratio",
    "cagr",
    "calmar",
    "capture_ratio",
    "cdar",
    "christoffersen_test",
    "classify_breaches",
    "clean_returns",
    "comp_sum",
    "comp_total",
    "compare_var_backtests",
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
    "forecast_garch_fit",
    "fytd_select",
    "gain_to_pain",
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
    "mtd_select",
    "multi_factor_greeks",
    "omega_ratio",
    "outlier_loss_ratio",
    "outlier_win_ratio",
    "pain_index",
    "pain_ratio",
    "parametric_var",
    "period_stats",
    "pnl_explanation",
    "qtd_select",
    "r_squared",
    "rebase",
    "recovery_factor",
    "rolling_greeks",
    "rolling_sharpe",
    "rolling_sortino",
    "rolling_var_forecasts",
    "rolling_volatility",
    "run_backtest",
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
    "ytd_select",
]
