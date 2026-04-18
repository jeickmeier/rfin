//! Return-based, tail-risk, and rolling analytics on simple return series.
//!
//! Start here when you already have a single `&[f64]` return slice and want a
//! scalar or rolling risk/performance metric without constructing
//! [`crate::performance::Performance`].
//!
//! Submodules are organized by domain:
//! - [`return_based`]: CAGR, mean return, volatility, Sharpe, Sortino, ruin estimation, and other return-level ratios
//! - [`tail_risk`]: VaR, Expected Shortfall, Cornish-Fisher VaR, skewness, kurtosis, and tail-shape metrics
//! - [`rolling`]: rolling Sharpe, Sortino, and volatility over sliding windows
//!
//! Important conventions:
//! - returns are simple decimal returns, such as `0.01` for +1%
//! - annualized means scale linearly with the periods-per-year factor
//! - annualized volatility-like quantities scale with `sqrt(periods_per_year)`
//! - historical VaR and ES are reported in return space and are not time-scaled
//!
//! Drawdown-derived ratios such as Calmar, Martin, Sterling, Burke, Pain, and
//! Recovery Factor live in [`crate::drawdown`]. Benchmark-relative ratios such
//! as Treynor and M-squared live in [`crate::benchmark`].

pub mod return_based;
pub mod rolling;
pub mod tail_risk;

pub use return_based::{
    cagr, cagr_from_periods, downside_deviation, estimate_ruin, gain_to_pain, geometric_mean,
    mean_return, modified_sharpe, omega_ratio, sharpe, sortino, volatility,
    AnnualizationConvention, RuinDefinition, RuinEstimate, RuinModel,
};
pub use rolling::{
    rolling_sharpe, rolling_sharpe_values, rolling_sortino, rolling_sortino_values,
    rolling_volatility, rolling_volatility_values, RollingSharpe, RollingSortino,
    RollingVolatility,
};
pub use tail_risk::{
    cornish_fisher_var, expected_shortfall, kurtosis, moments4, outlier_loss_ratio,
    outlier_win_ratio, parametric_var, skewness, tail_ratio, value_at_risk,
};
pub(crate) use tail_risk::{
    expected_shortfall_with_scratch, tail_ratio_with_scratch, value_at_risk_with_scratch,
};
