//! Performance analytics: returns, drawdowns, risk metrics, benchmark-relative
//! statistics, lookback selectors, and period aggregation.
//!
//! All analytics operate on `&[f64]` slices and `time::Date` -- no Polars
//! dependency. The [`crate::analytics::performance::Performance`] struct ties everything together as a
//! stateful orchestrator; individual sub-module functions are also public
//! for standalone use.

pub mod aggregation;
pub mod benchmark;
/// Count consecutive streaks in a numeric series.
pub mod consecutive;
pub mod drawdown;
pub mod lookback;
pub mod performance;
pub mod returns;
pub mod risk_metrics;

pub use aggregation::{group_by_period, period_stats, PeriodStats};
pub use benchmark::{
    align_benchmark, batting_average, calc_beta, capture_ratio, down_capture, greeks,
    information_ratio, multi_factor_greeks, r_squared, rolling_greeks, tracking_error, up_capture,
    BetaResult, GreeksResult, MultiFactorResult, RollingGreeks,
};
pub use consecutive::count_consecutive;
pub use drawdown::{
    avg_drawdown, cdar, drawdown_details, max_drawdown_duration, to_drawdown_series,
    DrawdownEpisode,
};
pub use lookback::{fytd_select, mtd_select, qtd_select, ytd_select};
pub use performance::{LookbackReturns, Performance};
pub use returns::{
    clean_returns, comp_sum, comp_total, convert_to_prices, excess_returns, rebase, simple_returns,
};
pub use risk_metrics::{
    average_drawdown, burke_ratio, cagr, calmar, cornish_fisher_var, downside_deviation,
    expected_shortfall, gain_to_pain, geometric_mean, kurtosis, m_squared, martin_ratio,
    mean_return, modified_sharpe, omega_ratio, outlier_loss_ratio, outlier_win_ratio, pain_index,
    pain_ratio, parametric_var, recovery_factor, risk_of_ruin, rolling_sharpe,
    rolling_sharpe_values, rolling_sortino, rolling_sortino_values, rolling_volatility,
    rolling_volatility_values, sharpe, skewness, sortino, sterling_ratio, tail_ratio, treynor,
    ulcer_index, value_at_risk, volatility, RollingSharpe, RollingSortino, RollingVolatility,
};
