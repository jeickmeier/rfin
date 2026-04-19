//! WASM bindings for the `finstack-analytics` crate.
//!
//! The bindings are split by the same domain boundaries as the Rust crate to
//! keep wrapper-only code reviewable and reduce drift pressure.
//!
//! The re-export lists below are the authoritative Rust-visible surface of
//! this module. Prefer explicit re-exports over `pub use submodule::*` so
//! that accidental additions or name collisions are caught at compile time.

mod aggregation;
mod backtesting;
mod benchmark;
mod comps;
mod drawdown;
mod lookback;
mod returns;
mod risk_metrics;
mod support;
mod tests;
mod timeseries;

pub use aggregation::{group_by_period, period_stats};
pub use backtesting::{
    christoffersen_test, classify_breaches, compare_var_backtests, kupiec_test, pnl_explanation,
    rolling_var_forecasts, run_backtest, traffic_light,
};
pub use benchmark::{
    align_benchmark, batting_average, beta, capture_ratio, down_capture, greeks, information_ratio,
    m_squared, multi_factor_greeks, r_squared, rolling_greeks, tracking_error, treynor, up_capture,
    WasmBenchmarkAlignmentPolicy,
};
pub use comps::{
    compute_multiple, peer_stats, percentile_rank, regression_fair_value, score_relative_value,
    z_score,
};
pub use drawdown::{
    burke_ratio, calmar, cdar, drawdown_details, martin_ratio, max_drawdown, max_drawdown_duration,
    mean_drawdown, mean_episode_drawdown, pain_index, pain_ratio, recovery_factor, sterling_ratio,
    to_drawdown_series, ulcer_index,
};
pub use lookback::{fytd_select, mtd_select, qtd_select, ytd_select};
pub use returns::{
    clean_returns, comp_sum, comp_total, convert_to_prices, excess_returns, rebase, simple_returns,
};
pub use risk_metrics::{
    cagr, cornish_fisher_var, downside_deviation, estimate_ruin, expected_shortfall, gain_to_pain,
    geometric_mean, kurtosis, mean_return, modified_sharpe, omega_ratio, outlier_loss_ratio,
    outlier_win_ratio, parametric_var, rolling_sharpe, rolling_sortino, rolling_volatility, sharpe,
    skewness, sortino, tail_ratio, value_at_risk, volatility, WasmCagrBasis, WasmRuinDefinition,
    WasmRuinModel,
};
pub use timeseries::{
    aic, arch_lm, bic, fit_egarch11, fit_garch11, fit_gjr_garch11, forecast_garch_fit, hqic,
    ljung_box,
};
