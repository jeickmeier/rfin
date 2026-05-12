//! Return-based, tail-risk, and rolling analytics on simple return series.
//!
//! Start here when you already have a single `&[f64]` return slice and want a
//! scalar or rolling risk/performance metric without constructing
//! [`crate::performance::Performance`].
//!
//! Internally, the implementation is organized by return-based, tail-risk, and
//! rolling metrics. The public surface is the curated re-export list below so
//! callers have one obvious path per metric.
//!
//! Important conventions:
//! - returns are simple decimal returns, such as `0.01` for +1%
//! - annualized means scale linearly with the periods-per-year factor
//! - annualized volatility-like quantities scale with `sqrt(periods_per_year)`
//! - historical VaR and ES are reported in return space and are not time-scaled
//! - `NaN` may represent missing observations for sentinel-returning metrics;
//!   strict `Result` APIs such as [`cagr`] and benchmark multi-factor
//!   regression reject invalid inputs explicitly
//!
//! Drawdown-derived ratios such as Calmar, Martin, Sterling, Burke, Pain, and
//! Recovery Factor live in [`crate::drawdown`]. Benchmark-relative ratios such
//! as Treynor and M-squared live in [`crate::benchmark`].

mod return_based;
mod rolling;
mod tail_risk;

/// Shared guard for invalid annualization factor inputs.
pub(crate) use return_based::invalid_annualization_factor;
pub use return_based::{
    cagr, downside_deviation, gain_to_pain, geometric_mean, mean_return, modified_sharpe,
    omega_ratio, sharpe, sortino, volatility, AnnualizationConvention, CagrBasis,
};
pub use rolling::{
    rolling_sharpe, rolling_sortino, rolling_volatility, DatedSeries, RollingSharpe,
    RollingSortino, RollingVolatility,
};
pub use tail_risk::{
    cornish_fisher_var, expected_shortfall, kurtosis, parametric_var, skewness, tail_ratio,
    value_at_risk,
};
