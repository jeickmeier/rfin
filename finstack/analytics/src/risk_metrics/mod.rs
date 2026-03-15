//! Risk and return metrics: Sharpe, Sortino, Calmar, VaR, ES, and more.
//!
//! Organized into focused submodules:
//! - [`return_based`]: CAGR, mean return, volatility, Sharpe, Sortino, Omega, etc.
//! - [`tail_risk`]: VaR, ES, Cornish-Fisher VaR, skewness, kurtosis, tail ratios.
//! - [`rolling`]: Rolling Sharpe, Sortino, and volatility over sliding windows.
//!
//! Drawdown-derived ratios (Calmar, Ulcer, Martin, Sterling, Burke, Pain,
//! Recovery Factor) live in [`crate::drawdown`]. Benchmark-relative
//! ratios (Treynor, M-squared) live in [`crate::benchmark`].
//!
//! All functions operate on `&[f64]` return slices and return scalar `f64`.

pub mod return_based;
pub mod rolling;
pub mod tail_risk;

pub use return_based::{
    cagr, cagr_from_periods, downside_deviation, gain_to_pain, geometric_mean, mean_return,
    modified_sharpe, omega_ratio, risk_of_ruin, risk_of_ruin_from_returns, sharpe, sortino,
    volatility,
};
pub use rolling::{
    rolling_sharpe, rolling_sharpe_values, rolling_sortino, rolling_sortino_values,
    rolling_volatility, rolling_volatility_values, RollingSharpe, RollingSortino,
    RollingVolatility,
};
pub use tail_risk::{
    cornish_fisher_var, expected_shortfall, expected_shortfall_with_scratch, kurtosis,
    outlier_loss_ratio, outlier_loss_ratio_with_scratch, outlier_win_ratio,
    outlier_win_ratio_with_scratch, parametric_var, skewness, tail_ratio, tail_ratio_with_scratch,
    value_at_risk, value_at_risk_with_scratch,
};
