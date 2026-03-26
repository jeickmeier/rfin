//! Performance analytics on numeric slices and `finstack_core::dates::Date`.
//!
//! Start with [`crate::performance::Performance`] when you want a stateful,
//! benchmark-aware facade over a full panel of ticker returns. Reach for the
//! individual modules when you want standalone, allocation-light functions on
//! pre-computed return or drawdown slices.
//!
//! Key conventions:
//! - returns are simple decimal returns unless a function explicitly says otherwise
//! - annualization uses the caller-supplied or [`crate::dates::PeriodKind`]-derived
//!   periods-per-year factor
//! - drawdown depths are non-positive fractions such as `-0.25` for a 25% loss
//! - benchmark-relative metrics operate on return series, not fill-forwarded prices
//!
//! See [`crate::risk_metrics`] for return- and tail-based ratios,
//! [`crate::drawdown`] for drawdown path analytics, and
//! [`crate::benchmark`] for benchmark-relative regressions and attribution.

pub(crate) use finstack_core::{dates, error, math};
type Result<T> = finstack_core::Result<T>;

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
    align_benchmark, align_benchmark_with_policy, batting_average, calc_beta, capture_ratio,
    down_capture, greeks, information_ratio, m_squared, m_squared_from_returns,
    multi_factor_greeks, r_squared, rolling_greeks, tracking_error, treynor, up_capture,
    BenchmarkAlignmentPolicy, BetaResult, GreeksResult, MultiFactorResult, RollingGreeks,
};
pub use consecutive::count_consecutive;
pub use drawdown::{
    average_drawdown, avg_drawdown, burke_ratio, calmar, calmar_from_returns, cdar,
    drawdown_details, martin_ratio, martin_ratio_from_returns, max_drawdown, max_drawdown_duration,
    max_drawdown_from_returns, pain_index, pain_ratio, pain_ratio_from_returns, recovery_factor,
    recovery_factor_from_returns, sterling_ratio, sterling_ratio_from_returns, to_drawdown_series,
    ulcer_index, DrawdownEpisode,
};
pub use lookback::{fytd_select, mtd_select, qtd_select, ytd_select};
pub use performance::{LookbackReturns, Performance};
pub use returns::{
    clean_returns, comp_sum, comp_total, convert_to_prices, excess_returns, rebase, simple_returns,
};
pub use risk_metrics::{
    cagr, cagr_from_periods, cornish_fisher_var, downside_deviation, estimate_ruin,
    expected_shortfall, expected_shortfall_with_scratch, gain_to_pain, geometric_mean, kurtosis,
    mean_return, modified_sharpe, omega_ratio, outlier_loss_ratio, outlier_loss_ratio_with_scratch,
    outlier_win_ratio, outlier_win_ratio_with_scratch, parametric_var, rolling_sharpe,
    rolling_sharpe_values, rolling_sortino, rolling_sortino_values, rolling_volatility,
    rolling_volatility_values, sharpe, skewness, sortino, tail_ratio, tail_ratio_with_scratch,
    value_at_risk, value_at_risk_with_scratch, volatility, RollingSharpe, RollingSortino,
    RollingVolatility, RuinDefinition, RuinEstimate, RuinModel,
};
