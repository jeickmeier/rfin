//! Python bindings for the `finstack-analytics` crate.
//!
//! Exposes the stateful [`PyPerformance`] class, standalone analytics functions,
//! and result types for benchmarks, drawdowns, rolling metrics, and ruin estimation.

mod backtesting;
mod comps;
mod functions;
mod performance;
mod timeseries;
mod types;

use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register the `analytics` submodule on the parent module.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "analytics")?;
    m.setattr(
        "__doc__",
        "Performance analytics: returns, drawdowns, risk metrics, benchmarks.",
    )?;

    types::register(py, &m)?;
    performance::register(py, &m)?;
    functions::register(py, &m)?;
    backtesting::register(py, &m)?;
    timeseries::register(py, &m)?;
    comps::register(py, &m)?;

    let all = PyList::new(
        py,
        [
            // Types
            "PeriodStats",
            "BetaResult",
            "GreeksResult",
            "RollingGreeks",
            "MultiFactorResult",
            "DrawdownEpisode",
            "LookbackReturns",
            "RollingSharpe",
            "RollingSortino",
            "RollingVolatility",
            "RuinDefinition",
            "RuinModel",
            "RuinEstimate",
            "BenchmarkAlignmentPolicy",
            // Performance class
            "Performance",
            // Aggregation
            "group_by_period",
            "period_stats",
            // Benchmark
            "align_benchmark",
            "align_benchmark_with_policy",
            "calc_beta",
            "greeks",
            "rolling_greeks",
            "tracking_error",
            "information_ratio",
            "r_squared",
            "up_capture",
            "down_capture",
            "capture_ratio",
            "batting_average",
            "multi_factor_greeks",
            "treynor",
            "m_squared",
            "m_squared_from_returns",
            // Consecutive
            "count_consecutive",
            // Drawdown
            "to_drawdown_series",
            "drawdown_details",
            "avg_drawdown",
            "average_drawdown",
            "max_drawdown",
            "max_drawdown_from_returns",
            "max_drawdown_duration",
            "cdar",
            "ulcer_index",
            "pain_index",
            "calmar",
            "calmar_from_returns",
            "recovery_factor",
            "recovery_factor_from_returns",
            "martin_ratio",
            "martin_ratio_from_returns",
            "sterling_ratio",
            "sterling_ratio_from_returns",
            "burke_ratio",
            "pain_ratio",
            "pain_ratio_from_returns",
            // Returns
            "simple_returns",
            "clean_returns",
            "excess_returns",
            "convert_to_prices",
            "rebase",
            "comp_sum",
            "comp_total",
            // Risk metrics — return-based
            "cagr",
            "cagr_from_periods",
            "mean_return",
            "volatility",
            "sharpe",
            "downside_deviation",
            "sortino",
            "geometric_mean",
            "omega_ratio",
            "gain_to_pain",
            "modified_sharpe",
            "estimate_ruin",
            // Risk metrics — rolling
            "rolling_sharpe",
            "rolling_sortino",
            "rolling_volatility",
            "rolling_sharpe_values",
            "rolling_sortino_values",
            "rolling_volatility_values",
            // Risk metrics — tail
            "value_at_risk",
            "expected_shortfall",
            "parametric_var",
            "cornish_fisher_var",
            "skewness",
            "kurtosis",
            "tail_ratio",
            "outlier_win_ratio",
            "outlier_loss_ratio",
            // VaR backtesting
            "classify_breaches",
            "kupiec_test",
            "christoffersen_test",
            "traffic_light",
            "run_backtest",
            // GARCH volatility models
            "fit_garch11",
            "fit_egarch11",
            "fit_gjr_garch11",
            "garch11_forecast",
            "ljung_box",
            "arch_lm",
            "aic",
            "bic",
            "hqic",
            // Comparable company analysis
            "percentile_rank",
            "z_score",
            "peer_stats",
            "regression_fair_value",
            "compute_multiple",
            "score_relative_value",
        ],
    )?;
    m.setattr("__all__", all)?;
    parent.add_submodule(&m)?;

    let parent_name: String = match parent.getattr("__name__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.finstack".to_string(),
        },
        Err(_) => "finstack.finstack".to_string(),
    };
    let qual = format!("{parent_name}.analytics");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
