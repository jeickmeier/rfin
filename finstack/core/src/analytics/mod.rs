//! Performance analytics: returns, drawdowns, risk metrics, benchmark-relative
//! statistics, lookback selectors, and period aggregation.
//!
//! All analytics operate on `&[f64]` slices and `time::Date` -- no Polars
//! dependency. The [`Performance`] struct ties everything together as a
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
    align_benchmark, calc_beta, greeks, information_ratio, r_squared, rolling_greeks,
    tracking_error, BetaResult, GreeksResult, RollingGreeks,
};
pub use consecutive::count_consecutive;
pub use drawdown::{avg_drawdown, drawdown_details, to_drawdown_series, DrawdownEpisode};
pub use lookback::{fytd_select, mtd_select, qtd_select, ytd_select};
pub use performance::{LookbackReturns, Performance};
pub use returns::{
    clean_returns, comp_sum, comp_total, convert_to_prices, excess_returns, rebase, simple_returns,
};
pub use risk_metrics::{
    cagr, calmar, expected_shortfall, mean_return, outlier_loss_ratio, outlier_win_ratio,
    risk_of_ruin, rolling_sharpe, sharpe, sortino, tail_ratio, ulcer_index, value_at_risk,
    volatility, RollingSharpe,
};
